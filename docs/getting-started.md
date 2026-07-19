# Getting started

This guide gets a Rust application rendering with `scena`.

## Install

Add the library:

```bash
cargo add scena
```

`cargo add` resolves the current compatible release and avoids a version number
in this living guide drifting behind the package metadata.

Install the bundled CLI tool when you need it:

```bash
cargo install scena
scena-convert --help
```

## Run an example

Clone the repository and run the model-viewer example:

```bash
git clone https://github.com/johannesPettersson80/scena.git
cd scena
cargo run --example glb_model_viewer
```

Run a deterministic headless render:

```bash
cargo run --example headless_ci
```

Compile all public examples:

```bash
cargo check --examples
```

## Create a first scene

```rust,no_run
use scena::{Assets, Color, GeometryDesc, MaterialDesc, Renderer, Scene};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new();
    let cube = assets.create_geometry(GeometryDesc::box_xyz(0.8, 0.5, 0.35));
    let material = assets.create_material(MaterialDesc::unlit(Color::BLUE));

    let (mut scene, camera) = Scene::with_default_camera()?;
    scene.mesh(cube, material).add()?;
    scene.frame_all_with_assets(camera, &assets)?;

    let mut renderer = Renderer::headless(320, 240)?;
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    let capture = renderer.capture_rgba8(&scene, Default::default())?;
    capture.write_png("first-scene.png")?;

    Ok(())
}
```

The important rule is simple: build scene state, prepare renderer resources,
then render prepared frames.

## Load a GLB

Use `Assets` to load the asset and `Scene` to instantiate it:

```rust,no_run
use scena::{Assets, Renderer, Scene};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "assets/model.glb".to_owned());
    let assets = Assets::new();
    let asset = pollster::block_on(assets.load_scene(path.as_str())).map_err(|error| {
        std::io::Error::other(format!("failed to load GLB {path:?}: {error}"))
    })?;

    let mut scene = Scene::new();
    let import = scene.instantiate(&asset).map_err(|error| {
        std::io::Error::other(format!("failed to instantiate GLB {path:?}: {error}"))
    })?;
    let camera = scene.add_default_camera().map_err(|error| {
        std::io::Error::other(format!("failed to add the model-viewer camera: {error}"))
    })?;
    scene.frame_import(camera, &import).map_err(|error| {
        std::io::Error::other(format!("failed to frame GLB {path:?}: {error}"))
    })?;

    let mut renderer = Renderer::headless(640, 480)?;
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    let capture = renderer.capture_rgba8(&scene, Default::default())?;
    capture.write_png("model.png")?;
    Ok(())
}
```

The exact helper you choose depends on the example workflow. Start with
`examples/glb_model_viewer.rs` for a complete runnable model viewer.

## Choose an output path

Use headless rendering when you need deterministic output in tests or CI:

```rust,no_run
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _renderer = scena::Renderer::headless(1280, 720)?;
    Ok(())
}
```

Use native or browser examples when your application owns a window or canvas:

- `examples/native_window.rs`
- `examples/browser_canvas.rs`

## Package browser builds

When bundling a wasm build produced by `wasm-bindgen`, copy
`pkg/snippets/**` alongside `scena.js` and `scena_bg.wasm`. The
browser package imports inline JavaScript shims from that directory for
canvas color-space setup; omitting it can make the module fail before
Scena reports a render error.

## Add interaction

For picking, hover, selection, and controls, start with:

- `examples/picking_selection_hover.rs`
- `examples/orbit_controls.rs`
- `examples/orbit_controls_native_adapter.rs`
- `examples/orbit_controls_browser_adapter.rs`

## Next steps

- [API overview](api.md)
- [Rendering](rendering.md)
- [Assets](assets.md)
- [Examples](examples.md)
- [Troubleshooting](troubleshooting.md)
