# LLM App Builder Guide

This guide is the public, model-agnostic companion to the repo-hosted
`.codex/skills/scena-app-builder` skill. Use it when asking Codex, Claude Code,
or another shell-capable LLM to build a `scena` model viewer, CAD inspection
scene, digital twin, product configurator, dashboard, documentation renderer,
or interaction proof.

## Required CLI Build

Install or run the CLI with the app-builder features:

```bash
cargo install scena --features scene-host,inspection
```

From a local checkout:

```bash
cargo run --bin scena --features scene-host,inspection -- <command>
```

Most agent-facing commands require `inspection`; recipe rendering and
interaction verification also require `scene-host`.

## Public-Surface Workflow

Do not guess recipe fields or read renderer internals first. Use public schema,
template, validation, render, inspection, diagnosis, and repair surfaces.

Discover the schema:

```bash
scena schema get scena.scene_recipe.v1 > scene_recipe.schema.json
```

Start from a template when possible. There is no `examples agent list`
subcommand; valid template names are:

- `primitive_scene`
- `cad_plate`
- `dashboard_bars`
- `machine_state_viewer`
- `product_configurator`
- `product-configurator`
- `live-state-viewer`
- `web-viewer`
- `data-visualization`
- `animated-viewer`
- `interaction-proof`
- `cad-inspection`
- `documentation-renderer`

Generate a template:

```bash
scena examples agent get primitive_scene --out target/scena-agent/primitive_scene > target/scena-agent/primitive_scene.manifest.json
```

The command prints an `scena.agent_smoke_template.v1` manifest to stdout and
writes the actual recipe, expectations, and artifacts under `--out`. Read the
manifest `files[]`, `required_features[]`, and `commands[]`; do not validate the
manifest as a recipe. Set `RECIPE` to the manifest recipe path; for the command
above:

```bash
RECIPE=target/scena-agent/primitive_scene/recipe.json
```

Validate before rendering:

```bash
scena validate-recipe "$RECIPE"
```

Render with introspection:

```bash
scena recipe render "$RECIPE" --introspect --out frame.png
```

Success means the command exits 0 and the top-level report says `ok:true`.
Never claim success from a PNG path or nonzero byte length alone.
When the recipe has an `expect` block, add `--verify`; that mode emits the
combined recipe build/capture/introspection/verification report instead of the
plain render-introspection report.
For presentation or beauty output, add `--gpu`; CPU remains the default, and
the report `capabilities.backend` / `gpu_device` fields say which backend
actually ran.

For CAD imports that render as an edge sliver or white-on-white blob, run the
inspection preset instead of hand-tuning a single camera:

```bash
scena recipe inspect-cad "$RECIPE" --out-dir target/cad-inspection
```

It generates broad-face, top-feature, and overview recipes, renders each through
`recipe render --introspect --verify`, then writes PNGs and
`scena.cad_inspection_result.v1`. Generated CAD inspection recipes apply
presentation-only `imports[].material`, `imports[].edge_emphasis`, and a
principal-face camera where appropriate; these controls do not change the
source geometry or CAD truth.

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
environment is flat; the bundled HDRI
(`tests/assets/environment/polyhaven/studio_small_03_1k.hdr`) gives reflections
and material response.
Use real glTF/GLB assets for realistic products and digital twins. Use authored
primitives for functional/CAD/diagram/chart scenes and tests. For visible
primitive boxes or cylinders in product-style scenes, add a small `bevel` or
`fillet` value so edges catch light; unsupported primitive kinds reject those
fields instead of ignoring them. For large scenes with repeated distant parts,
author explicit high/low geometry resources and add node `lods[]` thresholds so
small-on-screen parts render with cheaper geometry; scena switches among those
declared resources and does not invent simplifications.
Use `quality:"high"` / `anti_aliasing:"msaa4"` for smooth geometry edges.
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
the background softens, add
`render.depth_of_field:{focus_distance,aperture_f_stop,radius_px}`. Use a
small `aperture_f_stop`, keep `radius_px` moderate, and choose a textured or
structured `background_target` so blur is measurable. Add
`expect_quality.depth_of_field` with a focal `target`; `recipe render
--verify` renders a same-backend no-DoF baseline and emits
`depth_of_field_checked` or fails with actionable codes such as
`depth_of_field_blur_insufficient`, `depth_of_field_background_detail_missing`,
or `depth_of_field_focal_softened`.

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
For configurators or product renders with material variants, add
`expect_state` entries for the import id and expected
`active_material_variant`. Omit or set `active_material_variant:null` when the
default variant is intentional. Treat `material_variant_state_mismatch` as a
real state/variant failure.
When placement, scale, or orientation is load-bearing, add `expect_transform`
for the authored/imported node target with the expected world-space
`translation`, `scale`, and/or intrinsic X/Y/Z `rotation_degrees`. Treat
`transform_conformance_mismatch` as a real placement/composition failure.
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

Apply only repairs that return an explicit visual patch or recipe edit. If a
report says `auto_fixable:false`, stop and ask for host/user input.

## Scope Boundaries

`scena` owns rendering, scene graph state, assets, cameras, lights, materials,
interaction data, diagnostics, recipes, and visual proof.

The host application owns CAD kernels, DXF/DWG/B-rep parsing, constraints,
physics, simulation, robotics, PLC logic, pricing/SKU rules, networking,
persistence, and autonomous loops.
