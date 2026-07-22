# M2 lighting, depth, and clipping acceptance

Status: active evidence index

## Depth and camera

- [x] Reversed-Z support is capability-reported and guarded by
  `ARCH-REVERSED-Z`.
- [x] `Capabilities::diagnostics()` emits
  `DiagnosticCode::WebGl2DepthCompatibility` for WebGL2 depth compatibility warnings;
  doctor owns the boundary as `ARCH-WEBGL2-DEPTH`.
- [x] Camera depth-range and depth-fit helpers include `DepthRange::fit_sphere`.

## Clipping and large worlds

- [x] `pub struct ClippingPlaneKey` identifies an authored `ClippingPlane`.
- [x] The retained half-space is `dot(normal, position) + distance >= 0` and a
  stale key returns `ClippingPlaneNotFound`; doctor owns this as `ARCH-CLIPPING`.
- [x] `pub fn set_origin_shift` provides explicit large-world camera-relative
  rendering or origin-shift support under `ARCH-ORIGIN-SHIFT`.

## Lighting and environment

- [x] One opt-in shadowed directional light is enabled with
  `with_shadows(true)` and uses a Single shadow map with PCF 3x3 under
  `ARCH-SHADOW-MAP`.
- [x] The renderer owns a Depth pre-pass under `ARCH-DEPTH-PREPASS`.
- [x] Equirectangular HDR environment loading records `EnvironmentSourceKind`
  and performs Cubemap conversion before IBL preparation (`ARCH-ENV-IBL-PREP`).
- [x] `direct_lights_tint_pbr_mesh_output` proves direct light contribution;
  doctor owns it as `ARCH-DIRECT-LIGHT-SHADING`.
- [x] Camera-relative rendering or origin-shift support is registered by
  `ARCH-ORIGIN-SHIFT`.
- [x] The FXAA pass attached to renderer output is registered by
  `ARCH-FXAA-OUTPUT`.
- [x] Large-scene precision diagnostics are registered by `ARCH-DIAGNOSTICS`.
- [x] Required release proof includes
  `m2_resource_counters_return_to_baseline_after_empty_prepare` under
  `ARCH-M2-LEAK-STATS`.

## Rendered-output proof

- [x] `m2_headless_visual_artifacts_cover_lighting_depth_and_clipping` writes
  the `m2-headless-core.toml` companion registered by
  `VISUAL-M2-FIXTURE-METADATA`.
- [x] Direct light, receiver shadow, IBL, AA, bloom, SSAO, weighted OIT, and
  clipping each render an explicit off/on pair with a declared spatial mask.
  Shadow must visibly darken its receiver, IBL must visibly alter PBR material
  response, and the remaining effects must meet their masked footprint and
  direction thresholds.
- [x] M2 references use `local-structure-v2`: committed source frames are
  compared with windowed SSIM, dilated Sobel-edge IoU, and dilated foreground
  IoU. Broad quadrant means are diagnostic only. Heatmaps and worst-region
  boxes are emitted for every fixture, and mean-preserving structure mutations
  that rotate or collapse content inside each quadrant are rejected.
- [x] `Q03-M2-LOCAL-STRUCTURE` rejects missing source frames, weakened local
  thresholds, missing heatmap/region evidence, or removal of the
  mean-preserving mutation tests. `Q05-EFFECT-FOOTPRINTS` continues to reject
  missing effect pairs/masks and retired exact/hash metadata.
- [x] Browser smoke runs with
  `node tests/browser/m2_browser_lighting_clipping_smoke.js`, writes
  `m2-browser-lighting-clipping-smoke.json`, and is registered by
  `VISUAL-BROWSER-M2`.
