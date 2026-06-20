# Scene composition correctness — coverage-driven detection (not incident-driven)

Status: **TAXONOMY LOCKED AFTER 2 CODEX REVIEWS — READY FOR FOUNDATION
IMPLEMENTATION.** v1 (five-defect) was coverage-by-incident; v2 went
coverage-by-construction; this v3 folds in Codex's re-review (8 missing
categories, corrected root causes, data-layer gaps, coverage-report design,
ownership/anti-drift risks). Implement the **foundation slice first** (shared
projection + composition data/ownership layer + coverage report + Prong A
backbone + doctor coverage harness); per-category checks and the Dx fixes follow.

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
requires a category, `skipped` is a verification failure** (or at minimum a
top-level coverage warning an agent cannot miss). Never silently omit a check.

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

**Missing — must be added:** per-object **projected bbox** in a public report
(computable from draw-list bounds + capture projection, not emitted); per-object
**visible-pixel coverage/occlusion** (introspection only reports handle/kind/
visible/reason, `src/render/introspection/types.rs:97`); **expected colour for
every object** (only explicit `expect_color` targets compiled today,
`verification.rs:260`); **ground-plane semantic ownership** (manifest doesn't
record "this is the ground"); **label/measurement/callout → target ownership
graph** (`src/scene_host/measurements.rs:10` exposes limited projection);
**exact overlay geometry** (segment endpoints + label rects + owner/target ids).

**Owner (Codex):** a new `src/scene_host/composition.rs` projection/ownership
layer — **not** buried in CLI code. **Extract shared projection/region helpers
FIRST** (projection logic is duplicated across labels/lines/measurements; if
composition reimplements it, it drifts).

---

## The known defects (now instances; root causes corrected by Codex)

- **D1 [occlusion/helper-layer]** floor/grid over objects. **Corrected:** the
  floor is matte/opaque (`src/scene/framing.rs:653`) — NOT transparent/OIT. Real
  cause: **GPU overlay depth is unavailable when `sample_count != 1`**
  (`src/render/gpu/draw_overlays.rs:7`) → under MSAA the grid/stroke overlay
  stops depth-testing and draws over geometry. **Likely a regression from the AA
  slice (9ba888e); testable: `none` vs `msaa4` on the dashboard.** Renderer/verifier defect.
- **D2 [overlay]** measurement label placed at line midpoint with no offset
  (`src/scene/measurements.rs:173`). Renderer/layout defect — needs offset policy + collision verifier.
- **D3 [lighting/material]** metallic `1.0` faces → no diffuse → black. GPU IBL
  exists (`src/render/gpu/output_shader.wgsl:606`). **Mostly authoring + verifier**
  (bad material choice unless polished metal intended); don't claim a renderer fix
  unless the shader/effect is proven broken.
- **D4 [lighting/material]** no contact shadow. GPU SSAO exists
  (`src/render/gpu/post/mod.rs:115`). **Authoring/default unless requested-and-ineffective** —
  if the recipe didn't ask for SSAO/grounding, fix the showcase/defaults.
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

0. **Foundation:** extract shared projection/region helpers; build
   `scene_host/composition.rs` (per-object projected bbox + visible coverage +
   expected colour + ground-plane semantic + annotation ownership graph + exact
   overlay geometry); add the `composition` report block + coverage status enum;
   implement **Prong A spec-conformance** on top, wired into CLI verify; doctor
   coverage harness. Prove it flags D1/D3/D4-class discrepancies on the showcase.
1. **Prong B** structural checks (overlay-collision, placement, helper-layer,
   backend-conformance, clipping/section, state/variant), then pixel
   (subject black/blown, salience, texture result), then depth (occlusion).
2. **Dx fixes**, each gated by the check that now catches it (confirm D1 MSAA
   overlay-depth first).
3. **Prong C** rubric + critic, findings graduating into deterministic checks.

## Gates (every PR)

- [ ] fmt · clippy ×2 · test ×2 · doctor --full · doc -D warnings · publish <10 MiB.
- [ ] Each check fails-before/passes-after **through the CLI verify path** on the
      real showcase recipe; native-res; both backends; real GPU adapter.
- [ ] `composition` coverage report emitted; doctor pins every check (known-good/
      known-bad/exact-reason/CLI test); no decorative coverage; no silent gaps.
- [ ] Shared projection extracted (no reimplementation/drift).
- [ ] Docs/skill updated where authoring guidance changes.
