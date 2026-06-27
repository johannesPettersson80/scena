# Application Builder Lab Findings

Independent adversarial usability review of the application-builder tooling,
driven by `examples/application_builder_lab.rs`. The lab builds one
mini-application for each archetype in the application-builder roadmap and
writes PNG, capture, contract, and introspection artifacts per app.

Run it with:

```bash
cargo run --example application_builder_lab --features scene-host -- target/gate-artifacts/application-builder-lab
```

This review re-ran the lab from source, opened the generated PNGs, and read the
contract JSON for every archetype rather than trusting the example's own
self-description. Findings below are derived from those artifacts.

## Verdict

All 13 archetypes are buildable from public, documented APIs and render real
content through real render / SceneHost / contract paths. No archetype depends
on private internals, and the renderer-only boundary holds: domain logic
(CAD kernel, telemetry, pricing, app state, the app clock) stays host-side in
every case.

The remaining gaps are last-mile ergonomics and proof integrity, not missing
rendering capability.

## Proof-integrity finding (fixed in this pass)

The lab originally proved its renders two different ways, and only one was real:

- The nine SceneHost archetypes go through `render_host`, which runs
  `render_introspection` and fails closed unless `ok == true` (catches empty
  frame, no visible drawables, all-culled, NaN transforms). This is a real
  proof.
- The three raw-`Scene` archetypes (industrial dashboard, headless
  documentation, data visualization) went through `render_scene` +
  `ensure_capture_has_pixels`, which only checked
  `capture.descriptor.payload.byte_length != 0`. Because `byte_length` is
  `rgba8.len()` for a fixed `WIDTH * HEIGHT` capture, that value is never zero —
  the check could not fail and proved nothing.
- The model-viewer archetype asserted nothing at all about its pixels; it wrote
  a PNG and descriptor and moved on.

Fix applied in the first review pass:

- `render_scene` now runs the same `Renderer::introspect_capture` contract the
  SceneHost path uses, writes a `*.render-introspection.json` artifact, and
  fails closed on `ok == false`. Industrial dashboard, headless documentation,
  and data visualization now have the same fail-closed proof and artifact set as
  the SceneHost archetypes.
- `model_viewer` no longer uses a weaker non-uniform pixel heuristic. The
  high-level viewer helper now exposes `render_introspection`, so the model
  viewer emits the same fail-closed report as the SceneHost and raw-Renderer
  paths.

After the fix, all 13 archetypes prove their render through the
render-introspection contract.

## Ergonomics fixes applied after the adversarial review

- `FirstRender` and `HeadlessGltfViewer` now expose `render_introspection` and
  `render_introspection_json`, so high-level viewer examples can fail closed on
  the same `scena.render_introspection.v1` contract as SceneHost examples.
- SceneHost section boxes now accept zero-thickness planar bounds when a
  positive margin expands them into a real volume, so CAD-style plates no
  longer need caller-side bounds padding.
- CAD/documentation agent templates now emit runnable CLI smoke workflows for
  recipe validation, render introspection, visibility diagnosis, and
  recipe-authored section boxes, measurements, callouts, and exploded views.
- `Scene::frame_all_with_overlays` and `SceneHostCore::frame_all_with_overlays`
  frame visible geometry plus label anchors and reserve label-derived pixel
  margin, preventing the documentation cropping found in the first lab pass.
- `scena browser-proof [scene-host|m6]` wraps the existing wasm-pack +
  Playwright lanes and emits `scena.browser_proof_run.v1`, with `--dry-run`
  for deterministic agent command discovery.

## Defects found while reviewing artifacts

1. **Documentation render truncated its own annotations (fixed).** The first
   headless documentation PNG cut "SERVICE PANEL" off the right edge and
   "BODY WIDTH: 800 MM" off the left edge. The lab now uses
   `frame_all_with_overlays`, and
   `scene_host_frame_all_with_overlays_keeps_callout_labels_in_frame` asserts
   render introspection no longer emits a `cropped` warning.
2. **`framing.cropped` originally under-reported edge-truncated content.** For
   that same documentation frame the content bbox spans `x in [1, 318]` of 320
   with `fit_fraction 0.994`. The heuristic in `framing_summary`
   (`src/render/introspection.rs`) only fired when the bbox reached the exact
   outermost pixel (`<= 0` or `>= width - 1`), so content clipped one pixel
   short was not flagged. This is fixed: render introspection now treats
   near-edge content as a cropped warning, and overlay-aware framing prevents
   the documentation-frame crop.

## Per-application notes

| Archetype | Real path used | Artifact that proves it | Still weak / missing |
|---|---|---|---|
| Model viewer | `headless_gltf_viewer` + `ViewerProfile::model_viewer` | PNG + capture descriptor + `render-introspection ok` | No SceneHost event/patch loop on the first-render helper |
| CAD builder / inspection | `SceneHostCore` instantiate/frame/section/measure/callout/select | `render-introspection ok`; measurement `120.0 mm`; section-box bounds | CLI template now authors section boxes, measurements, callouts, and exploded views through `scene_recipe.v1` |
| Digital twin state viewer | Visual states + host-ticked timeline seek | `render-introspection ok`; seek applied `tints:1` | Telemetry / history / alarm semantics are host-side (correct) |
| Product configurator | Stored product options + apply patch groups | `finish-result applied material_variants:1`; option `active` set | Pricing / incompatibility / SKU persistence host-side (correct) |
| Industrial dashboard | Raw `Scene` + `Profile::Industrial` | `render-introspection ok` (added) | No telemetry binding / widget layer (correct boundary) |
| Headless documentation | Raw `Scene` + callouts + measurement overlay | `render-introspection ok` | Uses overlay-aware framing; page layout/prose stays host-side |
| Agent render loop | `validate_scene_recipe_json_with_assets` + render + introspection | recipe + validation JSON + `render-introspection ok` | Pure-shell agents should use the CLI, not the Rust validator directly |
| Scientific / data visualization | Procedural geometry + labels | `render-introspection ok` (added) | No chart grammar / axis-tick / per-mark verification |
| Animated viewer | Animation inventory + play/seek mixer | inventory `MoveTriangle` 1.0s + `render-introspection ok` | Host must choose semantic expectations per moving node |
| Interaction proof viewer | `SceneHost` hover/select pick path | host-event JSON: real hit + `selection_changed`; asserts hit handle | Run-stable handles are not portable identifiers across runs |
| `<scena-viewer>` browser app | Contract note + illustrative HTML only | HTML + contract JSON (no pixels) | Native lab cannot prove browser pixels; `scena browser-proof` now provides the one-command Playwright handoff |
| Guided tour / exploded assembly | Presentation timeline (camera bookmarks + explode patch) | `render-introspection ok` after seek to t=2.0 | Timeline JSON verbose to hand-author |
| SceneHost host-loop template | patch in -> prepare/render/capture -> events out | patch result + host-event JSON + `render-introspection ok` | Hosts still own app-state model + persistence (correct) |

## Concrete fix list (next)

Belongs in Scena:

1. [x] Add a "frame including overlays" helper (callouts, measurements, labels) so
   documentation renders do not crop their own annotations.
   Evidence: `Scene::frame_all_with_overlays`,
   `SceneHostCore::frame_all_with_overlays`, and
   `scene_host_frame_all_with_overlays_keeps_callout_labels_in_frame`.
2. [x] Provide a small presentation-timeline builder/authoring helper; hand-authored
   timeline JSON is verbose and error-prone.
   Evidence: `PresentationTimelineV1::new().with_camera_bookmark().at()` is
   used by the guided-tour lab.
3. [x] Add recipe/CLI overlay directives for measurements, section boxes,
   callouts, and exploded views so the CAD/documentation CLI smoke templates
   can author the full native workflows instead of only proving
   load/render/diagnose.
   Evidence: `scene_recipe_validation_accepts_overlay_authoring_sections` and
   `scena_examples_agent_cad_and_documentation_templates_are_runnable_with_overlay_notes`.

Nice-to-have:

4. [x] A one-command browser-proof wrapper around the wasm-pack + Playwright lane,
   so the browser archetype feels as turnkey as native headless rendering.
   Evidence: `scena browser-proof`, `scena.browser_proof_run.v1`, and
   `scena_cli_browser_proof`.
5. [x] Per-mark / per-target semantic verification for data visualization, beyond
   whole-frame render introspection.
   Evidence: `examples/application_builder_lab.rs` tags the peak throughput
   bar and writes `data-visualization.appearance-introspection.json` from a
   `node_bbox` appearance sample.

Acceptance for each item:

- The lab-generated `application-builder-lab.index.json` no longer reports the
  item as missing.
- A focused Rust or CLI test exercises the public surface that fixes the gap.
- The implementation uses existing renderer/SceneHost contracts; no app-domain
  document model, CAD kernel, telemetry loop, pricing rules, or chart grammar is
  introduced.
- `scena examples agent <template>` manifests stay machine-readable and runnable.

Belongs in the host application (correctly out of scope for Scena):

- CAD kernel, DXF/PDF-to-solid construction.
- Telemetry ingestion, history, units, alarm semantics.
- Pricing, option incompatibility, SKU persistence, business rules.
- App-state model, persistence, and the app clock.
- Chart grammar, axis/tick layout, and data binding.

## Note on the lab's self-report

The `worked_well` / `missing_or_awkward` strings in
`examples/application_builder_lab.rs` (and the generated
`application-builder-lab.findings.md`) are author commentary compiled into the
binary; they are narration, not measurement. The load-bearing evidence is the
per-app artifacts and their now fail-closed render-introspection checks.
