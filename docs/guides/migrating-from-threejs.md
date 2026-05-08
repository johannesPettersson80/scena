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

Instead of manually calculating camera distance, ask the scene to frame known bounds:

```rust
let camera = scene.add_default_camera()?;
scene.frame_all(camera)?;
scene.set_active_camera(camera)?;
```

For geometry created directly through `Assets`, use the asset-aware helper:

```rust
scene.frame_all_with_assets(camera, &assets)?;
```

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
