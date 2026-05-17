# Trust-platform Finding 2 WebGL material investigation

Status: Investigation only
Date: 2026-05-16
Applies to: scena 1.1.0, trust-platform browser WebGL2 downstream use
Evidence baseline: clean `v1.1.0` checkout at `e174bd3`; no production code or tests changed

## Finding

The reported failure is most consistent with missing external glTF texture bytes in the
browser asset-fetch path, followed by a quiet fallback through prepare and the WebGL2
material pipeline.

This is not proven as the only cause without trust-platform webview network/CSP evidence,
but the scena side supports the hypothesis:

- `Assets::load_scene()` delegates to `load_scene_with_report()` and then discards the
  `AssetLoadReport` warnings (`src/assets/scene_loading.rs:12-15`).
- External image `NotFound` and `Io` fetch failures are converted into
  `AssetLoadWarning::ExternalImageMissing` and the scene load continues
  (`src/assets/scene_loading.rs:193-211`).
- The only scene-load warning type is currently `ExternalImageMissing`
  (`src/assets/load.rs:24-27`).
- Cached scene reports return `warnings: Vec::new()` on cache hits
  (`src/assets/scene_loading.rs:93-107`), so even `load_scene_with_report()` only exposes
  the missing-image warning on the uncached parse.
- Explicit texture loading has a parallel silent path:
  `fetch_optional_texture_bytes()` maps `AssetError::NotFound` and `AssetError::Io` to
  `Ok(None)` (`src/assets.rs:455-470`). There is no report object for this path.
- glTF texture parsing keeps the texture handle when the image bytes are missing:
  `parse_textures()` records `source_bytes: None` when the resolved external image is not
  in the fetched-image map (`src/assets/gltf/textures.rs:63-72`), and material slots still
  receive a `TextureHandle`.
- `collect_backend_material_texture()` only returns a backend texture when
  `TextureDesc::has_decoded_pixels()` is true (`src/render/prepare/resources.rs:296-309`).
  It does not emit a diagnostic or increment a missing-texture counter.
- GPU material upload then binds 1x1 fallback textures for missing roles
  (`src/render/gpu/material_upload.rs:11-13`, `57-89`).

This means a browser fetch failure can become: successful scene load, material with a
texture handle, texture descriptor without decoded pixels, prepared material slot with
`base_color: None`, and WebGL2 render using fallback textures.

## Load-bearing Behavior

Do not remove the lenient missing-image path blindly. It is intentional today.

The test `m8_missing_external_image_records_load_warning` asserts that a scene with a
missing external image still loads and surfaces `ExternalImageMissing` through
`load_scene_with_report()` (`tests/m8_assets_materials_ecosystem.rs:933-955`).

The test around `memory://reload-texture/scene.gltf` asserts the same cache handle can be
loaded first without decoded pixels and later promoted to decoded pixels after the image
bytes become available (`tests/m8_assets_materials_ecosystem.rs:871-912`).

The doctor also enforces the existence of this surface, not strict behavior:
`crates/xtask/src/app/doctor_m7_m8_assets/assets_materials.rs:223-250` requires
`fetch_optional_texture_bytes`, `AssetLoadWarning::ExternalImageMissing`, and `warnings`;
it does not require callers to inspect warnings or prepare to diagnose undecoded material
textures.

## Diagnostics Gap

There is no current renderer or doctor check for "material has a texture handle but the
referenced texture has no decoded pixels."

Evidence:

- `validate_material_texture_handles()` only checks that a texture handle exists in
  `Assets`; it does not check decoded pixels (`src/render/prepare/materials.rs:34-50`).
- `RendererStats::material_texture_bindings` counts material texture handles from logical
  scene state, not decoded/uploaded texture success (`src/render/prepare/resources.rs:44-85`;
  `src/render.rs:173-180`).
- `Renderer::diagnostics()` is populated from camera/precision/frustum diagnostics during
  prepare (`src/render.rs:115-119`, `225-231`). No material missing-pixels diagnostic is
  collected.
- Grep found decoded-texture browser proof in `tests/browser/m6_rust_wasm_renderer_probe.js`,
  but those probes use data URI textures, not relative external glTF image fetches.

## WebGL2-specific Review

The WebGL2 path in v1.1.0 uses wgpu/naga, but it intentionally does not use batched
`texture_2d_array` material bindings:

- `material_texture_binding_mode()` returns `Texture2d` for `Backend::WebGl2` on wasm
  (`src/render/gpu.rs:76-85`).
- `MaterialTextureBindingMode::Texture2d` does not support batching
  (`src/render/gpu/materials.rs:39-57`), so WebGL2 uses one material bind group per
  material slot plus the synthetic fallback (`src/render/gpu/materials.rs:193-223`).
- The pipeline chooses `output_shader_texture_2d.wgsl` for `Texture2d`
  (`src/render/gpu/pipeline.rs:105-123`).
- The WebGL2 shader variant binds `texture_2d<f32>` for each material role and ignores
  `material_layer_index`; that is an intentional workaround because wgpu 29's GL backend
  sampled material texture arrays as black in Chromium WebGL2
  (`src/render/gpu/output_shader_texture_2d.wgsl:58-62`, `101-132`, `155-159`).

I did not find an additional precondition in `output_shader_texture_2d.wgsl` beyond this:
WebGL2 must use per-material bind groups with `Texture2d` views. The material bind-group
layout and shader agree on ordinary `texture_2d<f32>` roles.

Other WebGL2 paths worth noting:

- WebGL2 requests `wgpu::Limits::downlevel_webgl2_defaults()`
  (`src/render/gpu/build.rs:115-123`). This is expected and not material-specific.
- The wasm render path returns `Ok(false)` on surface `Timeout`, `Occluded`, `Outdated`,
  `Lost`, or `Validation` (`src/render/gpu/draw.rs:298-306`). That can produce no new
  frame, but it would not explain "generated unlit materials render, glTF materials blank"
  unless the material change also changes timing or surface state.
- WebGL2 browser readback is not available; the browser-probe readback resource is created
  only for `Backend::WebGpu` (`src/render/gpu.rs:455-465`). Downstream PNG proof therefore
  depends on the canvas/screenshot path, not scena's readback path.

## Other Contributing Factors

### PBR can look blank even with fallback textures

Missing decoded base-color pixels do not by themselves force black. The fallback base-color
texture is white, and material uniform factors are still uploaded. A PBR material can still
look near-black if lighting, environment, normals, depth, camera, or exposure make the PBR
result dark. The trust-platform robot-cell view appears to install a directional light, so
"no lights at all" is not the leading explanation, but it remains a scene-side variable to
verify in the exact failing webview payload.

### Geometry handles are unlikely to be the browser-specific cause

`SceneAssetMesh` stores `GeometryHandle` and `MaterialHandle` (`src/assets/gltf.rs:87-94`),
and the manual pattern `scene.mesh(mesh.geometry(), mesh.material())` is valid when the
same `Assets` store is passed to `prepare_with_assets()`.

If a geometry or material handle is missing from the consuming `Assets`, prepare fails with
`PrepareError::GeometryNotFound` or `PrepareError::MaterialNotFound`
(`src/render/prepare.rs:98-114`). Tests pin those fail-closed cases
(`tests/m1_geometry_materials.rs:1452-1476`) and wrong-store typed errors
(`tests/m8_stale_handle_proof.rs:25-52`). That is not a likely silent blank-canvas path.

### WebGL2 renderer deletion did not obviously remove a required bind step

The v1.1.0 wgpu WebGL2 prepare path creates the material bind-group layout, material
resources, output bind group, draw bind group, depth pre-pass resources, and surface
pipeline before rendering (`src/render/gpu.rs:384-454`). The render path sets the output,
material, and draw bind groups through `encode_unlit_pass()` before drawing. I did not find
a missing raw-WebGL2-era bind-group step still expected by `prepare_with_assets()`.

## Downstream Evidence To Confirm

The trust-platform checkout currently contains evidence that supports the missing-external
texture chain, but it is not enough to close the root cause without browser network proof:

- The YCB glTF references `"003_cracker_box_textured.png"`.
- The file exists in the checkout next to the glTF.
- The trust-twin webview rewrites only top-level asset URIs that begin with `trust-twin/`
  before passing them to Rust; scena then resolves relative glTF image URIs by string-joining
  against the glTF directory (`src/assets/gltf/external.rs:52-60`).
- The trust-twin package proof list includes the YCB `.gltf` and `.bin`, but not the `.png`.
  That means the downstream package smoke test does not prove that the texture file ships,
  even if `.vscodeignore` currently appears to include `media/trust-twin/**`.
- The current dirty downstream renderer code has an `asset_unlit_material()` wrapper around
  imported material handles. Treat that as workaround state, not the verbatim failing
  pattern, until the trust-platform branch is reconciled.

## Recommendation

Implement fixes in this order.

1. Add browser-side visibility for external image fetch failure.
   Add a `web_sys::console::warn_1` path when `BrowserAssetFetcher` returns `NotFound` or
   `Io` for texture/image fetches, or when scene loading records `ExternalImageMissing` on
   wasm. This is the fastest way to unblock trust-platform diagnosis. It does not change
   behavior and should make the failing webview say exactly which image URI failed.

2. Add a prepare/render diagnostic for material texture handles without decoded pixels.
   Prefer a typed `DiagnosticCode` plus `Renderer::diagnostics()` or a `RendererStats`
   counter over only console output. This turns the current silent material fallback into
   a source-visible failure family and gives doctor something concrete to enforce.
   This is the most important scena-side correctness fix.

3. Add load options for strict texture handling.
   A `LoadOptions { strict_textures: bool }` or equivalent strict scene-load entry point
   should promote `ExternalImageMissing` to an error. Keep the current lenient default
   because reload-promotion tests depend on it. This is useful for downstream release
   gates and CI, but it is not the first unblocker unless trust-platform wants to fail
   scene load on missing artwork immediately.

4. Document `load_scene_with_report()`.
   The API exists and is tested, but the main docs show `load_scene()` examples and do not
   put warning handling in the happy path. Documentation helps future users but does not
   by itself fix the trust-platform blank canvas.

Trust-platform also needs a downstream check: prove the packaged VS Code webview can fetch
the PNG at the exact resolved `vscode-webview` URI, and extend its package proof to include
every external image URI referenced by packaged glTF assets.

## Open Questions

- Does the failing VS Code webview actually return 404, CSP rejection, or another fetch
  error for `003_cracker_box_textured.png`? Capture browser console and network details.
- Does the packaged extension contain the PNG files in the environment where the bug was
  observed, not just the dirty checkout?
- Does `connect-src ${webview.cspSource}` allow `fetch()` for these rewritten asset URIs
  in VS Code's Firefox/Playwright harness?
- Is the reported failure from direct PBR material reuse, from the current downstream
  `asset_unlit_material()` wrapper, or from a branch between those states?
- Does the parallel clip/depth precision fix change this symptom independently? If it does,
  re-run this investigation because the root cause may be mixed: missing textures plus
  depth/camera precision.
