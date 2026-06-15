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
| CAD-style inspection, anchors, and connectors | `cad_inspection_viewer.rs` uses show-only, ghosting, selected-node framing, bounding-box/axes helpers, and measurement overlays; `guided_exploded_view.rs` writes assembled, exploded, and restored documentation frames; `anchor_alignment.rs`, `connect_objects.rs`, `imported_anchor_connection.rs`, `industrial_connector_assembly.rs`, `coordinate_connector_repair.rs`, `coordinate_units.rs` |
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

Phase-2-dependent templates such as `cad-inspection` and
`documentation-renderer` currently emit structured deferred manifests instead
of runnable commands.

```bash
cargo run --features scene-host -- examples agent interaction-proof --out target/scena-agent/interaction-proof
```

## CAD-style placement examples

Use these when imported assets need stable placement by authored metadata:

- `anchor_alignment.rs`
- `connect_objects.rs`
- `imported_anchor_connection.rs`
- `industrial_connector_assembly.rs`
- `coordinate_connector_repair.rs`
- `coordinate_units.rs`

See also:

- [Place and connect objects](guides/place-and-connect-objects.md)
- [Units, axes, and handedness](guides/units-axes-handedness.md)
- [Authoring glTF anchors and connectors](guides/authoring-gltf-anchors-connectors.md)
