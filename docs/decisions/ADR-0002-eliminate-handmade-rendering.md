# ADR-0002 - Eliminate hand-written rendering paths that duplicate wgpu/naga

Status: Accepted
Date: 2026-05-15
Accepted: 2026-05-16
Affects: scena 1.0.2 and the `wasm-demo-spike` checkout before the v1.1.0 deletion
Resolved in: scena 1.1.0
Execution checklist: [eliminate-handmade-rendering](../checklists/eliminate-handmade-rendering.md)

## Problem

`scena` currently has two browser rendering implementations:

| Backend path | Shader source | Render/resource path |
| --- | --- | --- |
| Native Vulkan / Metal / DX12 | `src/render/gpu/output_shader.wgsl` | wgpu + naga |
| Browser WebGPU | `src/render/gpu/output_shader.wgsl` | wgpu + naga |
| Browser WebGL2 | `src/render/gpu/webgl2_program.rs` | raw `web_sys::WebGl2RenderingContext` |

`Cargo.toml` already declares:

```toml
wgpu = { version = "29.0.3", features = ["webgl"] }
```

`Cargo.lock` confirms both `wgpu 29.0.3` and `naga 29.0.3` are in the dependency graph.
The WebGL2 backend is therefore paying for wgpu's WebGL2 backend while bypassing it for
the actual render path.

This has already produced real bugs:

- ADR-0001: Firefox WebGL2 rejected the hand-written GLSL because a shared uniform was
  declared with mismatched effective precision across vertex and fragment stages. This was
  a hand-written GLSL bug, not a naga bug.
- Active resource-lifetime failure: repeated `Renderer::prepare()` calls on the raw WebGL2
  path can leak GL buffers/textures until `glBufferData` reports `GL_OUT_OF_MEMORY`, which
  then panics through wasm and poisons the `wasm-bindgen` mutable guard.

The decision this ADR recommends is: delete the hand-written WebGL2 renderer and route
`Backend::WebGl2` through the same wgpu/naga pipeline as every other GPU backend.

## Audit method

The WebGL2 duplication was counted with raw `wc -l` and an approximate significant-line
count that excludes blank and line-comment-only lines:

```text
$ wc -l src/render/gpu/webgl2.rs src/render/gpu/webgl2_program.rs \
    src/render/gpu/webgl2_camera.rs src/render/gpu/webgl2_materials.rs \
    src/render/gpu/webgl2_texture_set.rs src/render/gpu/webgl2_lighting.rs \
    src/render/gpu/webgl2_vertices.rs
  490 src/render/gpu/webgl2.rs
  509 src/render/gpu/webgl2_program.rs
   95 src/render/gpu/webgl2_camera.rs
  166 src/render/gpu/webgl2_materials.rs
   88 src/render/gpu/webgl2_texture_set.rs
  102 src/render/gpu/webgl2_lighting.rs
  136 src/render/gpu/webgl2_vertices.rs
 1586 total

$ awk '...' same file list
466 src/render/gpu/webgl2.rs
461 src/render/gpu/webgl2_program.rs
91 src/render/gpu/webgl2_camera.rs
151 src/render/gpu/webgl2_materials.rs
82 src/render/gpu/webgl2_texture_set.rs
97 src/render/gpu/webgl2_lighting.rs
134 src/render/gpu/webgl2_vertices.rs
```

Significant WebGL2 renderer duplication: **1,482 LOC**.
Raw WebGL2 renderer duplication: **1,586 LOC**.

The prompt mentions `src/render/gpu/webgl2_shader_text.rs`; that file is not present in
this checkout. Shader text is still embedded in `src/render/gpu/webgl2_program.rs`.

History evidence:

```text
$ git log --stat --oneline -- src/render/gpu/webgl2*.rs
2a3f42b Fix WebGL2 shader linker failure on Firefox; release 1.0.2 (#2)
2e61997 Fix browser proof and GPU color gates
4e76ed2 Harden release readiness and architecture gates
0f22aa1 Phase 5.1: parse normalTexture.scale + occlusionTexture.strength
41f8f25 Land M6-M10 implementation and state-of-art replacement plan
ba5556f Advance Three.js replacement gates
```

`ba5556f` introduced the first 146-line WebGL2 renderer. `41f8f25` expanded it into the
current multi-file renderer while landing browser parity and release gates. The likely
reason was milestone pressure for browser WebGL2 proof, not a documented wgpu limitation.

## WebGL2 duplication inventory

| File | Raw LOC | Significant LOC | What it does today | wgpu/naga equivalent already present | Tests/rules that reference it |
| --- | ---: | ---: | --- | --- | --- |
| `src/render/gpu/webgl2.rs` | 490 | 466 | Creates a raw WebGL2 context, compiles/link GLSL, owns `WebGl2RenderCache`, uploads vertex buffers/textures, binds uniforms, calls `gl.drawArrays`, hashes draw/vertex state. | `src/render/gpu/build.rs`, `src/render/gpu/draw.rs`, `src/render/gpu/pipeline.rs`, `src/render/gpu/materials.rs`, `src/render/gpu/vertices.rs`, wgpu `Device`/`Queue`/`RenderPass`/`Surface`. | `src/render/gpu/materials.rs::webgl2_material_upload_uses_texture_sampler_metadata`; xtask `doctor_render/render_truth/webgl2.rs`, `doctor_visual_release/browser_probe.rs`, `doctor_architecture/module_boundaries.rs`; browser M6 tests indirectly. |
| `src/render/gpu/webgl2_program.rs` | 509 | 461 | Stores hand-written GLSL ES 3.00 vertex/fragment shader strings, raw compile/link helpers, FNV-like hashes, and wasm-gated shader string tests. | `src/render/gpu/output_shader.wgsl` consumed by `device.create_shader_module`, with naga emission inside wgpu. | Its own wasm-gated tests; `src/render/gpu/materials.rs::webgl2_material_shader_declares_fragment_texture_transform_uniforms`; `src/render/gpu/materials.rs::webgl2_shaders_have_no_cross_stage_uniform_precision_mismatch`; `src/render/gpu.rs::host_tests_guard_webgl2_khronos_pbr_neutral_source`; multiple xtask doctor rules. |
| `src/render/gpu/webgl2_camera.rs` | 95 | 91 | Queries and uploads camera/model/view/projection/color uniforms one raw location at a time. | `src/render/gpu/output.rs` output uniform buffer, `src/render/gpu/draw_uniform.rs` dynamic draw uniforms, WGSL uniform structs. | xtask `doctor_render/render_truth/webgl2.rs`; browser M6 tests indirectly. |
| `src/render/gpu/webgl2_lighting.rs` | 102 | 97 | Queries and uploads directional/point/spot/environment light uniforms with `uniform4f`. | `PreparedGpuLightUniform` is encoded into the wgpu output uniform and consumed by `output_shader.wgsl`. | xtask `doctor_render/render_truth/webgl2.rs`; browser M6 tests indirectly. |
| `src/render/gpu/webgl2_materials.rs` | 166 | 151 | Creates raw `WebGlTexture`, maps sampler metadata to GL enums, uploads RGBA8 with `texImage2D`, generates mipmaps, hashes pixel/sampler state. | `src/render/gpu/material_upload.rs`, `src/render/gpu/materials.rs`, `src/render/gpu/material_batched.rs`, `Queue::write_texture`, wgpu samplers/textures. | `src/render/gpu/materials.rs::webgl2_material_upload_uses_texture_sampler_metadata`; xtask material/render-truth rules. |
| `src/render/gpu/webgl2_texture_set.rs` | 88 | 82 | Owns five separate raw WebGL textures per material role and hashes per role. | `MaterialResources::PerMaterial` / `MaterialResources::Batched` and wgpu bind groups/array textures. | `src/render/gpu/materials.rs::webgl2_material_upload_uses_texture_sampler_metadata`; xtask material rules. |
| `src/render/gpu/webgl2_vertices.rs` | 136 | 134 | Re-encodes the wgpu vertex stream as `Vec<f32>` and manually configures raw vertex attributes. | `src/render/gpu/vertices.rs::encode_vertices`, `VERTEX_ATTRIBUTES`, and wgpu `VertexBufferLayout`. | xtask `doctor_render/diagnostics_stats_world.rs`; browser M6 tests indirectly. |

The call sites are the deletion boundary:

- `src/render/gpu.rs`: wasm `prepare()` always builds wgpu resources, then additionally
  calls `webgl2::encode_vertices()` and `webgl2::prepare_canvas_vertices()` when
  `target.backend == Backend::WebGl2`.
- `src/render/gpu/draw.rs`: wasm `render_to_surface()` branches before the wgpu render pass
  and calls `webgl2::render_canvas()`.
- `src/render/gpu/build.rs`: creates a wgpu `Instance`, `Surface`, `Adapter`, `Device`, and
  `Queue` for `Backend::WebGl2`, but the render path then bypasses those resources.
- `wasm-demo-spike` only: `src/demo_page.rs` has a `demo-page` feature
  `attach_to_canvas()` export that creates
  `PlatformSurface::browser_webgl2_canvas_element(...)`. That spike is not part
  of the clean v1.1.0 release branch, but it is the demo path that surfaced the
  repeated-prepare failure.

The repeated-prepare failure is visible in the code shape, not only in the external repro.
On wasm, `prepare()` calls `self.release_prepared_resources()` and rebuilds the shared wgpu
resources, but `release_prepared_resources()` only takes `self.resources`; it does not clear
`self.webgl2_render_cache`. That cache is cleared only by
`clear_prepared_resources_for_context_recovery()`. The raw WebGL2 cache keeps a
`WebGlBuffer`, `WebGlProgram`, and a grow-only vector of per-material
`WebGl2MaterialTextureSet` values, with no `delete_buffer`, `delete_texture`,
`delete_program`, or `Drop` implementation. The concrete bug class is therefore: WebGL2
prepare runs alongside the wgpu prepare path, and its raw GL cache has no normal
prepare-to-prepare deletion or shrinking path.

There is also WebGL2-specific test/doctor scaffolding outside the seven files:

- `src/render/gpu/materials.rs:513-665` contains 153 raw / 128 significant LOC of
  WebGL2 source-string tests, including a tiny GLSL uniform parser added for ADR-0001.
- `crates/xtask/src/app/doctor_render/render_truth/webgl2.rs` is a 146-line doctor rule
  whose job is to keep the raw WebGL2 implementation alive and aligned.
- Additional WebGL2 substring checks live in `doctor_m7_m8_assets`, `doctor_visual_release`,
  `doctor_architecture`, `doctor_scene_platform`, and `doctor_render/standard_math_prepare`.
- `crates/xtask/src/app/doctor_visual_release/browser_probe.rs` currently pins the opposite
  of this ADR's PR 1: `VISUAL-BROWSER-M6` requires `Cargo.toml` to contain
  `WebGl2RenderingContext`, `WebGlProgram`, and `WebGlShader`, and
  `crates/xtask/src/app/tests_07.rs::doctor_rejects_m6_browser_renderer_probe_missing_cargo_dep_regression`
  regression-tests that requirement.

These should be removed or inverted in the deletion PR. The new doctor rule should forbid
raw render-path `WebGl2RenderingContext` usage outside narrow browser capability probes.

Exact `rg` reference evidence for tests/rules that keep the files load-bearing today:

- `webgl2.rs`: included by `src/render/gpu/materials.rs` and checked by xtask
  `doctor_m7_m8_assets/assets_materials.rs`, `doctor_visual_release/browser_probe.rs`,
  `doctor_architecture/module_boundaries.rs`, and `doctor_render/render_truth/webgl2.rs`.
- `webgl2_program.rs`: included by `src/render/gpu/materials.rs` and `src/render/gpu.rs`;
  its own wasm-gated tests cover shader content; xtask checks it from
  `doctor_scene_platform/shadow_depth.rs`, `doctor_m7_m8_assets/assets_materials.rs`,
  `doctor_visual_release/browser_probe.rs`, `doctor_render/standard_math_prepare.rs`, and
  `doctor_render/render_truth/webgl2.rs`; ADR-0001 also documents the precision regression.
- `webgl2_camera.rs`: checked by `doctor_render/render_truth/webgl2.rs`.
- `webgl2_lighting.rs`: checked by `doctor_render/render_truth/webgl2.rs`.
- `webgl2_materials.rs`: included by `src/render/gpu/materials.rs` and checked by
  `doctor_m7_m8_assets/assets_materials.rs` and `doctor_render/render_truth/webgl2.rs`.
- `webgl2_texture_set.rs`: included by `src/render/gpu/materials.rs` and checked by
  `doctor_m7_m8_assets/assets_materials.rs`; it is also pulled through
  `webgl2.rs`.
- `webgl2_vertices.rs`: checked by `doctor_render/diagnostics_stats_world.rs` and pulled
  through `webgl2.rs`.

## WebGL2 replacement path

wgpu 29.0.3 already exposes the browser canvas API for WebGPU, and its `webgl`
feature exposes the GL/WebGL backend used by WebGL2:

- `wgpu::SurfaceTarget::Canvas(web_sys::HtmlCanvasElement)` exists.
- wgpu's `webgl` feature enables the WebGL2 backend; `src/render/gpu/build.rs` already
  selects `wgpu::Backends::GL` and `wgpu::Limits::downlevel_webgl2_defaults()` for
  `Backend::WebGl2`.

Implementation note from the v1.1.0 deletion: wgpu 29's safe
`SurfaceTarget::Canvas` path works for browser WebGPU but omits the
`WebDisplayHandle` still needed by `wgpu::Backends::GL` surface creation. The
accepted implementation therefore keeps a minimal wgpu raw-handle shim for
WebGL2 surface creation only. That shim does not create a raw GL context and
does not render; it exists solely to pass a canvas/display handle to wgpu.

Second implementation note: wgpu 29's WebGL2 backend rendered material
`texture_2d_array` samples as black in Chromium WebGL2 during the migration.
The accepted implementation keeps the array-texture material path for
WebGPU/native and uses a WebGL2-only wgpu material shader/layout variant with
ordinary `texture_2d` bindings. This is the smallest shim that keeps rendering
inside wgpu/naga without forking wgpu or naga.

The smallest implementation shape is:

1. Keep the public `Backend::WebGl2` and `PlatformSurface::browser_webgl2_canvas_element`
   API so callers can still request the compatibility lane.
2. In `request_browser_surface_gpu`, use `SurfaceTarget::Canvas` for WebGPU and
   the minimal wgpu raw-handle canvas/display shim for WebGL2 until wgpu's safe
   canvas target carries the display handle for `Backends::GL`.
3. Keep `Backend::WebGl2 => wgpu::Backends::GL` and
   `wgpu::Limits::downlevel_webgl2_defaults()`.
4. Remove the WebGL2 branch in wasm `prepare()`: only the shared wgpu resources are built.
5. Remove the WebGL2 branch in wasm `render_to_surface()`: use the same wgpu render pass
   used by browser WebGPU.
6. Delete all seven `webgl2*.rs` modules and the `WebGl2RenderCache`/`webgl2_vertices`
   fields.
7. Remove `WebGl2RenderingContext`, `WebGlBuffer`, `WebGlProgram`, `WebGlShader`,
   `WebGlTexture`, and direct WebGL method dependencies from `Cargo.toml` unless a test-only
   browser context probe still needs `WebGl2RenderingContext`.

## Other findings

These are the other hand-written areas found by grepping for `parse_*`, `decode_*`,
`encode_*`, raw shader paths, raw surface lifecycle, raw hashes, and asset parsing across
`src`, `tests`, `crates/xtask`, and manifests.

| Finding | Exact paths / significant LOC | Existing crate/API replacement | Why it appears hand-written | Delete risk | Migration estimate | Rank |
| --- | ---: | --- | --- | --- | --- | --- |
| Raw browser canvas surface helper | `src/render/gpu/build.rs:153-175`, 23 raw / 19 significant LOC | WebGPU: `wgpu::SurfaceTarget::Canvas` + `Instance::create_surface`; WebGL2: minimal wgpu raw-handle canvas/display shim until the safe API carries `WebDisplayHandle` for `Backends::GL`. | Historical or copied from wgpu internals; current wgpu safe API is sufficient for WebGPU but not for WebGL2 on wgpu 29. | Low if kept as a narrow surface-creation shim and no raw GL render path returns. | Done in v1.1.0 as a retained minimal shim, not a renderer. | 1a |
| Optional OBJ parser | `src/assets/obj.rs`, 155 raw / 144 significant LOC | Prefer deleting `obj` feature if out of scope; if kept, use `tobj` or another maintained OBJ parser crate. No OBJ parser crate is currently in `Cargo.toml`. | Commit `12c00ab` says "Add optional OBJ geometry loader"; it is a minimal loader for `v`, `vn`, `f`, triangulation, and negative indices. | Public feature/API risk: `Cargo.toml` has `obj = []`, docs list it, and `Assets::load_geometry()` exists under that feature. Removing is breaking; replacing is not. | Replace: 1 day, 1-2 commits. Remove/deprecate: 0.5 day but needs semver decision. | 2 |
| Browser smoke mini-renderers | `tests/browser/m0_browser_surface_smoke.html` 191 significant LOC, `m2_browser_lighting_clipping_smoke.html` 250 significant LOC, `m4_platform_smoke.html` 171 significant LOC | Production wasm browser probe in `src/browser_probe.rs` and `Renderer::from_surface_async`; Playwright can call the Rust renderer instead of JS mini-renderers. | M0/M2/M4 predate the mature `m6_rust_wasm_renderer_probe` path and were useful as independent browser/context smoke checks. `m4_platform_smoke.html` also hard-codes WebGL2 capability JSON such as `hardware_tier: "Low"` and disabled compute/storage buffers. | Medium test-design risk: they test platform availability independent of the Rust renderer, and M4/M9 release evidence consumes capability-matrix contracts. Do not delete until equivalent context smoke or production-renderer browser proof exists for both WebGPU and WebGL2, and do not carry forward stale WebGL2 capability constants after switching to wgpu adapter limits. | 0.5-1 day, 1 commit. | 3 |
| Tiny CLI argument parser and JSON escaping | `src/bin/scena-convert.rs:81-139`, 59 raw / 54 significant LOC inside a 145-line CLI | `clap`/`lexopt` for args; `serde_json` for dry-run JSON. `serde_json` is already a dependency; `clap`/`lexopt` are not. | Commit `d53eafd` added the CLI during M5 release gates; avoiding a CLI dependency was probably expedient and acceptable while the CLI stayed tiny. | Low runtime risk; minor dependency/API churn. Not related to renderer bugs. | 0.5 day, 1 commit if CLI grows. | 4 |
| Deterministic FNV-style proof hashes | `src/browser_probe.rs:218-227` is 10 LOC; similar copies exist in visual/release tests. | `sha2` is already a dev-dependency for tests; a production browser-probe hash would need either promoted `sha2` or a small hash crate. | The field is named `rgba8_fnv1a64`; this is an explicit deterministic proof fingerprint, not a general cache key. | Low; changing hashes invalidates artifact baselines. Keep unless artifacts are being revised. | 0.5 day, 1 commit if standardizing artifacts. | 5 |

## Load-bearing code not recommended for deletion

These were inspected because they match the grep candidates, but they are not deletion
targets in this ADR.

| Area | Paths / LOC | Why not delete now |
| --- | ---: | --- |
| glTF loading and extras parsing | `src/assets/gltf.rs` and `src/assets/gltf/*` | The loader already uses the `gltf` crate for JSON/GLB document structure, `serde_json` for extras, and typed glTF accessors. The remaining code maps glTF data into scena-owned typed scene/material/animation structures and handles asset-fetcher integration. That is product code, not a duplicate parser. |
| glTF JSON pre-massage | `src/assets/gltf.rs::open_gltf_with_massage`, `massage_json_for_gltf_crate` | This intentionally preserves old scena tolerance for malformed animation entries and empty material variant extension blocks. It is load-bearing compatibility behavior. Deleting it would reject fixtures that current tests expect to load. |
| Base64 data URI handling | `src/assets/gltf.rs`, `src/assets/gltf/textures.rs`, `src/assets/texture.rs`, `src/assets.rs` | These are thin glue around `base64` and the `gltf` crate's typed URI locations. A generic `data-url` crate could reduce some string splitting, but this is not the same bug class as raw WebGL rendering. |
| PNG/JPEG decode and mip generation | `src/assets/texture.rs`, `src/render/gpu/material_mips.rs` | Already fixed by commit `ad66a6b`: hand-written color conversions and box mips were replaced with the `image` crate / `image::imageops`. No deletion remains other than small adapter code. |
| Radiance HDR decode | `src/assets/environment.rs` | Already fixed by commit `e939fd5`: the hand-written RGBE/RLE decoder was replaced with the `radiant` crate. Keep the current `radiant::load` wrapper. |
| Equirectangular-to-cubemap projection math | `src/assets/environment_projection.rs`, 67 raw / 50 significant LOC | Commit `e939fd5` explicitly audited this and kept it in-tree as tight renderer-domain math with no general crate replacement. It samples decoded HDR pixels into scena's cubemap convention. |
| KTX2/Basis path | `src/assets/texture_ktx2.rs`, 247 raw / 237 significant LOC | It uses `ktx2::Reader` and `basisu_c_sys`; the remaining code validates scena's texture policy, color-space contract, wasm fail-closed behavior, and mip-level payload shape. This is a shim around libraries, not a replacement decoder. |
| EXT_meshopt_compression | `src/assets/gltf/meshopt.rs`, 431 raw / 408 significant LOC | It uses the `meshopt` FFI decoder. The hand-written part parses the glTF extension JSON, validates byte ranges/strides/modes, and stores decompressed views into scena's buffer resolver. The `gltf` crate exposes the extension value but does not perform this integration. Keep unless adopting a full glTF importer that owns meshopt. |
| `PlatformSurface` public abstraction | `src/platform.rs`, 279 raw / 244 significant LOC | This is scena's typed public surface descriptor and event vocabulary. It intentionally avoids replacing winit/wasm-bindgen; `src/viewer.rs` also documents that scena does not replace winit / wasm-bindgen. Only the raw helper in `src/render/gpu/build.rs` should go. |
| Doctor source scanners | `crates/xtask/src/app/*` | Doctor checks intentionally inspect source and artifact text. They should be updated to forbid raw WebGL2 render code after deletion, but replacing all doctor source scanning with a Rust AST/parser is separate governance work. |

## Deletion plan

### PR 0 - Replace the raw browser surface helper

Goal: remove raw browser rendering from surface setup. The accepted v1.1.0
implementation discovered that WebGL2 still needs a `WebDisplayHandle` shim in
wgpu 29, so this step is complete only as a minimal wgpu surface-creation shim.

Self-contained changes:

- Use `instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))`
  for browser WebGPU.
- Keep WebGL2 on `Instance::create_surface_unsafe(SurfaceTargetUnsafe::RawHandle)`
  only to pass wgpu the canvas/display handles.
- Keep `Backend::WebGl2 => wgpu::Backends::GL` and
  `wgpu::Limits::downlevel_webgl2_defaults()`.
- Keep `Backend::WebGpu => wgpu::Backends::BROWSER_WEBGPU`.

Expected commits: 1.
Estimate: 0.5 day.

### PR 1 - Collapse WebGL2 onto wgpu/naga

Goal: `Backend::WebGl2` still exists, but it uses wgpu's GL/WebGL backend and the same
WGSL/naga render pipeline as native/WebGPU.

Self-contained changes:

- Add a regression proof first for the current failure family: repeated `Renderer::prepare()`
  on attached WebGL2 with a real glTF such as DamagedHelmet must not grow raw GL resources or
  panic. If this cannot run in unit tests, record it as browser/Pi rendered-output proof.
- If PR 0 has not landed, apply the WebGPU safe canvas path plus the narrow
  WebGL2 wgpu raw-handle surface shim.
- Remove wasm `prepare()` WebGL2 special work: no `webgl2::encode_vertices`, no
  `prepare_canvas_vertices`, no `webgl2_vertices` in prepared resources.
- Remove wasm `render_to_surface()` WebGL2 branch and let the normal wgpu pass present to the
  WebGL2-backed surface.
- Delete `src/render/gpu/webgl2.rs`, `webgl2_program.rs`, `webgl2_camera.rs`,
  `webgl2_materials.rs`, `webgl2_texture_set.rs`, `webgl2_lighting.rs`,
  `webgl2_vertices.rs`.
- Remove direct WebGL2 renderer dependencies from `Cargo.toml`.
- Replace doctor rules that require raw WebGL2 code with rules that forbid raw render-path
  `WebGl2RenderingContext`, `gl.compileShader`, `gl.linkProgram`, `gl.bufferData`, and
  hand-written GLSL in `src/render`.
- Specifically flip `VISUAL-BROWSER-M6` in
  `crates/xtask/src/app/doctor_visual_release/browser_probe.rs` and its regression in
  `crates/xtask/src/app/tests_07.rs`: after deletion, `Cargo.toml` must not be required to
  contain `WebGl2RenderingContext`, `WebGlProgram`, or `WebGlShader` for the renderer probe.
  The rule should instead pin the wgpu-backed WebGL2 probe exports and browser proof
  artifact shape.
- Rebaseline capability-matrix contracts affected by the backend switch:
  `tests/browser/m4_platform_smoke.html`, `tests/m4_performance_platform.rs`, and
  `tests/m9_platform_release.rs`. The live browser lane must report measured wgpu WebGL2
  adapter limits/capabilities, not stale constants copied from the old raw renderer path.
- Verify the current public API/release baseline before removing symbols. In this checkout
  `tests/m5_release.rs` freezes `docs/api.md`; v1.1.0 updates the stale doctor
  references to use that same authoritative file. Confirm that deleting the private
  `webgl2_render_cache` field does
  not alter public API; public changes are expected only around backend behavior,
  diagnostics, capabilities, docs, and release evidence.
- Update ADR-0001 references so it remains historical evidence rather than an active
  maintenance rule. Once PR 1 lands, add a `Superseded by: ADR-0002` line at the top of
  ADR-0001.

Expected commits: 4-6.

Required gates:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo run -p xtask -- doctor --full
```

Additional required proof:

- Browser rendered-output proof for `Backend::WebGl2` and `Backend::WebGpu`.
- Firefox WebGL2 proof for the ADR-0001 family.
- Repeated-prepare proof on a constrained WebGL2 target if available, specifically the
  Raspberry Pi 5 Chromium/DamagedHelmet repro.
- Capability-matrix proof for the WebGL2 lane, including whether `hardware_tier`,
  `uniform_buffers`, `fragment_high_precision`, texture limits, and compute/storage-buffer
  states changed after moving to wgpu's WebGL2 adapter limits.

Main risk:

- wgpu WebGL2 may reject some current WGSL/resource assumptions under
  `downlevel_webgl2_defaults()` (for example texture arrays, dynamic uniform offsets, depth
  state, or sampler/format limits). If that happens, the fallback must stay inside the wgpu
  path: feature-gate or simplify the material/resource shape for `Backend::WebGl2`, but do
  not reintroduce raw GL rendering or fork wgpu/naga.

### PR 2 - Decide OBJ feature fate

Goal: remove the hand-written OBJ parser or replace it with a maintained parser.

Preferred path if OBJ is still public API:

- Add optional `tobj` dependency behind `obj`.
- Preserve `Assets::load_geometry()` API.
- Map `tobj` output into `GeometryDesc`.
- Keep `tests/m3a_app_features.rs::obj_feature_load_geometry_parses_triangle_faces`.
- Add one negative test for unsupported/non-triangulated/material-heavy OBJ behavior that the
  new parser reports clearly.

Preferred path if OBJ is out of renderer scope:

- Deprecate the feature first if semver requires it.
- Remove `src/assets/obj.rs`, `obj = []`, feature docs, and doctor requirements in the next
  breaking release.

Expected commits: 1-2.
Estimate: 0.5-1 day.

### PR 3 - Retire browser mini-renderers

Goal: stop maintaining JS/WGSL/GLSL renderer fragments in browser smoke tests.

Self-contained changes:

- Keep at most a tiny context availability probe for WebGPU/WebGL2.
- Move actual pixel/render assertions to the Rust/WASM renderer probe (`src/browser_probe` and
  `tests/browser/m6_rust_wasm_renderer_probe.*`), or create equivalent M0/M2/M4 Rust-backed
  probes.
- Delete hand-written JS shader/program setup from:
  - `tests/browser/m0_browser_surface_smoke.html`
  - `tests/browser/m2_browser_lighting_clipping_smoke.html`
  - `tests/browser/m4_platform_smoke.html`

Expected commits: 1.
Estimate: 0.5-1 day.

### PR 4 - Optional CLI parser cleanup

Goal: remove tiny hand-written CLI parsing only if the CLI grows.

Self-contained changes:

- Use `serde_json` for dry-run JSON instead of manual `json_escape`.
- Adopt `lexopt` or `clap` only if the command gains more options/subcommands.
- Keep `tests/m5_release.rs::scena_convert_cli_reports_fbx_to_gltf_plan`.

Expected commits: 1.
Estimate: 0.5 day.

### PR 5 - Optional proof-hash standardization

Goal: standardize visual/probe artifact hashes only if artifact schemas are changing anyway.

Self-contained changes:

- Replace FNV-style proof hashes with `sha2` in tests/artifacts, or document FNV as the
  artifact contract and leave it.
- Update baselines in the same commit.

Expected commits: 1.
Estimate: 0.5 day.

## Open questions

- Should `Backend::WebGl2` remain a public backend choice after PR 1? Recommendation: yes.
  The implementation should change; the public compatibility lane should not disappear in a
  patch/minor release.
- Does wgpu 29.0.3's WebGL2 backend accept the current `output_shader.wgsl` and bind-group
  layout under `downlevel_webgl2_defaults()`? This must be proven in the browser lane before
  deleting the raw fallback.
- If wgpu WebGL2 rejects current material batching, should WebGL2 use a simpler wgpu material
  resource shape? Recommendation: yes, but only through wgpu resources and WGSL, not raw GL.
- Is the OBJ feature part of the stable v1 surface? If yes, replace the parser. If no, mark it
  deprecated and remove it only in a breaking release.
- Do M0/M2/M4 browser smoke tests need to remain independent of the Rust renderer? If yes,
  keep a minimal context smoke. Do not keep separate mini-renderers unless they catch a class
  the production renderer probe cannot catch.
- Which public API baseline is authoritative for the PR 1 deletion branch?
  Resolved in v1.1.0: `docs/api.md` is authoritative; the stale
  `docs/api/m5-public-api-baseline.txt` and `docs/api/m5-semver-baseline.toml`
  references were removed from release-contract doctor checks.
- Should WebGL2 capability values remain conservative constants, or should the browser lane
  report live wgpu adapter limits after PR 1? Recommendation: measured wgpu limits should win;
  stale raw-renderer-era constants should not be treated as release proof.
- Should doctor add a new rule after PR 1 that fails on `WebGl2RenderingContext` usage under
  `src/render`? Recommendation: yes, with an allowlist for non-render capability probes if
  any remain.
- Is `webgl2_shader_text.rs` present on another branch or in an unpublished local change? It
  is absent in this checkout, so the deletion plan targets `webgl2_program.rs` as the shader
  owner.
