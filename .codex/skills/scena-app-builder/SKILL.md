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

Use a 3-point light rig and a real environment unless the task is intentionally
flat, technical, or unlit:

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

Use `studio` or `neutral_gray` for product/model inspection, `dark_studio` for
dashboards and status views, `white`/`transparent` for documentation exports,
and `custom` only when the user gives a color. The default environment is flat;
the bundled HDRI gives reflections and better material response. Import real
glTF/GLB assets for realistic products or twins; primitives are best for
functional scenes, CAD plates, diagrams, charts, and tests.
Use `quality:"high"` / `anti_aliasing:"msaa4"` for smooth geometry edges.
Use `render.supersample:2..4` only for hero captures or fine glossy/texture
details; it renders at N× resolution and downsamples, so cost grows with N^2.

`ok:true` proves the requested content rendered and passed checks. It does not
mean the image is aesthetically good. Inspect the rendered image when visual
quality matters.

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
  overlays are not cropped or tiny.
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
