# Place And Connect Objects Without Matrix Math

Type: Guide.

Use connector frames when one object must be placed relative to another. Application code
should name connection points and let `Scene` solve the source node transform.

## Scene-Authored Connectors

```rust
let motor_shaft = scene.add_connector(
    ConnectorFrame::new(motor, Transform::at(Vec3::new(0.5, 0.0, 0.0))).named("shaft"),
)?;
let pump_drive = scene.add_connector(
    ConnectorFrame::new(pump, Transform::at(Vec3::new(-0.25, 0.0, 0.0))).named("drive"),
)?;

scene.connect_by_key(motor_shaft, pump_drive, ConnectOptions::default())?;
```

Use `ConnectionAlignment::ForwardToBack` when plug/socket frames should face each other:

```rust
let options = ConnectOptions::default().with_alignment(ConnectionAlignment::ForwardToBack);
scene.connect(source_frame, target_frame, options)?;
```

Use roll options when the part is correctly mated but should keep or snap its twist around
the connector forward axis:

```rust
let options = ConnectOptions::default()
    .with_alignment(ConnectionAlignment::ForwardToBack)
    .choose_nearest_roll_degrees(90.0);
scene.connect(source_frame, target_frame, options)?;
```

Use `Scene::preview_connection` or `Scene::validate_connections` before applying a batch in
an editor UI.

## Imported Connectors

For glTF assets with `extras.scena.connectors[]`, connect imports by stable names:

```rust
scene.connect_import_connectors(
    &source_import,
    "mount",
    &target_import,
    "mount",
    ConnectOptions::default(),
)?;
```

This preserves connector kind, allowed mates, tags, snap tolerance, roll policy, polarity,
and opaque metadata from the glTF extras.

## Locked Nodes

Use `Scene::lock_node_for_connections(node)` when an assembly helper must not move a node.
The solver returns `ConnectionError::ConnectionWouldMoveLockedNode` before mutating the
scene.
