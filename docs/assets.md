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

`Assets::load_scene_with_report()` returns a stable
`scena.asset_load_report.v1` JSON view through
`AssetLoadReport<SceneAsset>::to_schema_json()`. The report includes fetched
bytes, cache-hit state, external buffer/image counts, geometry summary,
progress events, external buffer/image status rows, typed missing-resource
warnings (`external_buffer_missing` and `external_image_missing`), and material
fallback provenance such as optional compressed texture sources that used an
authored fallback image.

Use `AssetLoadOptions::with_strict_external_resources(true)` when missing
external buffers or images should fail loading instead of producing warnings.
Cache-hit reports retain the original warning, external-resource status, and
material-fallback evidence while reporting `fetched_bytes = 0` for the cache-hit
call itself.
Asset-aware `scena.scene_inspection.v1` reports reuse the same material source
evidence for rendered nodes and draw rows, including source material index,
generated-default reason, texture provenance, and matching fallback rows.

Loaded scene assets, textures, and environments expose generic
`AssetProvenance` metadata with source path, optional source SHA-256, optional
license/generator metadata, and generated derivatives. `AssetProvenance` is a
nested serde value contract; it is versioned through the containing asset-load
or summary reports rather than carrying its own top-level schema string.

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
- optional `KHR_materials_clearcoat` scalar factor/roughness parsing plus
  clearcoat, clearcoat-roughness, and clearcoat-normal texture-slot
  sampling for the CPU/reference material path and GPU shader/material path,
- optional `KHR_materials_sheen` color/roughness factor parsing plus sheen
  color and sheen roughness texture-slot sampling for the CPU/reference
  material path and GPU shader/material path,
- optional `KHR_materials_anisotropy` strength/rotation parsing plus
  anisotropy direction/strength texture-slot sampling for the CPU/reference
  material path and GPU shader/material path,
- optional `KHR_materials_iridescence` factor, IOR, thickness-range parsing
  plus iridescence factor/thickness texture-slot sampling for the
  CPU/reference material path and GPU shader/material path,
- optional `KHR_materials_dispersion` factor parsing plus CPU/reference and
  GPU shader/material channel-spread shading,
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
scena doctor path/to/model.glb
```

The xtask command first runs the official Khronos glTF Validator CLI in stdout mode
(`gltf_validator -o <asset>`). Set `SCENA_GLTF_VALIDATOR` when the executable
has a different path. The official validator owns glTF specification
compliance; scena does not reimplement that subset.

The runtime API emits the same renderer-owned finding shape without depending
on the external validator: `Assets::doctor_asset_path()` diagnoses a path,
`Assets::doctor_loaded_asset()` diagnoses an already loaded `SceneAsset`,
`SceneHostCore::asset_doctor_json()` returns the same JSON for native hosts,
and browser hosts can call `SceneHost.assetDoctorJson(url)`. The `scena doctor`
CLI prints the runtime `scena.asset_doctor.v1` report to stdout and exits
non-zero when any error finding is present.

Every finding includes `severity`, stable `code`, `path`, `message`, `help`,
and `suggested_fix`. CLI and library diagnostics share codes where checks
overlap, including `unsupported_required_extension`, `extension_supported`,
`extension_degraded`, `external_buffer_missing`, `external_image_missing`, and
`material_fallback_used`.

After the official validator runs, the xtask command also emits
scena-specific renderer guidance as `scena.asset_doctor.v1` JSON. Each guidance
entry includes the normalized fields above plus the historical `fix` string for
issues such as required clearcoat, sheen, anisotropy, iridescence, or
dispersion materials, Draco compression, feature-gated KTX2/meshopt assets, or
deferred WebP texture-source rebinding.

For example, optional `KHR_materials_clearcoat` factors and texture slots are
preserved and the CPU/reference plus GPU shader/material paths sample
clearcoat, roughness, and clearcoat-normal texture channels, but a required
clearcoat asset still gets an error when its look may depend on approved
backend screenshot or readback proof that is not yet release-proven.
Optional `KHR_materials_sheen` factors and texture slots are also preserved
and sampled through the CPU/reference plus GPU shader/material paths, with the
same required-extension release-proof guard.
Optional `KHR_materials_anisotropy` factors and texture slots are preserved
and sampled through the same CPU/reference plus GPU shader/material paths; a
required anisotropy asset keeps the same release-proof guard until approved
backend evidence exists.
Optional `KHR_materials_iridescence` factors and texture slots are preserved
and sampled through the same CPU/reference plus GPU shader/material paths; a
required iridescence asset keeps the same release-proof guard until approved
backend evidence exists.
Optional `KHR_materials_dispersion` factors are preserved and sampled through a
CPU/reference plus GPU shader/material channel-spread path; a required
dispersion asset keeps the same release-proof guard until approved backend
evidence and full transmission/volume glass behavior exist.
