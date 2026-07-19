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
authored fallback image or missing material texture bytes that bind the generated
renderer fallback texture.

Use `AssetLoadOptions::with_strict_external_resources(true)` when missing
external buffers should fail loading instead of producing warnings, and
`AssetLoadOptions::with_strict_textures(true)` when missing external images
should fail instead of producing texture warnings. Cache-hit reports retain the
original warning, external-resource status, and material-fallback evidence while
reporting `fetched_bytes = 0` for the cache-hit call itself.
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

### Quantized geometry, morphs, animation, and skins

`KHR_mesh_quantization` POSITION accessors accept every extension-defined
signed or unsigned BYTE/SHORT representation, including non-normalized integer
`POSITION` values whose dequantization is carried by the node transform.
Malformed or unsupported integer NORMAL encodings fail with a semantic-specific
asset error instead of being treated as an absent normal stream.

Morph import preserves target order even when a target omits POSITION, carries
optional normal and tangent deltas into render preparation, and fans an animated
weight channel out to every renderable primitive of a multi-primitive mesh.
Normal-map sampling transforms tangent-space values through the morphed tangent
frame before lighting.

Imported animation validation permits the glTF static case of a one-key clip at
time zero while rejecting empty, non-finite, duplicate/non-monotonic, or
shape-mismatched channels. Imported U8, U16, and floating-point skin vectors
must contain finite, non-negative, non-zero-sum skin weights; valid vectors are
renormalized to sum to one after decoding.

### Import unit boundary

Non-meter imports carry `meters_per_unit()` on one synthetic placement root returned by
`SceneImport::roots()`. Source node translations, authored scales, instance transforms,
and animation scale keys stay in their source-local or dimensionless form below that
root, preventing unit factors from compounding through nested hierarchies. Inherited
anchor/connector locals stay in import units; explicitly unit-tagged anchors are converted
once into import-unit locals and retain their authored unit metadata. Marker locals must
not be pre-converted to meters before the placement root is composed.

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

PNG, JPEG, and WebP image paths decode natively without an opt-in feature.
Embedded glTF/GLB image bytes use a content-addressed in-memory path, so two
assets cannot alias merely because both images have the same document-local
index; a reused cache entry also keeps immutable source provenance. KTX2/Basis
and meshopt support are available through feature flags. See [Feature
flags](feature-flags.md).

Plain `.webp` image URIs and embedded `image/webp` buffer views use the native
WebP decoder. `EXT_texture_webp` texture-source rebinding remains deferred:
export a plain PNG, JPEG, or WebP source URI, or use `KHR_texture_basisu` with
the `ktx2` feature, when a texture depends on extension-selected fallback
rebinding.

### Shared descriptor snapshots

The existing `Assets::geometry`, `material`, `texture`, and `environment`
getters remain clone-returning for compatibility. Hot paths can instead use
the matching `*_snapshot` accessor to receive an `Arc` to the immutable
descriptor stored by `Assets`. Repeated snapshot resolution for an unchanged
handle shares the same allocation, including the texture's sampler and decoded
pixel storage.

A snapshot is a stable view of the descriptor revision from which it was
resolved. After `reload_scene` and `Scene::replace_import`, discard old handles
and snapshots, resolve them from the replacement `SceneAsset`, and call
`Renderer::prepare` again. The replacement boundary intentionally does not
mutate a previously returned `Arc` behind the caller's back.

Draco mesh compression is intentionally not decoder-backed yet. On native and
browser targets, optional `KHR_draco_mesh_compression` reports structured
degradation metadata and required Draco usage fails with
`AssetError::UnsupportedRequiredExtension`. Re-export Draco assets uncompressed
or with `EXT_meshopt_compression` until a real user asset requires selecting a
maintained Draco decoder and carrying it through feature, license, browser,
and native proof.

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
non-zero when any error finding is present. Runtime `ok=true` means no
error-severity finding was produced; warning-severity findings such as missing
external images, missing optional buffers, or material fallbacks still require
review when the host needs complete authored assets.

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
Optional `KHR_materials_transmission`, `KHR_materials_ior`, and
`KHR_materials_volume` values are parsed and sampled through CPU/reference
transmission-volume shading, and attached GPU backends can claim physical
glass when the target capability report has
`physical_glass_transmission=supported`. Required assets that depend on those
extensions should stay optional, ship a fallback material, or be deployed only
to lanes whose capability report carries that supported state; CPU/reference
and unattached factory lanes remain degraded.
