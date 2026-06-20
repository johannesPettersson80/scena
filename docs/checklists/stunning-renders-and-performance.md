# Stunning renders + performance — consolidated roadmap

Created during the 2026-06 agent-surface dogfood, after the overlay-quality
saga. Goal: take scena from "correct render" to "product-photograph stunning,"
and keep it fast. This file is the single backlog for the visual-quality and
performance work identified against the renderer audit and
`next-release-easy-use-and-state-of-the-art.md` §3.1.

## Why this file exists / "nail it this time"

The overlay-quality work took ~10 round-trips because we patched symptoms one at
a time and repeatedly declared things "done" on proofs that *avoided the bug*
(glyph-only crops, dark backgrounds, straightness metrics for a smoothness
problem, crop medians that measured the scene not the pill). Every item below
carries the discipline that would have caught those.

### Non-negotiable discipline (applies to EVERY item)

- **Verify at native resolution.** Crop the region, zoom NEAREST, look. Never
  judge quality from a downscaled thumbnail or a green `ok:true`. See
  `[[feedback_judge_renders_at_native_res]]`.
- **Both backends.** CPU rasterizer AND GPU (lavapipe) must pass, within an
  engineered tolerance. A CPU-only or GPU-only proof is not acceptance.
- **ON/OFF reference pair.** A visual feature needs before/after (or ON/OFF, or
  order-invariance) reference images — a single "pretty render" only proves
  *something* rendered, not that the feature works.
- **The proof must exercise the failure condition.** Use a *realistic, lit*
  scene on a *light/neutral* background and measure the *full region* — not a
  setup that hides the defect (the dark-bg pill, the glyph-only crop). The
  known-bad fixture must FAIL the check by a meaningful margin.
- **Measure the right property.** Straight ≠ smooth; crop-median ≠ the target
  region. State exactly what each metric measures.
- **Gate it in the verifier.** Each feature adds a `render_quality.v1` check
  with an exact reason code that fails the OFF/bad case and passes the ON case,
  so it cannot silently regress.
- **One owner verifies the WHOLE bar firsthand before "done"** — not the
  implementing agent's self-proof.
- **Ship the guidance with the feature.** Every Part A item is not "done" until
  the user-facing cookbook is updated in the same change: the **"Make It Look
  Good"** section of `docs/guides/llm-app-builder.md`, the
  `.codex/skills/scena-app-builder/SKILL.md` quality guidance, and
  `docs/rendering.md`. The guide/skill must show the new knob in a recommended
  recipe and say which use-case it's for.
- **Docs must never recommend a path that isn't proven on the claimed
  backends.** A guide that recommends a knob which panics/fails on a backend it
  claims to support is a release blocker, not a doc nicety. (Live example: the
  guide currently recommends `anti_aliasing:"msaa4"` / `quality:"high"`, which
  panics on GPU — that recommendation is invalid until A1's GPU MSAA fix lands;
  if it can't land soon, steer the guide to `fxaa` + `supersample` meanwhile.)

## Status legend

`[gap]` missing · `[ergonomic-gap]` exists but no user surface · `[proof-gap]`
exists but unproven · `[deferred]` real but off critical path · `[reopened]`
back on the quality path · `[shipped]`.

---

# Part A — Visual quality (the path to stunning)

Priority order by visual payoff: **A1 → A2 → A3 → A4**, with **A5** folded in
(GPU/browser parity), then content levers **A6–A8**.

## A1 — Real anti-aliasing (MSAA + supersample) — [shipped]

Today only FXAA exists (`render.anti_aliasing ∈ {none, fxaa}`); it does not
smooth geometry edges (measured intermediate-edge fraction 0.000 on a bar
edge). Two tiers:

- [x] **Tier 1 — MSAA, the default edge AA.** GPU: multisampled colour/depth
      target (4×/8×) resolved *before* bloom/FXAA/overlays. CPU rasterizer:
      supersample-resolve to match (MSAA has no CPU analog). Smooths all
      silhouettes — straight, curved, diagonal, thin. On by default or via
      `quality:"high"`.
- [x] **Tier 2 — supersample quality tier (opt-in).** `render.supersample`
      factor (2/3/4 and guarded 8 for small captures) renders the whole frame
      at N× and downsamples, on both backends. `render.reconstruction` selects
      `box` (default), `tent`, or `gaussian` for hero stills. Removes edge
      aliasing *and* specular/highlight/texture shimmer MSAA can't touch. For
      hero shots; document the N² cost and the wider-kernel blur tradeoff.
      `tent` is the line-safe recommendation for floor grids/wireframes;
      `gaussian` is deliberately softer and must be inspected at native
      resolution.
- Owner: `src/render/gpu/pipeline.rs` (MSAA target) + `src/render/gpu/post/` +
  render-scale plumbing + `src/scene/recipe` render settings + CPU rasterizer.
- Proof class: reference-image ON/OFF, both backends.
- Acceptance: at native res, both backends — a straight bar edge, a **curved**
  edge (sphere/cylinder), a **thin** wireframe/grid line all smooth (AA
  gradient present, no stair-steps); Tier 2 additionally reduces a specular
  highlight's shimmer. Verifier edge-AA check (C1) fails FXAA-only, passes.
- Status: implemented in the agent-facing-surface working tree with CPU/GPU
  recipe proofs for hard geometry-edge quality and supersample changes across a
  curved edge, thin grid/wire line, and specular highlight.

## A2 — Reflections: SSR + reflective floor — [reopened]

The single biggest "premium product shot" lever. `chrome()` is currently
polished metal, *not* mirror — it explicitly does not claim SSR reflections, so
metals (flashlight head/bezel, BoomBox) don't reflect their surroundings.

- [ ] Screen-space reflections pass (reflective surfaces sample the rendered
      scene): roughness-aware, with a graceful fallback to IBL where SSR has no
      data (screen edges, occluded rays).
- [ ] A first-class **reflective ground plane** scene preset (floor that
      reflects the subject) — the classic studio-product look.
- [ ] `chrome()` upgraded to claim real environment+SSR reflections once the
      pass lands (coordinate with §2.6.1 material thresholds).
- Owner: new `src/render/gpu/post/ssr*` (+ CPU reference path) +
  `src/scene/recipe` ground/floor preset.
- Proof class: reference-image ON/OFF on a reflective-floor control scene; both
  backends (CPU reference may be lower-fidelity but must exist for parity).
- Acceptance: native res — a mirror/chrome subject shows recognizable
  environment reflection (structural high-contrast reflection contract, not just
  "brighter"); the reflective floor shows the subject's reflection; SSR ON/OFF
  measurably differs. Verifier reflection-presence check (C2).

## A3 — Soft area lights (LTC rect/disc/sphere) — [deferred → promote]

Today lighting is hard directional/point lights → hard shadows, hard speculars.
Area lights give **soft shadows and broad soft speculars** — the studio-softbox
"photographed product" look. Second-biggest realism upgrade after reflections.

- [ ] LTC (linearly-transformed cosines) rect/disc/sphere area lights, with
      soft-shadow support, exposed as light presets (`AreaLight::softbox()` etc.).
- [ ] Soft contact/penumbra shadows from area lights.
- Owner: `src/scene/lights.rs` + `src/render/prepare/lighting.rs` + shaders
  (`src/render/gpu/output_shader*.wgsl`) + CPU lighting.
- Proof class: reference-image before/after per light shape; both backends.
- Acceptance: native res — soft specular lobe + soft shadow penumbra vs the hard
  directional baseline, measurable ON/OFF. Pairs with clustered light culling
  (B2) for many-light scenes.

## A4 — Depth of field — [gap] (not even on the roadmap)

No `dof`/`bokeh`/`aperture`/`focus_distance` anywhere in the code. A blurred
background makes the subject pop — the top cinematic lever for hero/beauty shots
after lighting.

- [ ] Post-process depth-of-field (CoC from depth + focus distance + aperture),
      with bokeh quality adequate for product shots.
- [ ] Camera aperture / focus-distance controls in `src/scene/camera.rs` and the
      recipe (`camera.focus_distance`, `camera.aperture_f_stop` or similar);
      optional "focus on subject" auto-focus default.
- Owner: new `src/render/gpu/post/dof*` (+ CPU reference) + camera + recipe.
- Proof class: reference-image ON/OFF; both backends.
- Acceptance: native res — background measurably blurred while the focal subject
  stays sharp; focus distance changes move the sharp plane. Verifier DoF check
  (C3) optional/advisory (DoF is intentional blur).

## A5 — GPU/browser parity: SSAO + physical glass — [reopened]

SSAO (contact shadows) and physical glass (transmission/refraction) work on the
CPU/headless path but are reopened for GPU/WebGL2/WebGPU. Matters for
**grounding** (objects currently float on the GPU path — no contact shadow under
the dashboard bars / twin pumps) and for real glass in the browser/trust-platform
path.

- [ ] GPU + WebGL2 + WebGPU SSAO (contact darkening) matching the headless
      baseline.
- [ ] GPU/browser physical glass: scene-color transmission, IOR/thickness
      refraction, roughness blur, sorted transparency — to the §2.6.1 thresholds.
- Owner: `src/render/gpu/` SSAO + transmission passes; capability rows.
- Proof class: reference-image ON/OFF + browser-demo; capability matrix rows
  promoted from Degraded only with the full gate set.
- Acceptance: native res, GPU lane — contact shadow darkens under grounded
  objects; glass transmits/refracts the background per the thresholds; CPU↔GPU
  within tolerance. Verifier grounding/contact-shadow check (C4).

## A6 — Default environments + shadowed/reflective ground — [content]

Flat `neutral_gray`/`dark_studio` backgrounds read as basic. A good studio HDRI
+ a ground plane with contact shadow (and later SSR reflection) = instant
polish, mostly preset work.

- [ ] A small set of curated studio environment presets (HDRI + matching
      background gradient) tuned for product/CAD/twin.
- [ ] A "studio floor" preset (ground plane + contact shadow + optional
      reflection) usable from one recipe field.
- Owner: `src/scene/recipe` background/environment + bundled HDRIs (mind the
  <10 MiB publish gate; prefetch/sidecar for browser).
- Proof class: docs-image + reference-image; both backends.

## A7 — Real/textured materials (source-backed) — [gap, see §2.6.1]

Flat-coloured primitives look CGI; the textured WaterBottle already looks the
most "real." Close the §2.6.1 `Assets::material_presets()` source-backed,
texture-mapped material set (chrome, brushed steel, leather, rubber, satin,
glass) to its committed thresholds. Tracked in
`next-release-easy-use-and-state-of-the-art.md` §2.6.1 — do not duplicate; this
row is a pointer + dependency (A2/A5 unblock chrome/glass identity).

## A8 — Beveled primitive edges — [ergonomic-gap]

Perfectly sharp CG corners look fake; a slight bevel catches light. Add an
optional `bevel`/`fillet` parameter to box/bar/cylinder primitives.

- Owner: `src/geometry/primitive_meshes.rs` + recipe primitive params.
- Proof class: docs-image before/after. Low priority; nice polish.

---

# Part B — Performance / efficiency

scena already has: backface culling, object/frustum culling (`culled_objects`
stat), a depth prepass (currently fragile/conditionally disabled), GPU
instancing (`InstanceSet` + retained instances), a `RendererStats` system, and
the M9 4K benchmark. The visual features above ADD cost (SSR, supersample, area
lights, DoF), so perf work + perf budgets are part of "nail it," not optional.

## B1 — Frustum culling: confirm + prove — [proof-gap]

- [ ] Confirm CPU + GPU frustum culling is correct and active; add a test that
      off-screen objects are culled (`culled_objects` rises) and on-screen ones
      are not. Make sure shadow-caster culling uses the light frustum, not the
      camera.
- Owner: `src/render/prepare/` + `src/render/gpu/vertices.rs`. Proof: structural
  (stat assertions) + a render correctness check (nothing wrongly culled).

## B2 — Clustered / tiled light culling — [deferred]

- [ ] Cluster/tile light assignment so many-light scenes scale (per the roadmap;
      reopen when area lights A3 or richer scenes create many-light pressure).
- Owner: `src/render/prepare/lighting.rs` + GPU. Proof: many-light stress scene,
  stable frame-time/allocation, no dropped-light fallback (measured, not ON/OFF).

## B3 — Robust depth pre-pass — [reopened]

The prepass exists but is fragile / conditionally disabled (trust-platform
investigation). A robust default depth prepass cuts overdraw shading cost,
especially with expensive PBR/SSR/area-light shaders.

- [ ] Make the depth prepass robust and default-on for opaque geometry (handle
      mixed eligible/ineligible scenes without disabling for the whole scene);
      keep it correct with transmission/transparency ordering.
- Owner: `src/render/gpu/depth.rs` + prepare. Proof: measured overdraw/frame-time
  reduction on a high-overdraw scene + correctness (no depth artifacts), both
  backends where applicable.

## B4 — Draw-call batching / auto-instancing — [gap]

- [ ] Auto-batch/instance repeated geometry+material (e.g. the dashboard bars,
      twin pumps, a grid of parts) so N identical nodes are one instanced draw,
      not N draws. (GPU instancing exists for imported `InstanceSet`; extend to
      scene-authored repeats.)
- Owner: `src/render/prepare/` batching + `src/render/gpu/`. Proof: draw-call
  count drops for a repeated-part scene (stat assertion) + identical render.

## B5 — Level of detail (LOD) — [gap]

- [ ] Distance-based LOD (mesh simplification or imported LOD chains) so distant
      / small-on-screen meshes use cheaper geometry. Pairs with a mesh-simplify
      step (consider a proven crate per the dependency policy).
- Owner: `src/scene/` + `src/assets/`. Proof: triangle count drops with distance,
  no visible popping at tuned thresholds. Larger effort; schedule after B1–B4.

## B6 — Occlusion culling — [gap, later]

- [ ] Hi-Z / query-based occlusion culling for dense scenes (objects fully
      behind others not shaded). Deferred until scene complexity warrants it.

## B7 — Prepare/render caching + allocation gates — [proof-gap]

- [ ] Audit the prepare→render split for per-frame allocations and redundant GPU
      uploads; cache prepared state across frames where the scene is unchanged
      (retained instances already exist — extend). Add allocation/no-leak gates
      to the benchmark.
- Owner: `src/render/prepare*` + `src/render/gpu/prepare_resources*`. Proof:
  per-frame allocation count flat across frames for a static scene.

## B8 — CPU rasterizer throughput — [gap]

The CPU path (the default, and the trust-platform/WASM fallback) is software and
slow at high resolution.

- [ ] Parallelize the CPU rasterizer across cores (tiles) and/or SIMD the hot
      inner loops; measure frame-time on the M9 4K scene before/after.
- Owner: CPU rasterizer modules. Proof: measured 4K frame-time reduction, output
  identical within tolerance.

## B9 — Benchmark harness + perf budgets — [proof-gap]

- [ ] Extend the M9 4K benchmark into a small matrix (resolution × feature set:
      AA off/MSAA/SSAA, SSR on/off, area lights, DoF) writing
      `m9-benchmarks-*.json`; set per-scene frame-time + allocation **budgets**
      that gate in CI/doctor, so the new visual features can't silently tank
      perf. Each Part A feature must report its frame-time cost here.
- Owner: `crates/xtask` M9 lane + `RendererStats`. Proof: committed budgets;
  doctor/CI fails on regression beyond tolerance.

---

# Part C — Verifier extensions (gate every feature)

Extend `scena.render_quality.v1` so each visual feature has a check that fails
the OFF/bad case and passes the ON case (exact reason codes, native res, both
backends, realistic lit scene):

- [x] **C1 edge-AA** — geometry silhouette AA-gradient fraction; fails
      stair-stepped (FXAA-only) edges. Covers curved + thin edges, not just
      straight. Shipped + verified end-to-end on lavapipe: `none`→fail (0.0),
      `fxaa`→fail (0.149<0.30), `msaa4`→pass; real showcase scenes don't
      false-fail. **Refinement [ ]:** the metric is contrast-sensitive — on a
      synthetic low-contrast edge (light-gray bar on neutral_gray) it inverted
      (`fxaa` false-passed with 0 checks, `msaa4` false-failed). No effect on
      real scenes, but make the edge metric reliable across contrast levels.
- [ ] **C2 reflection-presence** — a reflective surface shows structured
      high-contrast reflection (not flat); fails when SSR/IBL reflection is
      absent.
- [ ] **C3 depth-of-field** (advisory) — focal subject sharp while background
      blurred when DoF requested.
- [ ] **C4 grounding/contact-shadow** — contact darkening present under grounded
      objects; fails the "floating object" look.
- [ ] **C5 perf budgets** — frame-time/allocation thresholds (B9) surfaced as
      gate failures.

Each: known-bad + known-good fixture, must fail-before / pass-after by a
meaningful margin, on a light/neutral realistic scene.

---

# Recommended build order

1. **A1 MSAA/SSAA** (in flight) + **C1** — foundational; every render benefits.
2. **B3 robust depth prepass** + **B1 culling proof** — cheap perf headroom
   *before* adding expensive passes.
3. **A2 SSR + reflective floor** + **C2** — biggest visual jump.
4. **A3 area lights (LTC)** + **B2 light culling** — soft studio lighting.
5. **A5 GPU/browser SSAO + glass parity** + **C4** — grounding + real glass.
6. **A4 depth of field** + **C3** — hero-shot polish.
7. **B4 batching**, **B9 budgets**, then **A6–A8** content/polish, **B5/B8** as
   scale demands.

Sequence visual + perf in pairs: land a perf-headroom item (B3/B1/B2) before or
with each expensive visual pass (A2/A3) so frame-time stays in budget.

# Gates (every PR in this backlog)

- [ ] `cargo fmt --check` · `clippy -D warnings` (default + `scene-host,inspection`)
      · `cargo test` (×2) · `doctor --full` · `RUSTDOCFLAGS=-D warnings cargo doc`.
- [ ] `cargo publish --dry-run` compressed size <10 MiB (HDRIs/fonts/LUTs add weight).
- [ ] M9 benchmark within budget (B9); no allocation regressions.
- [ ] Native-res CPU+GPU ON/OFF proof committed; verifier check fails-before/passes-after.
- [ ] **Docs/skill updated in the same PR** (Part A items): `docs/rendering.md`,
      the "Make It Look Good" section of `docs/guides/llm-app-builder.md`, and
      `.codex/skills/scena-app-builder/SKILL.md` show the new knob in a
      recommended recipe — and recommend only paths proven on the claimed backends.
