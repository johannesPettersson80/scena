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

## Make It Look Good

Correctness proof is not aesthetic proof. For scenes meant for a user-facing
screenshot or demo, add presentation defaults instead of relying on the flat
implicit setup:

```json
"lights": [
  { "id": "key", "kind": "directional", "preset": "key" },
  { "id": "fill", "kind": "directional", "preset": "fill" },
  { "id": "rim", "kind": "directional", "preset": "rim" }
],
"scene": {
  "background": { "kind": "studio" },
  "environment": {
    "kind": "uri",
    "uri": "tests/assets/environment/polyhaven/studio_small_03_1k.hdr"
  }
},
"render": {
  "quality": "high",
  "anti_aliasing": "msaa4",
  "supersample": 2
},
"capture": { "width": 1280, "height": 960 }
```

Use `studio` or `neutral_gray` for model/product inspection, `dark_studio` for
dashboards and twin state views, `white` or `transparent` for documentation
exports, and `custom` only when the user gave an explicit color. The default
environment is flat; the bundled HDRI gives reflections and material response.
Use real glTF/GLB assets for realistic products and digital twins. Use authored
primitives for functional/CAD/diagram/chart scenes and tests.
Use `quality:"high"` / `anti_aliasing:"msaa4"` for smooth geometry edges.
Use `render.supersample:2..4` only for hero captures or fine glossy/texture
details; it renders at N× resolution and downsamples, so cost grows with N^2.

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
