# ADR-0001 — WebGL2 shader linker failure: vertex/fragment precision mismatch on shared uniforms

Status: Accepted
Date: 2026-05-15
Affects: scena 1.0.1 (regression introduced before 1.0.0 release; first observed externally during downstream Firefox + Playwright headless visual proof)
Resolved in: scena 1.0.2

## Problem

`scena`'s WebGL2 rendering path failed to link the output shader on Firefox WebGL2, with the verbatim `gl.getProgramInfoLog` text:

```
Uniform `camera_position_exposure` is not linkable between attached shaders.
```

This surfaces in `scena` as:

```
prepare failed: GpuResourceUpload {
    backend: WebGl2,
    reason: "Uniform `camera_position_exposure` is not linkable between attached shaders.",
}
```

Chromium's WebGL2 linker does not flag this, which is why CI on `linux-browser-webgl2` (Chromium + SwiftShader) passes. Firefox's stricter conformance flags it.

The failure was blocking the trust-platform consumer's headless Firefox visual-proof gate.

## Architectural finding

The original bug report assumed `scena`'s WebGL2 path emits GLSL via `naga` from `src/render/gpu/output_shader.wgsl`. **It does not.** scena ships **two parallel shader paths**:

| Backend                   | Shader source                                     | Compilation path                      |
| ------------------------- | ------------------------------------------------- | ------------------------------------- |
| Vulkan / Metal / DX12     | `src/render/gpu/output_shader.wgsl` (WGSL)        | naga via wgpu                         |
| WebGPU                    | `src/render/gpu/output_shader.wgsl` (WGSL)        | naga via wgpu                         |
| **WebGL2**                | `src/render/gpu/webgl2_program.rs` (GLSL ES 3.00) | raw `gl.compileShader` / `linkProgram` |

The hand-written GLSL strings (`VERTEX_SHADER`, `FRAGMENT_SHADER`) are compiled and linked through `WebGl2RenderingContext` directly (`super::webgl2_program::compile_shader`, `link_program`). `naga`'s emission style is **not** the source of the bug.

## Root cause

GLSL ES 3.00 establishes different default precision qualifiers per stage:

- **Vertex stage:** implicit `precision highp float; precision highp int;` (§4.5.3).
- **Fragment stage:** no default for float — must be declared explicitly. `webgl2_program.rs:40` declares `precision mediump float;`.

When a uniform is declared in **both** stages with the same name, the linker requires matching precision qualifiers across stages. Unqualified declarations inherit each stage's default.

`webgl2_program.rs` declared four `vec4` uniforms in the vertex stage that are **also** declared in the fragment stage:

| Uniform                        | Vertex use | Fragment use | Implicit precision (V → F) |
| ------------------------------ | ---------- | ------------ | -------------------------- |
| `camera_position_exposure`     | (none)     | yes          | `highp` → `mediump`        |
| `color_management`             | (none)     | yes          | `highp` → `mediump`        |
| `base_color_uv_offset_scale`   | (none)     | yes          | `highp` → `mediump`        |
| `base_color_uv_rotation`       | (none)     | yes          | `highp` → `mediump`        |

Firefox reports the linker error for the first such mismatch it encounters in declaration order (`camera_position_exposure`, line 17 of the vertex shader). Fixing only one would expose the next.

This is candidate **#1** ("precision-qualifier mismatch across stages") from the original investigation, but located in hand-written GLSL rather than naga-emitted GLSL. Candidates #2 (naga dead-code elimination divergence) and #3 (host-side name lookup mismatch) are not applicable to this path.

## Resolution

Delete the four unused vertex-stage uniform declarations in `src/render/gpu/webgl2_program.rs`. The vertex shader never referenced them; their presence was vestigial. With the declarations removed, the fragment-stage declarations stand alone — there is no cross-stage matching requirement to violate.

This preserves computational semantics in both stages:
- Vertex stage: behavior unchanged. Removed declarations were unused.
- Fragment stage: behavior unchanged. Declarations and uses untouched.

`webgl2_program.rs:16-23` now carries a comment explaining the constraint, so future contributors who add cross-stage uniforms know to either use the fragment-stage `precision mediump float;` qualifier or add an explicit matching qualifier on both sides.

## Why not other fixes

- **Add `precision highp float;` to vertex (explicit):** no-op (matches existing implicit default). Doesn't resolve the mismatch because fragment is still `mediump`.
- **Add `precision highp float;` to fragment:** would resolve the mismatch but **changes fragment-stage precision globally**. Per the task constraint *"The fix must not change what the shader computes"*, this is rejected. (It is arguably the most semantically aligned with the WGSL path, which uses full f32, and may be revisited in a future minor release.)
- **Add `precision mediump float;` to vertex:** would resolve the mismatch but changes vertex-stage matrix arithmetic (clip-space transforms) from `highp` to `mediump`. Rejected on precision-loss grounds — distant or large-scale scenes would acquire visible vertex jitter.
- **Explicit `mediump` on each shared uniform in vertex:** keeps the dead declarations and clutters the code with qualifiers. Rejected as more invasive than removal.

## Other affected paths

- **Shadow caster shader** (`src/render/gpu/shadow.rs:60-78`): WGSL, compiled by naga. The `CameraUniform` struct contains `camera_position_exposure` as a field, but naga emits uniform blocks (not individual uniforms) and matches them at the block level, not per-field. Vertex stage uses only `camera.light_from_world` and `camera.clip_from_view` etc. — fields not used by vertex are kept in the block layout via padding fields that ensure the struct binary layout matches the host-side `output_uniform` buffer. No cross-stage link issue.
- **Native backends (Vulkan / Metal / DX12):** unaffected. They use the WGSL path via naga and naga's GLSL emitter is not in the link chain.
- **WebGPU:** unaffected. Same WGSL path; the linker is internal to wgpu's WGSL→WGSL/SPIR-V pipeline, which doesn't have the GLSL precision-qualifier concept.

## Verification

Before fix, against `tests/browser/m6_rust_wasm_renderer_probe.html` driven by Playwright Firefox 1511 headless:

```
status: thrown
message: prepare failed: GpuResourceUpload { backend: WebGl2,
         reason: "Uniform `camera_position_exposure` is not linkable between attached shaders." }
```

After fix, same harness:

```
status:           passed
backend:          WebGl2
draw_calls:       1
gpu_submissions:  1
pixel_source:     canvas-readback
pixels.nonblack:  880 / 4096
pixels.center:    [4, 224, 4, 255]   (green triangle visible)
```

The triangle workflow renders correctly and the reported center pixel matches the expected green-triangle color.

## Out of scope

- **Chromium-headless WebGPU blank screenshot** (Firefox/WebGPU was not tested). This appears to be a separate WebGPU-path issue unrelated to GLSL linking. Not isolated; separate investigation required.
- **Chromium-headless wgpu `CreateSurface` failure** seen during the demo-page WASM spike: also unrelated. This is a wgpu surface-creation failure with no shader compilation reached; tracked separately.

## Audit gate (regression prevention)

A unit-level test `webgl2_shaders_have_no_cross_stage_uniform_precision_mismatch` is added to `src/render/gpu/materials.rs::tests`. It uses `include_str!` to read `webgl2_program.rs` as text (the same pattern the neighbouring `webgl2_material_shader_declares_fragment_texture_transform_uniforms` test already uses), extracts the inline raw strings for `VERTEX_SHADER` and `FRAGMENT_SHADER`, and parses both for unqualified shared uniform declarations. The test fails (with a descriptive message naming the mismatched uniform and the effective precision on each side) if any uniform name is redeclared across stages with an unresolvable precision pairing.

The test lives in `materials.rs` rather than `webgl2_program.rs` so it runs as part of native `cargo test` — `webgl2_program.rs` itself is `#[cfg(target_arch = "wasm32")]`-gated and its tests only execute under `wasm-pack test`, which is not run for lib unit tests in CI today. `materials.rs` is unconditional, which means the regression check fires in every native CI lane.

Verified before/after:

- Before fix (test re-introduces a single `uniform vec4 camera_position_exposure;` in the vertex shader): `cargo test --lib webgl2_shaders_have_no_cross_stage` fails with
  `Mismatches: ["camera_position_exposure: precision mismatch (vertex `highp`, fragment `mediump`)"]`.
- After fix (vertex shader has no shared-uniform redeclarations): test passes.
