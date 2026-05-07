# scena

`scena` is a Rust-native scene-graph renderer intended to become an easy,
production-safe replacement for Three.js in Rust applications.

## Happy Path

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
    let camera =
        scene.add_perspective_camera(scene.root(), PerspectiveCamera::default(), Transform::default())?;
    scene.set_active_camera(camera)?;

    let mut renderer = Renderer::headless(320, 240)?;
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    Ok(())
}
```

`prepare()` is explicit. Asset fetching and parsing belong to `Assets`; GPU upload,
pipeline specialization, batching, and render statistics belong to `Renderer::prepare`.
`render()` draws prepared state and returns structured errors instead of silently fetching,
uploading, or guessing.

Non-goals are explicit: this is not a game engine, not a simulation engine, not robotics
logic, not PLC/domain logic, and not physics. `scena` owns rendering, scene graph,
assets, authoring helpers, diagnostics, and testable output.

## Documentation

The charter is in [`docs/RFC-rust-3d-renderer.md`](docs/RFC-rust-3d-renderer.md).
Implementation contracts live in [`docs/specs/`](docs/specs/), especially the
[`public API`](docs/specs/public-api.md), [`module boundaries`](docs/specs/module-boundaries.md),
and [`release gates`](docs/specs/release-gates.md). Milestone checklists live in
[`docs/checklists/`](docs/checklists/), accepted decisions live in
[`docs/decisions/`](docs/decisions/), and the M5 API baseline lives in
[`docs/api/m5-public-api-baseline.txt`](docs/api/m5-public-api-baseline.txt).

Examples cover primitives, glTF/GLB model viewing, picking and interaction styling,
instancing, labels/helpers, animation, native surfaces, browser canvas descriptors,
headless CI, and the industrial static-scene profile.

Initial local checks:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo check --examples
cargo run -p xtask -- doctor --full
```

The repo doctor is the source-derived guardrail for contract drift and known silent-failure
families. See [`docs/specs/doctor-contract.md`](docs/specs/doctor-contract.md).
