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

For a non-meter source, `scena` inserts one synthetic import placement root whose uniform
scale is `SourceUnits::meters_per_unit()`. `SceneImport::roots()` returns that placement
root. Source node translations and authored scales remain source-local beneath it, so a
nested hierarchy is converted to meters exactly once and animation scale keys remain
dimensionless. Coordinate-system conversion still applies to each source transform.

Anchor and connector locals remain expressed in the import's source-unit space until that
single root is composed. An anchor with an explicit `units` field is converted once into
the import-unit local space while retaining its authored unit metadata. Do not pre-convert
marker locals to meters; doing so would apply the placement-root scale a second time.

## Failure Modes

- Manual, unconverted connector frames with different source units fail with
  `ConnectionError::UnitMismatch`.
- Replacing a non-meter import placement root transform with a unit-scale transform loses
  the declared conversion. Preserve its scale when relocating the import.
- Manual, unconverted connector frames with different source coordinate systems fail with
  `ConnectionError::CoordinateSystemMismatch`.
- Left-handed imported connectors fail with `ConnectionError::HandednessMismatch` until an
  explicit winding and normal policy exists.
- Left-handed mesh imports fail with `InstantiateError::UnsupportedCoordinateSystem` until
  the renderer has explicit front-face winding and normal correction proof.
- Negative-determinant connector or node transforms fail with
  `ConnectionError::FlippedConnection`.

Use `examples/coordinate_connector_repair.rs` as the repair pattern.
