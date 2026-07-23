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
bytes, cache-hit state, requested and cache-entry load options, external
buffer/image counts, geometry summary,
progress events, external buffer/image status rows, typed missing-resource
warnings (`external_buffer_missing` and `external_image_missing`), and material
fallback provenance such as optional compressed texture sources that used an
authored fallback image or missing material texture bytes that bind the generated
renderer fallback texture. Geometry decisions are explicit too:
`computed_flat_normals` identifies a primitive that omitted `NORMAL`, while
`skin_influences_truncated` identifies how many vertices exceeded the prepared
four-influence limit after `JOINTS_0/1` and `WEIGHTS_0/1` were combined.

Use `AssetLoadOptions::with_strict_external_resources(true)` when missing
external buffers should fail loading instead of producing warnings, and
`AssetLoadOptions::with_strict_textures(true)` when missing external images
should fail instead of producing texture warnings. Use
`with_fetch_byte_limit(n)` to cap the combined scene and external-resource bytes
for one load. These are the complete semantic scene-load options in this
release: both strictness flags and the byte limit affect cache eligibility.

The scene cache never keys on a path alone. An exact option match is reusable;
an entry produced under different options is reusable only when its retained
telemetry proves the active request was satisfied (no relevant missing-resource
warning and fetched bytes within the requested limit). A successful strict load
can therefore satisfy a later lenient request without a duplicate entry, while
a lenient warning can never bypass a later strict request. Cache-hit reports
retain the original warning, external-resource status, and material-fallback
evidence while reporting `fetched_bytes = 0` for the cache-hit call itself.
Inspect `AssetLoadReport::options()` and `cache_entry_options()`—or the matching
JSON fields—to distinguish the active request from compatible cached evidence.
Operator-owned recipe sandbox limits remain active on every resolution; cache
reuse cannot weaken the current root, strictness, or fetch-budget decision.
The CLI exposes no unsandboxed mode. A recipe may add one or more explicit
external libraries with repeatable `--allow-root <directory>` on validation,
build, render, inspect, diagnose, doctor, and repair. Roots must exist and are
canonicalized before policy construction. Each resource is canonicalized
separately before containment checks, so parent traversal and a symlink inside
an allowed root cannot escape to an unauthorized file. `scena policy recipe
--allow-root <directory>` prints the exact effective policy without loading a
recipe; recipe command results repeat it under `policy`.
Asset-aware `scena.scene_inspection.v1` reports reuse the same material source
evidence for rendered nodes and draw rows, including source material index,
generated-default reason, texture provenance, and matching fallback rows.

Loaded scene assets, textures, and environments expose generic
`AssetProvenance` metadata with source path, optional source SHA-256, optional
license/generator metadata, and generated derivatives. `AssetProvenance` is a
nested serde value contract; it is versioned through the containing asset-load
or summary reports rather than carrying its own top-level schema string.

## Bundled environment presets and template assets

Use `Assets::load_environment_preset(EnvironmentPreset::Studio)` or recipe
`scene.environment:{"preset":"studio"}` for the packaged Poly Haven studio
HDR. Named presets resolve inside `Assets` from exact `scena://bundled/...`
identifiers, so loading never depends on the process working directory and the
renderer never fetches asset bytes. Recipe fetch and texture budgets still
apply to the encoded builtin source.

`scena examples agent` likewise uses a small exact catalog of package-embedded
scena-authored glTF fixtures. Arbitrary `scena://` input remains rejected; only
cataloged builtin identifiers bypass local-root resolution. Source files,
SHA-256/license metadata, and attribution are packaged under
`tests/assets/environment/PRESET-LICENSES.md` and
`tests/assets/gltf/AGENT-TEMPLATE-ASSETS-LICENSE.md`.

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

Their shared extras transform grammar is validated during the asset
transaction. Translation/scale/quaternion components must be finite, scales
must be nonzero, forward/up must be authored as a nonzero nonparallel pair, and
matrix transforms must be finite affine 4x4 values with an exact TRS
decomposition. Invalid metadata fails loading with its
`nodes[n].extras.scena.anchors[n]` or `connectors[n]` path; it is never silently
replaced with identity or deferred until partial scene instantiation.

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

### glTF scene selection

By default, scena imports the document's declared default scene, otherwise the
first declared scene, otherwise the implicit forest roots when the document has
no `scenes` array. Select another scene without rewriting the asset by passing
`AssetLoadOptions::with_gltf_scene_index(index)` or
`AssetLoadOptions::with_gltf_scene_name(name)` to
`Assets::load_scene_with_options`. Invalid index/name requests fail closed and
list available scenes. Empty scenes are valid, shared scene roots retain their
identity, and nodes reachable only from other scenes are not instantiated.

`SceneAsset::selected_gltf_scene()` and `scena.asset_load_report.v1` record the
resolved source index, optional name, and whether selection was default,
explicit-index, or explicit-name. The scene selector is part of the cache key,
so a cached default scene can never satisfy a later explicit selection.

### Quantized geometry, morphs, animation, and skins

`KHR_mesh_quantization` POSITION accessors accept every extension-defined
signed or unsigned BYTE/SHORT representation, including non-normalized integer
`POSITION` values whose dequantization is carried by the node transform.
TANGENT accepts normalized signed BYTE/SHORT and F32 while preserving its
handedness component. Morph POSITION deltas accept normalized and
non-normalized signed/unsigned BYTE/SHORT plus F32; morph NORMAL and TANGENT
deltas accept normalized signed BYTE/SHORT plus F32. Integer streams require
an explicit `KHR_mesh_quantization` declaration. Stride and sparse overrides
are honored, decoded values must be finite, and malformed, truncated, or
overflowing byte ranges fail with a semantic-specific asset error instead of
panicking, zero-filling, or being treated as an absent stream.

Triangle primitives that omit `NORMAL` use glTF flat shading. Scena computes
the geometric face normal and splits indexed geometry into one vertex per
triangle corner so a hard edge never inherits a neighboring face normal. Every
parallel stream—colors, UV0, tangents, morph deltas, joints, and weights—is
split through the same source index. A degenerate face has no recoverable
normal and fails with its triangle index. Successful computation is recorded as
`computed_flat_normals` in `scena.asset_load_report.v1` and as an informational
asset-doctor finding.

Morph import preserves target order even when a target omits POSITION, carries
optional normal and tangent deltas into render preparation, and fans an animated
weight channel out to every renderable primitive of a multi-primitive mesh.
Normal-map sampling transforms tangent-space values through the morphed tangent
frame before lighting.

Imported animation validation permits the glTF static case of a one-key clip at
time zero while rejecting empty, non-finite, duplicate/non-monotonic, or
shape-mismatched channels. Imported U8, U16, and floating-point skin vectors
must contain finite, non-negative, non-zero-sum skin weights; valid vectors are
renormalized to sum to one after decoding. `JOINTS_1` and `WEIGHTS_1` are read
with set 0, the strongest four nonzero influences are selected with stable
source-order tie breaking, and the retained values are renormalized once. A
vertex with more than four nonzero source influences produces the structured
`skin_influences_truncated` warning; attribute sets above 1 fail with the exact
eight-input/four-retained limit. A selected joint outside the node's bound skin
fails during render preparation with the joint index and binding width.

Node-level morph `weights` override mesh defaults independently for every node
that shares a mesh. The override width must match every primitive's morph-target
count. Overrides become the initial scene state before animation, and animated
weight channels continue to fan out to every renderable child of a
multi-primitive source node.

Rotation animation uses the same source-coordinate conversion as each node's
static transform. For `ZUpRightHanded` imports, linear and step quaternion keys
are basis-conjugated before playback; cubic-spline quaternion values and their
derivative tangents are conjugated separately so tangent magnitudes are not
normalized away. Translation keys follow the source-axis mapping, while scale
keys and morph weights retain their dimensionless semantics. This keeps the
rest pose and every sampled animated pose in one converted basis without
changing skin, anchor, or connector ownership.

### Import unit boundary

Non-meter imports carry `meters_per_unit()` on one synthetic placement root returned by
`SceneImport::roots()`. Source node translations, authored scales, instance transforms,
and animation scale keys stay in their source-local or dimensionless form below that
root, preventing unit factors from compounding through nested hierarchies. Inherited
anchor/connector locals stay in import units; explicitly unit-tagged anchors are converted
once into import-unit locals and retain their authored unit metadata. Marker locals must
not be pre-converted to meters before the placement root is composed.

## Materials and textures

The current prepared material layout carries `TEXCOORD_0`. Every core and
supported-extension texture-info object is validated before material creation;
an authored `texCoord` other than 0 fails with the material index, slot/path,
requested set, and supported set. `KHR_texture_transform.texCoord` is validated
independently under the same rule. Scena never substitutes UV0 for an authored
UV1 request.

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
resolved. After `reload_scene` and `Scene::replace_import`, resolve scene-owned
geometry and material handles from the replacement `SceneAsset`, discard old
snapshots, and call `Renderer::prepare` again. An explicit reload of an
external texture at the same path and sampler/color-space configuration keeps
its `TextureHandle`, but replaces the descriptor behind that handle with a new
immutable revision; a previously returned `Arc<TextureDesc>` still points to
the old revision and is never mutated behind the caller's back. Embedded image
changes remain content-addressed and therefore mint a new texture handle.

### Transactional reload

`Assets::reload_scene` is the explicit mutable-source boundary. It fetches all
referenced buffers and supported images strictly, decodes and parses through a
cloned asset-storage transaction, and publishes the new complete scene only
after every dependency succeeds. A changed external texture at the same path
reuses its cache handle and updates its source provenance and decoded pixels.
Shared material consumers observe that revision together. Same-byte reloads
are successful no-ops.

Use `Assets::reload_scene_with_report` when the host needs fetched-byte,
external-resource, warning, and progress evidence for a successful reload. A
missing, deleted, or malformed dependency returns `AssetReloadError`, which
retains the underlying structured `AssetError`, reload path, and
`previous_asset_preserved()` evidence. The last complete scene and its
descriptors remain cached and usable. The compatibility `reload_scene` method
continues to return `AssetError` directly. Ordinary `load_scene` calls retain
immutable cache provenance and do not silently turn into source replacement.

`Scene::replace_import` preserves host-owned root state deterministically.
Replacement roots are paired with prior roots by source-root ordinal; each
matched root keeps its host parent, local transform, direct visibility, and
host-added tags even if its source name changed. Removed roots are discarded.
Added roots attach to the first prior host parent while retaining the new
asset-authored local state. If instantiation fails, no old root or host override
is removed.

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

## FBX conversion CLI

`scena-convert` delegates FBX-to-glTF/GLB conversion to FBX2glTF or a compatible
tool. Use `--json` for the stable `scena.asset_conversion.v1` contract and
`--human` for plain text with live child-process output. In JSON mode the child
process is captured: its progress and warning lines appear only in
`diagnostics`, so stdout remains one parseable document even when conversion
fails. `--dry-run` reports the exact command without starting the tool.

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
