# Public API contract

Status: active public-surface index

The crate's public API is owned by focused modules and re-exported from
`src/lib.rs`. `Scene` owns scene-graph state, `Assets` owns loading and decoded
asset state, and `Renderer` consumes prepared state through the explicit
prepare/render lifecycle. Platform code adapts host surfaces and does not own
renderer policy.

Capabilities are inspectable rather than inferred from backend names. In
particular, `pub reversed_z_depth: CapabilityStatus` is reported through
`Capabilities::reversed_z_depth`. Unsupported or conditional behavior remains
typed and diagnostic; it must not silently become a success claim.

Typed handles are runtime identities, not persistent interchange IDs. Public
errors and diagnostics carry structured variants/codes. Compatibility changes
must update the API-freeze artifact, examples, schema catalog where relevant,
and versioned release notes.

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
`DepthPrecisionRisk`; a far/near ratio greater than the supported precision
envelope participates in the depth warning.

`pub struct RendererStats` exposes `shadow_maps`, `depth_prepass_passes`,
`depth_prepass_draws`, `ambient_occlusion_passes`, `fxaa_passes`,
`live_logical_handles`, `pub buffers: u64`, `pub gpu_textures: u64`, and
`pub target_height: u32`. `textures` counts logical handles;
`gpu_textures` counts physical allocations, and `render_targets` classifies a
subset of those allocations. `pub struct DevicePoll` reports `DevicePollStatus::Automatic`,
`Unsupported`, `Submitted`, or `Confirmed`; pending destructions retire only on
confirmed backend completion. A prepared resource inventory is complete before
render, and render may not allocate a missing output resource.
