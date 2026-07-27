# Public API contract

Status: active public-surface index

The crate's public API is owned by focused modules and re-exported from
`src/lib.rs`. `Scene` owns scene-graph state, `Assets` owns loading and decoded
asset state, and `Renderer` consumes prepared state through the explicit
prepare/render lifecycle. Platform code adapts host surfaces and does not own
renderer policy.

Capabilities are inspectable rather than inferred from backend names. In
particular, `pub reversed_z_depth: CapabilityStatus` is reported through
`Capabilities::reversed_z_depth`. Renderer-owned attachment support is reported
by `Capabilities::render_sample_counts` and
`Capabilities::depth_sample_counts`; `Capabilities::explicit_msaa` states
whether an exact caller-selected multisample count can be honored. Unsupported
or conditional behavior remains typed and diagnostic; it must not silently
become a success claim.

Typed handles are runtime identities, not persistent interchange IDs. Public
errors and diagnostics carry structured variants/codes. Compatibility changes
must update the API-freeze artifact, examples, schema catalog where relevant,
and versioned release notes.

`SceneRecipeV1` is a versioned interchange/build input, not the canonical host
document model. Its caller IDs are recipe-local stable correlation keys rather
than application-persistence identities. Hosts own durable identity,
migrations, undo/history, and preservation of domain or extension data.

Recipe validation distinguishes `RecipeValidationModeV1::SyntaxOnly` from
`FullResolution`. Full reports expose `SceneRecipeResourceResolutionV1` rows
and resource-attached diagnostics, and use the same policy resolution plan that
gates SceneHost recipe build. Syntax-only reports never claim execution
equivalence.

`Transform::with_scale(Vec3)` and `Transform::with_uniform_scale(f32)` are
explicit replacement builders. `Transform::scale_by(f32)` composes
multiplicatively with the current scale and preserves translation/rotation,
matching the composition vocabulary of `rotate_x_deg`, `rotate_y_deg`, and
`rotate_z_deg`. Code relying on the v1.8.0 replacement behavior migrates to
`with_uniform_scale`.

`Scene::move_origin_to(NodeKey, Vec3)` is the explicit node-origin alignment
operation. `Scene::center_visible_bounds_on(NodeKey, &Assets<_>, Vec3)` centers
visible non-helper subtree bounds; the ambiguous `Scene::center_on` spelling is
deprecated. `Scene::frame_all_with_options`,
`Scene::frame_all_with_assets_and_options`, and
`Scene::frame_import_with_options` extend the existing `FramingOptions`
contract to aggregate bounds without introducing a second framing model.
`FramingOptions::include_helpers` defaults to false, while viewport, view
direction/preset, fill, margin, and depth tightening remain explicit. High-level
viewer and SceneHost paths must pass their actual target dimensions rather than
reusing a camera's previous aspect.

`GeometryDesc::try_polyline(&[Vec3]) -> Result<GeometryDesc, GeometryError>` is
the preferred public constructor for runtime and untrusted point lists. The
panicking `GeometryDesc::polyline` method is a deprecated compatibility wrapper
retained for semver; recipe construction and repository examples use the
fallible owner.

## Prepare, depth, and large-world controls

The asset-aware entry point is `pub fn prepare_with_assets<F>`. Prepared depth
work is visible through `pub depth_prepass_passes: u64` and
`pub depth_prepass_draws: u64`; M2 also prepares a depth pre-pass rather than
maintaining a separate milestone renderer path.

`pub fn set_origin_shift` provides explicit large-world rendering support.
Clipping planes use `pub struct ClippingPlaneKey`, retain the half-space
`dot(normal, position) + distance >= 0`, and return `ClippingPlaneNotFound` for
a stale key.

## Capabilities, diagnostics, and statistics

`Capabilities::diagnostics()` can report
`DiagnosticCode::WebGl2DepthCompatibility`. `pub struct Diagnostic` includes
`InvalidCameraProjection`, `ObjectsBehindCamera`,
`SceneOutsideCameraFrustum`, `LargeScenePrecisionRisk`, and
`DepthPrecisionRisk`; `Capabilities::subject_visible_mask` reports whether the
active composition path can produce exact subject-pixel masks from semantic AOV
attribution; `Capabilities::auto_exposure_metering_*` reports each public
metering mode separately so recipe authors can distinguish active metering
support from accepted-but-not-yet-routed contracts;
`AutoExposureMeteringDomain` distinguishes strict scene-linear pre-tonemap
metering from degraded encoded-output feedback metering; a far/near ratio greater than
the supported precision envelope participates in the depth warning.

`pub struct RendererStats` exposes `shadow_maps`, `depth_prepass_passes`,
`depth_prepass_draws`, `ambient_occlusion_passes`, `fxaa_passes`,
`live_logical_handles`, `pub buffers: u64`, `pub gpu_textures: u64`, and
`pub target_height: u32`. `textures` counts logical handles;
`gpu_textures` counts physical allocations, and `render_targets` classifies a
subset of those allocations. `pub struct DevicePoll` reports `DevicePollStatus::Automatic`,
`Unsupported`, `Submitted`, or `Confirmed`; pending destructions retire only on
confirmed backend completion on native GPU and browser WebGPU. WebGL2 reports
`Automatic` after wgpu retires its logical queue records under
`GlFenceBehavior::AutoFinish`; this does not claim physical GPU completion,
because GL owns the lifetime of deleted objects still used by in-flight work.
A prepared resource inventory is complete before render, and render may not
allocate a missing output resource.
