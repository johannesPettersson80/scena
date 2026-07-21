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

// Rotation animation clips are rebound through the same coordinate basis.
let mixer = scene.create_animation_mixer(&import, "RotateArm")?;
scene.seek_animation(mixer, 0.5)?;
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

The coordinate option applies consistently to a node's rest transform and its
rotation animation. Linear and step quaternion keys are basis-conjugated before
sampling. For glTF `CUBICSPLINE` rotation channels, quaternion values are
normalized as rotations after conversion, while derivative tangents are
basis-conjugated without normalization. Translation animation follows the
selected axis mapping; scale and morph-weight animation remains dimensionless.
Skins, anchors, and connectors continue to resolve through their existing
import-local ownership and the same converted node hierarchy.

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
