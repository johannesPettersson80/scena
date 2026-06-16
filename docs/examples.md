# Examples

The `examples/` directory is the fastest way to learn `scena`. Each example is
kept small and focused on one workflow.

Run an example:

```bash
cargo run --example glb_model_viewer
```

Compile all public examples:

```bash
cargo check --examples
```

## By task

| Task | Examples |
|---|---|
| First render | `first_visible_render.rs`, `headless_ci.rs` |
| Primitive geometry | `primitive_shapes.rs` |
| GLB model viewer | `glb_model_viewer.rs` |
| Camera framing | `camera_framing.rs` |
| Animation | `animation.rs` |
| Picking, hover, selection | `picking_selection_hover.rs` |
| Orbit controls | `orbit_controls.rs`, `orbit_controls_native_adapter.rs`, `orbit_controls_browser_adapter.rs` |
| Instancing | `instancing.rs` |
| Static batching | `static_batching.rs` |
| Labels and helpers | `labels_helpers.rs` |
| Layers and visibility | `layers_visibility.rs` |
| Measurement overlays | `MeasurementOverlay` in `Scene::add_measurement_overlay()`; rendered proof in `tests/measurement_visual_proof.rs` |
| Native window | `native_window.rs` |
| Browser canvas | `browser_canvas.rs` |
| Browser host contracts | `scene_host_contracts.rs`, `scene_host_release_1_7.rs`, `scene_host_browser_contracts.js` |
| CAD-style inspection, anchors, and connectors | `cad_inspection_viewer.rs` uses show-only, ghosting, selected-node framing, bounding-box/axes helpers, and measurement overlays; `guided_exploded_view.rs` writes assembled, exploded, and restored documentation frames; `assembly_connector_browser.rs` emits `scena.connector_browser.v1` for compatible/incompatible connector targets; `anchor_alignment.rs`, `connect_objects.rs`, `imported_anchor_connection.rs`, `industrial_connector_assembly.rs`, `coordinate_connector_repair.rs`, `coordinate_units.rs` |
| Industrial/static scene | `industrial_static_scene.rs` |
| Diagnostics and asset readiness | `beginner_diagnostics.rs`, `scene_inspection.rs`, `asset_catalog_picker.rs`; `Assets::validate_asset_catalog()` for `scena.asset_catalog.v1` manifests |

## Recommended learning order

1. `first_visible_render.rs`
2. `primitive_shapes.rs`
3. `glb_model_viewer.rs`
4. `camera_framing.rs`
5. `picking_selection_hover.rs`
6. `orbit_controls.rs`
7. `headless_ci.rs`
8. `browser_canvas.rs` or `native_window.rs`

## Output-oriented examples

Use these when you need generated images or regression artifacts:

- `headless_ci.rs`
- `glb_model_viewer.rs`
- `industrial_static_scene.rs`
- `labels_helpers.rs`
- `scene_host_contracts.rs`

## Interaction-oriented examples

Use these when you are building model viewers, inspection tools, or editors:

- `orbit_controls.rs`
- `picking_selection_hover.rs`
- `layers_visibility.rs`
- `scene_inspection.rs`
- `scene_host_browser_contracts.js`
- `scene_host_release_1_7.rs`

## Stable JSON contract examples

Use `scene_host_contracts.rs` to print representative
`scena.capability_report.v1`, `scena.scene_host_asset_import.v1`,
`scena.scene_inspection.v1`, `scena.annotation_projection.v1`, and
`scena.capture.v1` reports plus the `scena.visual_patch.v1` patch result and
`scena.host_event.v1` event batch from the native `SceneHostCore` path,
including visual patch selection, material variants, and label anchors. Use
`scene_host_browser_contracts.js` as the TypeScript/JavaScript shape for a
browser host that owns its own render cadence, applies visual patches, and
drains events with `drainEventsJson()`.

Use `scene_host_release_1_7.rs` for the release 1.7 public surface:
post-processing setters, instanced import, visibility/tint APIs, camera preset
framing, animation inventory/play/pause/advance, and eased transform/tint
updates.

Camera bookmarks and fly-to transitions use the same host-ticked camera state:
build a `CameraBookmark` from a `FramingOutcome`, pass its `state()` to
`SceneHostCore::set_camera_eased`, or store bookmarks on the viewer builders
with `with_camera_bookmark(s)`. Browser hosts can call `setCameraEased(...)` or
`setCameraBookmarkJson(...)`; both still require the host to advance time and
render explicitly.

Viewer profiles are builder presets for common app shapes. Use
`ViewerProfile::cad_inspection()`, `product()`, `industrial()`,
`model_viewer()`, or `documentation()` with
`headless_gltf_viewer(...).with_viewer_profile(profile)` or
`interactive_gltf_viewer(...).with_viewer_profile(profile)` when you want the
existing lighting, environment, background, grid, picking, and orbit-control
helpers wired consistently without creating a separate viewer engine.
The `viewer_profiles.rs` example renders all five presets to PNG artifacts.

Transform gizmo usage is shown in `simple_scene_editor_gizmo.rs`: the example
creates a selected mesh, computes a constrained translate drag from two
`GizmoRay` values, applies the returned `Transform`, attaches helper stroke
geometry, frames the result, and renders once. The example is intentionally a
renderer-side interaction adapter, not an undo/redo or CAD-document model.

Asset catalog readiness uses stable JSON contracts: deserialize a
`scena.asset_catalog.v1` manifest into `AssetCatalogV1`, call
`Assets::validate_asset_catalog(&catalog).await`, and consume the
`scena.asset_readiness_report.v1` result. For a ready generated-preview entry,
`render_asset_catalog_preview_png(&asset).await` returns deterministic PNG
bytes plus a stable FNV hash. The `asset_catalog_picker.rs` example selects the
first ready catalog entry, writes its generated preview, instantiates the same
asset into `SceneHostCore`, frames, renders, and writes the SceneHost PNG. The
host still owns catalog search, versioning, storage, approval workflow, and
business rules.

```bash
cargo run --example asset_catalog_picker --features scene-host -- target/catalog-picker
cargo run --example scene_host_release_1_7 --features scene-host
```

Use `headless_documentation_renderer.rs` when you need a CI/documentation
snapshot with callouts, leader lines, pixels, and the `scena.capture.v1`
descriptor metadata:

```bash
cargo run --example headless_documentation_renderer -- target/docs-render
```

Use `guided_exploded_view.rs` when an assembly/documentation workflow needs a
reversible presentation-only exploded view. The example imports a small
assembly, renders the assembled state, applies `ExplodedView` transform
updates, renders the exploded state, then restores the original transforms:

```bash
cargo run --example guided_exploded_view -- target/guided-exploded-view
```

Use `assembly_connector_browser.rs` when an assembly UI needs to list authored
connectors and preview compatible or rejected mates before committing a
connection:

```bash
cargo run --example assembly_connector_browser --features scene-host
```

Use `product_configurator.rs` when a host wants visual-only option groups over
material variants, tints, visibility, and camera state. The example stores
`scena.product_options.v1`, applies options through `VisualPatchV1`, and prints
active option JSON:

```bash
cargo run --example product_configurator --features scene-host
```

Use `application_builder_lab.rs` to exercise one mini-application for every
application-builder archetype in a single run: model viewer, CAD inspection,
digital-twin state playback, product configurator, industrial dashboard,
headless documentation renderer, agent render loop, data visualization,
animated viewer, interaction proof, browser contract handoff, guided tour, and
SceneHost host loop. Each archetype proves its render through the
`render_introspection` contract and fails closed if the frame is not ok, so the
run is a real per-archetype check rather than a screenshot dump. The example
writes PNGs plus capture/contract/introspection JSON and a findings report under
the output directory:

```bash
cargo run --example application_builder_lab --features scene-host -- target/gate-artifacts/application-builder-lab
```

The checked-in findings summary is
[`docs/checklists/application-builder-example-findings.md`](checklists/application-builder-example-findings.md).

Use `scena.presentation_timeline.v1` from `SceneHostCore` when a guided tour
needs host-ticked seek/advance over visual states, camera bookmarks, labels,
tints, transforms, and animation mixer sampling. Timeline seek emits and
applies a normal `VisualPatchV1`; the host remains responsible for ticking.

Use `SceneHostCore::apply_product_grounding_preset_json()` when a product or
viewer scene needs one call to compose studio visuals, a floor receiver, and
SSAO-backed contact darkening. The returned `scena.scene_host_grounding.v1`
report states the active paths and keeps directional shadow receiver support as
an explicit fallback until that proof gate is closed.

Golden JSON fixtures for these contracts live in
`tests/assets/stable-contracts/` and are checked by `tests/stable_contracts.rs`
plus `xtask doctor --full`. Intentional contract-shape changes must update the
matching fixture in the same reviewed change.

## Agent smoke templates

Use `scena examples agent <template> --out <dir>` to generate a small
CLI-runnable smoke template. The command writes a `scena.scene_recipe.v1`
recipe plus any expectation files needed by that workflow, then emits a
`scena.agent_smoke_template.v1` manifest containing the exact `scena` commands
to run and the artifacts they should create.

Ready templates:

- `product-configurator`
- `live-state-viewer`
- `web-viewer`
- `data-visualization`
- `animated-viewer`
- `interaction-proof`
- `cad-inspection`
- `documentation-renderer`

The CAD and documentation templates are CLI-runnable smoke workflows for asset
load, render introspection, visibility diagnosis, and recipe-authored section
boxes, measurements, callouts, and exploded views. Browser proof uses the
separate `scena browser-proof` wrapper over the wasm-pack + Playwright lanes;
the M6 lane rebuilds the browser-probe package before running the proof so it
does not use stale WASM.

```bash
cargo run --features scene-host -- examples agent interaction-proof --out target/scena-agent/interaction-proof
cargo run --features scene-host,inspection -- browser-proof scene-host --dry-run
```

## CAD-style placement examples

Use these when imported assets need stable placement by authored metadata:

- `anchor_alignment.rs`
- `assembly_connector_browser.rs`
- `connect_objects.rs`
- `imported_anchor_connection.rs`
- `industrial_connector_assembly.rs`
- `coordinate_connector_repair.rs`
- `coordinate_units.rs`

See also:

- [Place and connect objects](guides/place-and-connect-objects.md)
- [Units, axes, and handedness](guides/units-axes-handedness.md)
- [Authoring glTF anchors and connectors](guides/authoring-gltf-anchors-connectors.md)
