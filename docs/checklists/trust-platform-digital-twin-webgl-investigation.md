# Trust-platform digital twin WebGL investigation

Status: Investigation only
Date: 2026-05-16
Applies to: scena 1.1.0, trust-platform trust-twin browser proof
Evidence baseline: clean scena `v1.1.0` checkout at `e174bd3`, current dirty trust-platform checkout, no production code or tests changed

## Summary

The trust-platform stop is justified. The current green downstream proof is not proving
the reported hard path. It runs Scena WebGL2 and uses newly-created unlit override
materials, while the failing path uses source glTF/PBR material handles.

There are three likely root causes, not one:

1. WebGL2 PBR is still explicitly degraded in Scena. The browser PBR probes prove small
   synthetic cases, but Scena still reports `ForwardPbrDegraded` because dense glTF
   material, texture, and IBL output is not production-proven.
2. Missing browser external-image bytes can still become a quiet material fallback. This
   remains the Finding #2 chain described in the focused report.
3. Depth is load-bearing and currently fragile. In v1.1.0 the depth prepass is skipped
   for single primitives, and any line/wire/edge primitive can disable the prepass for the
   whole scene. The trust-platform `RENDER-DEPTH-SENTINEL` and edge-overlay experiment are
   therefore suspicious, not reassuring.

## Evidence

### Finding 1: WebGL PBR path is not usable for this proof yet

Verdict: confirmed.

Scena 1.1.0 reports all `forward_pbr` capabilities as degraded:

- `src/diagnostics/capabilities.rs:242-247` emits `ForwardPbrDegraded` with the message
  that PBR remains degraded until GPU material, texture, and IBL shading are proven.
- `src/diagnostics/capabilities.rs:330-332` returns `CapabilityStatus::Degraded` for
  `forward_pbr_status`.
- `tests/m4_performance_platform.rs:64-165`, `tests/browser/m4_platform_smoke.html:127`,
  and xtask doctor rules pin that degraded claim.

There are browser PBR probes, but they are narrow synthetic proofs:

- `src/browser_probe/workflows/pbr.rs:13-222` renders simple boxes for point, spot,
  normal-map, environment, and shadow-visibility checks.
- `tests/browser/m6_rust_wasm_renderer_probe.js:454-478` runs those workflows, and
  `tests/browser/m6_rust_wasm_renderer_probe.js:302-400` validates sampled pixels.

Those probes do not prove UR10/Schunk/YCB/table glTF source materials in a dense
industrial scene. They also do not make the public capability flip to Supported.

Downstream-specific note: the trust-platform screenshot analyzer keys on override colors.
It searches for orange box pixels and cyan jaw pixels
(`scripts/trust_twin_robot_cell_playwright.mjs:798-804`) and fails with
`missing gripper/wrist flange pixels` when those color predicates disappear
(`scripts/trust_twin_robot_cell_playwright.mjs:829-838`). Source glTF materials can fail
that gate even when geometry is present, because source material colors are not the
compiler-authored cyan/orange overrides.

### Finding 2: glTF source materials/textures cannot be trusted in the WebGL proof path

Verdict: confirmed as an unproven path; root cause still needs browser network evidence.

Trust-platform's current green path bypasses source PBR material handles:

- Asset nodes are marked material-overridden whenever `node.asset` is present
  (`/home/johannes/projects/trust-platform/crates/trust-twin-renderer/src/lib.rs:223`).
- Asset instantiation creates an override material for those nodes
  (`crates/trust-twin-renderer/src/lib.rs:396-418`).
- Even without explicit override, `instantiate_asset_node()` converts source materials to
  unlit through `asset_unlit_material()` (`crates/trust-twin-renderer/src/lib.rs:747-810`).
- `material_desc()` creates `MaterialDesc::unlit(...)`, never PBR
  (`crates/trust-twin-renderer/src/lib.rs:896-925`).

Scena does parse source PBR material handles:

- `src/assets/gltf/materials.rs:32-42` maps glTF metallic-roughness material factors to
  `MaterialDesc::pbr_metallic_roughness(...)` unless `KHR_materials_unlit` is present.
- `src/assets/gltf/materials.rs:43-121` wires base-color, metallic-roughness, normal,
  occlusion, and emissive texture slots.

The external-image failure path is still silent for `load_scene()` callers:

- `Assets::load_scene()` discards `AssetLoadReport` warnings
  (`src/assets/scene_loading.rs:12-15`).
- External image `NotFound`/`Io` errors become `ExternalImageMissing` warnings and the
  scene load continues (`src/assets/scene_loading.rs:193-211`).
- `parse_textures()` keeps a texture record with `source_bytes: None` when the relative
  image was not fetched (`src/assets/gltf/textures.rs:63-72`).
- `collect_backend_material_texture()` drops textures without decoded pixels and emits no
  diagnostic (`src/render/prepare/resources.rs:296-309`).

Trust-platform does currently copy PNGs into `editors/vscode/media`, and the files exist
for YCB and table assets. But the proof is incomplete:

- The build script includes `.png` (`editors/vscode/scripts/build-trust-twin-webview.js:110-111`).
- The package smoke list omits required `.png` files
  (`editors/vscode/scripts/check-trust-twin-package-assets.js:5-33`).
- The runtime asset proof lists only top-level `.gltf` entries, not external image URIs
  (`editors/vscode/src/trustTwinPanel.ts:901-930` and
  `scripts/trust_twin_robot_cell_playwright.mjs:678-724`).

So the trust-platform proof can say "packaged asset present" without proving that Scena's
browser fetcher can retrieve `003_cracker_box_textured.png` or `table_wide.png` at the
resolved webview URL.

### Finding 3: the proof is running WebGL2, not WebGPU

Verdict: confirmed.

The current artifact records:

- `renderer_origin = "scena_webgl"`.
- Playwright Firefox reports `webgpu: false`, `webgpu_adapter: false`, and `webgl2: true`.
- The downstream renderer tries WebGPU first and falls back to WebGL2
  (`crates/trust-twin-renderer/src/lib.rs:1085-1101`).

Scena's WebGL2 path is the wgpu/naga GL backend:

- `src/render/gpu/build.rs:115-123` requests
  `wgpu::Limits::downlevel_webgl2_defaults()`.
- `src/render/gpu/build.rs:182-196` maps `Backend::WebGl2` to `wgpu::Backends::GL`.
- `src/render/gpu.rs:76-85` forces WebGL2 material binding mode to `Texture2d`, not
  `Texture2dArray`.

This means the root fix either has to make WebGL2 good enough for the proof or make WebGPU
reliably available in the webview proof environment. The current evidence does not support
assuming WebGPU.

### Finding 4: depth/prepass behavior is suspicious

Verdict: confirmed, and stronger than reported.

Triangles are depth-prepass eligible by default:

- `src/geometry/primitive.rs:7-15` and `src/geometry/primitive.rs:18-29` set
  `depth_prepass_eligible: true`.

But v1.1.0 has two fragile gates:

- `src/render/prepare/stats.rs:7` sets `DEPTH_PREPASS_MIN_PRIMITIVES` to `2`.
- `src/render/prepare/stats.rs:80-83` requires every primitive to be
  `depth_prepass_eligible`.

Line, wireframe, and edge materials are not eligible. They are converted to screen-space
line quads and explicitly marked `without_depth_prepass()`:

- `src/render/prepare/strokes.rs:169-184`.

The trust-platform sentinel uses exactly that path:

- `examples/trust-twin/robot-cell/hmi/views/robot-cell.view.toml:375-388` defines
  `RENDER-DEPTH-SENTINEL`.
- `crates/trust-twin-renderer/src/lib.rs:863-871` creates it as
  `GeometryDesc::line(...)`.
- `crates/trust-twin-renderer/src/lib.rs:882-890` assigns `MaterialDesc::line(...)`.

Therefore the sentinel does not prove dense glTF depth correctness. If it survives
preparation, it can disable the depth prepass for the entire scene under v1.1.0's
all-or-nothing gate. The edge-overlay experiment has the same problem: edge and wire
materials generate ineligible stroke primitives and can also suppress the prepass.

The parallel dirty Scena checkout contains an uncommitted depth/precision fix that points
at the same class:

- It changes `DEPTH_PREPASS_MIN_PRIMITIVES` from `2` to `1`.
- It adds a test that single-primitive GPU scenes still run the depth prepass.
- It changes depth and color shaders to use the same `camera.clip_from_world *
  world_position` transform.

That patch may close Finding #4 and part of Finding #1, but the current investigation uses
released v1.1.0 as the baseline.

### Finding 5: edge/wire technical materials are not a safe presentation workaround

Verdict: confirmed.

The implementation is a technical overlay, not a presentation material replacement:

- `src/render/prepare/strokes.rs:12-95` derives wire/edge/line primitives by appending
  screen-space line segments.
- The existing tests prove simple flat-square and small headless cases only
  (`tests/m1_geometry_materials.rs:1021-1145`).
- Doctor enforces the existence of wire/edge proof, not loaded industrial SceneAsset
  aesthetics.

Because these generated strokes are depth-prepass ineligible, adding them to the robot-cell
scene can both add pale artifact pixels and disable the depth prepass. Treat edge/wire as
a CAD inspection feature until a loaded-SceneAsset visual regression says otherwise.

## What I missed before

The focused Finding #2 report was too narrow. It correctly identified the silent external
texture failure path, but it underweighted two downstream facts:

- The current trust-platform green path is deliberately not source PBR. It creates unlit
  override materials for every asset node, so it cannot prove source material handles.
- Depth helpers are not harmless. The sentinel and edge overlay are ineligible stroke
  primitives, and v1.1.0's depth-prepass gate rejects mixed eligible/ineligible scenes.

The "blank canvas" and "posterized black/white" reports are therefore plausibly mixed:
PBR degradation plus depth precision/prepass behavior plus possibly missing external
image bytes.

## Recommended Scena fix order

1. Add the dense WebGL2 repro first.
   Use `Assets::load_scene(...)`, `SceneAsset`, and
   `scene.mesh(mesh.geometry(), mesh.material())` with the UR10/Schunk/table/YCB class of
   assets. Capture browser WebGL2 PNGs for source glTF material, unlit override, and PBR
   override on the same mesh. This is the deciding gate.

2. Fix depth before judging PBR output.
   The released depth gate is correctness-load-bearing. The regression should prove dense
   glTF meshes render without any sentinel line, prove single-primitive meshes get depth,
   and prove line/wire/edge overlays do not disable depth for unrelated opaque triangles.

3. Add browser diagnostics for missing external images.
   Keep lenient loading as the default, but surface `ExternalImageMissing` in wasm console
   and renderer diagnostics, and add a strict texture option for release proofs.

4. Close the WebGL2 PBR visual gate.
   Do not flip `ForwardPbrDegraded` until WebGL2 renders a dense textured glTF scene with
   stable, correctly colored pixels under the same proof style trust-platform needs.

5. Gate edge/wire separately.
   Add a loaded-SceneAsset edge/wire visual regression only if Scena wants to advertise
   CAD-style overlays for imported meshes. Do not use it as a fallback for investor-proof
   presentation.

## Trust-platform follow-up

These are downstream proof gaps, not Scena-only root fixes:

- Extend the package proof to include external `.png`/`.ktx2` image URIs referenced by
  packaged glTF files, not just top-level `.gltf` IDs.
- Capture browser network/console evidence for `003_cracker_box_textured.png`,
  `005_tomato_soup_can_textured.png`, and `table_wide.png` inside the VS Code webview.
- Split the Playwright analyzer into geometry-presence checks and override-color checks.
  The current cyan/orange predicates are useful for the unlit override scene, but they are
  not a neutral source-material correctness test.
- Remove `RENDER-DEPTH-SENTINEL` from the proof once the Scena depth regression is fixed.

## Open questions

- Where are the exact failed PBR/source-material screenshots and console/network logs?
  The current local `.invalid` artifacts are older procedural/webview evidence, not the
  black/white PBR failure.
- Did the failed run preserve source PBR exactly, use `asset_unlit_material()`, or use a
  branch between those states?
- Does VS Code webview CSP reject the external image fetches, or are the relative image
  URLs resolving and loading successfully?
- Is the target proof allowed to require WebGPU, or must Firefox/WebGL2 remain a supported
  investor-proof path?
- Should edge/wire overlays be presentation-grade for loaded digital-twin meshes, or only
  technical/CAD inspection aids?
