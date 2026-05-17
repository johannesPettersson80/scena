# Changelog

All notable user-facing changes are recorded here.

## [Unreleased]

### Changed

- Updated the browser demo showcase so connector mating is the default first
  render with a synced Rust code panel, visible replay action, collapsed
  diagnostics, and a README connector-snap hero GIF.

### Fixed

- Browser HDR/IBL diffuse lighting now uses prepared diffuse irradiance instead
  of sampling raw HDR radiance in the surface-normal direction, avoiding dark or
  washed-out connector renders while preserving the specular path.
- Public demo timing logs are quiet by default and remain available through
  `?perf=1` or `?timing=1`.

## [1.2.0] - 2026-05-17

### Added

- Added `AssetLoadOptions` with `with_strict_textures(true)` plus
  `Assets::load_scene_with_options` and `Assets::load_scene_with_report_options`
  so browser hosts can promote missing external glTF image fetches from warnings
  to hard load errors.
- Added `DiagnosticCode::MaterialTextureMissingDecodedPixels` and
  `RendererStats::material_textures_missing_decoded_pixels` so descriptor-only
  material textures are visible during `prepare_with_assets`.
- Added browser WebGL2/WebGPU visual coverage for manual `SceneAsset` source
  material reuse on the dense Khronos WaterBottle glTF with external relative
  PBR textures.

### Fixed

- Depth prepass eligibility now ignores ineligible helper/stroke primitives
  instead of disabling the prepass for the whole scene.
- WebGL2 and WebGPU color/depth passes now use the same
  `clip_from_world * world_position` path, avoiding precision disagreement in
  dense browser scenes.
- Browser asset loading now emits console warnings when optional external
  textures cannot be fetched and the caller did not request strict texture
  loading.

## [1.1.0] - 2026-05-16

### Changed

- WebGL2 now renders through the shared wgpu/naga path instead of the deleted
  hand-written raw WebGL2 renderer. The public `Backend::WebGl2` API remains
  intact.
- WebGL2 material sampling uses a small wgpu shader/layout shim with ordinary
  `texture_2d` bindings because wgpu 29's GL backend rendered material
  `texture_2d_array` samples black in Chromium WebGL2.

### Fixed

- Repeated WebGL2 `Renderer::prepare()` no longer retains the old raw GL
  buffer/texture/program cache, closing the GL out-of-memory and subsequent
  wasm-bindgen mutable-guard poisoning failure family.

### Removed

- Removed the hand-written WebGL2 renderer modules and raw `web_sys`
  render-path bindings.

## [1.0.2] - 2026-05-15

### Fixed

- WebGL2 program link failure on Firefox: the output shader no longer redeclares fragment-only uniforms (`camera_position_exposure`, `color_management`, `base_color_uv_offset_scale`, `base_color_uv_rotation`) in the vertex stage with implicit `highp` precision that conflicts with the fragment stage's `precision mediump float;` directive. Firefox WebGL2 reported `Uniform \`<name>\` is not linkable between attached shaders`; Chromium did not enforce the rule. See `docs/decisions/ADR-0001-webgl2-camera-uniform-precision-mismatch.md`.

### Added

- Unit-level regression test `webgl2_shaders_have_no_cross_stage_uniform_precision_mismatch` in `src/render/gpu/materials.rs::tests` that statically parses both WebGL2 shaders (read via `include_str!` of `webgl2_program.rs`) and fails if any uniform name is declared in both stages with an unresolvable precision mismatch. Catches the bug class in every native `cargo test` run without requiring a browser.

## [1.0.1] - 2026-05-14

### Changed

- Reworked the repository documentation into a user-facing documentation set with guides for getting started, API concepts, assets, rendering, browser use, headless rendering, capabilities, lifecycle, errors, feature flags, and troubleshooting.
- Updated release tooling so documentation gates validate the public documentation surface shipped to users.
- Moved benchmark baseline data out of the public documentation tree and into test fixtures.

## [1.0.0] - 2026-05-14

### Added

- Published the first stable `scena` release.
- Added Rust-native scene graph, asset loading, renderer lifecycle, diagnostics, headless rendering, native platform lanes, browser WebGPU/WebGL2 paths, and public examples.
- Added glTF/GLB workflows for model-viewer, CAD-style inspection, industrial visualization, and digital-twin UI use cases.

### Documentation

- Published README, install instructions, examples, platform notes, and release notes for the stable API.
