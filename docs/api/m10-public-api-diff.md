# M10 Public API Diff From M5 Baseline

Status: active compatibility evidence

## Added or expanded public contracts

- `Renderer::diagnose_scene` returns structured diagnostics.
- `AssetLoadControl` exposes explicit load/cancellation policy.
- `AssetError::UnsupportedTextureFormat` reports unsupported texture input.
- The FXAA pass attached to renderer output is checked by
  `ARCH-FXAA-OUTPUT`.
- `pub struct Diagnostic` includes camera/scene findings such as
  `InvalidCameraProjection`, `ObjectsBehindCamera`,
  `SceneOutsideCameraFrustum`, `LargeScenePrecisionRisk`, and
  `DepthPrecisionRisk`. A far/near ratio greater than the supported precision
  envelope participates in Large-scene precision diagnostics owned by
  `ARCH-DIAGNOSTICS`.
- `pub struct RendererStats` reports `shadow_maps`, `depth_prepass_passes`,
  `depth_prepass_draws`, `ambient_occlusion_passes`, `fxaa_passes`,
  `live_logical_handles`, `pub buffers: u64`, and `pub target_height: u32`.
- `pub struct DevicePoll` exposes explicit device-work completion state.

Stats count logical `TextureHandle` values only where the field documents
logical handles; backend allocations remain separately accounted.

## Semver Decision

The M10 surface is additive relative to the recorded M5 baseline. Any change to
existing signatures, enum exhaustiveness, schema meaning, or handle semantics
must be classified again against the exact API-freeze artifact before release.
