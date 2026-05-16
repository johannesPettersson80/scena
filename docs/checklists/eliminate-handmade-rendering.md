# Eliminate hand-written rendering checklist

Status: In progress - PR 1 and v1.1.0 release closeout implemented locally;
release gates, push, CI, and crates.io publish still pending.
Date: 2026-05-16
Governing decision: [ADR-0002](../decisions/ADR-0002-eliminate-handmade-rendering.md)
Historical bug record: [ADR-0001](../decisions/ADR-0001-webgl2-camera-uniform-precision-mismatch.md)

## Scope

This checklist turns ADR-0002 into execution work. The goal is to remove the raw
WebGL2 renderer and other hand-written duplication without losing the public
WebGL2 compatibility lane.

The target end state:

- `Backend::WebGl2` remains public.
- WebGL2 renders through wgpu + naga, using `wgpu::Backends::GL`.
- `render()` does not hide shader compilation, asset fetch, or GPU upload.
- No raw WebGL2 renderer path exists under `src/render`.
- Browser proof, capability matrices, doctor rules, and release evidence match
  the new wgpu-backed behavior.

## Non-negotiable rules

- [ ] Add or update the narrowest failing test/proof before production code in
  each implementation PR, or document why the proof cannot run before the code.
- [ ] Keep all fallbacks inside the wgpu path. Do not fork wgpu/naga and do not
  reintroduce raw GL rendering.
- [ ] Keep `Scene`, `Assets`, `Renderer`, `Animation`, and `platform` ownership
  boundaries intact.
- [ ] Do not mark an item complete without naming its local test, browser proof,
  doctor rule, or documented exception.
- [ ] Do not remove public API without checking the active API/semver baseline.

## Branch blockers

- [x] Fix or explicitly track the current full-doctor blocker:
  `ARCH-DEPENDENCY-DIRECTION: src/demo_page.rs is not mapped to an architecture owner`.
  This is not caused by ADR-0002. It applies to the `wasm-demo-spike` branch;
  the clean v1.1.0 release branch is based on `main` and has no `src/demo_page.rs`.
- [x] Resolve the public API baseline location before PR 1 lands. `docs/api.md`
  is authoritative in this checkout, and the stale release-contract doctor
  references to `docs/api/m5-public-api-baseline.txt` and
  `docs/api/m5-semver-baseline.toml` were removed.
- [x] Decide whether WebGL2 capability values are conservative public constants
  or measured wgpu adapter-limit reports. ADR-0002 recommends measured wgpu
  values for release proof; v1.1.0 kept the conservative WebGL2 low-tier matrix
  and remeasured it with the M6 browser probe.
- [x] Decide the target branch before implementation. The release branch is
  based on `main`; `wasm-demo-spike` remains a separate demo branch and can
  rebase after the renderer path is fixed.

## PR 0 - Replace the raw browser surface helper

Purpose: remove the unsafe raw browser canvas handle creation before the larger
renderer deletion.

- [ ] Add or update a focused browser-surface test/proof that exercises
  `PlatformSurface::browser_webgl2_canvas_element` and
  `PlatformSurface::browser_webgpu_canvas_element`.
- [ ] Replace `src/render/gpu/build.rs::create_browser_canvas_surface()` with
  `instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))`.
- [ ] Preserve `Backend::WebGl2 => wgpu::Backends::GL`.
- [ ] Preserve `Backend::WebGpu => wgpu::Backends::BROWSER_WEBGPU`.
- [ ] Preserve `wgpu::Limits::downlevel_webgl2_defaults()` for WebGL2.
- [ ] Confirm `cargo check --target wasm32-unknown-unknown --features browser-probe`.
- [ ] Run `cargo fmt --check`.
- [ ] Run `cargo clippy --all-targets -- -D warnings`.
- [ ] Run `cargo test`.
- [ ] Run `cargo run -p xtask -- doctor --full` or record the exact unrelated blocker.

Completion evidence:

- [ ] Commit contains only the safe wgpu surface-target swap and matching tests/proof.
- [ ] Browser WebGPU and WebGL2 surface creation still reaches renderer construction.

## PR 1 - Collapse WebGL2 onto wgpu/naga

Purpose: delete the raw WebGL2 renderer while keeping the public WebGL2 backend.

Test-first proof:

- [ ] Add a repeated-prepare browser proof for attached WebGL2 with a real glTF
  fixture such as DamagedHelmet.
- [ ] Prove the old branch fails for the expected resource-lifetime reason, or
  document why the repro can only be captured on Raspberry Pi 5 Chromium.
- [ ] Add or update Firefox WebGL2 proof for the ADR-0001 precision-family
  regression.
- [ ] Add or update WebGL2 capability-matrix proof that captures live wgpu
  adapter limits after the switch.
- [ ] Use `tests/browser/m6_rust_wasm_renderer_probe.*` as the primary
  production browser proof vehicle for WebGL2/WebGPU unless a narrower repro
  page is explicitly required.
- [ ] If the hosted demo is the external repro vehicle, prove the fix against
  `scena-demo.pages.dev` or the same hosted-demo deployment path with
  DamagedHelmet on Raspberry Pi 5 Chromium.

Dependency decision:

- [x] Decide whether `wgpu = 29.0.3` is sufficient for scena's WGSL and bind
  group layout on WebGL2.
- [ ] If wgpu WebGL2 has a blocking upstream bug, decide whether to bump wgpu
  before deletion. Keep the fallback inside wgpu; do not fork wgpu/naga and do
  not reintroduce raw GL.
- [x] Keep wgpu 29.0.3. The observed blocker is limited to WebGL2 material
  `texture_2d_array` sampling rendering black in Chromium WebGL2, so v1.1.0
  uses a WebGL2-only wgpu `texture_2d` material shader/layout shim instead of a
  raw GL fallback or wgpu fork.

Renderer deletion:

- [x] Remove the wasm `prepare()` WebGL2 special path in `src/render/gpu.rs`.
- [x] Remove `webgl2::encode_vertices`.
- [x] Remove `webgl2::prepare_canvas_vertices`.
- [x] Remove `webgl2_vertices` from prepared resources.
- [x] Remove the wasm `render_to_surface()` WebGL2 branch in
  `src/render/gpu/draw.rs`.
- [x] Route WebGL2 presentation through the shared wgpu render pass.
- [x] Delete `src/render/gpu/webgl2.rs`.
- [x] Delete `src/render/gpu/webgl2_program.rs`.
- [x] Delete `src/render/gpu/webgl2_camera.rs`.
- [x] Delete `src/render/gpu/webgl2_lighting.rs`.
- [x] Delete `src/render/gpu/webgl2_materials.rs`.
- [x] Delete `src/render/gpu/webgl2_texture_set.rs`.
- [x] Delete `src/render/gpu/webgl2_vertices.rs`.
- [x] Remove `WebGl2RenderCache`.
- [x] Remove all direct raw WebGL2 renderer dependencies from `Cargo.toml`
  unless a small capability probe still needs a narrow `WebGl2RenderingContext`
  allowlist.

Doctor and test flips:

- [x] Flip `VISUAL-BROWSER-M6` in
  `crates/xtask/src/app/doctor_visual_release/browser_probe.rs` so it no longer
  requires `WebGl2RenderingContext`, `WebGlProgram`, or `WebGlShader` in
  `Cargo.toml`.
- [x] Update
  `crates/xtask/src/app/tests_07.rs::doctor_rejects_m6_browser_renderer_probe_missing_cargo_dep_regression`.
- [x] Replace raw-WebGL2-presence checks in
  `crates/xtask/src/app/doctor_render/render_truth/webgl2.rs`.
- [x] Replace raw-WebGL2-presence checks in
  `crates/xtask/src/app/doctor_m7_m8_assets/assets_materials.rs`.
- [x] Replace raw-WebGL2-presence checks in
  `crates/xtask/src/app/doctor_visual_release/browser_probe.rs`.
- [x] Replace raw-WebGL2-presence checks in
  `crates/xtask/src/app/doctor_architecture/module_boundaries.rs`.
- [x] Replace raw-WebGL2-presence checks in
  `crates/xtask/src/app/doctor_scene_platform/shadow_depth.rs`.
- [x] Replace raw-WebGL2-presence checks in
  `crates/xtask/src/app/doctor_render/standard_math_prepare.rs`.
- [x] Add a doctor rule that fails on raw render-path `WebGl2RenderingContext`,
  `gl.compileShader`, `gl.linkProgram`, `gl.bufferData`, or hand-written GLSL
  under `src/render`.
- [ ] Allowlist only narrow non-render browser capability probes, if any remain.

Capability and release evidence:

- [ ] Rebaseline `tests/browser/m4_platform_smoke.html` so it does not carry
  stale raw-renderer-era WebGL2 constants.
- [ ] Rebaseline `tests/m4_performance_platform.rs` for WebGL2 capability
  expectations.
- [ ] Rebaseline `tests/m9_platform_release.rs` and release-lane artifacts so
  `linux-webgl2-chromium` uses measured wgpu WebGL2 capabilities.
- [ ] Verify `hardware_tier`, `uniform_buffers`, `fragment_high_precision`,
  texture limits, texture arrays, compute shaders, and storage buffers after the
  backend switch.
- [x] Verify `docs/api.md` and whichever semver baseline is authoritative.
- [ ] Update docs that describe WebGL2 internals: `docs/browser.md`,
  `docs/platforms.md`, `docs/capabilities.md`, `docs/rendering.md`, and release
  notes if user-visible behavior changes.
- [x] Add `Superseded by: ADR-0002` to ADR-0001 after the raw WebGL2 renderer is
  actually deleted.

Browser proof:

- [ ] Browser rendered-output proof for `Backend::WebGl2`.
- [ ] Browser rendered-output proof for `Backend::WebGpu`.
- [ ] `tests/browser/m6_rust_wasm_renderer_probe.*` proof remains green for
  WebGL2 and WebGPU.
- [ ] Firefox WebGL2 proof for the ADR-0001 family.
- [ ] Repeated-prepare proof for attached WebGL2.
- [ ] Raspberry Pi 5 Chromium / DamagedHelmet proof if that hardware remains the
  deciding repro target.
- [ ] Hosted demo proof against the same DamagedHelmet workflow that reproduced
  the leak, including no `GL_OUT_OF_MEMORY`, no wasm panic, and no subsequent
  `recursive use of an object detected` pointer-event failures.

Required gates:

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [ ] `cargo check --target wasm32-unknown-unknown --features browser-probe`
- [ ] `cargo run -p xtask -- doctor --full`
- [ ] `cargo run -p xtask -- claim-audit`
- [ ] `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`
- [ ] Browser proof artifacts stored under `target/gate-artifacts/` or the
  existing release-lane artifact location.

Completion evidence:

- [ ] `rg -n "WebGl2RenderingContext|WebGlProgram|WebGlShader|compileShader|linkProgram|bufferData|VERTEX_SHADER|FRAGMENT_SHADER" src/render`
  returns no raw render-path hits.
- [ ] `rg -n "webgl2_" src/render crates/xtask/src/app tests` has only
  backend-vocabulary, capability, historical ADR, or intentionally allowlisted
  probe references.
- [ ] No hand-written GLSL remains in `src/render`.

## PR 1 release closeout

Purpose: make the WebGL2 renderer deletion releasable. This is release-notable
backend behavior, so default to a minor release such as `v1.1.0` unless the
maintainer explicitly chooses a different version.

Version and pin sweep:

- [x] Bump `Cargo.toml` from `version = "1.0.2"` to `version = "1.1.0"`.
- [x] Add `docs/release-notes/v1.1.0.md` or the chosen
  `docs/release-notes/v<version>.md`.
- [x] Update `tests/m5_release.rs::m5_release_surface_files_and_examples_are_present`
  so the new release-notes file is required.
- [x] Update `tests/m5_release.rs::m5_package_metadata_is_ready_for_dry_run`
  so the manifest version literal matches the chosen version.
- [x] Update
  `crates/xtask/src/app/doctor_scene_platform/release_contracts.rs`
  (`ARCH-M5-RELEASE`) version/release-note pins.
- [x] Update `crates/xtask/src/app/tests_10.rs` release-contract fixture
  version literals.
- [x] Update
  `crates/xtask/src/app/release/lane_artifacts.rs::check_release_readiness_adr`
  to read the new release-notes file and report the new version in findings.
- [x] Update
  `crates/xtask/src/app/doctor_visual_release/ci_release_lanes.rs`
  (`CLAIM-AUDIT-M10`) so claim-audit checks the new release-notes file.
- [x] Update `docs/api.md` docs.rs URL to the new version.
- [x] Update `docs/README.md` docs.rs URL and release-notes link to the new
  version.
- [x] Update any top-level `README.md` release-note table or docs.rs version
  reference that still points at the old release.
- [x] Update `CHANGELOG.md` if it carries unreleased or versioned release
  entries for this change.

Release evidence:

- [x] The new release notes name the user-visible behavior change: WebGL2 now
  uses the wgpu/naga path instead of the hand-written raw WebGL2 renderer.
- [x] The new release notes name the regression family closed:
  repeated-prepare WebGL2 GL resource growth / out-of-memory / wasm guard
  poisoning.
- [x] The release notes include browser proof references for WebGL2 and WebGPU.
- [x] The release notes include capability-matrix changes or explicitly state
  that capability values were remeasured and unchanged.
- [x] The release notes include migration guidance if any diagnostics,
  capability reports, or WebGL2 visual differences changed.

Release gates:

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `cargo check --target wasm32-unknown-unknown --features browser-probe`
- [x] `wasm-pack build --dev --target web --out-dir target/m6-browser-pkg . --features browser-probe`
- [x] `SCENA_BROWSER_BACKENDS=webgl2 node tests/browser/m6_rust_wasm_renderer_probe.js`
- [x] `SCENA_BROWSER_BACKENDS=webgpu node tests/browser/m6_rust_wasm_renderer_probe.js`
- [x] `cargo run -p xtask -- doctor --full`
- [x] `cargo run -p xtask -- claim-audit`
- [x] `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`
- [x] `cargo publish --dry-run`

GitHub release follow-through, only when the maintainer asks to publish:

- [ ] Verify the intended branch, remote, and dirty tree separately before
  tagging.
- [ ] Ensure the release commit is on the intended remote branch.
- [ ] Create and push the matching `v<version>` tag only after local gates pass.
- [ ] Monitor the release workflow until it completes.
- [ ] Verify the GitHub release object exists and points at the expected tag.
- [ ] Verify post-release CI evidence separately from local gate evidence.

## PR 2 - Decide and fix OBJ

Purpose: remove or replace the optional hand-written OBJ parser.

Decision:

- [ ] Decide whether the `obj` feature remains part of the stable v1 surface.
- [ ] If yes, replace `src/assets/obj.rs` with a maintained parser such as
  `tobj` behind the existing `obj` feature.
- [ ] If no, deprecate first and remove only in a breaking release.

If replacing:

- [ ] Add optional parser dependency behind `obj`.
- [ ] Preserve `Assets::load_geometry()` behavior where currently documented.
- [ ] Preserve
  `tests/m3a_app_features.rs::obj_feature_load_geometry_parses_triangle_faces`.
- [ ] Add one negative test for unsupported/non-triangulated/material-heavy OBJ
  behavior with a structured error.
- [ ] Update `docs/feature-flags.md`.

If removing:

- [ ] Update `Cargo.toml`.
- [ ] Remove `src/assets/obj.rs`.
- [ ] Update `docs/feature-flags.md`.
- [ ] Update release notes and migration notes.
- [ ] Verify semver/public API baseline impact.

Required gates:

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test --features obj`
- [ ] `cargo run -p xtask -- doctor --full`

## PR 3 - Retire browser mini-renderers

Purpose: stop maintaining separate JS/WGSL/GLSL renderer fragments in browser
smoke tests.

- [ ] Decide whether M0/M2/M4 smoke tests must remain independent of the Rust
  renderer.
- [ ] Keep only minimal WebGPU/WebGL2 context-availability probes if independent
  platform smoke still matters.
- [ ] Move render/pixel assertions to Rust/WASM renderer probes in
  `src/browser_probe.rs` and `tests/browser/m6_rust_wasm_renderer_probe.*`, or
  add equivalent Rust-backed M0/M2/M4 probes.
- [ ] Delete hand-written JS shader/program setup from
  `tests/browser/m0_browser_surface_smoke.html`.
- [ ] Delete hand-written JS shader/program setup from
  `tests/browser/m2_browser_lighting_clipping_smoke.html`.
- [ ] Delete hand-written JS shader/program setup from
  `tests/browser/m4_platform_smoke.html`.
- [ ] Preserve capability-matrix artifact shape after the deletion.
- [ ] Run browser smoke/probe tests for WebGPU and WebGL2.
- [ ] Run `cargo run -p xtask -- doctor --full`.

## PR 4 - Optional CLI parser cleanup

Purpose: remove small hand-written CLI parsing only if the CLI is growing or
release-gate work touches it anyway.

- [ ] Keep this PR deferred unless the CLI gains options/subcommands.
- [ ] Replace manual dry-run JSON escaping with `serde_json`.
- [ ] Adopt `lexopt` or `clap` only if the CLI needs real argument parsing.
- [ ] Preserve
  `tests/m5_release.rs::scena_convert_cli_reports_fbx_to_gltf_plan`.
- [ ] Run `cargo test scena_convert_cli_reports_fbx_to_gltf_plan`.
- [ ] Run `cargo run -p xtask -- doctor --full`.

## PR 5 - Optional proof-hash standardization

Purpose: standardize visual/probe artifact hashes only if artifact schemas are
already changing.

- [ ] Decide whether `fnv1a64` is an explicit artifact contract or accidental
  duplication.
- [ ] If standardizing, switch test/artifact hashing to `sha2` or another
  maintained hash crate.
- [ ] Update artifact schemas and baselines in the same commit.
- [ ] Keep old hashes stable if release artifacts depend on them and there is
  no user-visible reason to change.
- [ ] Run release artifact validation and doctor.

## Closeout gates

The deletion effort is complete only when all of these are true:

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `cargo check --target wasm32-unknown-unknown --features browser-probe`
- [ ] `cargo run -p xtask -- doctor --full`
- [ ] `cargo run -p xtask -- claim-audit`
- [ ] `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`
- [ ] `cargo publish --dry-run`
- [ ] Browser rendered-output proof exists for WebGL2 through wgpu.
- [ ] Browser rendered-output proof exists for WebGPU.
- [ ] Hosted demo proof exists for the DamagedHelmet WebGL2 workflow if that is
  the active external repro.
- [ ] Capability matrix rows are measured or explicitly marked
  `missing-lane-artifact`; no stale factory constants are used as proof.
- [ ] API/semver baseline is checked and documented.
- [ ] Release notes explain any user-visible backend behavior, capability, or
  diagnostic changes.
- [ ] ADR-0001 is marked superseded by ADR-0002 after the raw renderer is gone.
- [ ] ADR-0002 can move from `Proposed` to `Accepted` or another explicit final
  status chosen by the maintainer.
