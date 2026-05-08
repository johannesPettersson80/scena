# Troubleshooting Misplaced Assets

Type: Guide.

Use this checklist when an imported model is invisible, rotated sideways, scaled wrong, or
connects to another object in the wrong place.

## Start With Diagnostics

Run scene diagnostics before changing transforms:

```rust
for diagnostic in renderer.diagnose_scene_with_assets(&scene, &assets) {
    eprintln!("{}: {}", diagnostic.code(), diagnostic.message());
}
```

Warnings for missing cameras, invalid near/far values, objects behind the camera, bounds
outside the frustum, missing lights, or degraded backend capabilities usually explain blank
frames faster than manual matrix edits.

## Check Units

glTF is meter-based by default. If the source asset was authored in millimeters, inches, or
feet, declare that at import time:

```rust
let import = scene.instantiate_with(
    &asset,
    ImportOptions::gltf_default().with_source_units(SourceUnits::Millimeters),
)?;
```

Connection helpers fail with `ConnectionError::UnitMismatch` when manual connector frames
carry incompatible unit metadata.

## Check Axes

Standard glTF is Y-up right-handed. If a source file is Z-up right-handed, import with
explicit coordinate metadata:

```rust
let options = ImportOptions::gltf_default()
    .with_source_coordinate_system(SourceCoordinateSystem::ZUpRightHanded);
let import = scene.instantiate_with(&asset, options)?;
```

`scena` converts node transforms, anchor transforms, connector transforms, and animation
translations before placement helpers run.

## Check Handedness And Mirroring

Left-handed sources are not silently mirrored. Left-handed connectors return
`ConnectionError::HandednessMismatch`; left-handed mesh imports return
`InstantiateError::UnsupportedCoordinateSystem` until the renderer has explicit winding and
normal correction proof.

Negative determinant transforms are also rejected for connection solving because they can
flip triangle winding and normals.

## Check Pivots, Anchors, And Connectors

If an object connects with the right position but wrong rotation, inspect the connector
basis. Prefer `forward` and `up` fields in `extras.scena.connectors[]` over hand-authored
quaternions:

```json
{
  "name": "mount",
  "forward": [1.0, 0.0, 0.0],
  "up": [0.0, 1.0, 0.0]
}
```

Use `Scene::preview_connection` to inspect the proposed transform and connection line
before mutating the scene.

## Check Parent Transforms

Nested imported connector nodes move the import root by default so the part stays intact.
Opt-in reparenting must preserve the solved world transform. If an assembly unexpectedly
moves a locked node, the solver returns `ConnectionError::ConnectionWouldMoveLockedNode`
before changing the scene.

## Check Camera And Clipping

If the object exists but renders blank:

- ensure an active camera is set;
- call `frame_all`, `frame_node`, `frame_all_with_assets`, or `frame_node_with_assets`;
- check near/far values with `DepthRange::fit_sphere`;
- if orbit controls move a framed model farther away, use `OrbitControls::apply_to_scene`
  so the camera transform and depth range are updated together;
- verify camera layer masks and node visibility;
- inspect `diagnose_scene_with_assets` for asset bounds outside the frustum.

Avoid fixing blank frames by scaling or rotating model data first. Most blank-frame cases
are camera, clipping, visibility, or coordinate metadata problems.
