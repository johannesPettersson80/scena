# API overview

`scena` exposes a small set of public types that cover the normal 3D
application workflow: create assets, build a scene, prepare renderer resources,
and render frames.

The authoritative API reference is generated on docs.rs:

<https://docs.rs/scena/latest/scena/>

Use this page as the conceptual map.

Additive public API changes in Unreleased:

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
  `HostEventHitV1`, `HostEventTargetKindV1`, `HostEventButtonV1`,
  `HostEventModifiersV1`, and `HostEventHoverPhaseV1` (gated behind
  `scene-host`)
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
- `VISIBILITY_DIAGNOSIS_SCHEMA_V1`, `VisibilityDiagnosisReportV1`,
  `VisibilityDiagnosisOptions`, `VisibilityDiagnosisReasonV1`,
  `VisibilityDiagnosisFixV1`, `VisibilityDiagnosisSummaryV1`,
  `VisibilityDiagnosisTargetV1`, `VisibilityDiagnosisEvidenceV1`, and
  `Renderer::diagnose_visibility` (gated behind `inspection`)
- `SCHEMA_CATALOG_SCHEMA_V1`, `SCHEMA_ENTRY_SCHEMA_V1`,
  `SchemaCatalogV1`, `SchemaCatalogEntryV1`, `SchemaEntryReportV1`,
  `schema_catalog_v1`, `schema_catalog_entry`, `schema_entry_report_v1`,
  and `nearest_schema_name`
- The `scena` binary with `schema list`, `schema get <schema>`, and, when
  built with `inspection`, asset-input `render --introspect`, `inspect`, and
  `diagnose --visibility` JSON commands

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
- `Scene::world_distance`
- `Scene::node_world_bounds`
- `Node::tint`
- `Scene::remove_node`
- `Scene::remove_import`
- `Scene::remove_tag`
- `SceneAsset::primitive_count`
- `SceneAsset::bounds`
- `SceneAsset::geometry_summary`
- `ASSET_LOAD_REPORT_SCHEMA_V1`
- `AssetLoadReportV1`, `AssetLoadWarningV1`, `AssetLoadProgressV1`,
  `AssetExternalResourceV1`, and `AssetMaterialFallbackV1`
- `AssetMaterialSource` and `AssetMaterialSourceKind`
- `AssetLoadReport<SceneAsset>::to_schema_report`
- `AssetLoadReport<SceneAsset>::to_schema_json`
- `AssetLoadOptions::with_strict_external_resources`
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

The `scene-host` feature also exports a WASM `SceneHost` wrapper on
`wasm32`. Its node handles are opaque `u64` values owned by the host. The same
handle values are used for construction, transform updates, picking, and
`inspectJson()` output. Phase 3 also exports real `capture()` /
`captureJson()` and `capturePng()` methods that return `scena.capture.v1`
metadata for the latest rendered RGBA8 frame; these are not placeholders.
Capture descriptors are bound to the renderer's last rendered scene/camera
state and fail with `CaptureError::StaleRender` if the scene is mutated before
capture. PNG helpers delegate to `CaptureRgba8`, so viewer, renderer,
SceneHost, and browser captures use the same descriptor-bound byte path.
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
schema `scena.asset_geometry_summary.v1` with node/mesh/primitive counts,
asset-local bounds, and source metadata where the asset stores it.
Phase 5 adds stable asset-load reports. Native callers use
`AssetLoadReport<SceneAsset>::to_schema_json()` for
`scena.asset_load_report.v1`; browser hosts can call
`instantiateUrlWithReportJson` or `instantiateUrlUnderWithReportJson` to get
the created import handle plus the same asset-load report. Cache-hit reports
preserve typed warnings and external resource counts from the original load.
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
stable host handles and sorted `tags` for identification.
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
- `AutoExposureResult`

Additive public API changes in 1.4.0:

Named primitives — "write a name, not a number":

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
  `AssetHotReloadWatcher`, `AssetHotReloadError`
  (gated behind the `hot-reload` feature)
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
- `AntiAliasing`
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

| Type | Role |
|---|---|
| `Scene` | Owns graph state: nodes, transforms, cameras, lights, renderables, labels, imports, animations, picking targets, and dirty state. |
| `Assets` | Owns logical resources: geometry, materials, textures, environments, parsed glTF/GLB assets, cache identity, reload, and retain policy. |
| `Renderer` | Owns rendering state: backend resources, prepared scene data, surfaces, targets, stats, diagnostics, capability reports, and frame output. |
| `SceneImport` | Represents an instantiated imported asset with roots, names, paths, anchors, connectors, bounds, clips, and stale-import checks. |

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

Common animation calls:

- `Scene::play_animation_by_name`
- `Scene::update_animation`
- `Scene::set_animation_loop_mode`
- `Scene::set_animation_speed`

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
- `Renderer::set_debug`
- `Renderer::set_debug_overlay`
- `Renderer::capability_report`
- `Renderer::gpu_adapter_report`

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

Common public event and output types:

- `SurfaceEvent`
- `DebugOverlay`
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

Common viewer helpers:

- `FramingOptions`
- `FramingOutcome`
- `GridFloorOptions`
- `GridFloorHandles`
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
- `Renderer::headless_default()`
- `Renderer::set_auto_exposure`

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

Most errors include a stable category plus contextual data. Use pattern matching
for application behavior and `.help()` or diagnostics output for user-facing
messages.

glTF extension diagnostics from `SceneAsset::extension_diagnostics()` also
include `suggested_fix()` and `decoder_policy()` so importer and asset-review
UIs can show the same actionable remediation used by the asset doctor.

## Stats and capabilities

`Renderer` exposes runtime information for:

- backend capability reports,
- GPU adapter reports,
- renderer statistics,
- debug overlays,
- resource and frame counters.

Use capability reports when selecting optional effects or platform-specific
paths. Use stats for testing, diagnostics, and performance visibility.
`RendererStats::draw_calls` and `RendererStats::primitives` are deprecated
aliases of `RendererStats::triangles` and retain their historical triangle-count
meaning until the next schema version. Use `RendererStats::gpu_draw_submissions`
for the actual number of GPU draw/draw-indexed/draw-instanced calls submitted
last frame, and `RendererStats::instances` for the number of visible per-instance
records drawn last frame.
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
