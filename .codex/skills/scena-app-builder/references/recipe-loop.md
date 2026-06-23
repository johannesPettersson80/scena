# Recipe Loop

Use this loop for any LLM-built scena scene unless the user explicitly asks for
Rust API code.

## 1. Discover

Use a CLI built with app-builder features:

```bash
cargo install scena --features scene-host,inspection
```

When running from a local checkout, prefix commands with:

```bash
cargo run --bin scena --features scene-host,inspection --
```

Use the shipped schema instead of guessing JSON fields:

```bash
scena schema get scena.scene_recipe.v1 > scene_recipe.schema.json
```

Template names are explicit; there is no `examples agent list` command:

```bash
scena examples agent get primitive_scene --out target/scena-agent/primitive_scene > target/scena-agent/primitive_scene.manifest.json
```

The stdout is an `scena.agent_smoke_template.v1` manifest, not a recipe. Read
its `files[]`, `required_features[]`, and `commands[]`, then use the recipe file
written under `--out`. Set `RECIPE` to the manifest recipe path; for the command
above:

```bash
RECIPE=target/scena-agent/primitive_scene/recipe.json
```

## 2. Author

Create one declarative recipe JSON file and set `RECIPE` to its path. Prefer authored primitives,
materials, nodes, lights, cameras, labels, section boxes, callouts,
measurements, instance sets, animation clips, and particles when the recipe
schema supports them. Use glTF imports when the user supplies assets.

Rules:

- use globally unique ids;
- use meters and Y-up right-handed coordinates;
- use `rotation_degrees` only as documented by the schema;
- use opaque colors unless a field explicitly supports alpha;
- prefer ergonomic helpers in recipe form: `material.preset`, `camera.lens`,
  `camera.framing`, `scene.preset`, `scene.environment.preset`,
  `render.auto_exposure`, named color constants, `kind:"studio_rig"` lights,
  and the default `scene.grid.under_bounds:true`;
- use `scene.preset:"product_studio"` plus
  `render.auto_exposure:"product_studio"` for product/model shots unless a
  fixed exposure is explicitly required; `auto_exposure` and `exposure_ev` are
  mutually exclusive;
- add a `studio_rig` light or a key/fill/rim directional light rig and an HDRI
  environment for user-facing renders unless the task is deliberately flat or
  unlit;
- use an area `softbox` light only when a finite-emitter highlight or partial
  penumbra is load-bearing, and pair it with `expect_quality.area_light`;
- use `studio`, `neutral_gray`, or `dark_studio` backgrounds by default and
  raise capture size for images people will inspect;
- use `render.quality:"high"` or `anti_aliasing:"msaa4"` for smooth geometry
  edges; add `render.supersample:2..4` only for hero captures because cost
  grows with N^2;
- add `expect_backend` with `{"backend":"headless_gpu","gpu_device":true}` for
  GPU/hero renders so CPU fallback fails verification instead of weakening the
  proof;
- keep labels and callouts clear of leader, measurement, and helper lines;
- add `expect_grounded` for any object that must touch a floor, base, or grid;
- add `expect_helper_occluded` when a depth-tested helper line, grid, or
  wireframe must stay behind a solid subject;
- add `expect_occlusion` when one solid object must visibly occlude another
  overlapping object;
- add `expect_clipping` when clipping planes or a section box are load-bearing
  for the view;
- add `expect_state` when an import's material variant is load-bearing for the
  view;
- include an `expect_quality` profile for user-facing renders so composition
  verification catches object-level weak framing, black-crush, blown
  highlights, weak subject/background salience, and flat decoded texture
  results;
- mark optional assets as optional only when missing content is acceptable;
- keep host/domain state out of recipe JSON.

## 3. Validate

Run:

```bash
scena validate-recipe "$RECIPE"
```

If validation fails, fix the recipe from the diagnostic `path`, `code`,
`message`, and `help`. Do not work around validation by deleting requested
content unless the user agrees.

## 4. Render And Introspect

Run:

```bash
scena recipe render "$RECIPE" --introspect --out frame.png
```

Accept success only when:

- command exits 0;
- top-level `ok` is true;
- requested artifacts exist;
- no required content was skipped.

When the recipe has an `expect` block, add `--verify`; that mode emits the
combined recipe build/capture/introspection/verification report instead of the
plain render-introspection report.
For presentation or beauty output, add `--gpu`; never claim GPU output unless
the returned report says `capabilities.gpu_device == true`.

## 5. Diagnose And Repair

For failures:

```bash
scena inspect "$RECIPE"
scena diagnose "$RECIPE" --visibility --handle <handle>
scena repair "$RECIPE" --from diagnosis.json
```

Apply only repairs that produce an explicit visual patch or recipe edit. If the
diagnosis says `auto_fixable:false`, stop and report the host/user input needed.

## 6. Stop Conditions

Stop when the machine reports pass. Escalate instead of looping forever when:

- the same failure repeats after two repair attempts;
- a required asset is missing;
- a requested feature is outside scena scope;
- repair requires domain knowledge the host owns.

## Dedicated Verifiers

Use direct verifier commands when a task needs focused proof:

```bash
scena verify appearance "$RECIPE" --expect appearance-expectation.json --out appearance.png
scena verify animation "$RECIPE" --clip <clip-name> --times 0,1 --expect-change
scena verify interaction "$RECIPE" --expect interaction-expectation.json
```
