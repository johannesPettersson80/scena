# API overview

`scena` exposes a small set of public types that cover the normal 3D
application workflow: create assets, build a scene, prepare renderer resources,
and render frames.

The authoritative API reference is generated on docs.rs:

<https://docs.rs/scena/latest/scena/>

Use this page as the conceptual map.

## GPU resource lifecycle invariant

`prepare()` owns GPU resource creation. Output settings that change the resource
shape, including MSAA and post-processing, invalidate the prepared state and
must be followed by another `prepare()` before `render()`. A successful prepare
creates every retained buffer, texture, render target, pipeline, and bind group;
`render()` neither lazily creates output resources nor changes their reported
inventory. Unsupported sample counts fail from prepare with
`PrepareError::UnsupportedSampleCount`.

Resource-shaped setting changes advance `Renderer::output_resources_revision()`.
Rendering stale output state returns
`NotPreparedReason::OutputSettingsChanged`; it is distinct from a surface or
target resize. Native attached surfaces use present-only rendering by default,
while headless GPU rendering keeps synchronous pixel readback for compatibility.
Call `render_with_readback_mode(..., RenderReadbackMode::PresentOnly)` when a
native render loop explicitly wants no copy/map/wait, or select
`RenderReadbackMode::Synchronous` when `frame_rgba8()` must be current before the
call returns. Managed auto exposure never turns an attached native frame into a
synchronous full-frame readback: it submits a fixed 16x16 surface sample grid
with that frame, polls it non-blockingly, and applies a completed prior-frame
sample on a later render call. `Renderer::auto_exposure_status()` reports
`Pending` until a valid sample is applied and `Converged` afterward. Headless
CPU/GPU rendering keeps deterministic same-call convergence for capture and
reference generation. `Unavailable` means the selected native surface lacks
the copy/format capability required by the bounded meter; choose fixed exposure
on that surface. Browser proof capture remains asynchronous.
After a present-only GPU frame, typed capture returns `CaptureError::NoRenderedFrame`
instead of relabeling stale CPU bytes as a fresh capture.
For ordered multi-frame native capture,
`Renderer::render_batch_with_async_readback(scene, cameras)` alternates two
prepared readback buffers. It submits the next copy/map before resolving the
oldest occupied slot, returns owned `PixelReadback` values in camera-input
order, and leaves the final returned frame in `frame_rgba8()`.

`RendererStats::textures` counts logical texture handles, while
`RendererStats::gpu_textures` counts physical texture allocations owned by the
active prepared GPU resource set. `render_targets` is a classified subset of
those physical textures, not an additional destruction count. Shader modules
report prepare-time compilation inputs and are not retained destruction
objects. Pending destruction therefore counts retained buffers, physical
textures, pipelines, and bind groups exactly once.

`wgpu::PipelineCache` is intentionally deferred as a later backend-portability
decision. It is not used as a substitute for the prepare/render lifecycle: all
required resource creation must already be absent from `render()`.

`Renderer::poll_device()` reports typed completion through `DevicePollStatus`.
Native GPU polling returns `DevicePollStatus::Confirmed` only after the device
confirms completion. Browser backends instead retire scena's logical records as
`DevicePollStatus::Automatic` without claiming completion. In browser WebGPU,
wgpu's device poll is automatic/no-op and the JavaScript WebGPU implementation
owns object lifetime after Rust releases its wrappers, including objects still
referenced by submitted work. WebGL2 uses wgpu's `GlFenceBehavior::AutoFinish`;
GL likewise retains deleted objects that in-flight commands still reference.
This avoids making browser bookkeeping depend on delayed or throttled queue
completion callbacks.
`Automatic` and `Unsupported` therefore never fabricate confirmation, and the
compatibility `gpu_polled` boolean is true only for `Confirmed`.

Additive public API changes in Unreleased:

- `scena photo plan <asset-or-recipe>` and
  `scena photo render <asset-or-recipe>` CLI workflows for bounded
  camera-behavior stills, plus the stable `scena.photo_plan.v1`,
  `scena.photo_candidate_plan.v1`,
  `scena.photo_shaded_candidate_selection.v1`,
  `scena.photo_render_result.v1`, and `scena.photo_report.v1` contracts
  (gated behind the agent/scene-host CLI surface).
- Additive scene-recipe fields for photographic rendering:
  top-level `photo.intent:"camera_behavior"`, `photo.subject`,
  `render.metering`, subject-focus `render.depth_of_field.focus`, and
  `render.exposure_compensation_ev`. Recipes that omit `photo.intent` retain
  the existing explicit render contract: fixed `render.exposure_ev` remains
  full manual exposure, manual `render.depth_of_field.focus_distance` remains
  valid, and explicit `render.metering:{mode:"average"}` stays average
  metering rather than becoming subject metering.
- `FocusReportV1`, `ExposureReportV1`, `SubjectObservationV1`, and their
  nested frame-key/measurement structs. These reports are capture-bound
  evidence and are additive in render introspection and photo reports.
- Migration guidance: for auto-exposed product/studio renders, prefer
  `render.exposure_compensation_ev` to nudge the metered exposure. Keep
  `render.exposure_ev` for intentionally fixed/manual shots; it remains
  mutually exclusive with auto exposure.
- `RecipeValidationModeV1`, `SceneRecipeResourceResolutionV1`,
  `SceneRecipeResourceStatusV1`, and `SceneRecipeDiagnosticResourceV1` for the
  shared validation/build resource-resolution contract
- `Capabilities::render_sample_counts`,
  `Capabilities::depth_sample_counts`, and
  `Capabilities::explicit_msaa`
- `DiagnosticCode::MultisampleFallback`
- `RenderReadbackMode`, `Renderer::render_with_readback_mode`,
  `Renderer::render_batch_with_async_readback`, and
  `Renderer::output_resources_revision`
- `VISUAL_PATCH_SCHEMA_V1` (gated behind `scene-host`)
- `VisualPatchV1`, `VisualPatchTransformV1`, `VisualPatchTintV1`,
  `VisualPatchVisibilityV1`, `VisualPatchTransformEasedV1`,
  `VisualPatchTintEasedV1`, `VisualPatchCameraEasedV1`,
  `VisualPatchAnimationTimeV1`, `VisualPatchAnimationTimeModeV1`,
  `VisualPatchResultV1`, `VisualPatchAppliedCountsV1`,
  `VisualPatchEntryErrorV1`, and
  `VisualPatchRevisionDeltaV1` (gated behind `scene-host`)
- `SceneHostCore::apply_patch` and `SceneHostCore::apply_patch_json`
  (gated behind `scene-host`)
- `HOST_EVENT_SCHEMA_V1`, `HostEventBatchV1`, `HostEventV1`,
  `HostEventHitV1`, `HostEventTargetKindV1`, and
  `HostEventHoverPhaseV1` (gated behind `scene-host`)
- `SceneHostCore::set_event_sink`, `SceneHostCore::clear_event_sink`,
  `SceneHostCore::drain_events`, `SceneHostCore::drain_events_json`,
  `SceneHostCore::hover`, and `SceneHostCore::select` (gated behind
  `scene-host`)
- `RENDER_INTROSPECTION_SCHEMA_V1`, `RenderIntrospectionReportV1`,
  `RenderIntrospectionOptions`, `RenderIntrospectionReasonV1`,
  `RenderIntrospectionFixV1`, `RenderIntrospectionFramingV1`,
  `RenderIntrospectionNodesSummaryV1`, `RenderIntrospectionNodeDetailV1`,
  `RenderIntrospectionArtifactsV1`, and
  `Renderer::introspect_capture` (gated behind `inspection`)
- `RENDER_QUALITY_SCHEMA_V1`, `RenderQualityReportV1`,
  `RenderQualityCheckV1`, `RenderQualitySummaryV1`,
  `RenderQualityRegionV1`, `RenderQualityStatusV1`, `RenderQualityProfile`,
  `RenderQualityFrameMetrics`, `RenderQualityLabelMetrics`,
  `ReferenceQualityMetrics`, `evaluate_render_quality`,
  `evaluate_render_quality_rgba8`, `evaluate_label_region_quality`,
  `frame_metrics`, `label_metrics`, `reference_quality_metrics`, and
  `ssim_grayscale` (gated behind `inspection`)
- `SceneHostCore::render_introspection` and
  `SceneHostCore::render_introspection_json` (gated behind `scene-host`)
- `VISIBILITY_DIAGNOSIS_SCHEMA_V1`, `VisibilityDiagnosisReportV1`,
  `VisibilityDiagnosisOptions`, `VisibilityDiagnosisReasonV1`,
  `VisibilityDiagnosisFixV1`, `VisibilityDiagnosisSummaryV1`,
  `VisibilityDiagnosisTargetV1`, `VisibilityDiagnosisEvidenceV1`, and
  `Renderer::diagnose_visibility` (gated behind `inspection`)
- `VISUAL_REPAIR_PLAN_SCHEMA_V1`, `AGENT_LOOP_RESULT_SCHEMA_V1`,
  `VisualRepairPlanV1`, `VisualRepairActionV1`,
  `VisualRepairSkippedActionV1`, `VisualRepairRemainingReasonV1`, and
  `AgentLoopResultV1` (gated behind `inspection`)
- `APPEARANCE_EXPECTATION_SCHEMA_V1`,
  `APPEARANCE_INTROSPECTION_SCHEMA_V1`, `AppearanceExpectationV1`,
  `AppearanceTargetExpectationV1`, `AppearanceIntrospectionReportV1`,
  `AppearanceIntrospectionOptions`, `AppearanceTargetReportV1`,
  `AppearanceReasonV1`, `AppearanceFixV1`, and
  `Renderer::introspect_appearance` (gated behind `inspection`)
- `SCENE_RECIPE_SCHEMA_V1`, `SCENE_RECIPE_VALIDATION_SCHEMA_V1`,
  `SCENE_RECIPE_BUILD_SCHEMA_V1`, `SceneRecipeV1`,
  `SceneRecipeAlphaModeV1`, `SceneRecipeImportV1`, `SceneRecipeCaptureV1`,
  `SceneRecipeExpectedExtentV1`, `SceneRecipeColorV1`,
  `SceneRecipeGeometryV1`, `SceneRecipeMeshV1`, `SceneRecipePrimitiveV1`,
  `SceneRecipeMaterialV1`, `SceneRecipeTextureSlotV1`,
  `SceneRecipeTextureColorSpaceV1`, `SceneRecipeNodeV1`,
  `SceneRecipeTransformV1`, `SceneRecipeTransformConversionError`,
  `SceneRecipeCameraV1`, `SceneRecipeLightV1`, `SceneRecipeTargetV1`,
  `SceneRecipeBuildV1`, `SceneRecipeBuildImportV1`,
  `SceneRecipeBuildResourceV1`, `SceneRecipeBuildTargetV1`,
  `SceneRecipeValidationReportV1`, `SceneRecipeDiagnosticV1`,
  `validate_scene_recipe_json`, `validate_scene_recipe_json_with_policy`,
  `validate_scene_recipe_value`, `validate_scene_recipe_value_with_policy`,
  `parse_valid_scene_recipe_json`, `parse_valid_scene_recipe_json_with_policy`,
  and `recipe_too_large_report`
- `Transform::try_from(&SceneRecipeTransformV1)` is the shared local transform
  resolver for recipe imports and authored nodes. `raw` validates and
  normalizes `[x,y,z,w]`; `trs` applies degree rotations in intrinsic X, then
  Y, then Z call order. Placement transform variants require scene context and
  return `SceneRecipeTransformConversionError::PlacementRequiresScene` from
  this local-only conversion.
- `SCENE_PLACEMENT_RESULT_SCHEMA_V1`, `ScenePlacementResultV1`,
  `ScenePlacementDiagnosticV1`, `placement_center_transform`,
  `placement_ground_transform`, `placement_fit_to_size_transform`,
  `placement_look_at_transform`, `placement_align_to_feature_transform`, and
  `placement_place_on_feature_transform`
- `SCHEMA_CATALOG_SCHEMA_V1`, `SCHEMA_ENTRY_SCHEMA_V1`,
  `SchemaCatalogV1`, `SchemaCatalogEntryV1`, `SchemaEntryReportV1`,
  `schema_catalog_v1`, `schema_catalog_entry`, `schema_entry_report_v1`,
  and `nearest_schema_name`; `vocabulary_report_v1` / `vocabulary_v1` expose
  closed renderer and recipe vocabularies, and
  `RecipeBuildPolicy::with_allowed_root` adds one operator-owned root without
  removing compiled defaults, while `RecipeBuildPolicy::to_schema_report`
  exposes the effective sandbox, per-root source, and limits used by recipe
  commands
- `SceneRecipePatchResultV1` is the source-digest-bound, recipe-local stable-ID
  placement update returned by `scena place --apply`; it carries a complete
  canonical updated recipe and makes no formatting-preservation promise
- `CameraState`, `CameraBookmark`, `CameraFlyTo`, `CameraTransitionError`,
  `TransitionEasing`, `OrbitControls::camera_state`, and
  `OrbitControls::fly_to`
- `HeadlessGltfViewerBuilder::with_camera_bookmark`,
  `HeadlessGltfViewerBuilder::with_camera_bookmarks`,
  `HeadlessGltfViewer::camera_bookmarks`,
  `FirstRender::camera_bookmarks`,
  `InteractiveGltfViewerBuilder::with_camera_bookmark`,
  `InteractiveGltfViewerBuilder::with_camera_bookmarks`, and
  `InteractiveGltfViewer::camera_bookmarks`
- `ASSET_CATALOG_SCHEMA_V1`, `ASSET_READINESS_REPORT_SCHEMA_V1`,
  `AssetCatalogV1`, `AssetCatalogAssetV1`, and related catalog field types
- `AssetReadinessReportV1`, `AssetReadinessAssetReportV1`,
  `AssetReadinessFindingV1`, `AssetReadinessSeverityV1`, and
  `Assets::validate_asset_catalog`
- `ASSET_DOCTOR_REPORT_SCHEMA_V1`, `AssetDoctorReportV1`,
  `AssetDoctorFindingV1`, `AssetDoctorSeverityV1`,
  `Assets::doctor_asset_path`, `Assets::doctor_loaded_asset`, and
  `SceneHostCore::asset_doctor_json`
- `CONNECTOR_BROWSER_SCHEMA_V1`, `ConnectorBrowserReportV1`,
  `ConnectorBrowserConnectorV1`, `ConnectorBrowserCandidateV1`,
  `SceneHostCore::connector_browser_json`,
  `SceneHostCore::connector_browser_subtree_json`, and
  `SceneHostCore::connector_browser_selection_json`
- `PRODUCT_OPTIONS_SCHEMA_V1`, `ProductOptionsV1`,
  `ProductOptionGroupV1`, `ProductOptionV1`,
  `SceneHostCore::store_product_options`,
  `SceneHostCore::store_product_options_json`,
  `SceneHostCore::product_options`, `SceneHostCore::product_options_json`,
  `SceneHostCore::apply_product_option`, and
  `SceneHostCore::apply_product_option_json` (gated behind `scene-host`)
- `PRESENTATION_TIMELINE_SCHEMA_V1`, `PresentationTimelineV1`,
  `PresentationTimelineActionV1`, `PresentationTimelineActionKindV1`,
  `PresentationTimelineCameraBookmarkV1`,
  `SceneHostCore::timeline_patch`, `SceneHostCore::timeline_patch_json`,
  `SceneHostCore::seek_timeline`, `SceneHostCore::seek_timeline_json`,
  `SceneHostCore::advance_timeline`, and
  `SceneHostCore::advance_timeline_json` (gated behind `scene-host`)
- `SCENE_HOST_GROUNDING_SCHEMA_V1`, `SceneHostGroundingReportV1`,
  `SceneHostGroundingPathV1`, `SceneHostGroundingFallbackV1`,
  `SceneHostCore::apply_product_grounding_preset`, and
  `SceneHostCore::apply_product_grounding_preset_json` (gated behind
  `scene-host`)
- `render_asset_catalog_preview_png`, `AssetCatalogPreviewPng`,
  `AssetCatalogPreviewError`,
  `HeadlessGltfViewerBuilder::with_background_color`, and
  `InteractiveGltfViewerBuilder::with_background_color`
- `MeasurementOverlay`, `MeasurementKind`, `MeasurementAxis`,
  `MeasurementReport`, `MeasurementOverlayReport`, `UnitFormat`, and
  `Scene::add_measurement_overlay`
- `SceneHostMeasurementOverlayReportV1` and
  `SceneHostMeasurementAuthorityV1`; the authority metadata declares that
  overlay values are uncalibrated scene-space inspection output, names the
  source-unit/world-transform/`f32` assumptions, and explicitly rejects
  manufacturing-tolerance, survey-accuracy, and calibrated-metrology claims.
- `LabelMetrics`, `LabelDesc::metrics`, `LabelDesc::background`,
  `LabelDesc::halo`, `LabelDesc::with_background`,
  `LabelDesc::without_background`, `LabelDesc::with_halo`, and
  `LabelDesc::without_halo`; `LabelDesc::new()` renders through the
  embedded TrueType atlas path.
- `Scene::isolate`, `Scene::show_only`, `Scene::hide`, `Scene::show`,
  `Scene::toggle_visibility`, `Scene::ghost`, `Scene::restore_visibility`,
  `Scene::restore_visibility_with_report`,
  `Scene::restore_tints`, `Scene::fit_selection_with_assets`,
  `Scene::add_bounding_box_overlay`, `Scene::add_world_axes_triad`,
  `Scene::add_local_axes_triad`, `Scene::inspection_toolkit_report`,
  `SceneVisibilitySnapshot`, `SceneVisibilityRestoreReport`, `SceneTintSnapshot`,
  `InspectionHelperKind`,
  `InspectionHelperReport`, and `InspectionToolkitReport`
- `ExplodedView`, `ExplodedViewPlan`, `ExplodedTransformUpdate`, and
  `ExplodedView::from_node(...).transforms(...)` for reversible
  presentation-only assembly exploded views. Radial and axis directions are
  world-space directions. Hierarchy-depth mode assigns each target an absolute
  world-space offset proportional to its depth, then solves locals against the
  already planned final parent world so ancestor displacement is applied
  exactly once. `ExplodedTransformUpdate::original` is the exact local value
  used for idempotent restore.
- `SceneHostCore::exploded_view_patch`,
  `SceneHostCore::exploded_view_patch_json`,
  `SceneHostExplodedViewOptionsV1`, and `SceneHostExplodedViewModeV1` for
  emitting existing visual-patch transform channels from stable host handles;
  SceneHost JSON patches include
  `metadata.scena_exploded_view_restore_patch`, an immediate-transform
  `VisualPatchV1` for restoring the pre-exploded local transforms (gated behind
  `scene-host`)
- `SCENE_HOST_VISUAL_STATE_SCHEMA_V1`,
  `SCENE_HOST_VISUAL_STATES_SCHEMA_V1`, `SceneHostVisualStateV1`,
  `SceneHostVisualStateSummaryV1`, `SceneHostVisualStatesReportV1`,
  `SceneHostCore::store_visual_state`,
  `SceneHostCore::store_visual_state_json`,
  `SceneHostCore::visual_state`, `SceneHostCore::visual_state_json`,
  `SceneHostCore::visual_states`, `SceneHostCore::visual_states_json`,
  `SceneHostCore::apply_visual_state`, and
  `SceneHostCore::apply_visual_state_json` for host-named visual patch
  presets (gated behind `scene-host`)
- `SceneHostCore::set_camera_bookmark` and
  `SceneHostCore::set_camera_bookmark_json` (gated behind `scene-host`)
- `SCENE_HOST_GIZMO_DRAG_SCHEMA_V1`, `SceneHostGizmoDragV1`,
  `SceneHostGizmoModeV1`, `SceneHostGizmoAxisV1`,
  `SceneHostGizmoSpaceV1`, `SceneHostGizmoConstraintV1`,
  `SceneHostGizmoRayV1`, `SceneHostCore::apply_gizmo_drag`, and
  `SceneHostCore::apply_gizmo_drag_json` for applying caller-supplied gizmo
  rays through the existing visual-patch transform channel (gated behind
  `scene-host`)
- `ScenaViewerAnnotationLayoutOptions`,
  `ScenaViewerAnnotationLayoutInput`,
  `ScenaViewerAnnotationLayoutReport`,
  `ScenaViewerAnnotationLayoutEntry`, and
  `layout_scena_viewer_annotations` for deterministic custom-element
  annotation clamping and decluttering reports
- The `scena` binary with `schema list`, `schema get <schema>`,
  `policy recipe [--allow-root <directory>]...`,
  `validate-recipe <recipe.json> [--full|--syntax-only] [--allow-root
  <directory>]...`, `place <recipe.json> (--import <id>|--node <id>)
  --verb <center|ground|fit_to_size|look_at|align_to_anchor|place_on>`,
  `recipe build <recipe.json> [--max-imports <n>] [--allow-root
  <directory>]...`, `recipe render <recipe.json> --verify --out
  <png> [--allow-root <directory>]...`,
  `recipe inspect-cad <recipe.json> --out-dir <dir>`,
  `recipe capture <recipe.json> --out-dir <dir> [--views
  front,top,right,isometric|none] [--turntable <frames>] [--clip <name>
  --frames <n>]`, `recipe aov <recipe.json> --out-dir <dir> [--passes
  id,depth,normal]`, `diff <before.recipe.json> <after.recipe.json>
  [--numeric-tolerance <n>] [--render --out-dir <dir>] [--exit-code]`,
  `examples agent list`, `examples agent get <template> [--out <dir>]`, and,
  when built with `inspection`, asset-or-recipe-input
  `render`, `inspect`, `diagnose --visibility`, and
  `repair --from <report.json>`, and
  `verify appearance --expect <appearance-expectation.json>` JSON commands
- Global and per-command `--help`/`-h` are successful
  `scena.cli_help.v1` stdout reports. Diff inequality exits 0 unless the caller
  explicitly selects `--exit-code` CI semantics.
- Asset-or-recipe commands parse the input kind once. Raw glTF/GLB uses the
  direct asset loader, while every recipe is resolved through the same
  `RecipeBuildPolicy` and SceneHost build manifest as `recipe build`; no CLI
  adapter constructs a scene from only `imports[0]`. Recipe build rejection
  emits `scena.recipe_build_result.v1` with a nonzero exit.
- Recipe-aware validation, build, render, inspect, diagnose, doctor, and repair
  accept the same repeatable `--allow-root <directory>` option. Each directory
  is canonicalized before policy construction; direct asset inputs reject the
  option because it governs authored recipe references. Successful and
  structured recipe-failure results expose the effective top-level `policy`.
- `scena browser-proof [scene-host|m6] [--backend webgl2] [--dry-run]`
  for a machine-readable wrapper over the wasm-pack + Playwright browser lanes;
  the M6 lane rebuilds its browser-probe package before running Playwright
- `SCENE_HOST_SEMANTIC_AOV_SCHEMA_V1`, `SceneHostSemanticAovCaptureV1`,
  `SceneHostSemanticAovLegendEntryV1`,
  `SceneHostSemanticAovExclusionsV1`,
  `SceneHostCore::capture_semantic_aovs`,
  `SceneHostCore::capture_semantic_aovs_gpu`,
  `SceneHostCore::set_semantic_aov_capture_enabled`, and `palette_rgba8` for
  deterministic CPU or opt-in GPU ID/depth/world-normal output with
  runtime-scoped host identity (gated behind `scene-host`). WASM exposes
  `setSemanticAovCaptureEnabled` and async `captureSemanticAovs` with typed ID,
  depth, normal, and RGBA arrays on WebGPU and WebGL2.
- `SceneRecipeBuildInstanceV1` and additive
  `SceneRecipeBuildV1::instances` rows mapping recipe-local authored instance IDs
  to runtime-scoped set/instance identity

Additive public API changes in 1.7.0:

- `Transform`, `Aabb`, `Color`, `GeometryTopology`, capability enums, and
  capability report structs now serialize through stable serde shapes.
- `CAPABILITY_REPORT_SCHEMA_V1`
- `CapabilityReportV1`
- `CapabilityReport::to_schema_report`
- `CapabilityReport::to_schema_json`
- `SCENE_INSPECTION_SCHEMA_V1` (gated behind `inspection`)
- `SceneInspectionReportV1`, `SceneNodeInspectionV1`,
  `SceneDrawInspectionV1`, `SceneCameraFrustumInspectionV1`,
  `SceneNormalInspectionV1`, `SceneInspectionCountsV1`, and
  `SceneInspectionRevisionsV1` (gated behind `inspection`)
- `SceneInspectionReport::to_schema_report`
- `SceneInspectionReport::to_schema_report_with_node_handles`
- `SceneInspectionReport::to_schema_json`
- `SceneInspectionReportV1::node_by_handle`
- `SceneInspectionReportV1::children_of`
- `SceneInspectionReportV1::roots`
- `SceneInspectionReportV1::find_by_tag`
- `Transform::compose`
- `impl Mul<Transform> for Transform`
- `Assets::load_scene_from_bytes`
- `Scene::instantiate_under`
- `Scene::set_transforms`
- `Scene::set_node_tint`
- `Scene::node_tint`
- `Scene::set_annotation_anchor`
- `Scene::clear_annotation_anchor`
- `Scene::annotation_projection_report`
- `Scene::add_callout`
- `Scene::clear_callout`
- `Scene::world_distance`
- `Scene::node_world_bounds`
- `Scene::set_authored_node_bounds`
- `Scene::node_local_bounds`
- `Node::tint`
- `Scene::remove_node`
- `Scene::remove_import`
- `Scene::remove_tag`

Transform mutation invariant:

- every public scene transform insertion or mutation rejects non-finite translation,
  rotation, or scale components with `LookupError::InvalidTransform` before changing scene
  state or a revision;
- `Scene::set_transforms` and SceneHost transform batches preflight the complete batch before
  applying any node or instance-root update, and a rejected batch does not cancel active
  transitions;
- orbit pointer/touch events containing non-finite positions or deltas are no-ops, and the
  next finite event can continue the gesture;
- orbit pan follows camera view-right/view-up at the current yaw and pitch, matching the
  established drag signs at yaw zero instead of using fixed world X/Y axes.
- `SceneAsset::primitive_count`
- `SceneAsset::bounds`
- `SceneAsset::geometry_summary`
- `ASSET_LOAD_REPORT_SCHEMA_V1`
- `AssetLoadReportV1`, `AssetLoadWarningV1`, `AssetLoadProgressV1`,
  `AssetExternalResourceV1`, and `AssetMaterialFallbackV1`
- `AssetLoadWarningV1::ComputedFlatNormals` and
  `AssetLoadWarningV1::SkinInfluencesTruncated` expose geometry computation and
  four-influence degradation instead of leaving importer decisions implicit
- `AssetMaterialSource` and `AssetMaterialSourceKind`
- `AssetLoadReport<SceneAsset>::to_schema_report`
- `AssetLoadReport<SceneAsset>::to_schema_json`
- `AssetLoadReport::options` and `AssetLoadReport::cache_entry_options` expose
  active-request policy and the provenance of compatible cache evidence
- `AssetLoadOptions::with_strict_external_resources` for referenced buffers and
  `AssetLoadOptions::with_strict_textures` for referenced images, plus
  `AssetLoadOptions::with_fetch_byte_limit` for the combined source-byte budget
- `AssetProvenance` and `AssetDerivative`
- `SceneAsset::provenance`
- `TextureDesc::provenance`
- `EnvironmentDesc::provenance`
- `SceneMaterialInspectionV1`, `SceneMaterialSourceInspectionV1`, and
  `SceneMaterialSlotInspectionV1`
- `AnnotationAnchor`, `AnnotationAnchorTarget`,
  `AnnotationProjectionReportV1`, `AnnotationProjectionV1`,
  `SCENE_ANNOTATION_PROJECTION_SCHEMA_V1`,
  `SceneAssetGeometrySummary`, and
  `ASSET_GEOMETRY_SUMMARY_SCHEMA_V1`
- `Callout`, `CalloutAnchor`, `CalloutAnchorKind`, and `CalloutReport`
- `SceneHostCore` (gated behind `scene-host`)
- `SceneHostError` and `SceneHostErrorCode` (gated behind `scene-host`)
- `SceneHostCameraState` (gated behind `scene-host`)
- `SCENE_HOST_ASSET_IMPORT_SCHEMA_V1` and
  `SceneHostAssetImportReportV1` (gated behind `scene-host`)
- `SCENE_HOST_SUBTREE_SCHEMA_V1`, `SceneHostSubtreeReportV1`, and
  `SceneHostSubtreeNodeV1` (gated behind `scene-host`)
- `SCENE_HOST_ANIMATION_INVENTORY_SCHEMA_V1`,
  `SceneHostAnimationInventoryV1`, `SceneHostAnimationClipV1`,
  `SceneHostAnimationPlayOptions`, `SceneHostAnimationLoopMode`, and
  `SceneHostEasing` (gated behind `scene-host`)
- `RendererStats::gpu_draw_submissions` and `RendererStats::instances`
- `AntiAliasing`, `PostBloomConfig`,
  `ScreenSpaceAmbientOcclusionConfig`, `Renderer::set_anti_aliasing`,
  `Renderer::set_bloom`, and `Renderer::set_screen_space_ambient_occlusion`
- `CAPTURE_SCHEMA_V1`
- `CAPTURE_BASELINE_SCHEMA_V1`
- `capture_rgba8`
- `Renderer::capture_rgba8`
- `Renderer::capture_png_bytes`
- `Renderer::capture_png`
- `FirstRender::capture`
- `HeadlessGltfViewer::capture`
- `InteractiveGltfViewer::capture`
- `CaptureRgba8::to_png_bytes`
- `CaptureRgba8::write_png`
- `capture_contact_sheet_rgba8`
- `compare_captures_with_tolerance`
- `CaptureDescriptor`, `CaptureRgba8`, `CaptureOptions`,
  `CaptureRevisions`, `CaptureCamera`, `CaptureProjection`,
  `CaptureViewport`, `CapturePayload`, `CapturePayloadKind`,
  `CaptureAutoFrame`, `CaptureAutoFrameViewport`, `CapturePoint2`,
  `CaptureScreenRect`, `CapturePixelSummary`, `CapturePixelBounds`, and
  `CaptureError`
- `CapturePngError`, `CaptureContactSheet`, `CaptureContactSheetTile`,
  `CaptureContactSheetError`, `CaptureBaselineReport`,
  `CaptureBaselineDiff`, `CaptureBaselineTolerance`, and
  `CaptureBaselineError`
- `fnv1a64_hex`, `sample_rgba8`, `summarize_rgba8`,
  `summarize_pixel_readback`, and `auto_frame_metadata`

For a complete agent/self-verification build, select the opt-in `agent`
feature. It enables `scene-host`, and `scene-host` already enables
`inspection`; default builds remain feature-empty. Select either lower-level
feature directly only when its smaller owner surface is intentional.

The `scene-host` feature also exports a WASM `SceneHost` wrapper on
`wasm32`. Its node, import, instance-root, and animation handles are opaque
kind-tagged `u64` values owned by the host and kept within JavaScript's exact
integer range. A node handle has the same value across construction, transform
updates, picking, and `inspectJson()` output; different handle kinds are not
interchangeable. Phase 3 also exports real `capture()` /
`captureJson()` and `capturePng()` methods, plus WebGPU-safe `captureAsync()`,
`captureJsonAsync()`, and `capturePngAsync()` methods, that return
`scena.capture.v1` metadata for the latest rendered RGBA8 frame; these are not
placeholders. Use the async family when the browser backend is WebGPU because
mapped GPU-buffer readback cannot complete synchronously.
Capture descriptors are bound to the renderer's last rendered scene/camera
state and fail with `CaptureError::StaleRender` if the scene is mutated before
capture. PNG helpers delegate to `CaptureRgba8`, so viewer, renderer,
SceneHost, and browser captures use the same descriptor-bound byte path.
`renderIntrospectionJson(detail)` and the WebGPU-safe
`renderIntrospectionJsonAsync(detail)` return `scena.render_introspection.v1`
over the same browser capture readback path, so agent/browser hosts can fail closed
on empty, offscreen, or culled frames without inventing a JavaScript-only
visibility report.
Browser hosts can also call `handleSurfaceContextLost(recoverable)` and
`handleSurfaceContextRestored()` from real browser context lifecycle signals to
emit the same `scena.host_event.v1` context events as native `SurfaceEvent`
handling.
Phase 4 adds real `removeNode` and `removeImport` host methods. Removed node
handles are invalidated in the host table, so later use returns
`SceneHostErrorCode::StaleNodeHandle` rather than aliasing a recycled node.
Phase 4 also adds per-node tint/highlight render state. Native callers use
`Scene::set_node_tint(node, Some(color))` or `None` to clear it; browser hosts
use `setNodeTint` and `clearNodeTint`. Tint is node-owned render state, not a
material clone, and `scena.scene_inspection.v1` reports it as
`nodes[].tint`.
Phase 4 also adds engine-owned annotation projection and geometry helpers.
Native callers can store `AnnotationAnchor::node` / `AnnotationAnchor::world`
anchors and call `Scene::annotation_projection_report` for schema
`scena.annotation_projection.v1`. Browser hosts use `setNodeAnnotation`,
`setWorldAnnotation`, `clearAnnotation`, and `annotationProjectionsJson()`,
which returns CSS-pixel projection coordinates and the same host `node_handle`
for node anchors that `setTransforms`, `inspectJson`, and `pick` use. World
anchors report `node_handle: null`. `SceneAsset::geometry_summary` returns
Callouts compose those same annotation anchors with leader-line geometry and a
screen-aligned label. Native callers use `Callout::node`, `Callout::world`,
`Callout::anchor`, or `Callout::connector` with `Scene::add_callout()`;
SceneHost/WASM callers use `add_node_callout` / `add_world_callout` or
`addNodeCallout()` / `addWorldCallout()` for stable-handle node/world helpers.
The returned `anchor_id` is the annotation ID reported by
`annotation_projections_json()` and remains compatible with the 0.1C `labels`
visual-patch channel; there is no parallel host text-update model.

Generated overlay ownership invariant:

- measurement line/label nodes and callout leader/label nodes have exactly one
  scene-owned overlay owner;
- removing either generated child with `Scene::remove_node` removes the
  complete owned overlay closure atomically, including its sibling, registry
  entry, and any callout annotation, while leaving the anchor target intact;
- SceneHost invalidates every handle in that closure, so subsequent use of
  either generated child returns `SceneHostErrorCode::StaleNodeHandle`;
- use `clear_measurement_overlay` or `clear_callout` when the overlay ID is the
  natural removal key; these operations enforce the same ownership invariant.

Measurement values are computed from current world-space `f32` positions after
the import policy converts declared source units into scene meters. Formatting
can convert those scene meters to millimeters or caller-labeled units, but it
does not add calibration. Snapping is a separate connector/placement operation,
and overlay visibility or occlusion is presentation behavior rather than a
metrology guarantee. Treat these values as inspection aids, not authoritative
manufacturing, survey, tolerance, or safety measurements.

Labels use an embedded TrueType font by default with `LabelDesc::new`, or an
explicit TrueType/OpenType face with `LabelFontFace::from_truetype_bytes`,
`LabelDesc::truetype`, or recipe `fonts[]` plus label `font`. Labels support
basic Latin metrics, kerning, glyph shapes, and renderer-owned antialiasing
coverage through the label atlas path. Complex-script text fails closed instead
of rendering fallback garbage. Explicit label text, background, and halo colors
are opaque-only; omit the background/halo for no quad instead of passing
translucent user colors.

Recipe-authored skin and morph data deform vertex positions through the same
prepare path used by imported glTF deformation data. Lighting normals remain the
source/geometric normals for morph targets, and skinned normals use the joint
direction transform rather than an inverse-transpose normal matrix. That means
non-uniform joint scale and morph-normal deformation are not lighting-correctness
guarantees in the current renderer.
Browser custom-element annotations use the same screen-projection data but
perform HTML layout in CSS-pixel space. Native/browser hosts can call
`layout_scena_viewer_annotations()` with
`ScenaViewerAnnotationLayoutOptions` and
`ScenaViewerAnnotationLayoutInput` to get a deterministic report with each
annotation's original position, clamped position, visibility, and
`hidden_reason` (`hidden`, `behind_camera`, `occluded`, or `overlap`). The
`<scena-viewer>` element exposes the same report as
`scena-viewer-annotations-rendered.detail.layout_report` after
`setAnnotationProjections(...)`.
`SceneAsset::geometry_summary` returns
schema `scena.asset_geometry_summary.v1` with node/mesh/primitive counts,
asset-local bounds, and source metadata where the asset stores it.
Phase 5 adds stable asset-load reports. Native callers use
`AssetLoadReport<SceneAsset>::to_schema_json()` for
`scena.asset_load_report.v1`; browser hosts can call
`instantiateUrlWithReportJson` or `instantiateUrlUnderWithReportJson` to get
the created import handle plus the same asset-load report. Cache-hit reports
preserve typed warnings and external resource counts from the original load.
For asset-picker and component-library workflows, the host can build a
`scena.asset_catalog.v1` manifest and pass it to
`Assets::validate_asset_catalog()`. The returned
`scena.asset_readiness_report.v1` keeps catalog/search ownership in the host
while Scena validates renderer-relevant readiness: fetchable sources and
required files, explicit units and source coordinate systems, finite bounds and
scale limits, authored anchors/connectors/tags, declared material variants,
base-color texture requirements, external-resource warnings, and material
fallback provenance. Findings include stable severity, code, message, help,
path, and field values so agents can act on the report without parsing prose.
Release 1.7 adds explicit host-owned instanced imports. Native callers use
`SceneHostCore::instantiate_url_instanced` or
`SceneHostCore::instantiate_url_instanced_under`; browser hosts use
`instantiateUrlInstanced` or `instantiateUrlInstancedUnder`. Each returned
handle is an instance-root handle, not a scene node. The standard transform,
visibility, tint, remove, and pick APIs accept these handles. Other node-tree
APIs continue to reject them with structured host errors. Per-instance tint is
opaque-only in this release.
Release 1.7 also adds post-processing controls, world-space stroke rendering,
SceneHost animation playback, and presentation transitions. Browser and native
hosts explicitly call `advance(delta_seconds)` once per frame to step active
animation mixers and transition fades; `SceneHost` does not own the host
application's render loop. Eased transforms and tints are renderer presentation
smoothing for low-rate visual updates, not simulation, physics, robotics, or
process-control behavior.
Release 1.7 subtree reports use schema `scena.subtree.v1`; node `name` is
reserved for future stable naming policy and is always `null` in 1.7. Use
stable host handles and sorted `tags` for identification. The same report also
includes `parent` and direct `children` handle fields so hosts can build a
part-tree UI without reparsing the full scene inspection report.
SceneHost asset-import reports and host-backed scene inspection reports expose
declared material variant names plus the current active variant, using the same
stable import handles accepted by the 0.1C `material_variants` visual-patch
channel.
The stable contract surface also includes generic `AssetProvenance` metadata.
Loaded `SceneAsset`,
`TextureDesc`, and `EnvironmentDesc` values expose `provenance()` with a
serde-stable source path, optional source SHA-256, optional license/generator,
and generated derivatives. `scena.asset_load_report.v1` and
`scena.asset_geometry_summary.v1` include the same provenance value. Existing
environment source accessors continue to delegate to the same provenance
record.
Asset-aware scene inspection also reports material evidence without exposing raw
asset handles. `SceneMaterialInspectionV1` names the source kind
(`source_material`, `generated_default`, `user_created`, or `unknown`), source
asset path/material index when known, texture provenance rows, and material
fallback rows such as optional Basis/KTX2 texture fallbacks.
SceneHost includes interactive camera state without giving the host a render
loop. Native code can call `SceneHostCore::set_camera`,
`SceneHostCore::get_camera`, `SceneHostCore::camera_json`,
`camera_pointer_down`, `camera_pointer_move`, `camera_pointer_up`, and
`camera_wheel`. The WASM facade exposes the corresponding `setCamera`,
`setCameraJson`, `getCameraJson`, `cameraPointerDown`, `cameraPointerMove`,
`cameraPointerUp`, and `cameraWheel` methods.
The visual patch contract accepts batched host-owned visual deltas through
`SceneHostCore::apply_patch`, `SceneHostCore::apply_patch_json`, and WASM
`applyPatch`. The `scena.visual_patch.v1` envelope supports immediate
transforms, tints, visibility, camera state, eased transform/tint/camera
targets, explicit animation mixer time changes, programmatic selection/hover,
material variants, host-owned label anchors, and optional echoed metadata. It
returns changed counts, per-entry failures, and revision deltas.
The host event contract reports renderer-to-host observations through
`SceneHostCore::set_event_sink`, `drain_events`, `drain_events_json`, and WASM
`drainEventsJson`. `scena.host_event.v1` batches include pick, hover,
selection, load, diagnostic, capture, surface, context, device, and capability
events using the same stable `u64` handles as inspection and visual patches.
Pick and hover coordinates are CSS pixels; physical dimensions are named
explicitly. Native event sinks are push-only: while a sink is registered,
events are delivered to it and are not queued for later drains.

Runnable SceneHost examples:

```bash
cargo run --example scene_host_contracts --features scene-host
cargo run --example scene_host_release_1_7 --features scene-host
```

`examples/scene_host_release_1_7.rs` is the compact native 1.7 surface sample:
post-processing setters, instanced import, visibility/tint, camera preset
framing, animation inventory/play/pause/advance, and eased transform/tint
updates. `examples/scene_host_browser_contracts.js` shows the matching WASM
method names: `setAntiAliasing`, `setBloom`, `setAmbientOcclusion`,
`instantiateUrlInstancedUnder`, `setVisible`, `setNodeTint`,
`animationInventoryJson`, `playAnimation`, `pauseAnimation`, `advance`,
`setTransformEased`, `setTransformsEasedTyped`, `setNodeTintEased`, and
`applyPatch`.
Golden JSON fixtures for the shipped v1 reports live under
`tests/assets/stable-contracts/`.

Additive public API changes in 1.2.0:

- `AssetLoadOptions`
- `Assets::load_scene_with_options`
- `Assets::load_scene_with_report_options`
- `DiagnosticCode::MaterialTextureMissingDecodedPixels`
- `DiagnosticContext`
- `RendererStats::material_textures_missing_decoded_pixels`

Additive public API changes in 1.3.0:

- `Scene::frame_bounds`
- `Scene::frame_all_with_overlays`
- `SceneHostCore::frame_all_with_overlays` and browser `frameAllWithOverlays`
- `FramingOptions`
- `FramingOptions::azimuth_elevation`
- `FramingOptions::front`
- `FramingOptions::back`
- `FramingOptions::left`
- `FramingOptions::right`
- `FramingOptions::top`
- `FramingOptions::bottom`
- `FramingOptions::three_quarter_front_left`
- `FramingOptions::three_quarter_front_right`
- `FramingOptions::three_quarter_back_left`
- `FramingOptions::three_quarter_back_right`
- `FramingOutcome`
- `ScreenRect`
- `ProjectedPoint`
- `Scene::project_world_point`
- `Scene::bounds_for_transforms`
- `Scene::add_grid_floor`
- `GridFloorOptions`
- `GridFloorHandles`
- `Aabb::union`
- `OrbitControls::focus_on_framing`
- `OrbitControls::from_framing`
- `Scene::add_studio_lighting`
- `Renderer::set_auto_exposure`
- `AutoExposureConfig`
- `AutoExposureMeteringDomain`
- `AutoExposureResult`
- `AutoExposureStatus`

Additive public API changes in 1.4.0:

Named primitives — "write a name, not a number":

`GeometryDesc::cylinder` and `GeometryDesc::cone` emit seam-safe side UVs: the
closing side vertex is duplicated at `u=1`, and cone tips retain face-local UVs.
This changes generated cylinder vertex counts but preserves cap topology and
prevents the last side quad from sampling backward across the texture.

- `Color::TRANSPARENT`, `Color::BLACK`, `Color::WHITE`, `Color::GRAY`,
  `Color::LIGHT_GRAY`, `Color::DARK_GRAY`, `Color::CHARCOAL`,
  `Color::STUDIO_BACKDROP`, `Color::WARM_WHITE`, `Color::COOL_WHITE`,
  `Color::RED`, `Color::GREEN`, `Color::BLUE`, `Color::ORANGE`,
  `Color::YELLOW`, `Color::CYAN`, `Color::MAGENTA`
- `Color::from_hex`
- `Color::from_kelvin`
- `PerspectiveCamera::wide_angle`
- `PerspectiveCamera::standard`
- `PerspectiveCamera::portrait`
- `PerspectiveCamera::telephoto`
- `PerspectiveCamera::with_fov_degrees`
- `Transform::looking_at`
- `DirectionalLight::sun`
- `DirectionalLight::key_light`
- `DirectionalLight::fill_light`
- `DirectionalLight::rim_light`
- `PointLight::softbox`
- `PointLight::bulb_warm`
- `PointLight::bulb_cool`
- `MaterialDesc::matte`
- `MaterialDesc::plastic`
- `MaterialDesc::metal`
- `MaterialDesc::rubber`
- `Background` (enum: `Studio`, `DarkStudio`, `NeutralGray`, `White`,
  `Black`, `Sky`, `Transparent`, `Custom(Color)`)
- `Renderer::set_background`
- `OrbitControls::cinematic`
- `OrbitControls::snappy`
- `OrbitControls::presentation`
- `OrbitControls::turntable`
- `OrbitControls::zoom_limits_bounds_relative`
- `OrbitControls::with_distance_limits`
- `AutoExposureConfig::product_studio`
- `AutoExposureConfig::indoor`
- `AutoExposureConfig::outdoor`
- `AutoExposureConfig::mixed`

Bundled content + one-call helpers:

- `EnvironmentPreset`, `EnvironmentPresetMetadata`,
  `Assets::load_environment_preset`
- `KhronosSample`, `KhronosSamples`, `KhronosSampleMetadata`,
  `Assets::khronos`
- `Scene::play_animation_by_name`
- `HeadlessGltfViewer::play_clip`,
  `InteractiveGltfViewer::play_clip`
- `Scene::add_perspective_camera_default_for`
- `ConnectOptions::with_axial_gap`
- `Scene::preview_connector_magnet`, `ConnectionMagnetPreview`,
  `ConnectionMagnetVisualCue`

Viewer ergonomics — pointer callbacks, screenshots, hot reload, URL state:

- `InteractiveGltfViewer::on_click`,
  `InteractiveGltfViewer::on_hover`,
  `InteractiveGltfViewer::clear_click_callback`,
  `InteractiveGltfViewer::clear_hover_callback`,
  `InteractiveGltfViewer::click_at`,
  `InteractiveGltfViewer::hover_at`,
  `InteractiveGltfViewer::pick_at`,
  `InteractiveGltfViewer::pick_and_select_at`,
  `InteractiveGltfViewer::pick_and_hover_at`
- `HeadlessGltfViewer::capture_png_bytes`,
  `HeadlessGltfViewer::capture_png`,
  `InteractiveGltfViewer::capture_png_bytes`,
  `InteractiveGltfViewer::capture_png`,
  `FirstRender::capture_png_bytes`,
  `FirstRender::capture_png`,
  `HeadlessGltfViewerBuilder::render_png_bytes`,
  `HeadlessGltfViewerBuilder::render_png`,
  `ViewerCaptureError`, `ViewerPngError`
- `Assets::watch_scene_for_hot_reload`,
  `Assets::reload_scene`,
  `Assets::reload_scene_with_report`
- `AssetReloadError` (`path`, underlying `AssetError`, and
  `previous_asset_preserved` failure evidence)
- `AssetHotReloadWatcher`, `AssetHotReloadError`
  (the filesystem watcher is gated behind the `hot-reload` feature; explicit
  reload is part of the base asset API)
- `CameraOrbitUrlState`
- `FollowControls`, `FlyControls`
- `ReferenceImage::from_rgba8`, `ReferenceImage::regress`,
  `ReferenceImage::regress_with_tolerance`

`<scena-viewer>` custom element (browser):

- `defineScenaViewer()`
- `ScenaViewerDropDecision`, `ScenaViewerVariantSelection`,
  `ScenaViewerInspectorSnapshot`, `ScenaViewerProgress`,
  `ScenaViewerProgressPhase`, `ScenaViewerAccessibilityDefaults`,
  `ScenaViewerKeyboardAction`, `ScenaViewerGestureAction`,
  `ScenaViewerAnnotationAnchor`

Renderer features:

- `Renderer::set_bloom`
- `Renderer::clear_bloom`
- `PostBloomConfig`
- `Renderer::set_anti_aliasing`
- `Renderer::set_cpu_occlusion_culling`
- `Renderer::set_supersample_factor`
- `Renderer::set_reconstruction_filter`
- `AntiAliasing`
- `ReconstructionFilter`
- `Renderer::set_screen_space_ambient_occlusion`
- `Renderer::clear_screen_space_ambient_occlusion`
- `ScreenSpaceAmbientOcclusionConfig`
- `Renderer::set_order_independent_transparency`
- `Renderer::clear_order_independent_transparency`
- `OrderIndependentTransparencyConfig`
- `MaterialDesc::with_clearcoat_factor`,
  `MaterialDesc::with_clearcoat_roughness_factor`,
  `MaterialDesc::with_clearcoat_texture`,
  `MaterialDesc::with_clearcoat_roughness_texture`,
  `MaterialDesc::with_clearcoat_normal_texture`,
  `MaterialDesc::clearcoat_factor`,
  `MaterialDesc::clearcoat_roughness_factor`,
  `MaterialDesc::clearcoat_texture`,
  `MaterialDesc::clearcoat_roughness_texture`,
  `MaterialDesc::clearcoat_normal_texture`,
  `MaterialDesc::clearcoat_normal_scale`
- `MaterialDesc` sheen / anisotropy / iridescence / dispersion /
  transmission / IOR / volume builders and accessors
- `OutputColorSpace`,
  `RendererOptions::with_output_color_space`,
  `Capabilities::wide_gamut_output`,
  `DiagnosticCode::WideGamutOutputUnavailable`
- `GltfExtensionDiagnostic::suggested_fix`
- `RendererStats::ambient_occlusion_passes`
- `RendererStats::order_independent_transparency_passes`
- `RendererStats::bloom_passes`

Additive public API changes in 1.5.0:

- `MaterialDesc::rough_metal`
- `MaterialDesc::chrome`
- `MaterialDesc::brushed_steel`
- `MaterialDesc::clearcoat_plastic`
- `MaterialDesc::satin`
- `MaterialDesc::leather`
- `MaterialDesc::clear_glass`
- `MaterialDesc::frosted_glass`

## Core types

Most applications can start with `use scena::prelude::*;`. The curated prelude
contains stable everyday scene, asset, material, controls, and renderer types;
versioned JSON/report contracts remain explicit root or owner-module imports so
schema-heavy wildcard imports do not pollute application code.

| Type | Role |
|---|---|
| `Scene` | Owns graph state: nodes, transforms, cameras, lights, renderables, labels, imports, animations, picking targets, and dirty state. |
| `Assets` | Owns logical resources: geometry, materials, textures, environments, parsed glTF/GLB assets, cache identity, reload, and retain policy. |
| `Renderer` | Owns rendering state: backend resources, prepared scene data, surfaces, targets, stats, diagnostics, capability reports, and frame output. |
| `SceneImport` | Represents an instantiated imported asset with roots, names, paths, anchors, connectors, bounds, clips, and stale-import checks. |
| `Transform` | Stores TRS state. `with_scale(Vec3)` and `with_uniform_scale(f32)` replace scale; `scale_by(f32)` multiplies the current scale, matching the compositional `rotate_*_deg` helpers. |

The common pattern is:

```rust
let mut assets = scena::Assets::new();
let asset = assets.load_scene("model.glb")?;

let mut scene = scena::Scene::new();
let import = scene.instantiate(&asset)?;
let bounds = import.bounds_world(&scene).ok_or("model has no bounds")?;
scene.add_studio_lighting()?;
scene.add_grid_floor(&assets, scena::GridFloorOptions::new().under_bounds(bounds))?;

let camera = scene.add_perspective_camera(
    scene.root(),
    scena::PerspectiveCamera::standard(),
    scena::Transform::default(),
)?;
let framing = scene.frame_bounds(
    camera,
    bounds,
    scena::FramingOptions::new()
        .three_quarter_front_right()
        .fill(0.72)
        .viewport(1280, 720),
)?;
let controls = scena::OrbitControls::from_framing(framing);

let mut renderer = scena::Renderer::headless(1280, 720)?;
renderer.set_auto_exposure(scena::AutoExposureConfig::default());
renderer.prepare_with_assets(&mut scene, &assets)?;
renderer.render(&scene, camera)?;
```

See the exact signatures on docs.rs and the runnable examples in `examples/`.

Application-generated textures use checked, path-free descriptors with a
stable application identity. Use a slot-typed constructor/loader to avoid
silently treating data maps as sRGB (or color maps as linear):

```rust
let pixels = vec![255, 0, 0, 255, 0, 255, 0, 255];
let texture = assets.create_texture(
    scena::TextureMemoryDesc::rgba8_for_slot(
        scena::TextureMemoryId::new("ui/status-strip")?,
        2,
        1,
        pixels,
        scena::TextureSlot::BaseColor,
    )
    .with_mip_policy(scena::TextureMipPolicy::Generate),
)?;
```

`TextureMemoryDesc::linear_rgba32f` accepts finite linear HDR values and stores
them as filterable `Rgba16Float`; values outside the finite half-float range
fail instead of saturating. `TextureMemoryId` is immutable cache identity:
identical content deduplicates, while changed pixels/options under the same ID
return `AssetError::TextureIdentityCollision`. `Assets::load_texture_for_slot`
applies the same slot color-space contract to path-backed images.

Native decoded images and generated textures share explicit dimension and
allocation limits. Limit failures return `AssetError::TextureSizeLimit` with
actual dimensions/bytes and configured maxima. On browsers, a source above the
WebGL2-safe limit is still resized, but the decision is also available as
`AssetLoadWarning::TextureDownscaled` through scene-load reports and
`Assets::texture_warnings()`; console output is not the only signal.

`Scene::frame_all_with_options` frames bounds stored directly in the scene;
`Scene::frame_all_with_assets_and_options` also resolves geometry owned by
`Assets`. Both frame aggregate visible world bounds using an explicit target
viewport:

```rust
let framing = scene.frame_all_with_assets_and_options(
    camera,
    &assets,
    FramingOptions::new()
        .three_quarter_front_right()
        .fill(0.72)
        .margin_px(24.0)
        .tighten_depth_range(true)
        .include_helpers(false)
        .viewport(output_width, output_height),
)?;
```

`Scene::frame_import_with_options` applies the same contract to one import.
`Scene::frame_node_with_options` and
`Scene::frame_node_with_assets_and_options` apply it to one visible subtree.
Hidden nodes are excluded, and inspection-helper geometry is excluded unless
`FramingOptions::include_helpers(true)` is explicit. Legacy `Scene::frame_all`
`Scene::frame_import`, `Scene::frame_node`, and their `with_assets` convenience
forms retain their no-options signatures and derive a viewport from the
camera's current aspect; high-level viewers instead use the option-bearing
forms with actual output dimensions. `frame_all_with_overlays` remains a
purpose-specific wrapper because it derives pixel margin from label metrics;
it is not a second general framing model.

`Scene::move_origin_to` moves a node origin to a world-space point.
`Scene::center_visible_bounds_on` translates the node so the center of its
visible, non-helper subtree reaches that point. `Scene::center_on` is deprecated
because its name did not reveal that it moved only the origin.

Fallible geometry construction is the default for runtime data. Use
`GeometryDesc::try_polyline(&points)`, which returns
`GeometryError::PolylineTooShort { point_count }` for zero or one point without
unwinding. `GeometryDesc::polyline` remains only as a deprecated compatibility
wrapper for fixed, trusted point literals; new code should not use it.

Transform builder order is explicit:

```rust
let scaled = Transform::IDENTITY
    .with_scale(Vec3::new(2.0, 3.0, 4.0))
    .scale_by(0.5); // [1.0, 1.5, 2.0]

let replaced = scaled.with_uniform_scale(3.0); // [3.0, 3.0, 3.0]
```

`Transform::scale_by` preserves translation and rotation;
`Transform::with_scale` and `Transform::with_uniform_scale` replace scale. In
published v1.8.0, `scale_by` replaced scale despite its compositional name; use
`with_uniform_scale` when migrating code that intentionally depended on that
old behavior.

## Typed handles

`scena` uses typed handles instead of raw integers or string identifiers for
renderer-owned objects.

Examples include:

- `NodeKey`
- `CameraKey`
- `GeometryHandle`
- `MaterialHandle`
- `TextureHandle`
- `EnvironmentHandle`
- `AnimationMixerKey`
- `InstanceSetKey`
- `HitTarget`

Typed handles make wrong-kind usage visible at compile time. Missing or stale
handles return structured errors.

## Scene construction

`Scene` is the place for graph state:

- node hierarchy,
- transforms,
- cameras,
- lights,
- renderable instances,
- labels and helper geometry,
- imported asset instances,
- animation mixers,
- picking state,
- visibility and layers.

Scene builders return typed keys or handles. Hosts keep application-specific
state in their own model and map the visible portion into `Scene`.

`SceneHostCore` treats physical size as authoritative for `SurfaceEvent::Resize`
and retains the current DPR; `ScaleFactorChanged` retains physical size and
recomputes logical size. Either event order therefore converges on the same
viewport. `ViewportChanged` and `resize(logical_width, logical_height, dpr)` are
the explicit all-fields forms. A zero/minimized physical resize is forwarded to
the renderer and emitted to the host, but does not replace the last valid
non-zero picking viewport. Every valid path updates perspective aspect through
`Scene::set_camera` before rendering, picking, or emitting resize metadata.

Common animation calls:

- `Scene::play_animation_by_name`
- `Scene::update_animation`
- `Scene::set_animation_loop_mode`
- `Scene::set_animation_speed`

Caller-authored source clips should use `AnimationSourceClip::try_new` and
`try_rebind`. Both return `AnimationError::InvalidClip` with a channel/keyframe
path when duration, time ordering, interpolation shape, output type/width, or
finite-value validation fails. The legacy unchecked `new` and panic-on-error
`rebind` wrappers remain only for source compatibility and are deprecated.
Imported glTF clips pass through the same channel validator before rebinding.
Mixer sampling also rejects a non-finite computed transform, so even finite but
overflowing cubic tangents cannot poison scene state.

Viewer helpers also expose `play_clip(name)` for the loaded import. The
returned mixer key is still scene-owned, so hosts explicitly drive update,
loop, speed, prepare, and render.

## Asset ownership

`Assets` owns resource creation and loading:

- primitive geometry,
- materials,
- textures,
- environments,
- glTF/GLB scene assets,
- cache and reload state,
- external asset fetching.

The renderer does not fetch or parse assets during `render()`. Load and decode
assets before preparation.

## Renderer lifecycle

`Renderer` has an explicit lifecycle:

1. Build or mutate `Scene` and `Assets`.
2. Call `prepare()` or `prepare_with_assets()`.
3. Call `render()` or `render_active()`.
4. If scene, assets, surface, target, environment, or renderer settings change,
   call `prepare()` again.

This keeps fallible work visible to the host and makes frame rendering
predictable.

Common renderer calls:

- `Renderer::headless`
- `Renderer::headless_gpu`
- `Renderer::from_surface`
- `Renderer::prepare`
- `Renderer::prepare_with_assets`
- `Renderer::render`
- `Renderer::render_active`
- `Renderer::capability_report`
- `Renderer::gpu_adapter_report`

GPU construction names are literal. `Renderer::headless_gpu` and
`SceneHostCore::headless_gpu[_with_fetcher]` are strict and propagate adapter or
device failure. `SceneHostCore::headless_prefer_gpu[_with_fetcher]` is the
explicit fallback-capable contract and returns `HeadlessBackendSelectionReport`
with requested/selected backends and the original GPU `BuildError`. The
high-level viewer mirrors this split with `with_headless_gpu` versus
`with_headless_prefer_gpu`; preferred construction exposes the same report.
Recipe hosts mirror it with `build_recipe_json_gpu` versus
`build_recipe_json_prefer_gpu`; the latter exposes the selection report through
`SceneHostRecipeBuild::backend_selection_report` without changing the result
type's public field shape. Release proof must use the strict forms.

Common scene interaction calls:

- `Scene::pick_with_assets`
- `Scene::pick_and_select_with_assets`
- `Scene::connect_import_connectors`
- `Scene::frame_bounds`
- `Scene::project_world_point`
- `Scene::bounds_for_transforms`
- `Scene::add_grid_floor`
- `Scene::add_studio_lighting`
- `Scene::with_default_camera()`

### Picking result semantics

`Scene::pick_with_assets` intersects the same current vertex pose used by
render preparation: morph targets are evaluated first, skinning second, and
node plus instance transforms last. It never silently falls back to base
vertices when a skin binding is missing or invalid.

`Hit::distance` is measured in world-space units along the normalized camera
ray. `Hit::world_position` is the corresponding world-space intersection.
`Hit::normal` is the normalized geometric normal derived from transformed
triangle winding, so negative scale reverses it and nonuniform scale changes it
according to the transformed face. A singular transform is permitted as scene
state, but any triangle it collapses to zero area is not hittable. Invalid
deformation data returns `LookupError::InvalidSkinBinding`.

Static triangle geometry owns a clone-shared, deterministic model-space BVH.
Picking inverse-transforms one world ray into mesh/instance local space, rejects
the mesh bounds first, and transforms only candidate triangles for final
world-space distance and winding-normal truth. Nonuniform and negative scale
are supported; singular transforms fail closed. Morph/skin poses never reuse
the static BVH: they rebuild from the current deformed positions. CPU shadow
prepare builds one world-space BVH from the current transformed/deformed
occluders, so transform, instance, morph, skin, and light changes cannot reuse
stale visibility. Repeated indexed corners share a prepare-scoped visibility
cache keyed by the exact deformed world-position bits plus deterministic light
and occluder-state signatures; the cache is discarded before the next prepare.
`PickingMetrics` and `PrepareWorkMetrics` separate BVH bounds work, cache
hits/misses, and exact ray/triangle tests for scaling evidence.
Prepare metrics also report GPU shader-module creations and triangle-shader
cache hits/misses, nonblocking/blocking prepare polls, and the retained resource
work attributed to the current prepare. `RenderWorkMetrics` reports native
scene-color passes and queue submissions, final CPU output encodes, row-bin
candidate work, and the one-time primitive-flag scan. These are deterministic
work counters; wall-clock performance claims still require a controlled
adapter-specific distribution.

Common public event and output types:

- `SurfaceEvent`
- `PostBloomConfig`
- `RendererStats`
- `CapabilityReport`
- `GpuAdapterReport`
- `AdapterLimitsReport`
- `AssetEvictionStats`
- `AssetStoreId`
- `ReferenceImage`
- `ReferenceImageReport`
- `ReferenceImageTolerance`

Common asset-store calls:

- `Assets::store_id()`
- `Assets::load_scene_with_options()`
- `Assets::load_scene_with_report_options()`
- `Assets::contains_geometry`
- `Assets::contains_material`
- `Assets::contains_texture`
- `Assets::contains_environment`
- `Assets::release_unreferenced`

Common import and connector contracts:

- `SceneImport`
- `AnchorKey`
- `ConnectorKey`
- `AnchorFrame`
- `ConnectorFrame`
- `ConnectorMetadata`
- `ConnectionAlignment`
- `ConnectionRoll`
- `ConnectionLineOverlay`
- `ConnectionMagnetPreview`
- `ConnectionMagnetVisualCue`
- `ConnectorRollPolicy`
- `ConnectorPolarity`
- `ConnectorBrowserReportV1`

Common viewer helpers:

- `FramingOptions`
- `FramingOutcome`
- `GridFloorOptions`
- `GridFloorHandles`
- `TransformGizmo`
- `GizmoMode`
- `GizmoAxis`
- `GizmoConstraint`
- `GizmoSpace`
- `GizmoRay`
- `ViewerProfile`
- `InteractiveGltfViewer`
- `InteractiveGltfViewerBuilder`
- `interactive_gltf_viewer(path, surface)`
- `InteractiveGltfViewer::handle_surface_event`
- `HeadlessGltfViewerBuilder::build_with_progress`
- `HeadlessGltfViewerBuilder::render_png_bytes`
- `InteractiveGltfViewerBuilder::build_with_progress`
- `AssetLoadProgress`
- `HeadlessGltfViewer::set_active_material_variant`
- `InteractiveGltfViewer::set_active_material_variant`
- `SceneHostCore::material_variants`
- `SceneHostCore::active_material_variant`
- `SceneHostCore::set_active_material_variant`
- `Renderer::headless_default()`
- `Renderer::set_auto_exposure`

Viewer profiles are named builder presets for common application shapes:
`ViewerProfile::model_viewer()`, `cad_inspection()`, `product()`,
`industrial()`, and `documentation()`. Apply them with
`with_viewer_profile(profile)` on headless or interactive glTF viewer
builders. A profile configures existing renderer profile/render mode,
background, environment, lighting, grid, picking styles, and optional orbit
controls; it does not create a separate viewer engine or own the host event
loop.

Without a profile, `headless_gltf_viewer`, `interactive_gltf_viewer`, and
`first_render_gltf_headless` still promise a neutral first presentation. They
preserve authored light/environment choices; otherwise they add a directional
fallback and studio background. Read `FirstRender::diagnostics()` or viewer
`diagnostics()` for the structured `MissingLightingOrEnvironment` warning,
including `Diagnostic::setting()` and `Diagnostic::fallback_applied()`. Use
`without_default_lighting()` for an intentional diagnostic opt-out. Direct
`Renderer` constructors remain explicit and retain the black/no-light
low-level contract.

Transform gizmos are platform-neutral manipulation helpers. Build a
`TransformGizmo` with a `GizmoMode`, optional `GizmoConstraint`, and
`GizmoSpace`; pass caller-derived `GizmoRay` values to `drag_transform(...)`;
then apply the returned `Transform` directly to a `Scene` or emit a
`VisualPatchV1` with `to_visual_patch(...)` when using SceneHost. Gizmo helper
visuals are ordinary line-stroke scene nodes, so they stay renderer-owned and
do not add undo/redo, snapping, collision, or document-model behavior.
SceneHost browser/native hosts can also call `apply_gizmo_drag_json(...)` /
`applyGizmoDragJson(...)` with `scena.scene_host_gizmo_drag.v1`; the helper
computes one drag transform from caller-supplied rays and returns the normal
`scena.visual_patch.v1` result JSON.

Common visual-regression helpers:

- `ReferenceImage::from_rgba8`
- `regress`
- `regress_with_tolerance`

## Errors and diagnostics

Public failures use structured errors such as:

- `BuildError`
- `AssetError`
- `ImportError`
- `InstantiateError`
- `LookupError`
- `PrepareError`
- `PrepareError::GpuDeviceRebuildRequired` distinguishes terminal Device/Queue
  loss from recoverable context loss; rebuild the renderer before preparing
  retained scene/assets
- `RenderError`
- `AnimationError`
- `ConnectionError`
- `ColorParseError`
- `ReferenceImageError`
- `ViewerCaptureError`
- `ViewerPngError`
- `CapturePngError`
- `CaptureContactSheet`
- `CaptureBaselineReport`

Most errors include a stable category plus contextual data. `BuildError`,
`AssetError`, `ImportError`, `InstantiateError`, `PrepareError`, `RenderError`,
`LookupError`, `AnimationError`, and top-level `Error` expose `.help()` plus
`.diagnostic() -> ErrorDiagnostic { code, message, help, context }`; `Display`
remains concise. Use pattern matching for application behavior and the
structured adapter for user-facing recovery. Named `LookupError` variants and `AnimationError::ClipNotFound` carry
up to three normalized, deterministically ranked `candidates`;
`SceneHostError::candidates()` preserves them through host and JSON conversion.
`RenderError::NoActiveCamera` names both `Scene::add_default_camera` and
`Scene::set_active_camera`, including after conversion to `SceneHostError`.

glTF extension diagnostics from `SceneAsset::extension_diagnostics()` also
include `suggested_fix()` and `decoder_policy()` so importer and asset-review
UIs can show the same actionable remediation used by the asset doctor.

## Stats and capabilities

`Renderer` exposes runtime information for:

- backend capability reports,
- GPU adapter reports,
- renderer statistics,
- resource and frame counters.

Use capability reports when selecting optional effects or platform-specific
paths. Use stats for testing, diagnostics, and performance visibility.
Use `scena capabilities --json` for an explicitly static, no-device planning
report before a renderer exists. `scena capabilities --live --json` strictly
requests a headless GPU and reports measured adapter/device identity, limits,
features, target formats, usable sample counts, and readback constraints. A
live failure remains `scena.capability_report.v1` on stdout with exit status 1
and a structured reason; it never falls back to a static row while claiming a
probe. `Renderer::live_capability_probe` exposes the same live metadata for an
already-created GPU renderer.
Native CPU rendering bounds deterministic parallel work to eight workers and
also respects Rayon's process-level `RAYON_NUM_THREADS` setting. Work invoked
from an existing Rayon worker and all WASM rendering remain serial to avoid
nested oversubscription. `RenderWorkMetrics` reports the selected CPU worker
count, row-binned triangle candidates versus the former full-band rescan count,
and retained-bin capacity growth. Environment bake metrics similarly report
eligible face/row tasks and the bounded worker count; output ordering and float
bits are identical to the one-worker path.
`RendererStats::draw_calls` and `RendererStats::primitives` are deprecated
aliases of `RendererStats::triangles` and retain their historical triangle-count
meaning until the next schema version. Use `RendererStats::gpu_draw_submissions`
for the actual number of GPU draw/draw-indexed/draw-instanced calls submitted
last frame, and `RendererStats::instances` for the number of visible per-instance
records drawn last frame.
`RendererStats::surface_timeout_skips` and
`RendererStats::surface_occluded_skips` distinguish non-submitted diagnostic
surface skips. `surface_reconfigurations` and `surface_acquire_retries` expose
recoverable native surface churn instead of hiding it behind `Ok(...)`.
`Capabilities::wide_gamut_output` is intentionally capability-gated: headless
and unattached reports stay disabled, attached browser reports stay degraded
until the browser smoke probe records Display P3 canvas support for the active
backend.

## Where to go next

- [Getting started](getting-started.md)
- [Rendering](rendering.md)
- [Assets](assets.md)
- [Lifecycle](lifecycle.md)
- [Errors](errors.md)
- [Capabilities](capabilities.md)
