# Assets

`Assets` owns loading, decoding, caching, and logical resource handles.
Applications keep their own domain data and use `Assets` for renderable
resources.

## Primary format: glTF/GLB

glTF/GLB is the primary interchange format for `scena`.

Use glTF/GLB for:

- model viewers,
- CAD-style inspection exports,
- industrial visualization assets,
- textured meshes,
- animations,
- skins and morph targets,
- cameras and lights,
- authored metadata such as anchors and connectors.

Start with:

- `examples/glb_model_viewer.rs`
- `examples/animation.rs`
- `examples/imported_anchor_connection.rs`

## Loading

The typical flow is:

```rust
let mut assets = scena::Assets::new();
let asset = assets.load_scene("model.glb")?;

let mut scene = scena::Scene::new();
let import = scene.instantiate(&asset)?;
```

`Assets` performs asset work before rendering. The renderer consumes prepared
scene and asset state.

## External buffers and textures

glTF files may reference external `.bin` buffers or image files. `scena` keeps
fetching and decoding under `Assets`, then passes typed resource handles into
the scene and renderer.

For browser use, make sure your application serves model, buffer, and texture
files from paths that the browser can fetch.

## Units, axes, and handedness

Imported assets can carry unit and coordinate metadata. `scena` provides typed
import options and diagnostics for:

- source units,
- Y-up and Z-up assets,
- right-handed coordinate systems,
- connector basis vectors,
- imported bounds.

See [Units, axes, and handedness](guides/units-axes-handedness.md).

## Anchors and connectors

Anchors and connectors let assets describe intended placement points without
hard-coding matrix math in the application.

Use them for:

- snapping components together,
- CAD-style placement,
- industrial assemblies,
- repeatable fixture alignment,
- imported metadata overlays.

See:

- [Place and connect objects](guides/place-and-connect-objects.md)
- [Authoring glTF anchors and connectors](guides/authoring-gltf-anchors-connectors.md)

## Supported asset features

## Materials and textures

`scena` supports common material workflows:

- unlit materials,
- metallic-roughness materials,
- base-color textures,
- normal textures,
- metallic-roughness textures,
- occlusion textures,
- emissive textures,
- alpha modes,
- texture transforms,
- material variants.

KTX2/Basis and meshopt support are available through feature flags. See
[Feature flags](feature-flags.md).

## Unsupported or unavailable features

Unsupported required glTF extensions fail explicitly with structured asset
errors. Optional features report structured degraded or unsupported status when
the application can continue safely.

Use `SceneAsset::extension_diagnostics()` to inspect optional extension
handling in application code. Each `GltfExtensionDiagnostic` exposes the
extension name, support status, decoder policy, user-facing help text, and a
`suggested_fix()` string so importer UIs can tell users whether to enable a
feature flag, export a fallback material, or choose a different compression
path.

See [Errors](errors.md).

## Asset Doctor

Use the asset doctor when a model loads incorrectly or when deciding whether a
third-party glTF/GLB is ready for scena:

```bash
cargo run -p xtask -- asset-doctor path/to/model.glb
```

The command first runs the official Khronos glTF Validator CLI in stdout mode
(`gltf_validator -o <asset>`). Set `SCENA_GLTF_VALIDATOR` when the executable
has a different path. The official validator owns glTF specification
compliance; scena does not reimplement that subset.

After the official validator runs, the command emits scena-specific renderer
guidance as `scena.asset_doctor.v1` JSON. Each guidance entry includes a
severity, status, message, and `fix` string for issues such as required
clearcoat materials, Draco compression, feature-gated KTX2/meshopt assets, or
deferred WebP texture-source rebinding.

For example, a required `KHR_materials_clearcoat` asset gets an error telling
the user to export a fallback material or wait for the matching renderer
feature before making clearcoat required.
