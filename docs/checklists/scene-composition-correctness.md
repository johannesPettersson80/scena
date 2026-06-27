# Scene composition correctness — coverage-driven detection (not incident-driven)

Status: **FOUNDATION IMPLEMENTED; PRONG B COVERAGE IMPLEMENTED FOR THE CURRENT
RECIPE SURFACE; D4 CONTACT SHADOW PIXEL CHECK IMPLEMENTED.** v1 (five-defect)
was coverage-by-incident; v2 went coverage-by-construction; this v3 folds in
Codex's re-review (8 missing categories, corrected root causes, data-layer
gaps, coverage-report design, ownership/anti-drift risks). The foundation slice
now exists in `src/scene_host/composition.rs` plus focused submodules and is
wired through `recipe render --verify`. The current `expect_occlusion` proof is
a fail-closed color-probe verifier, not the future exact depth/id-mask backend;
do not claim arbitrary overlapping-object mask attribution until that precision
layer lands.

## Why this exists (read first)

Over a week of agent-facing visual work, **every visual defect was found by the
human.** Neither the agent nor `render_quality.v1` ever *autonomously* surfaced
one — it is a regression guard for known defects, not a discovery tool, and it
measures only **pixel quality** with no model of spatial/composition correctness.
The trap to avoid: "a check per defect we found" = coverage-by-incident (catches
the five we hit, blind to the sixth). This plan is coverage-by-construction.

## The coverage model — three prongs + no silent gaps

### Prong A — Spec-conformance (backbone; catches UNKNOWN misplacement)

The recipe declares intent; verify the **render matches the declared build
manifest for every element** (not a defect list). Per declared node: visible (not
missing/blank/occluded) · at declared projected position · expected size · expected
colour present · grounded if on the ground · annotation attached if targeted.
**Prong A must explicitly compare the build manifest against the actual draw
output** — including catching *extra/stale* content (generated floors/overlays/
nodes left from prior state), not only missing content.

### Prong B — First-principles defect taxonomy (the whole category space)

One check per category (the found defects are instances). Full table below.

### Prong C — Structured adversarial review (catch-all)

A rubric pass, ideally a **separate critic**, in explicit "what's wrong?" mode.
**Authority rule (Codex):** the deterministic verifier owns the gates; the critic
is adversarial review and must **not** be a silent OK gate — its findings must
graduate into deterministic checks or recipe expectations.

### No silent coverage gaps

A `composition` report block lists every check with an explicit status:
`checked` · `failed` · `skipped_no_declared_intent` · `skipped_no_backend_support`
· `skipped_import_unknown` · `unsupported` · `not_applicable`. **If a profile
requires a category, missing coverage must be emitted as a `failed`/`error`
check, not as an informational skip. Informational skip checks remain in the
composition block as coverage inventory but do not become top-level warning
reasons. Never silently omit a check.

---

## The defect taxonomy (Prong B) — expanded with Codex's missing categories

| Category | Defect kinds | How checked | Instance |
|---|---|---|---|
| **Completeness/presence** | blank; declared object missing; partial/cut-off | spec-conformance (manifest vs draw); visible-coverage needs pixels/depth | — |
| **Unexpected/stale content** | extra generated floors/overlays/nodes from prior state | spec-conformance (manifest vs draw output — extras too) | — |
| **Placement/transform** | floating; sinking/penetrating; interpenetration; wrong scale/orientation; off-screen; behind camera; off-center | structural/projection (AABB vs floor / vs declared transform) | (floating) |
| **Occlusion/depth** | transparent/helper over opaque; z-fighting; wrong order; flat/no-depth; wrongly-occluded | depth-buffer/pixel readback (structural can only flag *risk*) | **D1** |
| **Helper/render-layer policy** | grid/strokes/labels/outlines/gizmos layered wrong (helpers-on-top vs depth-tested) | structural layer-policy + depth | **D1** |
| **Overlay/annotation** | line-through-text; label-label overlap; off-frame/clipped; detached from target; leader→empty/wrong; illegible/low-contrast | structural (exact segment endpoints, label rects, owner/target ids); contrast is pixel | **D2** |
| **Lighting/material (result)** | subject black-crush; blown highlights; flat/unlit; missing shadow/contact; dead metal (no IBL) | pixel on **subject mask**; material binding is structural | **D3, D4** |
| **Texture/UV/material mapping** | wrong UV orientation; missing texture fallback; wrong wrap/repeat; normal map not applied; environment missing | structural (binding) + pixel (result) | — |
| **Geometry/deformation/topology** | inside-out winding; missing triangles; stale skin/morph; wrong normals/tangents; broken tessellation | structural (mesh/normal/winding) + pixel | — |
| **State/time/variant** | wrong animation time; wrong visual state; wrong material variant; selection/hover/tint; stale GPU deformation | structural (declared vs applied state) + pixel | — |
| **Clipping/section/exploded** | wrong clip-plane count; clipped wrong target; section box ≠ declared; exploded transforms wrong | structural (declared vs applied) | — |
| **Backend/capability conformance** | requested gpu/msaa4/ssao/bloom/environment/quality:high but backend falls back / inactive / degraded | structural (capability) + pixel proof effect is visible | — |
| **Colour/exposure/tone** | under/over-exposed; cast; banding; muddy | pixel percentiles (exists) | — |
| **Visual salience** | object technically visible but blends into background / unreadable | pixel (subject/background separation, explicit) | — |
| **Numerical/artifact** | fireflies/noise; seams/wrap; edge clipping | pixel | (aliasing now A1) |
| **Framing/camera** | subject too small/dead margins; clipped; bad aspect; tilt | structural projection (fill fraction exists) + pixel sanity | **D5** |

> **NaN/Inf is NOT a final-capture check** (Codex): by RGBA8 the NaN is gone. It
> belongs in **renderer diagnostics / intermediate validation**, not PNG analysis.

---

## Data layer (Prong A feasibility — Codex Q2)

**Exists today:** build id→handle manifest (`src/scene/recipe/types/build_manifest.rs:12`);
scene inspection nodes/draw-list/world-transforms/bounds/material summaries
(`src/scene/inspection/schema.rs:16`); capture camera/projection/viewport
(`src/capture.rs:35`); the verify bundle already has host+manifest+capture+
inspection+introspection+expectations (`src/bin/scena/recipe/verification.rs:9`);
label/line quality regions (`src/scene_host/label_quality.rs:21`).

**Implemented in foundation:** per-object **projected bbox** in
`scena.scene_composition.v1`, material base-color intent from the draw-list
material summaries, grid/floor semantic ownership, callout target ownership,
measurement overlay output ownership, projected label rects, and projected line
endpoints. Evidence:
`scena_recipe_render_verify_emits_passing_composition_report_for_declared_node`
pins `material_base_color_available`, and
`scena_recipe_render_verify_checks_callout_annotation_ownership` pins
`callout_target_attached` + `callout_overlay_output_projected` through
`recipe render --verify`; `scena_recipe_render_verify_checks_grid_floor_ownership`
pins `grid_floor_output_owned` through the same CLI path; and
`scena_recipe_render_verify_checks_measurement_overlay_ownership` pins
`measurement_overlay_output_projected` through the same CLI path.
`scena_recipe_render_verify_fails_overlay_line_through_label` pins the first
Prong B overlay collision detector with exact code
`overlay_label_intersects_line`; clear labels emit `overlay_label_clear_of_lines`.
`scena_recipe_render_verify_checks_label_label_overlap_on_cpu_and_gpu` pins the
next overlay collision detector through `recipe render --verify`: overlapping
projected labels fail with exact code `overlay_label_intersects_label`, while
separated labels emit `overlay_label_clear_of_labels` on CPU and lavapipe GPU.
`scena_recipe_render_verify_checks_label_viewport_fit_on_cpu_and_gpu` pins the
off-frame/clipped-label detector through `recipe render --verify`: an unclipped
projected label rect that extends beyond the capture viewport fails with exact
code `overlay_label_clipped_by_viewport`, while fully inside labels emit
`overlay_label_inside_viewport` on CPU and lavapipe GPU.
`scena_recipe_render_verify_checks_helper_layer_occlusion_on_cpu_and_gpu` pins
the helper-layer/depth policy path through `recipe render --verify`: helpers
behind a declared subject emit `helper_layer_occluded_by_subject`, while helpers
in front fail with exact code `helper_layer_overdraws_subject` on CPU and
lavapipe GPU.
`scena_recipe_render_verify_checks_backend_conformance_on_cpu_and_gpu` pins
`expect_backend`, backend mismatch failure, and checked render-quality knob
conformance (`render_antialiasing_active`, `render_supersample_active`,
`render_reconstruction_active`) through `recipe render --verify` on CPU and
lavapipe GPU.
`scena_recipe_render_verify_checks_clipping_and_section_conformance_on_cpu_and_gpu`
pins clipping/section structural checks through `recipe render --verify`:
declared/expected active clipping-plane counts emit
`clipping_plane_count_satisfied`, active section boxes emit
`section_box_active`, inversion emits `section_box_inversion_satisfied`, and a
missing cutaway fails with exact codes `clipping_plane_count_mismatch` and
`section_box_missing` on CPU and lavapipe GPU.
`scena_recipe_render_verify_checks_material_variant_state_on_cpu_and_gpu`
pins state/variant structural checks through `recipe render --verify`:
expected default import material-variant state emits
`material_variant_state_satisfied` on CPU and lavapipe GPU, while a recipe that
expects an unapplied named variant fails with exact code
`material_variant_state_mismatch`.
`scena_recipe_render_verify_checks_transform_conformance_on_cpu_and_gpu` pins
placement/transform conformance through `recipe render --verify`: a declared
world-space translation/scale/intrinsic X/Y/Z rotation expectation emits
`transform_conformance_satisfied` on CPU and lavapipe GPU, while a mismatched
translation fails with exact code `transform_conformance_mismatch`.
`scena_recipe_render_verify_checks_world_bounds_separation_on_cpu_and_gpu`
pins interpenetration/clearance placement checks through
`recipe render --verify`: separated declared parts emit
`separation_conformance_satisfied` on CPU and lavapipe GPU, while intersecting
world-space bounds fail with exact code `separation_conformance_mismatch`.
`scena_recipe_render_verify_checks_object_exposure_and_salience_on_cpu_and_gpu`
pins the first object-scoped pixel checks through `recipe render --verify`:
healthy subject regions emit `subject_exposure_sane`, near-black subject regions
fail with `subject_black_crushed`, and low subject/background separation fails
with `subject_salience_too_low` on CPU and lavapipe GPU.
`scena_recipe_render_verify_checks_texture_material_result_on_cpu_and_gpu` pins
texture/material result checks through `recipe render --verify`: a decoded
base-color texture with valid UVs emits `texture_result_visible`, while the same
decoded texture mapped through degenerate UVs fails with `texture_result_flat`
on CPU and lavapipe GPU.
`scena_recipe_render_verify_checks_object_depth_order_on_cpu_and_gpu` pins the
object-vs-object occlusion/depth check through `recipe render --verify`: an
expected front object that occludes the expected back object emits
`object_depth_order_satisfied`, while the same objects with inverted z order
fail with exact code `object_depth_order_mismatch` on CPU and lavapipe GPU.
`scena_recipe_render_verify_rejects_ambiguous_object_depth_colors_on_cpu_and_gpu`
pins the fail-closed guard for the current native-resolution color-probe method:
front/back colours that cannot be separated fail with exact code
`object_depth_order_color_ambiguous` instead of pretending to prove depth order.
`scena_recipe_render_verify_checks_object_framing_on_cpu_and_gpu` pins
profile-driven object framing through `recipe render --verify`: a normally
framed subject emits `subject_fit_sane`, while a technically visible but tiny
subject fails with exact code `subject_too_small_in_frame` on CPU and lavapipe
GPU.

Remote audit proof on the synced `scena-reconstruction-quality` tree:
`cargo test --features scene-host,inspection --test scena_cli_recipe composition`
passes the base composition-report/coverage tests, and the named category tests
for overlay collision, callout/grid/measurement ownership, grounding, helper
layer occlusion, object depth order, backend conformance, clipping/section,
material-variant state, object exposure/salience, object framing, and texture
result all pass through `recipe render --verify` with
`VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json` for GPU cases. This is
evidence for the composition wiring and coverage net, not final whole-goal
completion.

**Implemented in foundation:** per-object native-capture visible coverage is
measured by clipping each projected declared-node region to the viewport and
counting background-relative foreground pixels. Evidence:
`scena_recipe_render_verify_emits_passing_composition_report_for_declared_node`
pins `visible_pixel_coverage_available`, and
`scena_recipe_render_verify_fails_required_composition_coverage_when_node_is_offscreen`
pins `visible_pixel_coverage_missing` through `recipe render --verify`.
`scena_recipe_render_verify_checks_grid_floor_ownership` also pins explicit
grounded placement intent with `ground_contact_present`, and
`scena_recipe_render_verify_fails_floating_grounded_node_on_cpu_and_gpu` proves
a floating declared target fails through `recipe render --verify` with
`ground_contact_missing` on CPU and lavapipe GPU.

**Precision limit:** exact depth/id object-mask attribution for arbitrary
overlapping projected bboxes is not implemented by the current foundation. The
public `expect_occlusion` check is a native-resolution color probe and now
fails closed with `object_depth_order_color_ambiguous` when the front/back draw
colours are not separable. Measurement target semantics are not advertised
because recipes do not yet have a measurement target field (today measurements
declare generated line/label output only); owner ids for generic overlay
geometry beyond callouts/grid/measurements are likewise not advertised as
covered. D4/C4 pixel-level contact-shadow checking is now covered by
`expect_quality.grounding` through `recipe render --verify` on CPU and
lavapipe GPU, including a native-PNG locality guard that fails broad SSAO
banding instead of accepting it as contact shadow.

**Owner (Codex):** a new `src/scene_host/composition.rs` projection/ownership
layer — **not** buried in CLI code. **Extract shared projection/region helpers
FIRST** (projection logic is duplicated across labels/lines/measurements; if
composition reimplements it, it drifts).

---

## The known defects (now instances; root causes corrected by Codex)

- **D1 [occlusion/helper-layer]** floor/grid over objects. **Corrected:** the
  floor is matte/opaque (`src/scene/framing.rs:653`) — NOT transparent/OIT. The
  regression was the GPU MSAA helper-depth path: depth-tested overlays need a
  single-sample depth view after the multisampled scene pass. The current fix
  creates an overlay-depth prepass when `sample_count > 1`
  (`src/render/gpu/draw.rs`) and routes overlay passes through
  `resolved_depth_view` (`src/render/gpu/draw_overlays.rs`), so grid/stroke
  helpers depth-test under `msaa4` like the sample-count-1 path. Renderer +
  verifier defect, now pinned by
  `scena_recipe_render_gpu_msaa_grid_floor_is_occluded_by_object`: on lavapipe
  HeadlessGpu the latest focused artifact reports
  `red_grid_pixels_inside_object_interior: 0` for the `msaa4` grid/box scene.
- **D2 [overlay]** measurement labels were placed at the measured line midpoint
  and callout leaders ended at the label center. **Corrected:** generated
  measurement labels are offset from their dimension line, callout leader lines
  stop before the label, and `recipe render --verify` now fails line-through-text
  output with `overlay_label_intersects_line`. Renderer/layout + verifier defect.
- **D3 [lighting/material]** metallic `1.0` faces → no diffuse → black. GPU IBL
  exists (`src/render/gpu/output_shader.wgsl:606`). **Mostly authoring + verifier**
  (bad material choice unless polished metal intended); don't claim a renderer fix
  unless the shader/effect is proven broken.
- **D4 [lighting/material]** no contact shadow. GPU SSAO exists
  (`src/render/gpu/post/mod.rs:115`). **Authoring/default unless requested-and-ineffective** —
  if the recipe didn't ask for SSAO/grounding, fix the showcase/defaults. The
  current composition foundation proves grounded placement and render-setting
  activation, and the C4 render-quality verifier now proves pixel contact
  darkening with exact `contact_shadow_missing` / `contact_shadow_checked`
  outcomes. Focused remote evidence:
  `scena_recipe_render_verify_checks_contact_shadow_grounding_on_cpu_and_gpu`;
  artifact directory
  `target/gate-artifacts/scena-cli-recipe-recipe-quality-contact-shadow-grounding-1283629/`.
- **D5 [framing]** weak hero. `expect_bbox_fit` has subject logic
  (`src/bin/scena/recipe/bbox_fit.rs:39`). **Authoring/verifier** — make it
  default/profile-enforced, not a renderer change.

Honesty rule: D3/D4/D5 are likely **authoring/default + verifier**, not renderer
wins. Fix the showcase recipe/defaults AND add the detector; never claim a
renderer fix for a recipe patch.

---

## Coverage report + doctor (Codex Q4 + risks)

- Add a separate `composition` block to `SceneRecipeVerificationReportV1`
  (`src/scene/recipe/types/render_result.rs:21`) with the status enum above,
  schema/fixture/doctor pins.
- **Coverage must not be decorative:** doctor requires every composition check to
  have a known-good + known-bad fixture, an exact reason code, and a
  `recipe render --verify` test proving it is actually wired end-to-end.
- Evidence added: `scena_recipe_render_verify_fails_required_composition_coverage_when_node_is_offscreen`
  proves a profile-required offscreen declared node fails through
  `recipe render --verify` with exact code `visible_pixel_coverage_missing`;
  `scena_recipe_render_verify_emits_passing_composition_report_for_declared_node`
  proves the corresponding visible-node pass path emits
  `visible_pixel_coverage_available`.
- Evidence added: `scena_recipe_render_verify_emits_passing_composition_report_for_declared_node`
  now proves material-backed nodes emit a checked structural color-intent fact
  (`material_base_color_available`) instead of a placeholder skipped color field.
- Evidence added: `scena_recipe_render_verify_checks_measurement_overlay_ownership`
  proves recipe-authored measurement overlays emit owned generated output through
  `recipe render --verify` with exact code `measurement_overlay_output_projected`.
- Evidence added: `scena_recipe_render_verify_fails_overlay_line_through_label`
  proves a line crossing a label region fails through `recipe render --verify`
  with exact code `overlay_label_intersects_line`.
- Evidence added: `scena_recipe_render_verify_checks_label_label_overlap_on_cpu_and_gpu`
  proves overlapping projected labels fail through `recipe render --verify` with
  exact code `overlay_label_intersects_label`, while separated labels emit
  `overlay_label_clear_of_labels` on CPU and lavapipe GPU.
- Evidence added: `scena_recipe_render_verify_checks_label_viewport_fit_on_cpu_and_gpu`
  proves partially off-frame labels fail through `recipe render --verify` with
  exact code `overlay_label_clipped_by_viewport`, while inside labels emit
  `overlay_label_inside_viewport` on CPU and lavapipe GPU.
- Evidence added: `scena_recipe_render_verify_fails_floating_grounded_node_on_cpu_and_gpu`
  proves explicit grounded placement intent fails through `recipe render --verify`
  with exact code `ground_contact_missing`; the grid-floor ownership recipe pins
  the passing `ground_contact_present` path.

---

## Prong C rubric (Codex Q6) — required items

Native-res full-frame (no thumbnails) · every declared object visible/position/
size/orientation/material/state · overlays readable+attached, no line-through-text,
no off-frame · depth/grounding: no helper-over-object, no floating/sinking/
interpenetration · lighting/material: no black-crush/blown/flat/wrong-reflection ·
camera/framing: fill/margins/crop/resolution · backend/effects: actual backend +
AA/SSAO/bloom/environment active if requested · coverage: list what couldn't be assessed.

## Agent discipline

Holistic composition pass at native resolution FIRST, report composition defects
FIRST, never "good" from a crop or green `ok:true`. Add to
`stunning-renders-and-performance.md`.

---

## Build order

0. **Foundation [implemented]:** extract shared projection/region helpers; build
   `scene_host/composition.rs` (per-object projected bbox + visible coverage +
   expected colour + ground-plane semantic + annotation ownership graph + exact
   overlay geometry); add the `composition` report block + coverage status enum;
   implement **Prong A spec-conformance** on top, wired into CLI verify; doctor
   coverage harness. Prove it flags D1/D3/D4-class discrepancies on the showcase.
   D1 helper-layer overdraw and D3 black-crush/salience are covered by current
   CPU+lavapipe recipe proofs; D4 structural grounding/backend conformance and
   pixel contact-shadow darkening are covered by current CPU+lavapipe recipe
   proofs. Exact depth/id object-mask attribution is not part of this foundation;
   current object-depth expectations fail closed when the color-probe cannot
   distinguish front/back objects.
1. **Prong B [implemented for the current recipe surface]** structural checks (overlay-collision, placement, helper-layer,
   backend-conformance, clipping/section, state/variant), then pixel
   (subject black/blown, salience, texture result), then depth (occlusion).
2. **Dx fixes**, each gated by the check that now catches it (confirm D1 MSAA
   overlay-depth first). D1 is confirmed fixed by
   `scena_recipe_render_gpu_msaa_grid_floor_is_occluded_by_object` on lavapipe
   HeadlessGpu: `red_grid_pixels_inside_object_interior == 0` under `msaa4`.
3. **Prong C** rubric + critic, findings graduating into deterministic checks.

## Gates (every PR)

- [x] fmt · clippy ×2 · test ×2 · doctor --full · doc -D warnings · publish <10 MiB.
- [x] Each check fails-before/passes-after **through the CLI verify path** on the
      real showcase recipe; native-res; both backends; real GPU adapter.
- [x] `composition` coverage report emitted; doctor pins every check (known-good/
      known-bad/exact-reason/CLI test); no decorative coverage; no silent gaps.
- [x] Shared projection extracted (no reimplementation/drift).
- [x] Docs/skill updated where authoring guidance changes.
