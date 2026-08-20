# LLM App Builder Guide

This guide is the public, model-agnostic companion to the repo-hosted
`.codex/skills/scena-app-builder` skill. Use it when asking Codex, Claude Code,
or another shell-capable LLM to build a `scena` model viewer, CAD inspection
scene, digital twin, product configurator, dashboard, documentation renderer,
or interaction proof.

Installed users can export this exact package-embedded guide without locating a
repository checkout: use `scena guide agent --json` for the versioned
`scena.agent_guide.v1` contract or `scena guide agent --markdown` for raw
Markdown. Root `AGENTS.md` remains contributor-only repository governance.

## Required CLI Build

Install or run the CLI with the app-builder features:

```bash
cargo install scena --features agent
```

From a local checkout:

```bash
cargo build --release --bin scena --features agent
target/release/scena <command>
```

Use `cargo run` without `--release` only while developing the CLI itself. Do
not use debug-profile render latency to judge renderer performance.

`agent` is the complete opt-in surface. It enables `scene-host`, which already
enables `inspection`; the default feature set remains empty.

Before choosing a backend or optional effect, distinguish compiled defaults
from current hardware:

```bash
scena capabilities --json
scena capabilities --live --json
```

Treat `probe.status:"static_no_device"` as planning metadata only. A hardware
claim requires `probe.status:"measured"`; stop on the command's nonzero
structured `unavailable` result instead of treating a missing adapter as a
skip. The live headless probe measures readback but cannot prove presentation.
Use `scena --version` to inspect every compiled feature before selecting a
feature-gated command.

## Public-Surface Workflow

Do not guess recipe fields or read renderer internals first. Use public schema,
template, validation, render, inspection, diagnosis, and repair surfaces.

Discover the schema:

```bash
scena schema get scena.scene_recipe.v1 > scene_recipe.schema.json
```

Discover accepted names before inventing a preset:

```bash
scena vocab list > scena-vocabulary.json
```

The report includes material, camera-lens, framing, named-color, scene,
environment, exposure, render-quality, tonemapper, easing, placement, and
per-light-kind registries. Each value also discloses accepted aliases,
deprecation, and required features or capabilities.

If a schema name is misspelled, read `scena.cli_error.v1.candidates`; do not
parse the prose message. The same capped structured field appears for unknown
template names and in recipe diagnostics for node, geometry/mesh-resource,
material, import, and environment-preset references.

For every CLI failure, branch on `exit_class` and `code`, not message text:
`usage` (2) means repair the command, `input` (65) means repair or locate the
contract, `unsupported` (69) means re-plan for features/capabilities,
`runtime`/`internal` (70) means inspect diagnostics, `io` (74) means repair the
stream or filesystem, `policy` (77) requires an operator-owned policy change,
and `interrupted` (130) is retryable. `comparison` (1) is a valid unequal
result. The authoritative per-command mapping is emitted by `scena --help`.

Start from a template when possible. Discover canonical names, aliases,
required features, and status without scraping an error:

```bash
scena examples agent list
```

Runtime node, animation, material-variant, anchor, and connector lookup errors
use that same deterministic candidate ranking. Apply the first candidate only
when the requested operation makes the correction unambiguous.

Canonical names use kebab-case. Historical underscore inputs remain aliases;
their generated manifest contains a migration note. In particular,
`product_configurator` is the compatibility alias for
`product-configurator-starter`, while `product-configurator` is the imported
material-variant workflow.

Generate a template:

```bash
mkdir -p target/scena-agent
scena examples agent get primitive-scene --out target/scena-agent/primitive-scene > target/scena-agent/primitive-scene.manifest.json
```

The command prints an `scena.agent_smoke_template.v1` manifest to stdout and
writes the actual recipe, expectations, and artifacts under `--out`. Read the
manifest `files[]`, `required_features[]`, and `commands[]`; do not validate the
manifest as a recipe. Set `RECIPE` to the manifest recipe path; for the command
above:

```bash
RECIPE=target/scena-agent/primitive-scene/recipe.json
```

The following marked block is the canonical clean-directory smoke workflow.
Release validation extracts and executes it verbatim with both the repository
binary and the packaged installed binary.

<!-- SCENA_CANONICAL_AGENT_SMOKE_BEGIN -->
```bash
mkdir -p target/scena-agent
scena schema get scena.scene_recipe.v1 > target/scena-agent/scene-recipe.schema.json
scena examples agent list > target/scena-agent/templates.json
scena examples agent get primitive-scene --out target/scena-agent/primitive-scene > target/scena-agent/primitive-scene.manifest.json
RECIPE=target/scena-agent/primitive-scene/recipe.json
scena validate "$RECIPE" > target/scena-agent/contract-validation.json
scena validate-recipe "$RECIPE" --full > target/scena-agent/validation.json
scena recipe build "$RECIPE" > target/scena-agent/build.json
scena recipe render "$RECIPE" --timings --out target/scena-agent/frame.png > target/scena-agent/render.json
```
<!-- SCENA_CANONICAL_AGENT_SMOKE_END -->

The installed template catalog is self-contained. Generate and execute it from
any working directory: imported template fixtures and the licensed `studio`
HDR are embedded in the package, and presentation defaults preserve any
explicitly authored `scene.environment`.

Render introspection is the machine-safe default. `--introspect` remains an
accepted compatibility no-op, but new commands and generated templates omit it.

Validate before rendering:

Use the generic schema dispatcher for recipes, expectation files, patches, and
capability reports before invoking their consuming workflow. It fails closed on
malformed, unknown, or mismatched contracts and suggests near schema names:

```bash
scena validate "$RECIPE"
scena schema json scena.scene_recipe.v1 > target/scena-agent/scene-recipe.json-schema.json
```

JSON Schema export cannot prove runtime resources, sandbox policy, cross-field
semantics, or backend capabilities. Follow it with the owner-specific full
validation when those checks matter.

For assembly/viewer snapshots, use recipe-local stable IDs in `anchors`,
`connectors`, `bounds`, and `named_states`; never persist the numeric handles
from `scena.scene_recipe_build.v1`. These recipe IDs correlate build, patch,
and proof output but are not application-persistence identities. The host owns
durable identity and document migration. Spatial targets are typed objects such as
`{"kind":"node","id":"shaft"}` or
`{"kind":"import_node","import":"pump","path":"Root/Flange"}`. Authored
offsets, snap tolerances, clearances, and bounds are scene meters. A connector
`mate` names another connector id, while an active named state may apply only
transform, tint, and visibility. See
`docs/specs/recipe-spatial-state-v1.md` for inheritance and failure rules.

Scene measurements are visualization/inspection aids, not calibrated or
authoritative metrology. They use current world transforms and `f32` scene
coordinates after the selected import-unit conversion. Display precision is
formatting, not measurement accuracy; snapping, occlusion, manufacturing
tolerances, survey accuracy, and certified dimensional claims are outside this
contract. Preserve the returned `measurement_authority` metadata when showing
measurement results to users.

```bash
scena validate-recipe "$RECIPE" --full
```

Full validation resolves imports, environments, fonts, authored textures, and
nested glTF dependencies through the build policy and reports the normalized
resource plan. Use `--syntax-only` only while editing shape; it performs no I/O
and returns `execution_equivalent:false`.

If the operator supplies a model library outside the working directory, use
one narrow, repeatable root option and carry it through the loop:

```bash
scena policy recipe --allow-root /srv/models
scena validate-recipe "$RECIPE" --full --allow-root /srv/models
scena recipe render "$RECIPE" --out frame.png --allow-root /srv/models
```

Confirm the canonical directory appears in `policy.allowed_roots` with
`source:"operator_override"`. Never emulate a sandbox-disable flag or widen to
an unrelated parent; traversal and symlink escapes intentionally remain denied.

Render with introspection:

```bash
scena recipe render "$RECIPE" --out frame.png
```

Success means the command exits 0 and the top-level report says `ok:true`.
Never claim success from a PNG path or nonzero byte length alone.
All asset-or-recipe commands route a parsed recipe through the same
policy-aware SceneHost builder. A later rejected import fails the command with
`scena.recipe_build_result.v1`; it cannot be silently omitted. Raw glTF/GLB
inputs remain on the direct asset path.
When the recipe has an `expect` block, add `--verify`; that mode emits the
combined recipe build/capture/introspection/verification report instead of the
plain render-introspection report.
For presentation or beauty output, add `--gpu`; CPU remains the default, and
the top-level `backend_selection` object records whether the request came from
the flag, which backend was selected, and actionable `reason`/`remedy` fields
when a CPU fallback was required. It does not emit unversioned fallback prose.
`SCENA_USE_GPU` never changes CLI execution.

For CAD imports that render as an edge sliver or white-on-white blob, run the
inspection preset instead of hand-tuning a single camera:

```bash
scena recipe inspect-cad "$RECIPE" --out-dir target/cad-inspection
```

It generates broad-face, top-feature, and overview recipes, renders each through
`recipe render --verify`, then writes PNGs and
`scena.cad_inspection_result.v1`. Generated CAD inspection recipes apply
presentation-only `imports[].material`, `imports[].edge_emphasis`, and a
principal-face camera where appropriate. They use the oriented studio rig and
a small +0.25 EV presentation adjustment so each generated view receives
reviewable key, fill, and rim illumination instead of three co-directional
light nodes; these controls do not change the source geometry or CAD truth.

For a general review sequence, reuse the same SceneHost and capture lifecycle:

```bash
scena recipe capture "$RECIPE" --out-dir target/capture \
  --views front,top,right,isometric --turntable 24
```

Add `--clip open --frames 12` to sample an authored or imported animation from
time zero through its duration. The command writes numbered PNG frames,
descriptor JSON beside every frame, a contact sheet, and one
`scena.capture_sequence_result.v1` report containing every camera, clip name,
sample time, and payload hash. Use `--views none` when only a turntable or clip
sequence is wanted. The core command deliberately does not choose a GIF/video
codec; pass its ordered PNGs to an external encoder when that delivery format
is needed.

For attributed pixels, depth, or normals, produce semantic AOVs separately:

```bash
scena recipe aov "$RECIPE" --out-dir target/semantic-aov \
  --passes id,depth,normal
```

Read persistent `recipe_node`, `recipe_instance`, or `import_node` identities
from the legend. Do not store `runtime_identity` handles. Treat transparent
geometry, strokes, labels, particles, helpers, and overlays as explicitly
unattributed in CPU v1; the report's exclusion counts keep that limitation
machine-visible.

To review a recipe change structurally without constructing a renderer, run:

```bash
scena diff before.recipe.json after.recipe.json
```

Add `--render --out-dir target/recipe-diff` when pixels also matter. The
rendered mode reuses the aggregate capture baseline diff and semantic AOVs; it
does not turn every changed pixel into a confident node claim. Always verify
the partition `changed_pixels = attributed_pixels + ambiguous_pixels + unattributed_pixels`
and inspect ambiguous/unattributed regions before accepting the change. Edge
pixels, excluded transparency/overlays, and different before/after identity
candidates remain explicit rather than being silently assigned.

For user-facing renders, add one more pass before calling the image done:
inspect the native-resolution frame in explicit "what is wrong?" mode. Check
the whole composition: declared objects visible and placed correctly, no stale
or extra content, labels readable and attached, helper/grid lines not drawn over
solid objects, objects grounded when intended, materials not black-crushed or
blown out, and camera framing suited to the app. Convert any finding into a
deterministic expectation or verifier gap; do not treat this critic pass as a
silent green gate.

## Make It Look Good

Correctness proof is not aesthetic proof. For scenes meant for a user-facing
screenshot or demo, start with the ergonomic recipe fields instead of the
low-level raw equivalents. They route to the same Rust helpers a Rust user
would call:

```json
"materials": [
  { "id": "body", "preset": "chrome", "roughness": 0.06 }
],
"lights": [
  { "id": "studio", "kind": "studio_rig", "preset": "studio_rig" }
],
"cameras": [
  {
    "id": "camera",
    "kind": "perspective",
    "lens": "portrait",
    "framing": { "preset": "three_quarter_front_right", "fill": 0.72 },
    "active": true
  }
],
"scene": { "preset": "product_studio" },
"render": {
  "auto_exposure": "product_studio",
  "exposure_compensation_ev": 0.0,
  "quality": "high",
  "anti_aliasing": "msaa4",
  "supersample": 1,
  "reconstruction": "box"
},
"capture": { "width": 960, "height": 720 }
```

For a product/model hero still, use the dedicated easy path before hand-authoring
camera distance, exposure, focus, floor, or background:

```bash
scena photo plan model.glb --out hero.plan.json
scena photo render model.glb --out hero.png --report hero.report.json --emit-recipe hero.resolved.recipe.json
```

`scena photo plan` writes `scena.photo_plan.v1`: a render-free candidate,
subject, staging, and selected-composition plan. Use
`--subject import:<id>` or `--subject node:<id>` when a recipe has more than
one possible subject. `scena photo render` writes a versioned
`scena.photo_render_result.v1` envelope to stdout, a `scena.photo_report.v1`
quality report to `--report`, and, when requested, a public recipe artifact to
`--emit-recipe`. Treat
`ok:true` plus the report `status:"passed"` as the easy-path acceptance signal.
The report includes the deterministic composition plan and a bounded
low-resolution shaded-candidate selection pass, including candidate render
count, candidate resolution, selected composition id, subject metrics, and
per-candidate `scena.render_quality.v1` results.
Those subject metrics come from geometry-owned semantic ID/depth/normal
buffers and include composition, highlight structure, contact-shadow
grounding, silhouette separation, color cast, saturation, and reflection
washout. Read `shaded_selection.asset_health` before accepting the image:
`safe_repair` entries have already been applied,
`appearance_change_required` entries require an explicit caller decision, and
`unrecoverable` entries make the photo command fail with the missing input
named. Treat `folded_geometry`, `self_intersecting_geometry`,
`hidden_subject_component`, `duplicate_subject_component`,
`microscopic_subject_component`, `detached_subject_component`, and
`outlier_subject_component` as asset-authoring findings, not exposure or
lighting problems.
For agent loops, read `photo_report.exposure_report.subject.mean_luminance_srgb8`,
`photo_report.exposure_report.subject.low_clip_fraction`,
`photo_report.exposure_report.subject.high_clip_fraction`, and
`photo_report.exposure_report.suggested_compensation_ev` before changing
anything. A nonzero `suggested_compensation_ev` means the renderer measured the
subject and can name the EV nudge; keep it as auto-exposure compensation on the
next recipe attempt instead of replacing metering with fixed exposure.
It also includes `retry.policy` and `retry.attempts`; photo rendering is bounded
to a small internal camera/exposure loop rather than an unbounded agent loop.
If it fails, change the subject asset or explicit constraints; do not replace it
with a guessed fixed `exposure_ev` unless you are intentionally switching to
manual photography.
The public demo hero is a contract example, not a place for per-shot constants:
it must use `scena photo render` or recipe `photo.intent` with no hand-tuned camera, exposure, focus, floor, grid, or background overrides. If a demo hero
needs one of those overrides to pass, fix the camera-behavior intent policy or its
acceptance gate instead of hiding the defect in the demo recipe.

Recipes can request the same camera-behavior easy path without manual camera,
light, exposure, floor, or background fields:

```json
{
  "schema": "scena.scene_recipe.v1",
  "imports": [{ "id": "subject", "uri": "model.glb" }],
  "photo": {
    "intent": "camera_behavior",
    "subject": { "kind": "import", "id": "subject" }
  },
  "capture": { "width": 1280, "height": 840 }
}
```

Render it with verification:

```bash
scena recipe render hero.recipe.json --verify --out hero.png
```

The `--verify` result includes product-profile import framing and exposure
checks such as `subject_fit_sane` and `subject_exposure_sane`. If the recipe
omits `photo.subject`, the first built import is used.
When you need an explicit subject fallback policy, use the subject spec form:

```json
"subject": {
  "target": { "kind": "import", "id": "subject" },
  "fallback": "error"
}
```

`fallback:"error"` is the default and fails closed when the subject cannot be
resolved. `fallback:"average_metering_with_warning"` is reserved for deliberate
degraded metering paths and must be visible in the result report. Authored
recipes may use `{ "kind": "node", "id": "hero" }` in the same direct or
wrapped subject form.

When you need a small exposure nudge, keep automatic metering and add
`render.exposure_compensation_ev` with `render.auto_exposure`; do not replace
the meter with fixed `render.exposure_ev` unless the shot is deliberately
manual. `photo.intent` owns exposure itself and rejects fixed EV/manual camera
settings that it would otherwise override.

Lower-level recipes may declare the intended metering policy under
`render.metering` when `render.auto_exposure` is active. Valid forms are
`{ "mode": "average" }`, `{ "mode": "center_weighted" }`,
`{ "mode": "highlight_weighted" }`,
`{ "mode": "subject", "target": { "kind": "import", "id": "subject" }, "fallback": "error" }`,
and normalized spot metering
`{ "mode": "spot", "rect": { "x": 0.35, "y": 0.25, "width": 0.3, "height": 0.4 } }`.
Subject and spot metering are validated as stable recipe contracts; the
headless CPU recipe path routes subject metering through visible semantic
subject pixels. Backend strict/degraded reports for GPU paths are tracked in
`docs/checklists/subject-driven-photo-rendering.md`.

For lower-level recipes, use `product_studio` for product/model screenshots,
`cad_studio` for technical CAD/documentation scenes, and `industrial_studio` for
dashboard or live-state views. Add explicit `scene.background`,
`scene.environment`, or `scene.grid` only when you need to override the preset.

Prefer `material.preset` (`chrome`, `metal`, `rough_metal`,
`brushed_steel`, `plastic`, `clearcoat_plastic`, `satin`, `leather`,
`rubber`, `matte`, `clear_glass`, `frosted_glass`) before raw PBR fields.
Use `base_color` as an optional tint and scalar overrides such as `roughness`
only where needed.

Mirror materials need a dense sphere. A low-roughness subject (`chrome`, a
polished `metal`) reflects the environment sharply and therefore reveals the
mesh facets, so reflective spheres must use a high subdivision count:
`"primitive":{"kind":"sphere","radius":0.5,"segments":256,"rings":192}` (the
showcase chrome hero uses `384, 256`). The sphere `segments`/`rings` default is
only `64, 48` — fine for matte/rough materials, but it renders chrome as a
blocky, faceted ball. Raise `segments`/`rings` whenever the material is a near
mirror.

Prefer `camera.lens` and `camera.framing` over manual
camera distances. Prefer named color constants (`orange`, `gray`,
`light_gray`, `dark_gray`, `charcoal`, `studio_backdrop`, `warm_white`,
`cool_white`) when they fit. Use
`scene.environment:{ "preset":"studio" }` or `"neutral_studio"` for bundled
HDRI IBL, and leave `scene.grid.under_bounds` at its default `true` for
auto-sized floors. Use `studio` (a real Poly Haven studio HDR with softboxes)
when a low-roughness `material.preset:"chrome"` subject must read as product
chrome; mirror materials show the environment, so the studio HDR gives
structured reflections where a smooth/flat environment makes chrome look black.

Use `studio` or `neutral_gray` for model/product inspection, `dark_studio` for
dashboards and twin state views, `white` or `transparent` for documentation
exports, and `custom` only when the user gave an explicit color. The default
environment is flat; the packaged `studio` preset gives reflections and
material response without exposing or requiring a repository-relative HDR
path.
Use real glTF/GLB assets for realistic products and digital twins. Use authored
primitives for functional/CAD/diagram/chart scenes and tests. For visible
primitive boxes or cylinders in product-style scenes, add a small `bevel` or
`fillet` value so edges catch light; unsupported primitive kinds reject those
fields instead of ignoring them. For large scenes with repeated distant parts,
author explicit high/low geometry resources and add node `lods[]` thresholds so
small-on-screen parts render with cheaper geometry; scena switches among those
declared resources and does not invent simplifications.
Use `quality:"high"` for portable smooth edges. Browser WebGPU/WebGL2 currently
degrade its automatic MSAA choice to FXAA with a structured capability warning.
Use `anti_aliasing:"msaa4"` only as an exact request on a backend whose
`render_sample_counts` and `depth_sample_counts` advertise 4.
For product-style floor reflections, enable `scene.grid.reflection`; it adds a
verified structured floor reflection preset without requiring material SSR. If
the reflection is load-bearing, add
`expect_quality.reflection` so a flat matte floor fails with
`reflection_structure_missing`.
For hero chrome product stills, prefer
`scene.environment:{ "preset":"studio" }` with a high-tessellation sphere
(segments>=256, rings>=192) before changing the material. Add
`expect_quality.reflection.target` with `min_bright_fraction` and
`min_dark_fraction` when the subject must visibly read as chrome; a flat
dark/gray mirror fails with
`reflection_chrome_read_missing`. For hero product/studio reflections that must
mirror neighboring scene geometry, use
`render.screen_space_reflections:{strength,roughness,horizon_fraction,fade}`.
It reflects rendered scene content in screen space for the floor band and
high-metallic/low-roughness materials such as chrome. Screen-edge and occluded
material samples fade back to the environment-lit material. Use bare
`expect_quality.reflection` for floor/reflection-surface checks, or add
`expect_quality.reflection.target:{kind:"node",id:"..."}` to prove a specific
chrome/mirror subject changes from structured reflected detail rather than just
being brighter. For chrome or polished-metal HDRI renders, add
`max_firefly_fraction` to the reflection expectation so isolated bright
specular specks fail with `reflection_firefly_outliers` instead of passing as
valid structure.
For recipe-authored glass, use scalar material fields:
`transmission_factor`, `ior`, `thickness_factor`, `attenuation_distance`, and
`attenuation_color`. Do not use `transmission_texture` or `thickness_texture`;
the recipe validator rejects those slots until the GPU/WebGL2 texture-binding
budget supports them. If glass output is load-bearing, render with `--gpu`, add
`expect_backend`, and inspect the native-resolution output.
Use `render.supersample:2..4` only for hero captures or fine glossy/texture
details; it renders at N× resolution and downsamples, so cost grows with N^2.
Do not use large captures plus `supersample:2` in the default iteration loop:
on CPU or lavapipe this can take minutes. Prove the recipe at `supersample:1`
first, then use `supersample:2` or higher only for final GPU-device hero
renders after composition is accepted.
For final hero stills with high-contrast silhouettes, add
`render.reconstruction:"tent"` after checking the native-resolution image;
prefer it for floor grids, wireframes, and other line-heavy scenes because it
keeps stroke contrast. For visible floor grids, set
`scene.grid.line_width_px` around `3.6`-`4.2` so the antialiased stroke has
enough coverage at native resolution. Add
`expect_quality:{"profile":"product"}` when grid-line quality is load-bearing;
`recipe render --verify` then emits `grid_line_quality_checked` or fails with
`grid_line_quality_too_low`. Use `"gaussian"` only when a softer
silhouette resolve is acceptable. Keep the default `"box"` for deterministic
verification.
For softer studio highlights or a partial penumbra, add an area softbox light:
`{"id":"softbox","kind":"area","shape":"rect","preset":"softbox"}`. This is a
finite-emitter softbox with LTC-style specular evaluation and deterministic
soft-shadow visibility on CPU and HeadlessGpu. Use `shape:"rect"`, `"disc"`,
or `"sphere"` for the intended emitter shape. If the soft shadow is
load-bearing, add `expect_quality.area_light` targeting the receiver;
`recipe render --verify` then emits `area_light_soft_shadow_checked` for broad
finite emitters and fails point-like emitters with
`area_light_soft_shadow_insufficient`.
For hero product/documentation frames where the subject should stay crisp while
the background softens, prefer subject focus:
`render.depth_of_field:{focus:{mode:"subject",target:{kind:"import",id:"subject"}},coverage:"all",strength:"subtle"}`.
`recipe render` resolves that form with a semantic-AOV prepass over visible
target pixels, then uses the median visible subject depth as the focal plane.
Manual `focus_distance` remains available for advanced fixed-camera shots.
Choose a textured or structured `background_target` so blur is measurable. Add
`expect_quality.depth_of_field` with a focal `target`; `recipe render --verify`
renders a same-backend no-DoF baseline and emits `depth_of_field_checked` or
fails with actionable codes such as `depth_of_field_blur_insufficient`,
`depth_of_field_background_detail_missing`, or `depth_of_field_focal_softened`.

## Comparison Cards And Contact Sheets

For A/B comparison cards, keep the camera and scene constant. Do not use
`scene.preset` or auto-framing when every panel must share the same view; those
helpers are for single hero frames and may reposition each panel. Use one fixed
camera/look-at, fixed capture size, fixed background/environment, and vary only
one field per panel, such as `camera.lens`, a light preset,
`environment.preset`, `render.auto_exposure`, or material preset.

An auto-exposure comparison needs genuinely different luminance per panel. The
four presets can legitimately converge on a single metal ball under one HDRI,
so use panels with different dark/bright/mixed lighting if the goal is to show
the preset difference.
`supersample:8` is available only for small captures that stay within renderer
limits.

For CAD, documentation, dashboards, and tours with overlays, run
`recipe render --verify` and check `verification.composition`: labels must be
visible, uncropped, clear of leader, dimension, or helper lines, and clear of
other label regions. Failed `overlay_label_intersects_line` or
`overlay_label_intersects_label` checks are real composition failures, not
aesthetic warnings. A failed `overlay_label_clipped_by_viewport` means the
unclipped projected label rectangle extends outside the capture, even if some
text remains visible. When an object must sit on a floor or grid, add an
`expect_grounded` entry with the target node, `plane_y`, and tolerance; a failed
`ground_contact_missing` means the subject is visibly floating or sinking
relative to the declared floor. When a helper line, grid, or wireframe must be
behind a subject, add `expect_helper_occluded` with the helper and occluder
targets; a failed `helper_layer_overdraws_subject` means the helper is visibly
drawn over the subject interior.
For overlapping solid objects whose ordering is load-bearing, add
`expect_occlusion` with `front`, `back`, and optional `tolerance_pixels`.
Use high-contrast opaque front/back materials for this check: it is a
native-resolution color-probe, so `object_depth_order_color_ambiguous` fails
closed when the colours cannot be separated reliably. Treat
`object_depth_order_mismatch` as a real occlusion/depth failure.
For GPU or hero renders, add `expect_backend` with
`{"backend":"headless_gpu","gpu_device":true}` so CPU fallback fails
verification instead of silently producing a weaker proof. The composition
report emits `backend_expectation_mismatch` when the backend does not match,
and checked `render_antialiasing_active`, `render_supersample_active`, or
`render_reconstruction_active` entries when requested render knobs are active.
For cutaways or sectioned views, add `expect_clipping` with the expected
`active_clipping_planes`, `section_box_active`, and `section_box_inverted`
values. Treat `clipping_plane_count_mismatch`, `section_box_missing`, and
`section_box_inversion_mismatch` as real composition failures.

A section box and a clipping plane section **model geometry only**. Since
1.10.0 they do not remove annotations: labels, leader lines, and dimension lines
stay visible so a cutaway still explains itself. This is the behavior a
sectioned technical view needs — the whole point of the cutaway is usually to
annotate what it exposes.

If a specific annotation *should* be sectioned along with the geometry, opt it
in explicitly with `LabelDesc::with_scene_clipping(true)`. Generated callout and
measurement overlays are always exempt. There is no global switch
back to the old behavior; the opt-in is per-overlay so one dimension line can
clip while its neighbours do not.

Before 1.10.0 every annotation clipped against the section box, so a cutaway
silently lost its labels. If a stored render relied on that, opt the affected
labels back in.

Nodes removed by a section box or clipping plane are reported through the
existing `nodes_detail[].reason_codes` vocabulary — see
`docs/schema-contracts.md`. There is no `clipped_by_section_box` code;
plane-clipped nodes report `clipped_by_active_clipping_plane`, which is
`warning` severity and advisory.
For configurators or product renders with material variants, add
`expect_state` entries for the import id and expected
`active_material_variant`. Omit or set `active_material_variant:null` when the
default variant is intentional. Treat `material_variant_state_mismatch` as a
real state/variant failure.
When placement, scale, or orientation is load-bearing, add `expect_transform`
for the authored/imported node target with the expected world-space
`translation`, `scale`, and/or intrinsic X/Y/Z `rotation_degrees`. Treat
`transform_conformance_mismatch` as a real placement/composition failure.
Author both `imports[].transform` and `nodes[].transform` with an explicit
`kind`. Prefer `kind:"trs"` for agent-authored degree rotations and reserve
`kind:"raw"` for a known quaternion in `[x,y,z,w]` order. Do not emit the old
untagged import shape: it is read only as a 1.8.0 migration alias and produces
`legacy_transform_shape`; applying the suggested fix adds `kind:"raw"`.
When two declared parts must not intersect, add `expect_separation` with
targets `a` and `b`. Use `min_gap` only when clearance matters; otherwise
`min_gap:0` proves no world-bounds intersection. Treat
`separation_conformance_mismatch` as a real assembly/composition failure.
When `expect_quality.profile` is present, the composition report checks each
declared object's projected native-resolution region for framing, exposure,
subject/background salience, and decoded base-color texture result. Product
renders that set `render.profile:"product"` or
`render.auto_exposure:"product_studio"` also get severe subject exposure checks
by default for authored and imported objects. The render-quality profile is a
baseline: adding explicit checks such as `text`, `line`, `reflection`, or
`grounding` does not disable profile-derived geometry-edge or grid-floor checks.
Treat `subject_too_small_in_frame`, `subject_too_large_in_frame`,
`subject_black_crushed`, `subject_blown_out`, `subject_salience_too_low`, and
`texture_result_flat` as real render defects: change camera/framing, lighting,
exposure, material, background, UVs, or texture mapping before accepting the
frame.

## Dedicated Verifiers

Use focused verifiers when the task depends on a specific behavior:

```bash
scena verify appearance "$RECIPE" --expect appearance-expectation.json --out appearance.png
scena verify animation "$RECIPE" --clip <clip-name> --times 0,1 --expect-change
scena verify interaction "$RECIPE" --expect interaction-expectation.json
```

Appearance verification is for product/configurator/material correctness.
Animation verification is for digital twins and timed state changes. Interaction
verification is for pick, hover, and select workflows.

## Diagnose And Repair

For visibility or framing failures:

```bash
scena inspect "$RECIPE"
scena diagnose "$RECIPE" --visibility --handle <handle>
scena repair "$RECIPE" --from diagnosis.json
```

The positional target is validated before the report is planned. Raw assets
run through asset doctor; recipes run through their full effective sandbox and
build. Treat `asset_doctor`, `scene_recipe_validation`, or
`recipe_build_result` output as a target problem to correct first. A second
positional target is invalid.

Apply only repairs that return an explicit visual patch or recipe edit. If a
report says `auto_fixable:false`, stop and ask for host/user input.

## Scope Boundaries

`scena` owns rendering, scene graph state, assets, cameras, lights, materials,
interaction data, diagnostics, recipes, and visual proof.

The host application owns CAD kernels, DXF/DWG/B-rep parsing, constraints,
physics, simulation, robotics, PLC logic, pricing/SKU rules, networking,
persistence, and autonomous loops.
