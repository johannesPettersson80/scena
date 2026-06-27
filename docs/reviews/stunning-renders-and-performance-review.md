# Review: stunning-renders-and-performance.md

Read-only adversarial review of
`docs/checklists/stunning-renders-and-performance.md`.
Standard applied (from the review brief and
`[[feedback_judge_renders_at_native_res]]`): a checked item that claims
*visual behavior* must be backed by **rendered-pixel** proof on the actual path.
Counters, logs, telemetry, `draw_calls`, `gpu_submissions`, nonblack-pixel
checks, artifact existence, and "the path was taken" do **not** count when the
claim is visual. Browser/public-demo claims need browser-rendered pixels.
Movement claims need before/after pixel movement **of the rendered object**, not
of a marker. Performance claims must also prove visible correctness.

---

## Executive Summary

**Overall verdict: NOT trustworthy as a whole — split by axis.**

- **Static visual quality (A1–A6, C1–C4) is largely trustworthy.** These items
  follow the file's own discipline: native-resolution pixel ON/OFF on CPU **and**
  lavapipe GPU, with known-bad fixtures that fail by a margin and known-good that
  pass, gated in the `render_quality.v1` verifier. They are not in the
  invalid-proof class. A handful of secondary gaps remain (below).
- **The dynamic / retained-state performance axis (B2, B4, B7, and the
  transform-only GPU fast path they extend) is NOT trustworthy.** Every proof on
  this axis is a counter / state / artifact-existence assertion. **There is not a
  single test anywhere in the tree that renders a frame, mutates a node
  *transform*, re-renders through the GPU dynamic path, and asserts the moved
  object's rendered pixels moved.** The only between-render property ever pixel-
  checked is *tint* — which the code path happens to refresh — masking the fact
  that the same path does **not** refresh a moved object's transform.
- **A source-level asymmetry reproduces the reported connector/mating freeze
  exactly** (Finding **F1**): on the dynamic fast path, ordinary (non-instanced,
  non-`model_node`) meshes keep a stale `world_from_model`, so a moved part
  renders frozen while a JS/overlay marker that reads the live transform moves.
- **A7's "live Cloudflare proof" is real browser-pixel work but is ephemeral and
  ungated** (Finding **F3**): the cited evidence artifact lives under `target/`
  (git-ignored, untracked) and the Cloudflare lane is not in CI.

**Highest-risk invalid checked items:** B7 (caching), B4 (auto-instancing), B2
(tiled lights / "tile buffers cannot go stale"), and the A7 Cloudflare claim.

**Is the connector/mating replay failure class covered? No.** No checklist item
asserts that a moving object's rendered pixels move, and no C-series verifier
gate exercises a transform change. The items most likely to have *caused* the
freeze (B7/B4/B2 + the transform-only fast path) are all marked `[shipped]` on
counter-only proofs.

---

## Repository State Reviewed

- **Local path:** `/home/johannes/projects/scena` (canonical for this review)
- **Branch:** `agent-facing-surface` (no upstream; origin has no such branch)
- **HEAD:** `929c6cd Add reconstruction filters, composition-verifier
  foundation, MSAA overlay-depth fix`
- **Dirty tree:** 237 changed/untracked entries; uncommitted files treated as in
  scope. The dynamic-path sources read below are part of the current working
  tree.
- **Remote builder / GitHub:** NOT used as source of truth. `scena-builder`
  (main @ `632e12d`, its own dirty tree) was not consulted; all source quoted
  here is from the local tree. No heavy local tests were run.

---

## Method

**Source files inspected (read in full or in the cited ranges):**

- `src/render/prepare_lifecycle.rs` — `prepare_inner`, the dynamic GPU fast-path
  branch, `dynamic_gpu_prepare_rejection_reason`, `reencode_retained_draws`.
- `src/render/prepare_retained.rs` — `filter_retained_primitives_for_scene`,
  `filter_retained_instances_for_scene`, `retained_template_covers_visible_sources`.
- `src/render/gpu/dynamic_draw_state.rs` — `update_dynamic_draw_state`.
- `src/render/gpu/vertices.rs` / `output_shader.wgsl` — draw-uniform
  `world_from_model` and vertex stage (`vs_main`).
- `src/render/prepare/primitives.rs`, `prepare/auto_instance.rs`
  (`MIN_AUTO_INSTANCE_GROUP_SIZE = 4`).
- `src/render/tests.rs`, `src/render/phase5_tests.rs`,
  `src/render/post_tests.rs`, `src/render/post_quality_tests.rs`.
- `src/render/quality/depth_of_field.rs`, `src/bin/scena/recipe/verification.rs`.
- Browser/proof harnesses: `tests/browser/m6_rust_wasm_renderer_probe.js`,
  `tests/browser/scene_host_browser_proof.js`,
  `scripts/probe_cloudflare_material_presets.mjs`, `demo/main.js`.
- CI: `.github/workflows/ci.yml`, `release.yml`; `package.json` scripts.

**Commands run:** `git` status/ls-files/rev-parse; `grep`/`sed`/`ls`/`wc` for
source navigation; inspection of `target/gate-artifacts/round-e-cloudflare-
material-proof.json` metadata (size/track status). Three read-only sub-agents
mapped test bodies for the A-, B-, and browser-proof families; **every
load-bearing sub-agent claim was independently re-verified** (two agents reported
tests "missing" that in fact exist in other files — see Findings F-note).

**Not run:** the test suite, doctor, benchmarks, wasm builds, or any live browser
session. Claims about *current rendered behavior* are therefore based on source
reading plus the existing test assertions, and are labeled as such.

---

## Connector/Mating Regression

**Does this checklist contain the item that caused it?** Not as an explicit row —
there is no "animation/replay moves rendered pixels" checkbox anywhere in the
file. **But the enabling mechanism is in scope**: the dynamic GPU fast path that
B2, B4, and B7 extend and depend on.

**Root cause (source-level, high confidence):**

1. `demo/main.js` drives the public demo replay via `replay_connector_snap(app)`
   and positions the marker from `connector_marker_positions(...)`
   (`demo/main.js:4,12,375–397`). The parts are imported glTF GLB
   (`drive_unit.glb`, `load_unit.glb`, `connector_snap_assembly.glb`).
2. Imported glTF meshes ride the **dynamic GPU fast path** — proven by the
   project's own test `eased_tint_transition_gpu_prepare_uses_dynamic_path...`
   (`src/render/phase5_tests.rs:148`), which instantiates a glTF and asserts the
   dynamic path is taken (not rejected as "model nodes present").
3. On a transform-only change, `prepare_inner` takes the dynamic branch
   (`prepare_lifecycle.rs:155–214`): it calls `reencode_retained_draws` →
   `filter_retained_primitives_for_scene`, then
   `update_dynamic_draw_state`, bumps `dynamic_template_prepares` /
   `draw_uniform_only_updates`, and returns.
4. **`filter_retained_primitives_for_scene` refreshes only the tint, never the
   transform, for ordinary primitives** (`prepare_retained.rs:72–82`: it clones
   the retained template and calls `set_tint(...)`; there is no
   `with_world_from_model(...)`). Contrast `filter_retained_instances_for_scene`
   (`prepare_retained.rs:98–108`), which **does** rebuild every record from
   `instance.transform()`.
5. The GPU vertex stage applies the transform via the draw-uniform
   `world_from_model` (`output_shader.wgsl:309`,
   `world_position = draw.world_from_model * instance_world_from_model * position`);
   `update_dynamic_draw_state` rewrites only the draw-uniform and instance
   buffers, **never the triangle vertex buffer** (`dynamic_draw_state.rs:30–44`).
   So the only way to move an ordinary mesh on this path is to refresh its
   draw-uniform `world_from_model` — which step 4 does not do.
6. A single moving part is not rescued by auto-instancing:
   `MIN_AUTO_INSTANCE_GROUP_SIZE = 4` (`prepare/auto_instance.rs:18,82`).

**Net effect:** a moved ordinary/imported mesh on the dynamic path re-renders
with its frame-0 transform — frozen — while a marker that reads the live scene
transform (or a JS/DOM overlay) moves. This is the reported symptom precisely.

**Why previous tests did not catch it:** the transform-only tests assert *only*
telemetry. `transform_only_gpu_prepare_updates_draw_uniforms_without_recollecting_primitives`
(`src/render/tests.rs:24`) moves a mesh and asserts
`dynamic_template_prepares + 1` and `draw_uniform_only_updates + 1` — it never
reads a pixel. `transform_animation_gpu_prepare_uses_dynamic_path...`
(`src/render/phase5_tests.rs:30`) **does** call `render()` after each animation
frame (lines 79–81) but asserts only `prepared_primitive_collections`,
`static_gpu_resource_rebuilds`, and `draw_uniform_only_updates` counts (lines
86–103) — the rendered frame is discarded. The one between-render *pixel* test,
`opaque_tint_gpu_prepare_updates_draw_uniforms...` (`src/render/tests.rs:152`),
mutates **tint**, which step 4 *does* refresh — so it passes and gives false
confidence in the whole path.

**Required replacement proof:** a GPU (lavapipe) + CPU test that renders frame A,
calls `set_transform` (and separately advances an animation clip) on an ordinary
**and** an imported-glTF mesh node, re-renders through the dynamic fast path, and
asserts the moved object's silhouette/centroid **moved in the rendered pixels**
by the expected direction/magnitude — plus a WASM/browser equivalent on the
demo replay path. A `render_quality.v1` / composition gate should assert
"declared-moved node's projected coverage shifted," failing the frozen case.

---

## Checklist Item Matrix

Proof-validity is judged against the item's own *visual/behavioral* claim.
"Valid (perf)" = appropriate proof for a pure performance/structure claim.
"INVALID (visual)" = visual behavior claimed but only counters/state/artifacts
proven. Findings are detailed in the next section.

| Line / Item | Status | Implementation source | Claimed proof | Valid? | Finding |
|---|---|---|---|---|---|
| L72 A1 Tier1 MSAA edge AA | shipped | gpu/pipeline, post, cpu raster | native-res edge-AA metrics CPU+GPU; C1 verify known-bad/good | **Yes** | F9 (minor) |
| L76 A1 Tier2 supersample/reconstruction | shipped | render-scale, recipe | grid+bar reconstruction metrics; GPU widen test | Mostly | F9 |
| L180 A2 SSR pass | shipped | gpu/post/ssr, cpu ref | `..material_reflection_changes_target_pixels..` on/off CPU+GPU | **Yes** | — |
| L194 A2 reflective ground plane | shipped | scene/recipe floor preset | A6 preset build test + C2 reflection | Mostly | F8 |
| L197 A2 chrome() real reflections | shipped | material presets | via §2.6.1 / A7 material thresholds | Cond. | F3 |
| L214 A3 LTC area lights | shipped | lights, prepare/lighting, wgsl | shadow/specular/LTC parity pixel tests CPU+GPU; doctor pins | **Yes** | F12 (minor) |
| L216 A3 soft contact/penumbra | shipped | prepare/lighting | `..area_light_caster_darkens_receiver..` luma-delta CPU+GPU | **Yes** | F12 |
| L291 A4 DoF post-process | shipped (CPU+HeadlessGpu) | render/output, gpu/post/dof, quality/dof | `depth_of_field_blurs_background..` (CPU px) + GPU post px + verify | **Yes** | F-br |
| L293 A4 DoF render controls | shipped | recipe render fields | config plumbing + verify | Valid (perf) | — |
| L297 A4 PostProcessingReportV1 | shipped | report types | report field exposure | Valid (report) | — |
| L301 A4 expect_quality.depth_of_field | shipped | bin/recipe/verification, quality/dof | C3 known-bad/good CPU+GPU (verifier computes from pixels) | **Yes** | — |
| L322 A5 GPU+WebGL2+WebGPU SSAO | shipped | gpu SSAO passes | headless GPU px ON/OFF; **browser = report + full-chain hash** | **INVALID (browser)** | F4 |
| L330 A5 scalar physical glass | shipped | gpu transmission, m6 probe | headless GPU px; **browser glass-pixel-probes (m6, in CI)** | **Yes** | F-br |
| L363 A6 scene presets | shipped | scene/recipe | build/render preset drawables + fail-closed | Struct. only | F8 |
| L367 A6 studio floor preset | shipped | scene/recipe | same preset build test | Struct. only | F8 |
| L380 A7 source-backed materials | shipped (header) | material presets / browser | **live Cloudflare DeltaE proof (uncommitted, not in CI)** | **INVALID (durability)** | F3 |
| L405 A8 beveled box/cylinder | shipped | geometry/primitive_meshes | vertex/index counts, winding, validation | Geometry yes; **visual no** | F7 |
| L431 B1 frustum culling | shipped | prepare, gpu/vertices | `culled` rises + composition `visible_pixel_coverage_missing` | **Yes** | — |
| L446 B2 clustered/tiled lights | shipped | prepare/lighting/tiled, gpu/light_assignment | late-light **static** px delta; "rejects on transform = no stale" is **logic-only** | Partial | F1, F10 |
| L483 B3 robust depth prepass | shipped | gpu/depth, prepare | prepass-present pins; grid occlusion | **Struct.; perf+artifact not measured** | F5 |
| L499 B4 auto-instancing | shipped | prepare batching, gpu | instance-count assertions; dynamic batches force reprepare | **INVALID (visual/dynamic)** | F1 |
| L517 B5 LOD chain | shipped | scene/lod, prepare | prepared-triangle count drops near/far | Valid (perf) | — |
| L522 B5 LOD fail-closed | shipped | scene/lod, recipe | rejects invalid thresholds | Valid | — |
| L536 B6 occlusion culling | shipped | render/culling | hidden triangle removed, no pixel leakage; partial kept | **Yes** | — |
| L542 B6 gpu_culling_dispatches==0 | shipped | stats | counter == 0 (honest negative) | Valid | — |
| L554 B7 prepare/render caching + alloc gates | shipped | prepare, gpu/prepare_resources | **allocation counters + budgets only** | **INVALID (correctness)** | F1, F2 |
| L572 B8 CPU rasterizer throughput | shipped | cpu_render | alloc counters + 4K time + **serial≡parallel pixel test** | **Yes** | — |
| L589 B9 benchmark matrix | shipped | xtask M9 | frame-time/alloc rows (remote, uncommitted) | Valid (perf) | F11 |
| L591 B9 perf budgets gate | shipped | xtask, baselines | budgets fail-closed in doctor/CI | Valid (perf) | — |
| L626 C1 edge-AA verifier | shipped | quality/geometry | known-bad/good CPU+GPU | **Yes** | — |
| L642 C2 reflection-presence | shipped | quality/reflection | known-bad/good CPU+GPU | **Yes** | — |
| L657 C3 depth-of-field verifier | shipped | quality/dof | known-bad/good CPU+GPU | **Yes** | — |
| L666 C4 grounding/contact-shadow | shipped | quality/grounding | known-bad/good + far-floor locality CPU+GPU | **Yes** | — |
| L681 C5 perf budgets verifier | shipped | xtask/baselines | = B9 | Valid (perf) | — |
| — C-series coverage of *movement* | (implied) | quality verifier | none | **Gap** | F10 |
| L710 Gate fmt/clippy/test/doctor/doc | shipped | CI/skill | process gate | Valid | — |
| L712 Gate publish <10 MiB | shipped | CI | dry-run size | Valid | — |
| L713 Gate M9 within budget | shipped | CI/doctor | budgets | Valid (perf) | F11 |
| L714 Gate native-res CPU+GPU ON/OFF committed | shipped | tests/artifacts | many metric JSONs live under `target/` | Overstated | F11 |
| L715 Gate docs/skill updated | shipped | guide/skill/rendering | doc edits | Not re-verified | — |

---

## Findings

### F1 — CRITICAL — No rendered-pixel movement proof; stale transform on the dynamic GPU fast path
- **Severity:** Critical (this is the connector/mating failure class).
- **Checklist lines:** B2 (L446–471), B4 (L499–513), B7 (L554–565); systemic
  for the transform-only fast path those rows depend on.
- **Source evidence:**
  - `src/render/prepare_retained.rs:72–82` — ordinary primitives refresh tint
    only, never `world_from_model`.
  - `src/render/prepare_retained.rs:98–108` — instances *do* refresh transform
    (asymmetry).
  - `src/render/gpu/dynamic_draw_state.rs:30–44` — rewrites draw-uniform +
    instance buffers only, not the vertex buffer.
  - `src/render/gpu/output_shader.wgsl:309` — transform applied via draw-uniform
    `world_from_model`.
  - `src/render/prepare_lifecycle.rs:155–214` — dynamic branch returns after a
    uniform-only update, bumping counters.
  - `src/render/prepare/auto_instance.rs:18,82` — single/paired moving parts are
    not auto-instanced.
- **Existing proof and why insufficient:** `src/render/tests.rs:24` and
  `src/render/phase5_tests.rs:30` assert only `dynamic_template_prepares` /
  `draw_uniform_only_updates` / `prepared_primitive_collections` counts; the
  latter even calls `render()` but discards the pixels. The one between-render
  pixel test (`src/render/tests.rs:152`) mutates **tint**, the one property this
  path refreshes. Sub-agent sweep confirmed: **zero** GPU tests mutate a node
  transform between renders and assert pixel movement.
- **User-visible impact:** any single/few moving opaque or imported-glTF mesh
  driven by `set_transform` or animation freezes on screen while overlays/markers
  move — the live Cloudflare connector/mating bug.
- **Required fix:** refresh `world_from_model`/`normal_from_model` for ordinary
  retained primitives on the dynamic path (mirror the instance path), or reject
  the fast path for moved ordinary meshes; ensure the consumed buffer is the one
  rewritten.
- **Required regression proof:** CPU + lavapipe GPU + WASM/browser test that
  moves an ordinary mesh and an imported-glTF mesh and asserts the rendered
  silhouette/centroid shifted; add a verifier/composition gate for
  "declared-moved node coverage shifted."
- **Doctor/CI coverage needed:** yes — pin the new movement test and add the
  verifier gate so the fast path cannot regress to frozen-pixels-but-green-
  counters.

### F2 — HIGH — B7 caching proven only by allocation counters, not by mutate-then-render correctness
- **Severity:** High.
- **Line:** B7 (L554–565).
- **Source/proof:** evidence is `p95/max_allocations_per_frame`, stored budgets,
  and `apply_benchmark_baselines` (allocation/frame-time only). "cache prepared
  state across frames where the scene is unchanged" has no test that mutates a
  cached scene and verifies the next frame reflects the mutation in pixels.
- **Why insufficient:** caching correctness is a *staleness* property; allocation
  counts cannot detect a stale cache. This is the same defect class as F1.
- **Impact / fix / proof:** as F1; add a "prepare, render, mutate, render, assert
  pixels changed" correctness test alongside the allocation budget.
- **Doctor/CI:** yes.

### F3 — HIGH — A7 "live Cloudflare proof" is ephemeral and ungated
- **Severity:** High (public-demo claim; durability).
- **Lines:** A7 (L380–397); gate L714.
- **Evidence:** `target/gate-artifacts/round-e-cloudflare-material-proof.json`
  exists (10.4 KB, generated 2026-06-22 10:12) but `git ls-files` reports it
  *not tracked* and `target/` is git-ignored — it vanishes on `cargo clean` and
  is not reviewable from the repo. The generator
  `scripts/probe_cloudflare_material_presets.mjs` **is** real Playwright browser-
  pixel work (live `https://scena-demo.pages.dev/proof/`, DeltaE2000 + Sobel +
  refraction-offset reads), and `package.json` exposes `cloudflare:materials`,
  **but neither `.github/workflows/ci.yml` nor `release.yml` runs it** (they run
  `browser:m6` and `browser:scene-host-proof` only).
- **Why insufficient:** the methodology is sound, but a `[shipped]` claim resting
  on an uncommitted, CI-excluded artifact is not durable proof; cf.
  `[[committed_artifacts_stand_alone]]`, `[[links_must_deliver_what_they_promise]]`.
  Also note the bug context: a green DeltaE on a *static* material grid would not
  detect the replay freeze (different scene/path).
- **Required fix/proof:** commit a small, reviewable proof artifact (outside
  `target/`) and/or add the Cloudflare lane to a scheduled/release CI job;
  reference the deployed bundle SHA in-repo.
- **Doctor/CI:** yes — gate or scheduled lane.

### F4 — MEDIUM — A5 browser SSAO is metadata + full-post-chain hash, not SSAO-isolated pixels
- **Severity:** Medium.
- **Line:** A5 SSAO (L322–328).
- **Evidence:** `tests/browser/scene_host_browser_proof.js` asserts
  `screen_space_ambient_occlusion === true` and
  `ssao_depth_source === "depth_color_target"` (report fields) and a
  `full_post_chain_changes_rendered_pixels` FNV hash of **all** effects ON vs
  OFF — SSAO is not isolated in the browser. (Headless GPU *does* isolate SSAO
  ON/OFF in pixels via `gpu_post_passes_have_independent_quality_measurements`,
  `src/render/post_quality_tests.rs:70–90+`.)
- **Why insufficient:** the browser-parity claim is "SSAO matches the headless
  baseline" on the browser lane; a whole-chain hash cannot attribute the pixel
  change to SSAO.
- **Required proof:** browser SSAO-only ON/OFF pixel delta in a contact region
  (mirror the headless isolation).
- **Doctor/CI:** strengthen the existing in-CI `browser:scene-host-proof`.

### F5 — MEDIUM — B3 depth-prepass perf reduction and artifact-free correctness not measured
- **Severity:** Medium.
- **Line:** B3 (L483–494).
- **Evidence:** cited proofs are structural pins (prepass present for eligible
  primitive; mixed eligible/ineligible scene; CPU honest stats) plus the MSAA
  grid-floor occlusion proof. Overdraw/frame-time *reduction* (the stated payoff)
  and a depth-artifact ON/OFF pixel proof are not in the cited evidence;
  `prepare/stats.rs:163–173` carries overdraw-correctness logic and
  `prepare/diagnostics.rs:61` a z-fighting *diagnostic*, but neither is a
  measured-reduction or artifact-free render proof.
- **Required proof:** a high-overdraw scene with prepass ON/OFF frame-time delta
  and a no-depth-artifact pixel assertion.

### F6 — MEDIUM — C-series verifier gates are static-only; movement axis ungated
- **Severity:** Medium (root-cause enabler for F1 slipping).
- **Lines:** C1–C5 (L626–682); "Each: known-bad + known-good" closing note.
- **Evidence:** every C check renders a single static scene/setting pair. None
  advances an animation or `set_transform` between two renders. The file's
  "Non-negotiable discipline" (L17–49) is entirely about static quality
  (native-res, ON/OFF, both backends) and never mentions temporal/movement
  correctness — which is why F1 had no gate.
- **Required fix:** add a movement/animation verifier check (see F1 required
  proof) to the discipline and the C-series.

### F7 — LOW — A8 bevel "catches light" not pixel-proven
- **Line:** A8 (L405–416). Proof is deterministic vertex/index counts, winding,
  and validation — these legitimately prove *geometry* exists, but the stated
  visual payoff ("a slight bevel catches light") has no native-res highlight
  ON/OFF render. Scope-honest as written, but the visual claim is unproven.

### F8 — LOW — A6 preset acceptance is structural, not native-res visual
- **Lines:** A6 (L363–377), and by reference A2 reflective floor (L194). Proof
  validates that the preset *produces* environment/background/floor drawables and
  fails closed on unknown names; it does not judge the preset at native
  resolution ("instant polish"). Acceptable for "preset wiring," not for the
  visual claim.

### F9 — LOW — A1 secondary tests: absolute thresholds / GPU-only / comparison-only
- **Line:** A1 (L72–170). `..grid_floor_lines_are_antialiased_and_stable..` uses
  one config with absolute thresholds and no in-test known-bad;
  `..reconstruction_widens_dashboard_bar..` is GPU-only and filter-vs-filter
  (no AA-off baseline). **Mitigated** because C1's verify test
  (`..verify_checks_grid_floor_line_quality..`) supplies a real known-bad
  (`grid_line_quality_too_low`) on both backends. Net: A1 is adequately proven;
  these two helpers alone would not be.

### F10 — LOW/INFO — B9 / "committed" gate: headline numbers and many metric JSONs live under `target/`
- **Lines:** B9 (L589–616), gate L714. The quoted benchmark rows and several
  A-item metric files (e.g. `material-reflection-delta-metrics.json`,
  `area-shadow-*-metrics.json`, the LTC parity JSON) are written under
  `target/gate-artifacts/...` and on remote scratch trees — not committed. The
  gate "Native-res CPU+GPU ON/OFF proof **committed**" is overstated for these.
  The *tests* are committed and regenerate them; the *evidence numbers* are not
  reproducible from the repo alone.

### F12 — LOW — A3 specular-spread robustness (sub-agent observation, not re-derived)
- **Line:** A3 (L240–244). `..area_light_broadens_specular_highlight..` measures
  FWHM growth in a fixed region; a uniform background brightening could in
  principle inflate FWHM without a genuinely wider lobe (peak location/value not
  pinned). Lower confidence — flagged for hardening, not asserted as broken. The
  LTC parity + shadow-darkening tests substantially cover A3.

**F-note (sub-agent reliability):** two sub-agents reported cited tests as
non-existent — `depth_of_field_blurs_background_and_preserves_focal_plane`,
`gpu_post_passes_have_independent_quality_measurements` (they exist at
`src/render/post_tests.rs:149` and `src/render/post_quality_tests.rs:13`), and
`browser-glass-pixel-probes` (exists in `tests/browser/m6_rust_wasm_renderer_probe*.js`
and is doctor-pinned). These false-missing claims were corrected on direct
inspection and are **not** findings against the checklist.

---

## Invalid Proofs

Proofs that should **not** count toward the visual/behavioral claim they back:

1. **Transform-only fast-path telemetry** (`src/render/tests.rs:24`,
   `phase5_tests.rs:30`) as evidence that moving objects render correctly —
   counters/path-usage only; pixels discarded. (F1)
2. **B7 allocation budgets** as evidence of cache *correctness* — allocation
   counts cannot detect a stale cache. (F2)
3. **B4 instance-count assertions** (`repeated_mesh_nodes_auto_instance_on_gpu_prepare_path`)
   as evidence that auto-instanced/repeated geometry renders correctly under
   motion — structure counts only. (F1)
4. **B2 "rejects tiled-light scenes on transform changes so tile buffers cannot
   go stale"** — backed by rejection-reason *logic*, no mutate-then-render pixel
   proof; and the protection does not extend to the common non-tiled moving-mesh
   case. (F1)
5. **A5 browser SSAO**: report-field assertions + full-post-chain pixel hash, as
   evidence of SSAO-specific browser behavior. (F4)
6. **A7 Cloudflare artifact** `target/gate-artifacts/round-e-cloudflare-material-proof.json`
   as committed/durable proof — untracked, git-ignored, CI-excluded. (F3)
7. **A8 geometry counts** as evidence of the "catches light" visual claim. (F7)
8. **A6 drawable-presence** as evidence of "instant polish" visual quality. (F8)
9. **B3 structural prepass pins** as evidence of measured overdraw/frame-time
   reduction or artifact-free depth correctness. (F5)
10. **"Native-res … committed" gate (L714)** where the artifacts are under
    `target/`. (F10)

---

## Required Fix Order

Public-demo / browser-visible and user-visible rendering breakages first.

1. **F1 — fix the frozen-transform dynamic path** (refresh `world_from_model`
   for ordinary retained primitives, or reject the fast path for moved ordinary
   meshes) **and add the CPU+GPU+browser rendered-pixel movement test** + a
   verifier/composition movement gate. This is the live connector/mating bug.
2. **F6 — add movement/temporal coverage to the C-series discipline** so F1
   cannot recur silently (do this with F1).
3. **F2 — add a mutate-then-render correctness test for B7 caching** beside the
   allocation budget.
4. **F3 — make A7's Cloudflare proof durable and gated**: commit a reviewable
   artifact outside `target/` and/or add the lane to scheduled/release CI.
5. **F4 — isolate SSAO ON/OFF in the in-CI browser proof.**
6. **F5 — measure B3 prepass overdraw/frame-time reduction + artifact-free
   correctness.**
7. **F7, F8, F9, F10, F12 — hardening**: bevel highlight ON/OFF; native-res
   preset quality judgement; commit/refer the metric artifacts the gate claims;
   pin A3 specular peak. Lower priority; none is a public-demo breakage.

*End of review. No production code was modified; this report is the only file
written.*
