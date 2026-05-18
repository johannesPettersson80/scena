# Getting started

This guide gets a Rust application rendering with `scena`.

## Install

Add the library:

```bash
cargo add scena
```

Equivalent `Cargo.toml` entry:

```toml
[dependencies]
scena = "1.3"
```

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

```rust
use scena::{
    Assets, Color, GeometryDesc, MaterialDesc, PerspectiveCamera, Renderer, Scene, Transform,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new();
    let cube = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(80, 160, 255)));

    let mut scene = Scene::new();
    scene.mesh(cube, material).add()?;

    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::default(),
        Transform::default(),
    )?;
    scene.set_active_camera(camera)?;

    let mut renderer = Renderer::headless(320, 240)?;
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;

    Ok(())
}
```

The important rule is simple: build scene state, prepare renderer resources,
then render prepared frames.

## Load a GLB

Use `Assets` to load the asset and `Scene` to instantiate it:

```rust
let mut assets = scena::Assets::new();
let asset = assets.load_scene("assets/model.glb")?;

let mut scene = scena::Scene::new();
let import = scene.instantiate(&asset)?;
scene.frame_import(import)?;
```

The exact helper you choose depends on the example workflow. Start with
`examples/glb_model_viewer.rs` for a complete runnable model viewer.

## Choose an output path

Use headless rendering when you need deterministic output in tests or CI:

```rust
let mut renderer = scena::Renderer::headless(1280, 720)?;
```

Use native or browser examples when your application owns a window or canvas:

- `examples/native_window.rs`
- `examples/browser_canvas.rs`

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
