# Changelog

All notable user-facing changes are recorded here.

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
