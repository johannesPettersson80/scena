# Review: source snapshot main@bea2a36 — bugs, performance, proof integrity, features

Internal review, 2026-07-16, of source snapshot `main@bea2a36`, whose Cargo
package version is 1.7.2 and which is 14 commits after tag `v1.7.2`
(`v1.7.2-14-gbea2a36`). This is not a review of the tagged release. Findings
were checked against the named source snapshot; reproducible command, fixture,
backend, result, and provenance evidence retained during the subsequent audit
is recorded in
`docs/checklists/full-repo-review-v1.7.2-remediation.md`. The original review
did not retain evidence sufficient to support claims about review-pass count,
independent reviewers, or universal verification, so those process claims are
withdrawn. Line numbers refer to the source snapshot above.

Overall verdict: the architecture is unusually disciplined — explicit
prepare/render lifecycle, fail-closed capability reporting, 45 entries exposed
by `schema_entry_rows()` at the snapshot, and a verify/repair CLI loop. The
catalog count excludes additional versioned schema literals, including the
public CLI help/version surfaces, so it is not a count of every public
contract. The problems cluster in
four places: (1) a handful of real correctness bugs, mostly in glTF import and
interactive controls; (2) systematic performance debt on the CPU/prepare path
(no spatial acceleration, no transform caching, heavy cloning); (3) a safety
net with holes — several "visual proofs" do not prove what they name, and
doctor fails open in one important case; (4) small, cheap gaps in the agent
surface that block zero-shot authoring.

---

## 1. Bugs

### Critical

**B1. `Color::from_hex` panics on multi-byte UTF-8 — crashes `validate-recipe`, `recipe render`, and the WASM host.**
`src/material/color.rs:214` — `parse_hex_srgb_value` byte-slices `&value[0..2]`/`[2..4]`/`[4..6]` after checking only byte length == 6. A 6-byte string with a multi-byte char (e.g. `"€abc"`) panics on a non-char-boundary slice. The recipe *validation* layer itself calls `Color::from_hex` on raw user strings (`src/scene/recipe/validation/authoring/resources/colors.rs:35`, `material_fields.rs:25`, `targets/lights.rs:100`, `targets/extras.rs:409`, `src/scene_host/recipe/authoring/common.rs:76`). **Reproduced:** `scena validate-recipe` on `{"schema":"scena.scene_recipe.v1","colors":{"c":"€abc"}}` → `thread 'main' panicked at src/material/color.rs:214`. The validator whose contract is JSON diagnostics aborts instead; in WASM this is an unrecoverable RuntimeError killing the host instance. Fix: parse via `value.as_bytes()` chunks or reject non-ASCII first.

### High

**B2. Hot-reload `replace_import` leaks the old import's nodes — unbounded scene growth.**
`src/scene/import/load.rs:28-38` — `replace_import` calls `import.mark_stale()` then instantiates the new asset; it never removes the old import's subtree from `scene.nodes`. The stale flag only gates import-API calls; the renderer iterates all nodes, so every documented hot-reload cycle (`hot_reload.rs` doc flow) accumulates orphaned, still-rendered geometry. `remove_import` requires `ensure_live`, so it cannot clean up after `mark_stale`. The existing proof `tests/round_d_asset_hot_reload.rs` masks this: old/new triangles overlap exactly, so the color-change assertion passes while the duplicate node goes undetected.

**B3. GLB-embedded textures collide across assets in a shared `Assets` store — asset B silently renders with asset A's pixels.**
`src/assets/gltf/textures.rs:91,137` mint cache path `memory:image-{index}.{ext}` with no per-asset namespace; `TextureCacheKey` = {path, color_space, sampler, source_format}. Two different GLBs each with embedded image index 0 (the common case) in the same slot/sampler produce identical keys; the second load cache-hits and the decode no-ops because pixels are already present (`texture.rs:295-321`). Fix: include an asset/source digest in the synthesized path.

**B4. Quantized-position assets fail to load despite KHR_mesh_quantization being advertised as supported.**
`src/assets/gltf/meshes.rs:214-231` — `read_vec3_attribute` handles F32 and *normalized* int arms only; non-normalized BYTE/SHORT positions (the default gltfpack encoding, dequant scale baked into the node transform) fall to `Ok(None)` → hard error "glTF primitive is missing POSITION attribute" (`:52`). The extension is listed as built-in-OK in `extensions.rs:54-68`. The same match silently defaults non-normalized int NORMALs to `(0,0,1)`.

**B5. Cubic-spline morph-target weight animation is silently frozen.**
`src/assets/gltf/animation.rs:108-140` reshapes CUBICSPLINE weight keyframes with `chunk_size = targets_per_keyframe * 3`, but `sample_cubic_weights` (`src/animation/sampling.rs:188-214`) expects flat `3 × keyframe_count` layout — its guard always trips → `None` → channel applies nothing. Blendshape/facial animation never moves, no error. Import constructs clips unchecked, so validation that would have caught the shape is bypassed. Fix: `chunk_size = targets_per_keyframe`.

**B6. Multi-primitive meshes: animated morph weights land on the Empty parent, never reach renderable children.**
`src/scene/import.rs:280-306` — a mesh with ≥2 primitives becomes an Empty parent + renderable children; `ImportedNode` records the Empty, so weight channels rebind to it, while rendering reads weights per child (`render/prepare.rs:193`). Animated weights freeze at initial pose. Single-primitive meshes are correct.

**B7. One NaN pointer delta permanently corrupts the orbit camera, silently. ✓✓**
`src/controls.rs:254-263`, `:360-364` — no finiteness checks; `clamp` of NaN returns NaN, so yaw/pitch/target stay NaN forever. Reachable from the public JS surface: `cameraPointerMove` → `camera_pointer_move` → `apply_camera_pointer` (`src/scene_host/camera.rs:239-255`) → `Scene::align_to` (`src/scene/view.rs:415-418`), which stores the NaN transform unvalidated. Camera renders blank with no error; only `set_camera`/framing recovers. Every other input path in the codebase validates NaN — this one is the outlier. Fix: reject non-finite deltas at `handle_pointer` and/or add a finite-check in the scene-level transform setters (closes the class for direct `Scene` users too).

**B8. Screen-space pan uses fixed world axes — wrong direction after orbiting.**
`src/controls.rs:260-263` — pan applies deltas to world X/Y regardless of yaw/pitch. Correct only at yaw=0: after 180° orbit, horizontal drag pans inverted; at ±90° it drifts along the view axis. Three.js `OrbitControls` (stated parity target) pans along camera right/up. Tests only assert `target != before` (`tests/scene_host.rs:2986-2997`), leaving direction unpinned.

### Medium

**B9. Presentation timeline `animation_clip` without `end_seconds` breaks after clip end.**
`src/scene_host/presentation_timeline.rs:261-265` — `sample_seconds` is only clamped when the optional `end_seconds` is set; past clip duration, `seek_animation` (`src/scene_host/animation.rs:112-116`) rejects the patch → the mixer freezes at the last sub-duration sample and every subsequent tick emits a `failed[]` entry. Fix: clamp to clip duration on the default path.

**B10. `RendererStats` under-report GPU memory/pipelines once post/MSAA/depth-color activate.**
Stats are seeded only at prepare (`prepare_lifecycle.rs:314`, `gpu/prepare_resources.rs:268-330`), but bloom/DoF/MSAA8/depth-color targets and pipelines are created lazily during `render()` (`gpu/draw.rs:55-231`, `gpu/draw_surface.rs:54-101`) and never fold back into stats; retoggle also bypasses `pending_destructions` accounting. For a project whose differentiator is capability/resource diagnostics, the numbers are wrong exactly when the heavy features are on.

**B11. Anchor/connector local transforms not normalized to meters for non-meter imports.**
`src/scene/import.rs:387-398` (anchors: converted to *import* units, not meters) vs `:444-448` (connectors: raw, no conversion) while node transforms are meter-scaled (`options.rs:35-49`). Correct for meter sources (the default); a `with_source_units(Millimeters)` import misplaces anchors/connectors ~1000×, and anchors/connectors diverge from each other.

**B12. Node and import handles share one numeric namespace — silent wrong-node.**
`src/scene_host/core.rs:101-104` — `node_handles` and `import_handles` both use generation base 1 (instances get 1_000_000, animations 6), so the first import handle equals the first node handle as u64; `resolve_node` (`core_handles.rs:10-18`) has no namespace check. Passing an import handle to `set_transform`/`frame_node` moves the wrong node silently. `docs/errors.md:64-69` overclaims that codes "distinguish wrong handle namespaces."

**B13. Morph targets without a POSITION delta are dropped → index misalignment.**
`src/assets/gltf/meshes.rs:99-113` — `filter_map` drops normals-only targets, shifting the remaining targets against `mesh.weights` and animation channels. Morph TANGENT deltas are ignored entirely.

**B14. `SceneHostCore::set_transforms` batch is not atomic when an instance-root handle is stale.**
`src/scene_host/transforms.rs:37-91` — plain-node handles are preflighted, but instance-root handles are only range-classified; existence is checked after plain-node updates already applied. A batch `[(valid_node, T1), (stale_instance, T2)]` applies T1 then errors — contradicting the preflight-then-apply contract the single-handle path preserves.

**B15. KTX2 color-space gate rejects valid slot/container combinations.**
`src/assets/texture_ktx2.rs:133-145` — hard error when the container's `is_srgb` flag differs from the slot-derived color space; per glTF the slot dictates interpretation and most loaders reinterpret. Explicit error (not silent-wrong), but stricter than the ecosystem.

### Minor

- **B16.** `GeometryDesc::polyline` `assert!`s on <2 points (`src/geometry.rs:223`) — panic in a public constructor, inconsistent with the module's `try_new`/`GeometryError` style; the recipe construction layer has no guard of its own (`src/scene_host/recipe/authoring/geometry/construction.rs:252-262`), so the panic is one refactor away from the JSON surface.
- **B17.** Removing one overlay node directly orphans its sibling: `src/scene/removal.rs:138-141` drops the measurement/callout registry entry but leaves the other node in the scene, unmanageable via the overlay id afterward.
- **B18.** `scena-convert --dry-run` hand-rolled JSON escapes only `\` and `"` (`src/bin/scena-convert.rs:137-139`) — a newline/tab in a path emits invalid JSON on the declared machine surface. Also `src/bin/scena.rs:47` panics on EPIPE (`scena … | head`).
- **B19.** wasm `poll_device` unconditionally reports `(pending, true)` and zeroes `pending_destructions` without polling (`gpu/readback.rs`) — public diagnostics report work that did not happen.
- **B20.** The import path bypasses clip validation via the unchecked clip constructor (`src/animation.rs:136-148`): zero-duration clips and 0-channel clips pass where the authored path rejects; quantized skin weights are not re-normalized after dequant (`meshes.rs:87`).

### Documentation bugs (first-run killers)

- **B21. The two primary onboarding snippets are broken.** `docs/getting-started.md:86-93` "Load a GLB" does not compile (`load_scene` is async, `?` on a `Future`; `frame_import` takes `(camera, &import)`, the snippet passes one arg and never creates a camera). `docs/getting-started.md:51-77` "first scene" compiles but places the camera at the origin *inside* the 1×1×1 cube — a degenerate frame, while `examples/first_visible_render.rs` does it right. Nothing compile-gates doc snippets. Also `getting-started.md:17` pins `scena = "1.5"` (actual 1.7.2).
- **B22. Stale checklist (reverse direction).** `docs/checklists/next-release-easy-use-and-state-of-the-art.md` still tags LTC area lights (L2231) and tiled light culling (L2243) `[deferred]` and SSR `[reopened]` — all three shipped by v1.7.2. Doctor pins depend on checklist text staying truthful.

---

## 2. Proof-integrity gaps (the safety net has holes)

These are not renderer bugs; they are places where a real regression would pass CI/doctor silently. Given the project's fail-closed philosophy, they matter as much as the bugs.

**S1. The headline PBR proof renders nothing in the default/CI lane. ✓✓**
`tests/m8_real_asset_proof.rs:341,603` early-return + `release_evidence=false` unless env-gated; the "Blender agreement" test (`:167-176`) compares two *committed* PNGs — the renderer is never invoked; the `SCENA_REFERENCE_DIFF` golden diff appears nowhere in `.github/`. The only live WaterBottle render in CI is macOS Metal with ±35 Chebyshev region tolerances and no reference diff.

**S2. Round E material-identity proofs validate committed PNGs, not live renders.**
`tests/examples_visual_proof.rs:668,879` and `round_e_material_contract.rs` assert chrome p99/p05/dynamic-range on git-tracked goldens no test regenerates. Chrome could flatten to gray plastic and every assert still passes. (The known "artifact exists ≠ spec satisfied" failure class.)

**S3. m3a/m3b/m7/measurement "visual proofs" assert only `nonblack_pixel_count > 0`.**
`tests/m3b_visual_proof.rs:30-34` (skin+morph+animation, 48×48), `m3a_visual_proof.rs:32-36`, `m7_visual_proof.rs:39-43`, `measurement_visual_proof.rs:38-41`. Dead morphs, half-drawn instancing, collapsed skinning, or a vanished measurement leader line all pass. The native-res label/line legibility verifier already exists (`src/render/quality/metrics.rs`) and is wired only into `label_text.rs` and CLI recipe verify.

**S4. "browser_rendered_output" tests are canvas-2D roundtrips; m6 "parity" asserts no parity.**
The m1/m3a/m3b browser tests render on CPU then prove a `put_image_data`→`get_image_data` roundtrip is lossless. `tests/m6_browser_renderer_parity.rs:45-50` renders WebGL2 but asserts only nonblack>0 + draw_calls==1 — the CPU frame it should match is never consulted.

**S5. The m2 lighting/shadow/AA/bloom/SSAO proof samples 3 midline pixels; the frame hash is recorded but never asserted** (`tests/m2_visual_proof.rs:50,77-82`). Most of each effect's footprint is unsampled.

**S6. Doctor's retired-doc exemption fails OPEN.**
`crates/xtask/src/app/doctor_docs.rs:338-344,373-382` — `require_contains` swallows missing files under `docs/specs|checklists|decisions|api|benchmarks/` (prefix match). Deleting `docs/checklists/m2-lighting-depth-clipping.md` (pinned by 16 contract sites) voids those contracts with zero findings. `require_markers` fails loudly on the same class — two helpers with opposite missing-file semantics.

**S7. The per-PR WebGPU browser lane can go green without rendering.**
`SCENA_BROWSER_ALLOW_UNAVAILABLE=1` is set in CI itself (`ci.yml:133`) and converts `NoAdapter`/zero-non-black into accepted results (`tests/browser/m6_rust_wasm_renderer_probe.js:462-477`). Undocumented — not in CLAUDE.md's env-flag table, and invisible to the `TESTS-ENV-FLAGS-DOCUMENTED` check, whose scanner only reads Rust `env::var` in top-level `tests/*.rs` (`runner.rs:396-414`). Release staging catches it later (`stage_artifacts.rs:261-273`), PR CI does not.

**S8. GPU→CPU fallback keeps lanes green.**
`tests/m9_platform_release.rs:694-700` silently falls back to CPU render, recording `host_gpu_available=false`; GPU proof is required from macos-metal + windows-dx12 only (`review_artifacts.rs:271-274`) — a linux-vulkan-specific GPU-init regression shows zero red anywhere. The same skip-shape (`return;` on missing adapter) gates `pbr_brdf_parity.rs:39`, the `label_text` GPU arms, `transmission_parity`, and `dynamic_transform_parity` on non-lavapipe hosts.

**S9. Weak oracles elsewhere:** the SSIM metric has only a negative test (`tests/scena_cli_recipe.rs:1673`) — no live render is ever asserted ≥ threshold; the ICC feature has zero rendered-output assertions; doctor's env-flag/no-ignore scans are non-recursive (they miss `tests/support/`, all of `src/`, and JS `process.env`), and `src/render/quality/tests/frame_reference.rs:48` ignores a test depending on an out-of-repo fixture directory.

**S10. `doctor --full` currently fails on clean main.** Four pre-existing `ARCH-KISS-SIZE` findings (verified with this document absent): `src/bin/scena/args.rs` (523), `src/render/quality/tests.rs` (534), `src/scene/recipe/validation/expectations.rs` (701), `src/scene/recipe/validation/imports.rs` (511) — all past the 500-significant-line split threshold. Since failing doctor blocks release-readiness, these four files need splitting before the next release.

**Doctor structural note:** doctor is a pure static substring checker (~1079 pin sites over 607 files; no compile, no test run). It does real work (link resolution, evidence-schema validation, known-bad fixtures that run in CI), but "contract enforced" == "string appears in file" — satisfiable by a comment. Recommended hardening, in order: explicit retired-docs allowlist + require existence of every actively-pinned doc (kills S6); recursive repo-wide env-flag audit incl. `process.env` (kills S7's invisibility); per-lane GPU-claim consistency check on `rendered-output.json` reusing the m8 software-adapter deny-list (kills S8); "pinned test fn must be a real `#[test]` in an actively-run CI target" (narrows the comment-satisfies-pin hole).

**Test-proof hardening, in order:** (1) one non-env-gated 256² CPU golden of WaterBottle in default `cargo test` (kills S1's "default lane renders nothing"); (2) run Round E thresholds on live renders, keep PNGs as artifacts (S2); (3) replace nonblack>0 with the existing region-quality verifier + a base-pose-vs-seeked differential assert for animation (S3); (4) make m6 an actual CPU↔WebGL2 pixel-delta comparison (S4); (5) one positive SSIM golden + a `SCENA_REQUIRE_PARITY=1` mode in CI lanes so adapter outages fail instead of skip (S8/S9).

---

## 3. Performance

Context: scena flattens the scene into per-triangle `Primitive`s at prepare (world-baked vertices + per-triangle matrices ≈ 380 B each). Costs scale with T = triangles, N = nodes. The CPU backend has **no incremental path** — any transform change triggers a full re-bake (`prepare_lifecycle.rs:157` gates the dynamic path on `self.gpu.is_some()`).

Ranked by impact:

**P1. O(3T × T) brute-force shadow rays in prepare — the worst asymptotic cost in the codebase.**
`src/render/prepare/shadows.rs:172-225`: `directional_shadow_factor`/`area_shadow_factor` linearly scan **all** occluder triangles (Möller–Trumbore, no BVH/grid) for **every corner of every triangle** (`prepare/primitives.rs:170-193`), ×16+ emitter samples for area lights. ~1e9+ ray tests for a 20k-tri scene; triggered on the GPU backend too whenever any material lacks a GPU slot or an area light exists. Fix: BVH/uniform grid over occluders per prepare + a per-vertex (not per-corner-per-triangle) shading cache — the shared-vertex dedup alone is ~6×.

**P2. Wholesale cloning of prepared geometry. ✓✓**
Per full prepare, the primitive list is materialized ~4× (`prepare_lifecycle.rs:246-276`: retained → drawn → gpu_retained → gpu → depth + instances); per CPU frame, `draw_cpu` clones primitives/strokes/labels again purely to satisfy borrowck (`src/render/cpu_render.rs:28-37` — verified). A 100k-tri scene moves ~150–200 MB of heap per prepare and ~40 MB per frame before any pixel work; each triangle also carries 128 B of per-node-identical matrices. Fix: split `Renderer` fields so frame buffers borrow disjointly from `prepared` (kills the per-frame clone outright); share lists via `Arc<[PreparedPrimitive]>`; store matrices per node, indexed.

**P3. Asset access: deep clones + a global mutex per texel sample.**
`src/assets.rs:363-365` — `Assets::geometry()` returns a full `GeometryDesc` clone (all vertex/index/morph buffers); called 2× per mesh per prepare, again in shadow collection (×2), and per mesh **per pointer event** in picking. `assets.rs:444-449` locks the storage mutex per texture sample; the CPU bake does ~15 samples per shaded corner, each bilinear tap doing 3 sRGB→linear transfer evaluations (`texture.rs:365-380`). Fix: return `Arc<GeometryDesc>`/`Arc<TextureDesc>`, resolve textures once per mesh, add a 256-entry sRGB LUT.

**P4. World transforms recomputed from scratch everywhere.**
`src/scene/transforms.rs:50-67` allocates a Vec and walks to root per call; visibility walks the chain again; the render path has ~34 traversal call sites. Worst case `prepare_retained.rs:68-90` runs walk + visibility + 2 matrix builds **per retained triangle** on every dynamic prepare. Fix: one O(N) top-down cached pass keyed on `transform_revision`+`structure_revision`.

**P5. MikkTSpace tangents regenerated per full prepare** for geometries without authored tangents, on world-transformed copies (`prepare/primitives.rs:124-134`, `prepare/tangents.rs:17-67`) — tens of ms for 100k-vertex meshes, re-paid on every structure/appearance change (and every transform change on CPU). Rigid transforms do not change the tangent basis. Fix: generate once per `GeometryHandle` in model space, cache in `Assets`, rotate per use.

**P6. Picking is brute force — and the module doc claims BVH.**
`src/picking.rs:314-377` iterates every triangle of every mesh, 3 (6 for instances) `transform_point` calls per triangle, no mesh-AABB early-out despite `GeometryDesc.bounds` existing; `picking.rs:1` says "triangle/BVH tests". Combined with P3's per-event geometry clone: a hover over a 100k-tri scene ≈ 300k quaternion rotations + a multi-MB clone per pointer-move. Fix: inverse-transform the ray into local space once per mesh, bounds test first, add a cached per-geometry BVH.

**P7. CPU texture bake explosion.**
`prepare/cpu_bake.rs:22-65`: any textured material → 48² = 2304 sub-triangles per source triangle, each fully shaded; `vec![corners]` heap-allocs per triangle; transmission thickness samples are evaluated eagerly before checking the material is transmissive (`primitives.rs:333-342`); `vertex_colors()`/`render_material_slot` (a linear scan over slots) are loop-invariant but computed per triangle. Fix: hoist invariants, gate transmission on factor>0, make subdivision adaptive to screen size.

**P8. Serial where rayon should be.**
rayon is used only in the CPU raster pass. Single-threaded: the whole per-mesh bake (shading+shadows+MikkTSpace) and the IBL prefilter (`prepare/environment_baker.rs:156-207`: up to 768 GGX samples/pixel × 6 faces × mips + a 64×64×1024-sample BRDF LUT) — seconds of one-core stall on first environment use. Both embarrassingly parallel (P3's mutex is the prerequisite). The rayon row-band split also re-runs full per-triangle setup per band (≤8×); bin triangles by band first.

**P9. Native `render()` always does a synchronous readback + two full device stalls — even when presenting to a window.**
`render_to_frame` is the only native path (`render.rs:311-357`); it unconditionally does `copy_texture_to_buffer` + `map_async` + `device.poll(wait)` (`gpu/readback.rs:44-77`), then `poll_device` waits again (`render.rs:213` → `gpu/lifecycle.rs:54`). Fix: skip readback unless `frame_rgba8()` is consumed; double-buffer readback for headless batch throughput.

**P10. Post/MSAA pipelines are compiled inside `render()`, not `prepare()` — an unbounded first-frame hitch and an explicit-prepare contract violation.**
The AA/bloom/DoF/SSR setters only call `mark_output_changed()` (`settings.rs:130-237,301-304`); the next `render()` compiles pipelines and allocates HDR targets mid-draw (`gpu/draw.rs:49-238`). Also each full prepare re-parses the big concatenated WGSL module 6+ times (`gpu/pipeline.rs:305-312`, `gpu/prepare_resources.rs:209-240`) — create the shader module once, reuse across variants; consider `wgpu::PipelineCache`. Fixing this also fixes B10 (stats become complete).

**P11. Assorted hot-path waste:** animation sampling linearly scans keyframes per channel per frame (`src/animation/sampling.rs` — `partition_point` is a drop-in; `sample_weights` also allocates per sample); the occlusion-culling prepass approaches a full extra software depth pass and runs even for GPU backends (`culling.rs:16,51-108`); ~4 redundant full 4×4 cofactor inversions per triangle in GPU encode (`gpu/vertices.rs:72-73,160-165` — two computed just to test `.is_some()` and discarded); per-frame `Vec::new` for supersample/SSR/transmission scratch (`render.rs:322`, `cpu_render.rs:250-344`); instance-set dedup is O(K²) with full record-vector equality (`gpu/instancing.rs:120-137`); WASM `render()` returns a serde_json `String` every frame and `setTransforms` JSON-parses per call (`wasm.rs:292,513`, `wasm_transforms.rs:52-113`); import animation rebind is O(channels × nodes) (`import.rs:196-206`); data-URI textures use the full multi-MB base64 string as a BTreeMap key (`textures.rs:80`); absent COLOR_0/TEXCOORD_0 still allocate full-length WHITE/zero vectors (`meshes.rs:60-72`).

**P12. None of the above is benchmarked.** `m4_performance_platform.rs` measures capability flags and a *skipped* frame; `m9_platform_release.rs:1821-1870` times cold prepare + unchanged-scene render on tiny scenes. No `benches/` directory. Every hotspot above can regress invisibly. Add: "move one node → prepare+render" (CPU and GPU), 100k-tri pick, textured CPU prepare, animation advance; assert allocation counts on the dynamic path (the counting-allocator infrastructure already exists).

---

## 4. Features

The audited scena surfaces are already differentiated: the 45 catalog entries,
render introspection, SSIM/DeltaE2000 quality reports, visibility diagnosis,
repair plans, placement previews, a fuzzy-suggesting validator with
`code/severity/path/help/suggestion/auto_fixable`, and a zero-to-PNG-to-verified
CLI loop. This review did not perform a dated, official-documentation feature
matrix against Three.js, Babylon, Bevy, model-viewer, rerun, or other products,
so it makes no universal uniqueness claim. The proposals below extend the
scena surfaces actually inspected, agent-surface first.

### Agent-surface quick wins (small, do first)

**F1. Field-level `schema get` + `scena vocab`.** `schema get` today returns only entry + one example + one invalid example (`src/schema_catalog.rs:55-66`); closed vocabularies (primitive kinds, presets, place verbs, framing presets, easings, tonemappers) are discoverable only via error help strings. Emit a generated JSON-Schema/field table (types, required, enums, ranges — schemars or hand tables) and a `scena vocab` dump. Biggest single win for zero-shot recipe authoring. (S/M)

**F2. `scena recipe build --dry-run`.** The build manifest (`scene_recipe_build.v1`: id→handle map, skipped, diagnostics) is only produced by `recipe render`, which requires `--out <png>` (`src/bin/scena/recipe.rs:161-222`). Agents need the handle map before rendering. The build path exists; stop before render. (S)

**F3. `scena place --apply`.** `place` returns a transform preview the agent must hand-splice into recipe JSON. Emit the updated recipe or a `visual_patch.v1` directly. Closes place→apply→render. (S/M)

**F4. Fix onboarding + declare per-command output schemas.** Rewrite the two broken getting-started snippets from `examples/first_visible_render.rs` (B21), compile-gate doc snippets, add `"emits": [...]` per command to `--help`, and document that `inspect`/`render`/`diagnose` return `asset_doctor.v1` on load failure (`src/bin/scena/input.rs:187-202`) — today an agent deserializing the declared schema breaks on the polymorphism. Also make policy sandbox roots queryable up front. (S)

**F5. Multi-view / turntable / clip-sequence capture.** `scena render --views front,top,right,iso` (+ contact sheet), `--turntable N`, `--clip <name> --frames N`. Single-view verification hides 3D errors; four canonical views per iteration is the cheapest possible boost to agent self-verification, and generates doc GIFs. All ingredients exist (`OrthographicCamera`, `capture_contact_sheet_rgba8`, per-sample frame rendering in verify_animation). (S/M)

### Differentiators (medium)

**F6. Semantic AOVs: per-pixel node-ID map + depth/normal passes.** `--aov id,depth,normal`: a paletted node-ID image + JSON legend mapping color→stable host handle. This is the "semantic screenshot" no renderer offers as an agent contract — pixel-exact occlusion/coverage answers, useful to downstream digital-twin auditing and CAD-inspection consumers, and the deterministic CPU rasterizer makes the first implementation cheap. The roadmap already notes per-node fragment coverage as future. (M)

**F7. Scene/recipe semantic diff + attributed visual diff.** `scena diff a.json b.json [--render]`: node/material/camera-level structural diff plus pixel-diff regions attributed to nodes via F6's ID map. "What visually changed, and was it only what I intended" is a missing verb in scena's inspected agent iterate loop and a useful change-auditing surface for digital-twin consumers. No cross-product uniqueness claim is made without a dated official-documentation matrix. (M)

**F8. Un-defer recipe sections for anchors/connectors/bounds/named-states.** Connector snapping is the hero feature, yet `scene_recipe.v1` fails these sections closed as `unsupported_feature` (`docs/checklists/application-builder-roadmap.md` L698-710) — agents must run `place` previews and hand-bake transforms. All owner features shipped; this is validation + wiring. Highest payoff per line of code after F1-F5. (M)

**F9. Draco decode (`KHR_draco_mesh_compression`), feature-gated.** Draco assets are currently an unsupported input the agent cannot repair itself. This review collected no failed-asset telemetry proving its frequency or priority, so implementation remains demand-driven (`application-builder-roadmap.md` L2260). Any candidate decoder needs a license, maintenance, malformed-input, deterministic, native, and WASM assessment. (M) Related import gaps worth evaluating separately: triangle-strip/fan/lines/points modes (`meshes.rs:132-140` hard-errors), a spec-gloss fallback warning, and EXT_texture_webp rebinding.

### Parity items (larger, schedule deliberately)

**F10. Point + spot shadow maps.** Capability rows, defaults, and diagnostics enums already exist (`src/diagnostics/capabilities.rs:65-70`) with no implementation behind them; the browser probe reports `"v1.x-deferred"`. Three/Babylon/Bevy all have this; industrial interiors (digital-twin scenes) are point/spot-lit. (M/L)

**F11. GPU OIT parity.** Weighted-blended OIT is CPU-only; the browser/GPU lanes — the shipped-to-users path — sort alpha surfaces. Glass/product-configurator scenes are the showcase. (L)

**F12. glTF/GLB scene export.** An explicitly deferred epic (`docs/checklists/wasm-scene-host-and-stable-contracts.md` L407-411). Strategic as the handoff format for digital-twin evidence bundles and CAD workflows; parity with Three GLTFExporter/Babylon serializers. Round-trip + no-silent-drop reporting mandated. (L)

**F13. Smaller parity batch:** section-box capping (cut solids render hollow today and capping would serve scena's `cad_inspection` profile; external-product prevalence was not audited) (M/L); KTX2 cubemap environments (browser delivery size; the current material decode path is not a cubemap implementation) (M); SDF/MSDF text (no public `LabelDesc::sdf()`/`msdf()` API exists at the snapshot, so this is new contract work) (M); linear/16-bit-or-EXR capture output (compositing + DeltaE headroom) (S/M); `scena --watch` live re-render loop on the existing hot-reload watcher (S); international text layout/shaping (currently fail-closed outside the declared surface) (L).

Also from the proof-debt ledger: the open `renderer-fidelity-dependencies.md` epics, mobile material lanes, and the transmission+IBL / compressed-asset `[proof-gap]` items gate "best renderer of its kind" claims more than any missing feature does.

---

## 5. Suggested priority order

**Batch 1 — correctness, small diffs:**
B1 hex panic (critical, reproduced), B7 NaN guard at controls + scene boundary, B5 cubic morph chunk_size, B2 replace_import leak, B3 texture cache namespace, B16 polyline assert→error, B9 timeline clamp, B12 handle namespaces, B21 getting-started fixes. Each is narrow; together they remove every known crash/corruption on the public surface.

**Batch 2 — agent-surface quick wins (F1-F5):** small code, direct hit on the agents-use-it-easily goal.

**Batch 3 — performance foundation:** P2 borrow-split (kills the per-frame clone), P3 Arc + sRGB LUT, P4 transform cache, P1 shadow BVH + per-vertex cache, P6 picking bounds/BVH, P5 tangent cache — plus P12 benchmarks *first* so the wins are measured and pinned.

**Batch 4 — proof integrity:** S1 default-lane CPU golden, S3 region-verifier wiring + animation differential, S6 doctor fail-closed registry, S7 env-flag audit, S4 real m6 parity.

**Batch 5 — differentiators:** F6 AOVs → F7 diff (F7 builds on F6), F8 recipe sections, F9 Draco; then the parity ladder (F10-F13) as demand dictates.
