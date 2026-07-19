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
- **Temporal claims need temporal pixels.** A movement, animation, retained-state,
  or cache-correctness claim must render frame A, mutate or seek the scene,
  render frame B, and prove the rendered object's own pixels/centroid moved or
  changed as declared. Marker/DOM motion, counters, draw-call counts, hashes of
  unrelated content, and "the dynamic path was taken" are not acceptance.
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
      Follow-up floor-grid fix: `scene.grid.line_width_px` now exposes the
      grid stroke width, and the built-in grid is lifted slightly above the slab
      to avoid coplanar depth artifacts; the regression
      `scena_recipe_render_grid_floor_lines_are_antialiased_and_stable_on_cpu_and_gpu`
      measures the actual recipe grid floor through `recipe render --verify` on
      CPU and lavapipe GPU. Agent starter
      templates now emit the proven starter path (`anti_aliasing:"msaa4"`,
      `supersample:2`, `reconstruction:"tent"`, and grid
      `line_width_px >= 3.6`, with `4.0` used in fixtures) instead of the old
      FXAA/thin-grid path; the
      `scena_examples_agent_get_starter_snippets_are_authored_and_runnable`
      test went red on `primitive_scene` before the template-default fix and
      now passes together with the full `scena_cli_agent_templates` suite.
      Fresh lavapipe proof on the synced scratch tree:
      `scena_recipe_render_gpu_reconstruction_widens_dashboard_bar_and_preserves_grid_edges_without_haloing`
      passes through `recipe render` on HeadlessGpu: mesh silhouettes must widen
      under hero reconstruction, while already-AA'd stroke/grid output must
      preserve a broad ramp without haloing or contrast collapse. The
      floor-grid verifier proof still runs bad/good through `recipe render
      --verify` on both backends. Measured native-output rows:
      - Dashboard bar, `box/ss2`: intermediate 1.221 px/edge, 10 luma levels,
        transition 1.221 px, halo 0.000, contrast 0.915.
      - Dashboard bar, `gaussian/ss4`: intermediate 1.779 px/edge, 41 luma
        levels, transition 1.779 px, halo 0.000, contrast 0.915.
      - Dashboard bar, guarded small-target `gaussian/ss8`: intermediate
        1.791 px/edge, 42 luma levels, transition 1.791 px, halo 0.000,
        contrast 0.915.
      - Synthetic grid stroke, `box/ss2`: intermediate 3.778 px/edge, 117
        luma levels, transition 3.778 px, halo 0.000, contrast 0.773.
      - Synthetic grid stroke, `tent/ss2`: intermediate 3.759 px/edge, 116
        luma levels, transition 3.759 px, halo 0.031, contrast 0.734.
      - Synthetic grid stroke, `gaussian/ss4`: intermediate 2.824 px/edge, 90
        luma levels, transition 2.824 px, halo 0.060, contrast 0.682. This
        guard is preservation, not a false widening claim, because stroke AA is
        supplied by the dedicated stroke pass.
      - Synthetic grid stroke, guarded small-target `tent/ss8`: intermediate
        3.111 px/edge, 97 luma levels, transition 3.111 px, halo 0.079,
        contrast 0.618. This remains within the ss8 preservation guard, but it
        is not the default recommendation; `tent/ss2` is the proven starter
        path for grid floors.
      - Previous actual recipe grid floor, `tent/ss2`: intermediate 3.027
        px/edge, 81 luma levels, transition 3.027 px, contrast 0.113.
      - Previous actual recipe grid floor, detail crop at `line_width_px:1.8`
        (test-first red): CPU transition 2.944 px/edge, 35 luma levels, halo
        0.009, contrast 0.889. The full-frame metric passed, but the visible
        lower-floor crop was still too narrow.
      - Current actual recipe grid floor uses `line_width_px:4.0` and a
        detail-crop verifier region. The product profile requires transition
        >=3.8 px/edge, halo <=0.10, and contrast >=0.70 on the lower-floor
        crop, through `recipe render --verify` on CPU and lavapipe GPU. The
        stroke coverage ramp is 3.25 px on CPU and GPU, so the fix improves
        renderer-owned stroke AA rather than only thickening the grid. This is
        a measurable floor-grid improvement, not a claim of vector-perfect
        floor lines; perspective grid strokes remain raster output and must be
        inspected at native resolution for final hero images.
        Fresh focused remote proof (`scena-builder`, scratch tree
        `scena-reconstruction-quality`, lavapipe) generated
        `target/gate-artifacts/scena-cli-recipe-recipe-render-grid-floor-line-quality-1312500/`:
        full grid metric CPU `5.053` intermediate px/edge, 118 luma levels,
        halo `0.008`, contrast `0.889`; GPU `4.863`, 131 luma levels,
        halo `0.008`, contrast `0.889`. Lower-floor detail metric CPU `3.882`,
        40 luma levels, halo `0.000`, contrast `0.897`; GPU `3.941`, 44 luma
        levels, halo `0.000`, contrast `0.897`. The corresponding native CPU
        and HeadlessGpu PNGs show the object occluding the floor grid and the
        grid rendered as broad antialiased raster strokes.
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
  The grid-line verifier gap is closed for recipes that opt into
  `expect_quality.profile`: `scena_recipe_render_verify_checks_grid_floor_line_quality_on_cpu_and_gpu`
  runs through `recipe render --verify` on CPU and lavapipe GPU, fails a
  deliberately low-quality grid with exact `grid_line_quality_too_low`, and
  passes the proven `msaa4` + `supersample:2` + `reconstruction:"tent"` +
  `line_width_px:4.0` grid with an explicit
  `grid_line_quality_checked` coverage check. This is intentionally separate
  from generic `expect_quality.line` so perspective grid segments do not
  false-fail as `line_not_straight`. The profile baseline is composable:
  `scena_recipe_render_profile_quality_keeps_grid_floor_check_with_explicit_quality_blocks`
  went red when adding an explicit `text` block skipped the grid check, and now
  passes on CPU and lavapipe GPU.

## A2 — Reflections: SSR + reflective floor — [shipped]

The single biggest "premium product shot" lever. `chrome()` remains a
polished-metal material preset and now participates in opt-in material SSR when
`render.screen_space_reflections` is enabled. This is a screen-space product
reflection path with environment fallback at missing samples, not a caustics or
path-traced mirror model.

- [x] Screen-space reflections pass (reflective surfaces sample the rendered
      scene): roughness-aware, with a graceful fallback to IBL where SSR has no
      data (screen edges, occluded rays).
      Evidence: `render.screen_space_reflections` provides a roughness-aware
      screen-space floor reflection band with CPU/GPU parity and
      `expect_quality.reflection` proof. Material SSR is applied to
      high-metallic/low-roughness fragments in the native and WebGL2 shader
      variants, and CPU parity records the same reflection samples with edge
      fade back to the environment-lit material. The CLI proof
      `scena_recipe_render_verify_material_reflection_changes_target_pixels_on_cpu_and_gpu`
      renders two independent chrome-like targets through `recipe render
      --verify`; SSR OFF fails with exact `reflection_structure_missing`, SSR
      ON passes, and the ON/OFF target-region deltas are recorded in
      `material-reflection-delta-metrics.json` on CPU and lavapipe GPU.
- [x] A first-class **reflective ground plane** scene preset (floor reflection
      decals that show structured subject colour/detail) for studio-product
      floor reflections, independent from material SSR.
- [x] `chrome()` upgraded to claim real environment+SSR reflections once the
      pass lands (coordinate with §2.6.1 material thresholds).
- Owner: new `src/render/gpu/post/ssr*` (+ CPU reference path) +
  `src/scene/recipe` ground/floor preset.
- Proof class: reference-image ON/OFF on a reflective-floor control scene; both
  backends (CPU reference may be lower-fidelity but must exist for parity).
- Acceptance: native res — a mirror/chrome subject shows recognizable
  environment reflection (structural high-contrast reflection contract, not just
  "brighter"); the reflective floor shows the subject's reflection; SSR ON/OFF
  measurably differs. Verifier reflection-presence check (C2).

## A3 — Soft area lights (LTC rect/disc/sphere) — [shipped]

Today lighting is hard directional/point lights → hard shadows, hard speculars.
Area lights give **soft shadows and broad soft speculars** — the studio-softbox
"photographed product" look. Second-biggest realism upgrade after reflections.

- [x] LTC (linearly-transformed cosines) rect/disc/sphere area lights, with
      soft-shadow support, exposed as light presets (`AreaLight::softbox()` etc.).
- [x] Soft contact/penumbra shadows from area lights.
- Progress: the authored API/recipe surface now accepts `kind:"area"` with
  `shape:"rect"|"disc"|"sphere"` and `luminous_flux_lumens`. Prepare encodes
  area lights into a dedicated bounded GPU uniform lane and shades them with a
  deterministic finite-emitter sample set on CPU and GPU, so area lights no
  longer consume point-light capacity. The prepare path also now computes
  deterministic area-light sample visibility and feeds it into both CPU PBR
  shading and the GPU area-light shader, giving finite emitters a partial
  soft-shadow signal instead of unshadowed radiance. The sample set was raised
  from four hard visibility bands to 16 samples per emitter shape
  (`area_shadow_visibility_uses_dense_emitter_samples`) so penumbrae are not
  limited to quadrant-sized steps. Recipe verification now has an opt-in
  `expect_quality.area_light` check that measures the projected receiver at
  native resolution and fails hard/point-like emitters with
  `area_light_soft_shadow_insufficient` while broad finite emitters report
  `area_light_soft_shadow_checked`. Evidence:
  `area_shadow_visibility_is_partial_when_occluder_covers_part_of_emitter`
  and
  `triangle_shader_multiplies_area_lights_by_prepared_area_shadow_visibility`;
  `scena_recipe_render_area_light_caster_darkens_tessellated_receiver_on_cpu_and_gpu`
  runs through `recipe render --verify` on CPU and lavapipe GPU and records
  native-resolution shadow-window deltas in `area-shadow-{cpu,gpu}-metrics.json`
  (latest 16-sample proof over the measured receiver shadow window: CPU
  `0.0161`, GPU `0.0188` linear-luminance delta).
  `scena_recipe_render_area_light_broadens_specular_highlight_on_cpu_and_gpu`
  records the native-resolution soft-specular spread of a broad finite emitter
  versus a point light in `specular-spread-{cpu,gpu}-metrics.json`; latest
  measurements widen the half-peak highlight footprint from CPU `3549 → 4253`
  pixels and GPU `3508 → 4264` pixels while retaining graded luma levels.
  `scena_recipe_render_verify_checks_area_light_soft_shadow_on_cpu_and_gpu`
  proves the quality check end-to-end through `recipe render --verify` on CPU
  and lavapipe GPU, and
  `scena_recipe_render_verify_checks_area_light_soft_shadow_for_all_shapes_on_cpu_and_gpu`
  proves the same finite-emitter soft-shadow quality check for `rect`, `disc`,
  and `sphere` area lights on both backends. Dedicated fitted-table LTC
  specular evaluation now runs in the CPU reference path and both GPU PBR
  shader variants with compact tables bilinearly resampled from the public
  selfshadow/ltc_code 64x64 reference (`ltc_area_light_specular_contribution`,
  `ltc_lookup_tables`, `ltc_clip_quad_to_horizon`, `ltc_integrate_edge`).
  Test-first evidence:
  `triangle_shader_contains_ltc_area_light_specular_path_for_both_texture_layouts`
  failed before the WGSL implementation and now passes. The CPU unit
  `area_ltc_specular_matches_selfshadow_reference_probe` failed against the
  previous hand-rolled approximation (`1.242` red vs reference `0.168`) and now
  passes through the shared selfshadow/ltc_code fitted-table path, with a
  compact-table oracle pinned against the full reference within 2.5%. The recipe
  proof `scena_recipe_render_area_light_ltc_specular_matches_cpu_and_gpu`
  renders a `rect`/`disc`/`sphere` roughness sweep through
  `recipe render --verify` on CPU and lavapipe HeadlessGpu and records
  native-resolution parity in
  `area-light-ltc-cpu-gpu-parity-{rect-low-roughness,rect-mid-roughness,rect-high-roughness,disc-mid-roughness,sphere-high-roughness}.json`.
  The same focused area-light test subset keeps point-vs-area specular spread
  and receiver-shadow darkening under rendered-pixel assertions on both
  backends.
  Doctor now pins the receiver-shadow
  artifact names, the all-shapes recipe proof, the dense 16-sample emitter
  visibility test, and the shader-side area-shadow multiplier so this cannot
  regress into a decorative docs claim. Doctor also rejects a future checklist
  edit that marks A3/LTC shipped while `src/render` lacks dedicated LTC
  implementation markers, and rejects marking B2 shipped without clustered/tiled
  light-assignment source markers; the guard is pinned by
  `doctor_rejects_shipped_area_light_claims_without_ltc_or_light_assignment_source`.
  A3 itself no longer depends on the old finite-sample-only specular
  approximation.
- Owner: `src/scene/lights.rs` + `src/render/prepare/lighting.rs` + shaders
  (`src/render/gpu/output_shader*.wgsl`) + CPU lighting.
- Proof class: reference-image before/after per light shape; both backends.
- Acceptance: native res — soft specular lobe + soft shadow penumbra vs the hard
  directional baseline, measurable ON/OFF. Pairs with clustered light culling
  (B2) for many-light scenes.

## A4 — Depth of field — [shipped for CPU + HeadlessGpu]

Depth of field is now an opt-in hero-shot post-process. It uses the active
camera projection to map the recipe/Rust `focus_distance` to depth-buffer space,
then blurs away-from-focus pixels from the CPU depth frame or the HeadlessGpu
depth-colour post target.

- [x] Post-process depth-of-field (depth-derived CoC from focus distance,
      aperture f-stop, and maximum radius) on CPU and HeadlessGpu.
- [x] Render-level controls through `DepthOfFieldConfig` and recipe
      `render.depth_of_field:{focus_distance,aperture_f_stop,radius_px}`. This
      deliberately lives in render setup rather than authored camera state so a
      host can vary hero-shot post effects without mutating camera identity.
- [x] `PostProcessingReportV1` exposes `depth_of_field` and
      `dof_depth_source` (`cpu_depth_frame` or `depth_color_target`) so reports
      state which path actually supplied focus depth.
- [x] `expect_quality.depth_of_field` compares the native-resolution DoF render
      against a same-backend no-DoF baseline; it fails with exact actionable
      codes for missing DoF, insufficient background blur, weak background
      detail, or a softened focal subject.
- Owner: `src/render/output.rs`, `src/render/gpu/post/dof*`,
  `src/render/quality/depth_of_field.rs`, `src/scene/recipe` render/expectation
  fields, and `src/bin/scena/recipe/verification.rs`.
- Proof class: ON/OFF same-backend baseline; CPU and lavapipe HeadlessGpu.
- Evidence: `depth_of_field_blurs_background_and_preserves_focal_plane` proves
  the CPU reference blur; `gpu_post_passes_have_independent_quality_measurements`
  proves the HeadlessGpu post pass changes only the DoF dimension; and
  `scena_recipe_render_verify_checks_depth_of_field_on_cpu_and_gpu` runs through
  `recipe render --verify` on CPU and lavapipe HeadlessGpu, failing missing-DoF
  and wrong-focus recipes before passing the focused recipe with
  `depth_of_field_checked`.
- Remaining outside this checkbox: browser WebGPU/WebGL2 DoF evidence and
  photographic bokeh kernels beyond the current bounded box blur.

## A5 — GPU/browser parity: SSAO + physical glass — [shipped for scalar glass]

SSAO (contact shadows) and physical glass (transmission/refraction) work on the
CPU/headless, HeadlessGpu, and browser GPU lanes for the scalar material path.
This matters for **grounding** and for real glass in the
browser/trust-platform path.

- [x] GPU + WebGL2 + WebGPU SSAO (contact darkening) matching the headless
      baseline. HeadlessGpu has depth-contact pixel proof in
      `gpu_post_passes_have_independent_quality_measurements`; the browser
      SceneHost proof asserts the post-processing report advertises
      `screen_space_ambient_occlusion`, uses `ssao_depth_source:
      "depth_color_target"`, and now isolates SSAO from bloom/FXAA with
      `phase2_ssao_only_changes_rendered_pixels`, which requires one SSAO pass,
      zero bloom/FXAA passes, and a rendered-pixel delta. The grounding preset
      also asserts an SSAO pass for the contact-shadow workflow.
- [x] GPU/browser scalar physical glass: positive `transmission_factor` PBR
      materials route through the scene-color transmission pass even when
      `alpha_mode` remains opaque, and GPU shaders apply `ior`,
      `thickness_factor`, `attenuation_distance`, and `attenuation_color`.
      HeadlessGpu lavapipe evidence from
      `m8_headless_gpu_transmission_volume_ibl_capability_when_available`
      records red volume `[225,15,19]` versus blue volume `[15,87,255]`.
      Browser M6 material-preset proof now emits projected glass backdrop probes
      and the Playwright harness samples the rendered browser pixels; the
      `browser-glass-pixel-probes` check must measure bright/dark structured
      backdrop contrast through both clear and frosted glass cells instead of
      accepting metadata plus nonblack pixels.
      `transmission_texture` and `thickness_texture` remain deliberately
      unsupported on GPU/WebGL2; recipe validation rejects them and GPU prepare
      fails closed instead of sampling/dropping unbound roles. The source guard
      is pinned by `check_material_reflection_quality_contracts`, and the
      browser proof guard is pinned by `HONEST-MATERIAL-PRESETS`.
- Owner: `src/render/gpu/` SSAO + transmission passes; capability rows.
- Proof class: reference-image ON/OFF + browser-demo; capability matrix rows
  promoted from Degraded only with the scalar GPU/browser gate set.
- Acceptance: native res, GPU lane — contact shadow darkens under grounded
  objects; scalar glass transmits/refracts/tints a structured background;
  unsupported texture-volume slots fail closed. Verifier grounding/contact-
  shadow check (C4).

## A6 — Default environments + shadowed/reflective ground — [shipped]

Flat `neutral_gray`/`dark_studio` backgrounds read as basic. A good studio HDRI
+ a ground plane with contact shadow (and later SSR reflection) = instant
polish, mostly preset work.

- [x] A small set of curated scene presets tuned for product/CAD/industrial:
      `scene.preset:"product_studio"|"cad_studio"|"industrial_studio"` applies a
      matching background and bundled checked environment while explicit
      `scene.background`, `scene.environment`, and `scene.grid` still override
      the preset.
- [x] A "studio floor" preset usable from one recipe field: the same
      `scene.preset` values add a bounds-sized grid floor, optional floor
      reflection for product/industrial presets, and contact-shadow SSAO unless the
      recipe already set `render.ssao`.
- Owner: `src/scene/recipe` background/environment + bundled HDRIs (mind the
  <10 MiB publish gate; prefetch/sidecar for browser).
- Proof class: `scene_recipe_scene_presets_apply_environment_background_and_floor`
  validates/builds/renders the preset path through `SceneHostCore`, proves an
  environment, matching background, and real floor/grid drawables, and
  `scene_recipe_slice4_scene_and_render_settings_fail_closed` pins unknown
  preset names at `$.scene.preset`.

## A7 — Real/textured materials (source-backed) — [shipped; live Cloudflare proof]

Flat-coloured primitives look CGI; the textured WaterBottle already looks the
most "real." Close the §2.6.1 `Assets::material_presets()` source-backed,
texture-mapped material set (chrome, brushed steel, leather, rubber, satin,
glass) to its committed thresholds. Tracked in
`next-release-easy-use-and-state-of-the-art.md` §2.6.1 — do not duplicate; this
row is a pointer + dependency (A2/A5 unblock chrome/glass identity).

Live audit status: A7's source-backed material proof is now closed for the
current browser proof surface. On 2026-06-22 the public Cloudflare proof at
`https://scena-demo.pages.dev/proof/?sample=material-presets` served the current
`1.7.1-proof-7b4a84a725cd` proof bundle, matched the local WASM SHA-256
`020866b32f35e17a638dc7fe8f1c4f0832e6a9232bf683f0cb0863a2cb4005ea`, and
passed all twelve external-reference DeltaE2000 gates plus the material-specific
chrome, brushed steel, glass, leather, rubber, satin, clearcoat, and
neighbor-distance gates. Evidence is durable in the tree at
`tests/assets/browser-proof/round-e-cloudflare-material-proof.json`, generated
by `npm run cloudflare:materials`. The WebGL2 CI and release browser lanes run
that command through `scripts/release_lane_command.sh`, so the proof is no
longer an ignored `target/`-only artifact.

## A8 — Beveled primitive edges — [shipped for box/cylinder chamfers]

Perfectly sharp CG corners look fake; a slight bevel catches light. Add an
optional `bevel`/`fillet` parameter to box/bar/cylinder primitives.

- [x] Recipe `box` and `cylinder` primitives accept optional `bevel` or
      `fillet` aliases and generate real chamfered geometry instead of echoing
      an inert knob. Unsupported primitive kinds fail closed, and specifying
      both aliases is rejected as ambiguous. This is a deterministic flat
      chamfer, not an arbitrary CAD fillet or SDF-rounded surface.
- Owner: `src/geometry/primitive_meshes.rs` + recipe primitive params.
- Proof class: deterministic vertex/index counts, triangle winding guards, and
  recipe validation/build tests. Evidence:
  `extended_primitives_have_deterministic_counts_bounds_and_normals`,
  `built_in_triangle_primitives_are_wound_against_vertex_normals`,
  `scene_recipe_beveled_box_and_cylinder_build_with_real_geometry`, and
  `scene_recipe_rejects_inert_bevel_knobs` pass on the synced remote scratch
  tree.

---

# Part B — Performance / efficiency

scena already has: backface culling, object/frustum culling (`culled_objects`
stat), a depth prepass (currently fragile/conditionally disabled), GPU
instancing (`InstanceSet` + retained instances), a `RendererStats` system, and
the M9 4K benchmark. The visual features above ADD cost (SSR, supersample, area
lights, DoF), so perf work + perf budgets are part of "nail it," not optional.

## B1 — Frustum culling: confirm + prove — [shipped]

- [x] Confirm CPU + GPU frustum culling is correct and active; add a test that
      off-screen objects are culled (`culled_objects` rises) and on-screen ones
      are not. Make sure shadow-caster culling uses the light frustum, not the
      camera.
- Owner: `src/render/prepare/` + `src/render/gpu/vertices.rs`. Proof: structural
  (stat assertions) + a render correctness check (nothing wrongly culled).
- Evidence: `scena_recipe_render_culls_offscreen_node_without_culling_visible_node_on_cpu_and_gpu`
  runs `recipe render --verify` on CPU and lavapipe GPU, asserts a visible-only
  baseline passes, then asserts adding an offscreen declared object preserves
  the visible output, increases `nodes_summary.culled`, and fails composition
  with `visible_pixel_coverage_missing` for the offscreen declaration. The
  shadow caster path builds its light-space projection from shadow projection
  points in `src/render/prepare/shadows.rs` and is not camera-frustum culled.

## B2 — Clustered / tiled light culling — [shipped]

- [x] Cluster/tile light assignment so many-light scenes scale. GPU prepares now
      keep the fixed 16-entry uniform lane for small scenes, then switch
      directional/point/spot lights to a screen-tiled storage-buffer assignment
      when the punctual count exceeds that lane. Area lights keep their separate
      bounded lane and still fail closed above `MAX_GPU_AREA_LIGHTS`.
- Implementation: `src/render/prepare/lighting/tiled.rs` builds
  `TiledLightAssignment` records, per-tile index lists, and tile metadata from
  the active camera projection. `src/render/gpu/light_assignment.rs` owns the
  storage buffers, `src/render/gpu/output.rs` binds them in the output bind
  group, and both `output_shader*.wgsl` variants consume `light_tile_indices`
  / `tiled_light_records` in the PBR punctual-light loop. The dynamic GPU fast
  path rejects tiled-light scenes on transform changes so tile buffers cannot go
  stale.
- Evidence:
  `gpu_lighting_stats_accept_many_point_lights_for_tiled_assignment` proves the
  old fixed-lane failure no longer rejects many punctual lights, while
  `area_lights_use_separate_gpu_capacity_from_point_lights` keeps the area-light
  cap separate. The recipe proof
  `scena_recipe_render_gpu_tiled_many_point_lights_use_late_light` runs
  `recipe render --verify --gpu` on lavapipe HeadlessGpu and proves a point
  light beyond the old 16-light lane changes pixels, not just state:
  `target/gate-artifacts/scena-cli-recipe-recipe-render-gpu-tiled-light-assignment-1328929/tiled-many-point-light-blue-delta.json`
  reports baseline blue `41.7204`, late-light blue `92.8608`, delta `51.1403`
  in the probe region. The paired
  `scena_recipe_render_gpu_many_lights_use_tiled_assignment_before_truncation`
  proof asserts the same recipe class renders successfully on HeadlessGpu.
- Owner: `src/render/prepare/lighting/tiled.rs` +
  `src/render/gpu/light_assignment.rs` + `src/render/gpu/output_shader*.wgsl`.
  Proof: many-light stress scene, no dropped-light fallback, measured pixel
  contribution from the late light.

## B3 — Robust depth pre-pass — [shipped]

The prepass exists but is fragile / conditionally disabled (trust-platform
investigation). A robust default depth prepass cuts overdraw shading cost,
especially with expensive PBR/SSR/area-light shaders.

- [x] Make the depth prepass robust and default-on for opaque geometry (handle
      mixed eligible/ineligible scenes without disabling for the whole scene);
      keep it correct with transmission/transparency ordering.
- Owner: `src/render/gpu/depth.rs` + prepare. Proof: prepass execution is
  reported by renderer stats, and a high-overlap GPU pixel regression compares
  the default prepass path against a test-only disabled-prepass path to prove the
  pass prevents far-geometry overdraw artifacts.
- Evidence: `single_primitive_scene_still_runs_depth_prepass_on_gpu_backends`
  pins a GPU depth prepass for any eligible opaque primitive;
  `ineligible_stroke_primitives_do_not_disable_depth_prepass_for_triangles`
  pins mixed eligible/ineligible scenes;
  `cpu_headless_renderer_does_not_report_gpu_depth_prepass` keeps the CPU stats
  honest; `depth_prepass_prevents_later_far_triangle_from_overdrawing_near_triangle`
  proves the default GPU path keeps the near green triangle visible while the
  disabled-prepass control shows the red overdraw artifact; and the MSAA
  grid-floor occlusion proof exercises depth-tested overlays on the GPU path.

## B4 — Draw-call batching / auto-instancing — [shipped]

- [x] Auto-batch/instance repeated geometry+material (e.g. the dashboard bars,
      twin pumps, a grid of parts) so N identical nodes are one instanced draw,
      not N draws. (GPU instancing exists for imported `InstanceSet`; extend to
      scene-authored repeats.) Shipped conservatively in GPU prepare:
      repeated visible ordinary mesh nodes with identical geometry/material and
      no morph/skin deformation are emitted as a render-only `PreparedInstanceSet`;
      CPU reference rendering remains on ordinary primitives, and render-only
      auto batches force a full reprepare instead of fabricating a scene
      `InstanceSetKey` for dynamic updates.
- Owner: `src/render/prepare/` batching + `src/render/gpu/`. Proof: draw-call
  proof uses prepared GPU instance batches because `RendererStats.draw_calls`
  currently counts primitive aliases, not exact GPU draw batches. Evidence:
  `repeated_mesh_nodes_auto_instance_on_gpu_prepare_path` fails before the
  grouping layer (`prepared.instances.len()==0`) and passes after with one
  instance batch containing all three repeated mesh-node transforms while the
  CPU prepared scene still has no instance batches.

## B5 — Level of detail (LOD) — [shipped]

- [x] Recipe-authored mesh nodes can declare `lods[]` entries that switch to a
      cheaper declared geometry resource when the node's projected bounds fall
      below `max_screen_fraction`. This is a real render-time LOD chain over
      authored/imported geometry handles; it is not automatic mesh
      simplification and it does not infer glTF extension LOD metadata.
- [x] LOD levels validate fail closed: thresholds must be finite and in
      `(0, 1]`, geometry references must exist, and the public
      `Scene::set_mesh_lods` API rejects invalid thresholds instead of silently
      filtering them into a no-op.
- Owner: `src/scene/lod.rs` + `src/render/prepare.rs` +
  `src/scene_host/recipe/authoring/nodes.rs`. Proof:
  `scene_recipe_lod_selects_lower_triangle_geometry_when_small_on_screen`
  renders the same recipe near/far and asserts prepared triangles drop from the
  high-detail geometry to the low-detail geometry; `scene_recipe_rejects_invalid_lod_levels`
  and `scene_set_mesh_lods_rejects_invalid_thresholds_without_silent_drop` pin
  fail-closed validation/API behavior.

## B6 — Occlusion culling — [shipped]

- [x] Conservative screen-space occlusion culling in the prepare path for
      opaque, depth-prepass-eligible triangles. The pass builds a capped
      software depth buffer from front-to-back prepared triangles and removes a
      candidate only when every covered sample is already behind accepted
      opaque depth. Transparent primitives, helper overlays, strokes, labels,
      and clipped/section-box scenes do not participate, avoiding the known
      false-cull classes.
- [x] The public `gpu_culling_dispatches` counter remains `0`; this is not the
      future GPU compute/Hi-Z/query path. It is a renderer-owned CPU prepare
      cull that benefits CPU and GPU backends before draw/upload.
- Owner: `src/render/culling.rs` + `src/render/prepare_lifecycle.rs`. Proof:
  `cpu_occlusion_culling_drops_fully_hidden_opaque_triangle` shows a fully
  hidden back triangle is removed before draw with no pixel leakage, while
  `cpu_occlusion_culling_keeps_partially_visible_triangle` proves partially
  visible geometry is not culled.

## B7 — Prepare/render caching + allocation gates — [shipped]

- [x] Audit the prepare→render split for per-frame allocations and redundant GPU
      uploads; cache prepared state across frames where the scene is unchanged
      (retained instances already exist — extend). Add allocation/no-leak gates
      to the benchmark.
- Owner: `src/render/prepare*` + `src/render/gpu/prepare_resources*`. Proof:
  correctness and allocation budgets are separate gates. Rendered-pixel
  correctness is pinned by
  `transform_only_gpu_prepare_updates_draw_uniforms_without_recollecting_primitives`
  and `transform_only_cpu_prepare_moves_rendered_mesh_pixels`: both render a
  frame, mutate a mesh transform, render again, and assert the rendered mesh
  centroid moved. The GPU test also asserts the dynamic draw-uniform update path
  was exercised instead of silently full-preparing. Retained stroke transform
  correctness is pinned by
  `line_geometry_gpu_prepare_updates_draw_uniforms_without_recollecting_strokes`,
  and animation correctness by
  `transform_animation_gpu_prepare_uses_dynamic_path_without_recollecting_primitives`.
  Visibility cache correctness is pinned by
  `visibility_middle_primitive_reencodes_batches_without_vertex_reupload`,
  which now renders before/after hiding a retained middle primitive and asserts
  rendered coverage drops, not just retained vertex ranges or counters.
  Imported glTF mesh-node transform correctness is pinned by
  `imported_gltf_transform_gpu_prepare_moves_rendered_pixels_via_dynamic_path`,
  which asserts retained draw uniforms update without static resource rebuilds
  and the rendered pixels move. `scena verify animation --expect-change` now
  records selected-node `rendered_centroid_css_px`,
  `rendered_coverage_px`, `summary.rendered_movement`, and fails frozen
  selected-node coverage with exact `rendered_node_coverage_frozen`; the
  source-level regression
  `report_fails_when_selected_rendered_coverage_is_frozen` and the CLI golden
  pin the public verifier surface. Browser/public-demo movement is pinned by
  `npm run cloudflare:demo`, which builds/serves the public demo in the
  WebGL2 CI and release lanes and runs
  `assertConnectorRenderedPixelsMoveDuringReplay`; the check samples the
  connector canvas before/mid/later during replay and fails if only the
  marker/DOM overlay moves. Allocation/performance remains guarded by
  `tests/m9_platform_release.rs`, `tests/fixtures/m9-baselines.json`, and
  `apply_benchmark_baselines`, which fail missing or exceeded
  `p95_allocations_per_frame` / `max_allocations_per_frame` budgets.

## B8 — CPU rasterizer throughput — [shipped]

The CPU path (the default, and the trust-platform/WASM fallback) is software and
slow at high resolution.

- [x] Parallelize the CPU rasterizer across row bands and keep the warmed
      render path allocation-bounded. The first implementation spawned scoped
      threads every frame and the dedicated benchmark caught it: warm parallel
      CPU renders allocated 39 times/frame. The fix uses the persistent Rayon
      worker pool, skips the parallel path for tiny primitive counts, and reuses
      CPU supersample scratch buffers across frames.
- Owner: CPU rasterizer modules. Proof: measured 4K frame-time reduction, output
  identical within tolerance. Focused proof:
  `m9_parallel_cpu_render_has_low_steady_state_allocations` went red at 39
  allocations/frame and now passes; `m9_cpu_supersample_render_reuses_steady_state_scratch_buffers`
  pins the MSAA/SSAA scratch-buffer case. Dedicated optimized remote proof on
  `scena-builder`, scratch tree `scena-reconstruction-quality`, command
  `SCENA_RUN_DEDICATED_4K_BENCHMARK=1 cargo test --profile perf-test --test m9_platform_release m9_dedicated_headless_4k_benchmark_writes_release_blocker_artifact -- --nocapture`:
  `headless-4k` p95 `75.956 ms`, max allocations/frame `4`, status `passed`.

## B9 — Benchmark harness + perf budgets — [shipped]

- [x] Extend the M9 4K benchmark into a small matrix (resolution × feature set:
      AA off/MSAA/SSAA, SSR on/off, area lights, DoF) writing
      `m9-benchmarks-*.json`. Each Part A feature must report its frame-time
      cost here.
- [x] Set per-scene frame-time + allocation **budgets** that gate in CI/doctor,
      so the new visual features can't silently tank perf.
- Owner: `crates/xtask` M9 lane + `RendererStats`. Proof: committed budgets;
  doctor/CI fails on regression beyond tolerance. The benchmark contract now
  gates `p95_frame_ms` and `max_allocations_per_frame`, writes explicit
  `frame_time_status`/`allocation_status`, fails artifacts with missing
  allocation budgets, and keeps the dedicated 4K lane fail-closed via
  `m9_dedicated_headless_4k_benchmark_writes_release_blocker_artifact`.
  Dedicated optimized remote proof writes
  `target/gate-artifacts/m9-platform/m9-benchmarks-4k.json` and
  `target/gate-artifacts/m9-platform/m9-benchmarks-feature-matrix.json`; both
  report `baseline_comparison.status:"passed"`. Measured rows:
  `1080p-aa-off` p95 `26.968 ms` / alloc `13`,
  `1080p-msaa4` `223.143 ms` / `14`,
  `1080p-ssaa2` `457.198 ms` / `14`,
  `1080p-ssr-on` `614.123 ms` / `6`,
  `1080p-area-lights` `34.046 ms` / `13`,
  `1080p-dof-on` `69.279 ms` / `13`,
  `4k-aa-off` `65.832 ms` / `13`,
  `4k-msaa4` `738.343 ms` / `14`,
  `4k-ssaa2` `1714.933 ms` / `14`,
  `4k-ssr-on` `2587.247 ms` / `6`,
  `4k-area-lights` `79.005 ms` / `13`, and
  `4k-dof-on` `183.953 ms` / `13`.

---

# Part C — Verifier extensions (gate every feature)

Extend `scena.render_quality.v1` so each visual feature has a check that fails
the OFF/bad case and passes the ON case (exact reason codes, native res, both
backends, realistic lit scene):

- [x] **C1 edge-AA** — geometry silhouette AA-gradient fraction; fails
      stair-stepped (FXAA-only) edges. Covers curved + thin edges, not just
      straight. Shipped + verified end-to-end on lavapipe: unsampled edges fail,
      FXAA-only fails the stricter no-sample fixture, and `msaa4` passes; real
      showcase scenes don't false-fail. **Refinement [x]:** low-contrast
      silhouettes now use the composition-projected subject region and a lower
      candidate contrast floor; product-profile default threshold is calibrated
      to `0.25`. Evidence:
      `scena_recipe_render_verify_low_contrast_geometry_edges_require_sample_aa_on_cpu_and_gpu`
      fails CPU/GPU no-AA and passes CPU `ss4` + GPU `msaa4` through
      `recipe render --verify` on lavapipe. Profile-derived geometry-edge
      checks apply to every render-quality profile, not only `product`, and
      stay active when an agent adds explicit quality sub-blocks:
      `scena_recipe_render_profile_quality_runs_geometry_edge_check_for_non_product_profiles`
      went red on the skipped `cad` profile check and now passes on CPU and
      lavapipe GPU.
- [x] **C2 reflection-presence** — a reflective surface shows structured
      high-contrast reflection (not flat); fails when SSR/IBL reflection is
      absent. Evidence: `scene.grid.reflection` is verified through
      `recipe render --verify` on CPU and lavapipe GPU:
      `scena_recipe_render_verify_fails_missing_reflection_quality_on_cpu_and_gpu`
      fails a matte floor with exact `reflection_structure_missing`;
      `scena_recipe_render_verify_passes_grid_reflection_quality_on_cpu_and_gpu`
      passes the reflected-floor preset; and
      `scena_recipe_render_verify_passes_screen_space_reflection_quality_on_cpu_and_gpu`
      proves `render.screen_space_reflections` can satisfy the same check on CPU
      and lavapipe GPU without `scene.grid.reflection`.
      `scena_recipe_render_verify_material_reflection_changes_target_pixels_on_cpu_and_gpu`
      also proves `expect_quality.reflection.target` fails SSR OFF and passes SSR
      ON for chrome-like materials, while measuring two independent material
      reflection target regions on CPU and lavapipe GPU.
- [x] **C3 depth-of-field** (advisory) — focal subject sharp while background
      blurred when DoF requested. Evidence:
      `scena_recipe_render_verify_checks_depth_of_field_on_cpu_and_gpu` renders
      the missing-DoF, wrong-focus, and focused recipes through
      `recipe render --verify` on CPU and lavapipe HeadlessGpu. It requires
      exact `depth_of_field_not_enabled`, `depth_of_field_blur_insufficient`,
      and `depth_of_field_checked` quality codes, and the verifier uses a
      same-backend no-DoF baseline so the check is not diluted by whole-frame
      presence.
- [x] **C4 grounding/contact-shadow** — contact darkening present under grounded
      objects; fails the "floating object" look. Evidence:
      `scena_recipe_render_verify_checks_contact_shadow_grounding_on_cpu_and_gpu`
      runs the bad/no-SSAO and good/SSAO recipes through `recipe render
      --verify` on CPU and lavapipe GPU, requires exact
      `contact_shadow_missing`/`contact_shadow_checked` reason codes, and loads
      the native PNGs to assert the far floor remains clean so broad SSAO
      banding cannot pass as contact shadow. Test-first red proof:
      before the CPU depth fix, the far-floor locality guard failed with mean
      luminance `0.638`, p05 `0.513`, stddev `0.077`; after normalizing CPU
      depth-buffer values to match GPU post depth, the focused remote proof
      passes and generated
      `target/gate-artifacts/scena-cli-recipe-recipe-quality-contact-shadow-grounding-1283629/`.
      Native CPU and HeadlessGpu present-case PNGs show localized contact
      darkening without the previous full-floor banding.
- [x] **C5 perf budgets** — frame-time/allocation thresholds (B9) surfaced as
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

These gate checkboxes apply to shipped rows A1–A8, B1–B9, and C1–C5. A7 is
also backed by the explicit §2.6.1 material-preset checklist and the live
Cloudflare proof artifact named above.

- [x] `cargo fmt --check` · `clippy -D warnings` (default + `scene-host,inspection`)
      · `cargo test` (×2) · `doctor --full` · `RUSTDOCFLAGS=-D warnings cargo doc`.
- [x] `cargo publish --dry-run` compressed size <10 MiB (HDRIs/fonts/LUTs add weight).
- [x] M9 benchmark within budget (B9); no allocation regressions.
- [x] Native-res CPU+GPU ON/OFF proof committed; verifier check fails-before/passes-after.
- [x] **Docs/skill updated in the same PR** (Part A items): `docs/rendering.md`,
      the "Make It Look Good" section of `docs/guides/llm-app-builder.md`, and
      `.codex/skills/scena-app-builder/SKILL.md` show the new knob in a
      recommended recipe — and recommend only paths proven on the claimed backends.
