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
| Native window | `native_window.rs` |
| Browser canvas | `browser_canvas.rs` |
| CAD-style anchors and connectors | `anchor_alignment.rs`, `connect_objects.rs`, `imported_anchor_connection.rs`, `industrial_connector_assembly.rs`, `coordinate_connector_repair.rs`, `coordinate_units.rs` |
| Industrial/static scene | `industrial_static_scene.rs` |
| Diagnostics | `beginner_diagnostics.rs`, `scene_inspection.rs` |

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

## Interaction-oriented examples

Use these when you are building model viewers, inspection tools, or editors:

- `orbit_controls.rs`
- `picking_selection_hover.rs`
- `layers_visibility.rs`
- `scene_inspection.rs`

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
