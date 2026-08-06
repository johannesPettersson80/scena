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

The default install is the core discovery, validation, capability, and
conversion CLI. Rendering and the self-verification loop are intentionally
opt-in through the one `agent` feature below; see
[`specs/cli-install-contract.md`](specs/cli-install-contract.md) for the measured
build/API tradeoff and packaged-install contract.

Plan an FBX conversion without installing the external converter yet:

```bash
scena-convert --json --input model.fbx --output model.glb --dry-run
```

Machine mode emits exactly one `scena.asset_conversion.v1` result for success
or failure and captures tool progress/warnings as diagnostics. Pass `--human`
when an operator wants plain text and live converter output.

For recipe and agent-template commands, install the agent-facing features:

```bash
cargo install scena --features agent
scena examples agent list
scena examples agent get primitive-scene --out scena-agent/primitive-scene
scena validate scena-agent/primitive-scene/recipe.json
scena validate-recipe scena-agent/primitive-scene/recipe.json --full
scena recipe build scena-agent/primitive-scene/recipe.json
scena recipe render scena-agent/primitive-scene/recipe.json --out first-scene.png
```

Use `scena validate <file>` for a fast, schema-dispatched check of any public
recipe, expectation, patch, or capability JSON before its consuming command.
`scena schema json <scena.*.vN>` exports draft 2020-12 JSON Schema and reports
the runtime/cross-field checks that JSON Schema cannot express.

The JSON result is render introspection by default. Existing scripts may keep
`--introspect`; it is an accepted compatibility no-op.

This sequence is portable from any working directory. The generated recipe
uses package-embedded sample assets and the licensed `studio` environment
preset; it does not require a cloned `scena` repository.
The opt-in `agent` feature is the one-step self-verification and
material-authoring surface: it enables `scene-host` and the native
`material-library` compiler; `scene-host` already enables `inspection`. Default
library builds remain feature-empty; do not redundantly request both lower-level
inspection features.
Commands that accept `<asset-or-recipe>` keep raw glTF/GLB on the direct asset
path, but always build a parsed recipe in full through the same sandbox as
`recipe build`. A rejected later import is a nonzero structured failure, never
a successful partial scene.

For a product/model hero screenshot, use the photo-intent path before writing a
camera rig or exposure constants:

```bash
scena photo render model.glb --out hero.png --report hero.report.json
```

The recipe-native equivalent is `photo.intent` with an explicit subject:

```json
{
  "schema": "scena.scene_recipe.v1",
  "imports": [{ "id": "subject", "uri": "model.glb" }],
  "photo": {
    "intent": "camera_behavior",
    "subject": { "kind": "import", "id": "subject" }
  }
}
```

The intent path handles composition, staging, subject metering, and focus from
the declared subject. It is the first path when you need a good product image
with no manual camera, exposure, or focus. Drop down to Rust framing and
lighting APIs only when the application intentionally owns those choices.

To replace a handmade model's flat material with a map-complete product finish,
list the offline catalog of 301 audited CC0 finishes and explicitly compile one
source archive:

```bash
scena materials list --category metal --query brushed
scena materials fetch ambientcg-metal009 --resolution 1k --out materials/brushed-steel/1k
scena materials fetch ambientcg-metal009 --resolution 2k --out materials/brushed-steel/2k
scena materials fetch ambientcg-metal009 --resolution 4k --out materials/brushed-steel/4k
```

Reference the emitted pack from a specific source material in the import:

```json
{
  "id": "subject",
  "uri": "model.glb",
  "edge_rounding": {},
  "material_bindings": [
    {
      "source_material": {
        "index": 0,
        "name": "machined_housing"
      },
      "material": {
        "material_pack": {
          "uri": "materials/brushed-steel/1k/scena-material-pack.json",
          "expected_archive_sha256": "<fetch-result archive_sha256>",
          "tile_size_m": 0.18
        }
      }
    }
  ]
}
```

Set the recipe photo profile to final:

```json
{
  "photo": {
    "intent": "camera_behavior",
    "quality": "final",
    "subject": { "kind": "import", "id": "subject" }
  }
}
```

Then render the final product still:

```bash
scena photo render product.recipe.json --gpu \
  --out product.png --report product.photo.json
```

The final profile derives studio staging, reflections, shadowed area lighting,
contact grounding, composition, exposure, 3840x2520 capture, SSAA 2, and tent
reconstruction. After framing, it selects the smallest installed `1k`, `2k`,
or `4k` sibling that retains at least one material texel per output pixel.

The catalog is bundled and deterministic. Only `materials fetch` downloads;
recipe build and render use local, hash-checked color, normal, and
occlusion/roughness/metallic maps. For provenance-sensitive builds, copy the
fetch result's archive hash into `expected_archive_sha256`. The source material
index is required; adding its exact name protects the recipe against a changed
or reordered GLB. A missing or mismatched selector rejects the complete
assignment instead of partially restyling the model. Omit bindings for parts
that should retain their imported appearance, or use the singular `material`
field to replace the complete imported subtree.

If a recipe intentionally references a model library outside the working
directory, authorize only that directory and reuse the option on every step:

```bash
scena policy recipe --allow-root /srv/models
scena validate-recipe recipe.json --full --allow-root /srv/models
scena recipe build recipe.json --allow-root /srv/models
scena recipe render recipe.json --out frame.png --allow-root /srv/models
```

`--allow-root` is repeatable. Roots must be existing directories and are
reported canonically under `policy.allowed_roots`; canonical resource paths
must remain below one of those roots, so `..` and symlink escapes remain denied.

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
cargo check --examples --all-features
```

## Create a first scene

For a PBR glTF/GLB model viewer, start with the high-level path:

```rust,no_run
# async fn first_render_example() -> Result<(), Box<dyn std::error::Error>> {
let first = scena::first_render_gltf_headless("machine.glb", 800, 600).await?;
for diagnostic in first.diagnostics() {
    eprintln!("{}", diagnostic.message());
}
# Ok(())
# }
```

The high-level viewer frames imported bounds and, only when the asset has no
authored light or environment, applies a neutral directional fallback against
a studio background. The structured diagnostic names `viewer.lighting` and
sets `fallback_applied` so the fallback is never silent. Use
`without_default_lighting()` with an explicit background for deliberately dark
diagnostic renders.

When assembling the lower-level scene yourself, lighting and background remain
your responsibility. This explicit unlit example is deterministic by design:

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
