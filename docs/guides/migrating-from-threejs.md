# Migrating Common Three.js Workflows

Type: Guide.

`scena` does not copy Three.js method names. It keeps the workflow but changes the failure
mode: typed handles, explicit `prepare()`, structured diagnostics, and renderer-owned proof
instead of silent runtime guesses.

## Load A Model

Three.js loaders usually return objects that can be inserted directly into a scene. In
`scena`, `Assets` owns loading and `Scene` owns instantiated nodes:

```rust
let assets = Assets::new();
let asset = assets.load_scene("model.gltf").await?;
let mut scene = Scene::new();
let import = scene.instantiate(&asset)?;
```

Use `SceneImport` for roots, names, paths, anchors, connectors, clips, and diagnostics.
Do not keep string paths as long-lived object IDs.

## Frame The Camera

In Three.js, camera fitting often starts from a `Box3`, a center point, and a
hand-maintained orbit target:

```js
const box = new THREE.Box3().setFromObject(model);
const center = box.getCenter(new THREE.Vector3());
const size = box.getSize(new THREE.Vector3());
const distance = Math.max(size.x / aspect, size.y) / (2 * Math.tan(fovY / 2));

camera.position.set(center.x + distance * 0.8, center.y + distance * 0.35, center.z + distance * 0.7);
controls.target.copy(center);
controls.update();
```

In `scena`, ask the scene to frame known bounds in the actual viewport and
seed the orbit controller from the framing result:

```rust
let bounds = import.bounds_world(&scene).ok_or("model has no bounds")?;
let camera = scene.add_perspective_camera(
    scene.root(),
    PerspectiveCamera::default().with_aspect(width as f32 / height as f32),
    Transform::default(),
)?;
let framing = scene.frame_bounds(
    camera,
    bounds,
    FramingOptions::new()
        .three_quarter_front_right()
        .fill(0.72)
        .margin_px(48.0)
        .viewport(width, height),
)?;
let controls = OrbitControls::from_framing(framing);
```

`frame_bounds()` solves from projected AABB corners on both axes, so wide
objects on portrait/mobile viewports do not get clipped or under-filled.

Three.js turntable code often stores the view in `Spherical::theta` and
`Spherical::phi`. In `scena`, the same intent is expressed as azimuth from the
front axis and elevation above the horizon:

```js
// Three.js
controls.target.copy(box.getCenter(new THREE.Vector3()));
const spherical = new THREE.Spherical().setFromVector3(
  camera.position.clone().sub(controls.target),
);
spherical.theta = THREE.MathUtils.degToRad(-28);
spherical.phi = THREE.MathUtils.degToRad(90 - 18);
camera.position.copy(
  new THREE.Vector3().setFromSpherical(spherical).add(controls.target),
);
```

```rust
// scena
scene.frame_bounds(
    camera,
    bounds,
    FramingOptions::new().azimuth_elevation(-28.0, 18.0),
)?;
```

For a complete load-light-floor-frame-render flow, see
[Easy scene setup](easy-scene-setup.md).

## Connect Objects

Prefer named anchors or connectors over copied matrices:

```rust
scene.connect_import_connectors(
    &source_import,
    "mount",
    &target_import,
    "socket",
    ConnectOptions::default().with_alignment(ConnectionAlignment::ForwardToBack),
)?;
```

This lets `scena` validate units, axes, handedness, roll policy, and stale connector names
before the source node is moved.

## Render

Rendering is a two-step lifecycle:

```rust
renderer.prepare_with_assets(&mut scene, &assets)?;
renderer.render_active(&scene)?;
```

`render()` must not hide asset fetches, first-use GPU uploads, shader compilation, or
browser initialization. Structural scene, asset, surface, or environment changes require
another `prepare()`.

## Pick And Select

Use camera-ray picking helpers instead of projecting coordinates manually:

```rust
let viewport = Viewport::new(width, height, device_pixel_ratio)?;
let cursor = CursorPosition::physical(x, y);
scene.pick_and_select_with_assets(camera, cursor, viewport, &assets)?;
```

Picking uses camera rays and world-space triangles. Use `pick_with_assets` for the normal
`scene.mesh(...)`/glTF workflow, or `pick_and_select_with_assets`/`pick_and_hover_with_assets`
when UI state should update in one call. Plain `pick` exists for legacy primitive
renderables. A miss should be treated as a diagnostic problem only after camera framing,
layer masks, and visibility are checked.

## Materials And Current Limits

Three.js has mature GPU material and postprocessing stacks. This checkout still reports
PBR, directional shadows, and WebGL2 material parity as degraded until backend visual proof
lands. Use capability reports and diagnostics as the source of truth for what a backend can
show today.
