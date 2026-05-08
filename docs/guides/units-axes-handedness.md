# Units, Axes, And Handedness

Type: Guide.

Imported assets must declare source units and coordinate system explicitly when they are not
standard glTF meter-based Y-up right-handed data.

```rust
let import = scene.instantiate_with(
    &scene_asset,
    ImportOptions::gltf_default()
        .with_source_units(SourceUnits::Millimeters)
        .with_source_coordinate_system(SourceCoordinateSystem::ZUpRightHanded),
)?;
```

`scena` converts imported node transforms, anchor transforms, connector transforms, and
animation translations into scene units before placement helpers run.

## Failure Modes

- Manual, unconverted connector frames with different source units fail with
  `ConnectionError::UnitMismatch`.
- Manual, unconverted connector frames with different source coordinate systems fail with
  `ConnectionError::CoordinateSystemMismatch`.
- Left-handed imported connectors fail with `ConnectionError::HandednessMismatch` until an
  explicit winding and normal policy exists.
- Left-handed mesh imports fail with `InstantiateError::UnsupportedCoordinateSystem` until
  the renderer has explicit front-face winding and normal correction proof.
- Negative-determinant connector or node transforms fail with
  `ConnectionError::FlippedConnection`.

Use `examples/coordinate_connector_repair.rs` as the repair pattern.
