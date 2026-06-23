---
name: scena-app-builder
description: Use when an LLM needs to build, verify, debug, or iterate on a scena application, scene recipe, viewer, CAD inspection workflow, digital twin, product configurator, dashboard, documentation renderer, or agent verification loop using public scena CLI/schema/docs instead of repo internals.
---

# Scena App Builder

## Core Rule

Build through public surfaces first: schema discovery, scene recipes, CLI
validation, render introspection, verification, diagnostics, and repair. Do not
read `src/` or guess private Rust APIs unless the user explicitly asks for
library development or a Rust-only integration.

The success condition is machine-verifiable output, not a PNG that merely
exists. A render is complete only after the appropriate introspection or
verification report says it is complete.

## Start Here

1. Use a binary built with the agent-facing features.

For an installed CLI:

```bash
cargo install scena --features scene-host,inspection
```

From a local checkout:

```bash
cargo run --bin scena --features scene-host,inspection -- <command>
```

Most app-builder commands need `inspection`; recipe rendering and interaction
verification also need `scene-host`.

2. Discover the current contract:

```bash
scena schema get scena.scene_recipe.v1
```

If `scena` is not installed but you are inside the repository, use:

```bash
cargo run --bin scena --features scene-host,inspection -- schema get scena.scene_recipe.v1
```

3. Start from a template when possible. There is no `examples agent list`
command. Use one of:

`primitive_scene`, `cad_plate`, `dashboard_bars`, `machine_state_viewer`,
`product_configurator`, `product-configurator`, `live-state-viewer`,
`web-viewer`, `data-visualization`, `animated-viewer`, `interaction-proof`,
`cad-inspection`, `documentation-renderer`.

```bash
scena examples agent get primitive_scene --out target/scena-agent/primitive_scene > target/scena-agent/primitive_scene.manifest.json
```

The command prints an `scena.agent_smoke_template.v1` manifest to stdout and
writes the actual recipe, expectations, and artifacts under `--out`. Read the
manifest `files[]`, `required_features[]`, and `commands[]`; do not validate
the manifest as if it were the recipe. Set `RECIPE` to the recipe file named in
the manifest `files[]`; for the command above:

```bash
RECIPE=target/scena-agent/primitive_scene/recipe.json
```

4. Validate before rendering:

```bash
scena validate-recipe "$RECIPE"
```

5. Render with introspection, not just capture:

```bash
scena recipe render "$RECIPE" --introspect --out frame.png
```

This emits `scena.render_introspection.v1`. Add `--verify` when the recipe has
an `expect` block and you need the combined recipe build/capture/introspection/
verification result.
For beauty renders, add `--gpu`; CPU remains the default, and the report
`capabilities.backend` / `gpu_device` fields say which backend actually ran.

6. Make the output presentable when the user will look at it.

Use the ergonomic recipe fields first unless the task is intentionally flat,
technical, or unlit. These fields route to the same Rust helpers a Rust user
would call, so they are safer than hand-tuned low-level values:

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
  "quality": "high",
  "anti_aliasing": "msaa4",
  "supersample": 1,
  "reconstruction": "box"
},
"capture": { "width": 960, "height": 720 }
```

Use `product_studio` for product/model screenshots, `cad_studio` for technical
CAD/documentation scenes, and `industrial_studio` for dashboard or live-state
views. Add explicit `scene.background`, `scene.environment`, or `scene.grid`
only when you need to override the preset.

Prefer `material.preset` (`chrome`, `metal`, `rough_metal`,
`brushed_steel`, `plastic`, `clearcoat_plastic`, `satin`, `leather`,
`rubber`, `matte`, `clear_glass`, `frosted_glass`) before raw PBR fields.
Use `base_color` as an optional preset tint, then add scalar overrides such as
`roughness` only when needed. Prefer named color constants such as `orange`,
`gray`, `light_gray`, `dark_gray`, `charcoal`, `studio_backdrop`,
`warm_white`, and `cool_white` instead of ad hoc hex values when one matches.
Prefer `camera.lens` and `camera.framing` over manual camera distances.
`scene.environment:{ "preset":"studio" }` or `"neutral_studio"` uses the
bundled HDRI presets through the same asset policy as other recipe assets.
`scene.grid.under_bounds` defaults to `true`; leave it on for auto-sized floors.

Use `studio` or `neutral_gray` for product/model inspection, `dark_studio` for
dashboards and status views, `white`/`transparent` for documentation exports,
and `custom` only when the user gives a color. The default environment is flat;
the bundled HDRI (`tests/assets/environment/polyhaven/studio_small_03_1k.hdr`)
gives reflections and better material response. Import real
glTF/GLB assets for realistic products or twins; primitives are best for
functional scenes, CAD plates, diagrams, charts, and tests. For visible
primitive boxes or cylinders in product-style scenes, add a small `bevel` or
`fillet` value so edges catch light; unsupported primitive kinds reject those
fields instead of ignoring them. For large scenes with repeated distant parts,
author explicit high/low geometry resources and add node `lods[]` thresholds so
small-on-screen parts render with cheaper geometry; scena switches among those
declared resources and does not invent simplifications.
Use `quality:"high"` / `anti_aliasing:"msaa4"` for smooth geometry edges.
For product-style floor reflections, enable `scene.grid.reflection`; it is a
verified structured floor-reflection preset that does not require material SSR.
If it matters, add `expect_quality.reflection` and treat
`reflection_structure_missing` as a real render-quality failure.
For hero product/studio reflections, add
`render.screen_space_reflections:{strength,roughness,horizon_fraction,fade}`.
It mirrors rendered scene content in screen space for the floor band and
high-metallic/low-roughness materials such as chrome. Screen-edge and occluded
material samples fade back to the environment-lit material. Use bare
`expect_quality.reflection` for floor/reflection-surface checks, or add
`expect_quality.reflection.target:{kind:"node",id:"..."}` when a specific
chrome/mirror subject is load-bearing.
For portable recipe-authored glass, use scalar material fields only:
`transmission_factor`, `ior`, `thickness_factor`, `attenuation_distance`, and
`attenuation_color`. Do not use `transmission_texture` or `thickness_texture`;
recipes reject them until the GPU/WebGL2 texture-binding budget supports those
roles. If glass output is load-bearing, render with `--gpu`, add
`expect_backend`, and inspect the native-resolution image.
Use `render.supersample:2..4` only for hero captures or fine glossy/texture
details; it renders at N× resolution and downsamples, so cost grows with N^2.
Do not put large captures plus `supersample:2` into the default iteration loop:
on CPU or lavapipe this can take minutes. First prove the recipe at
`supersample:1`; use `supersample:2` or higher only for final GPU-device hero
renders after the composition is already accepted.
For visible floor grids, set `scene.grid.line_width_px` around `3.6`-`4.2` and
use `render.reconstruction:"tent"` on hero stills so grid lines have enough
native-resolution coverage without softening the whole image like `"gaussian"`.
Add `expect_quality:{"profile":"product"}` when grid-line quality is
load-bearing; `recipe render --verify` then emits `grid_line_quality_checked`
for a sufficiently reconstructed grid or fails with `grid_line_quality_too_low`.
For softer studio highlights or a partial penumbra, add an area softbox light:
`{"id":"softbox","kind":"area","shape":"rect","preset":"softbox"}`. This is a
finite-emitter softbox with LTC-style specular evaluation and deterministic
soft-shadow visibility on CPU and HeadlessGpu. Use `shape:"rect"`, `"disc"`,
or `"sphere"` for the intended emitter shape. When the soft shadow matters, add
`expect_quality.area_light` targeting the receiver so `recipe render --verify`
must emit `area_light_soft_shadow_checked` and will fail point-like emitters
with `area_light_soft_shadow_insufficient`.
For hero shots where the subject should pop from the background, add
`render.depth_of_field:{focus_distance,aperture_f_stop,radius_px}`. Use a
small `aperture_f_stop` and a textured or structured background, then add
`expect_quality.depth_of_field` with a focal `target` and optional
`background_target`; `recipe render --verify` compares against a same-backend
no-DoF baseline and emits `depth_of_field_checked` or actionable failures such
as `depth_of_field_blur_insufficient` and `depth_of_field_focal_softened`.

`ok:true` proves the requested content rendered and passed checks. It does not
mean the image is aesthetically good. Inspect the rendered image when visual
quality matters.

For A/B comparison cards and contact sheets, keep the comparison controlled.
Do not use `scene.preset` or auto-framing if each panel must share the same
view; those helpers are for single hero frames and may reposition panels.
Use one fixed camera/look-at, fixed capture size, fixed environment/background,
and vary exactly one field per panel (`camera.lens`, light preset,
`environment.preset`, `render.auto_exposure`, or material preset). An
auto-exposure comparison needs genuinely different scene luminance per panel;
four presets on the same metal ball under one IBL can converge visually even
though all presets are working.

Before accepting any user-facing render, do a native-resolution composition
review in explicit "what is wrong?" mode. Check the full frame, not only crops:
declared objects visible and correctly placed, no stale/extra content, labels
readable and attached, no helper lines over solid objects, objects grounded when
intended, materials/lights not black-crushed or blown out, and camera framing
appropriate for the app. If you find a problem, add the matching deterministic
expectation (`expect_grounded`, `expect_helper_occluded`, `expect_occlusion`,
`expect_transform`, `expect_separation`, `expect_quality`, etc.) or record a
new verifier gap; never treat the critic pass as a silent green gate.

7. If it fails, diagnose from structured JSON:

```bash
scena inspect "$RECIPE"
scena diagnose "$RECIPE" --visibility --handle <handle>
scena repair "$RECIPE" --from diagnosis.json
```

## Workflow Selection

- **Basic scene or app shell**: read `references/recipe-loop.md`.
- **CAD inspection, digital twin, configurator, dashboard, documentation, web
  viewer, interaction, or guided tour**: read `references/app-patterns.md`.
- **Blank frame, wrong color, missing asset, bad pick, tiny object, cropped
  labels, validation failure, or non-converging repair**: read
  `references/debugging.md`.

Load only the reference needed for the task.

## Verification Rules

- For static scenes, require `render_introspection.ok == true` and verify the
  expected object is visible at a reasonable size.
- For material/configurator work, use appearance expectations; pixel change
  alone is not enough.
- For animation or digital twins, sample time/state changes and verify the
  named target changes as expected.
- For interactive viewers, use synthetic pick/hover/select verification.
- For CAD/docs overlays, verify measurements/callouts/section boxes render and
  overlays are not cropped, tiny, or crossed by leader/dimension lines.
- For objects that must sit on a floor or grid, add `expect_grounded` with the
  target node, `plane_y`, and tolerance; treat `ground_contact_missing` as a
  real placement failure.
- For depth-tested helper lines, grids, or wireframes that must stay behind a
  subject, add `expect_helper_occluded`; treat
  `helper_layer_overdraws_subject` as a real render-layer failure.
- For overlapping solid objects whose depth order matters, add
  `expect_occlusion` with `front`, `back`, and optional `tolerance_pixels`.
  Use high-contrast opaque front/back materials; the current verifier is a
  native-resolution color probe and fails closed with
  `object_depth_order_color_ambiguous` when the colours cannot be separated.
  Treat `object_depth_order_mismatch` as a real occlusion/depth failure.
- For GPU or hero renders, add `expect_backend` with
  `{"backend":"headless_gpu","gpu_device":true}` so CPU fallback fails
  verification instead of silently weakening the proof. Treat
  `backend_expectation_mismatch` as a real backend/capability failure; checked
  `render_antialiasing_active`, `render_supersample_active`, and
  `render_reconstruction_active` entries confirm the requested quality knobs
  were actually active.
- For cutaways, clipping planes, and section-box views, add `expect_clipping`
  with `active_clipping_planes`, `section_box_active`, and
  `section_box_inverted`. Treat `clipping_plane_count_mismatch`,
  `section_box_missing`, and `section_box_inversion_mismatch` as real
  composition failures.
- For configurators and product renders with material variants, add
  `expect_state` entries for each load-bearing import variant. Omit
  `active_material_variant` for the default variant, or set it to the exact
  expected variant name. Treat `material_variant_state_mismatch` as a real
  state/variant failure.
- When object placement, scale, or orientation is load-bearing, add
  `expect_transform` for the authored/imported node target with the expected
  world-space `translation`, `scale`, and/or intrinsic X/Y/Z `rotation_degrees`.
  Treat `transform_conformance_mismatch` as a real placement/composition
  failure.
- When two declared parts must not intersect, add `expect_separation` with
  targets `a` and `b`. Use `min_gap` only when clearance matters; otherwise
  `min_gap:0` proves no world-bounds intersection. Treat
  `separation_conformance_mismatch` as a real assembly/composition failure.
- For overlay-heavy CAD, documentation, dashboard, or tour scenes, inspect
  `verification.composition` for `overlay_label_intersects_line` and
  `overlay_label_intersects_label`. Both are real composition failures: move
  labels, shorten/offset leaders, or reduce label size before accepting the
  render. Treat `overlay_label_clipped_by_viewport` the same way; it means the
  full projected label is off-frame even if part of the text is visible.
- When `expect_quality.profile` is present, `verification.composition` also
  checks each declared object region for object-level framing, exposure,
  salience, and decoded base-color texture result. The quality profile is a
  baseline: adding explicit `text`, `line`, `reflection`, or `grounding`
  checks does not turn off profile-derived geometry-edge or grid-floor checks.
  Treat
  `subject_too_small_in_frame`, `subject_too_large_in_frame`,
  `subject_black_crushed`, `subject_blown_out`, `subject_salience_too_low`,
  and `texture_result_flat` as real render defects: adjust camera, lighting,
  exposure, material, background, UVs, or texture mapping instead of accepting
  an `ok:true` capture.
- For browser claims, run the browser proof path. Do not substitute native
  headless proof for browser-rendered output.

## Direct Verification Commands

Use these when the recipe manifest or task asks for a dedicated verifier:

```bash
scena verify appearance "$RECIPE" --expect appearance-expectation.json --out appearance.png
scena verify animation "$RECIPE" --clip <clip-name> --times 0,1 --expect-change
scena verify interaction "$RECIPE" --expect interaction-expectation.json
```

For a local checkout, prefix each command with:

```bash
cargo run --bin scena --features scene-host,inspection --
```

## Scope Boundaries

Keep application/domain logic in the host:

- no CAD kernel, DXF/DWG/B-rep parsing, constraints, or feature recognition;
- no physics, simulation, particle lifetime integration, robotics, PLC logic,
  pricing rules, SKU logic, networking, or document model;
- no hidden render loop owned by scena. The host ticks time and owns state.

When the user asks for out-of-scope behavior, build the visual/rendering layer
that scena owns and clearly state what the host/kernel/simulation must provide.
