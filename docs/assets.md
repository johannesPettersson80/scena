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

Buffer and image URIs are RFC 3986 references, so a file called
`Box With Spaces.bin` is referenced as `Box%20With%20Spaces.bin`. How a
relative reference is resolved depends on how the asset itself is addressed:

- **File-system path** (no URL scheme, native targets): the reference is
  percent-decoded and then joined to the asset's directory, so
  `Box%20With%20Spaces.bin` opens `Box With Spaces.bin` and `caf%C3%A9.png`
  opens `café.png`. A `+` is a literal plus sign, never a space. A reference
  whose escapes do not decode to UTF-8 is kept as written.
- **URL** (the asset path contains `://`, and every path in the browser): the
  reference keeps its encoding and is joined to the asset's base URL, because
  decoding `%23` or `%3F` would turn part of the file name into a fragment or
  a query.

Data URIs and absolute URLs are passed through unchanged. A custom
`AssetFetcher` receives exactly these resolved paths. A file whose name on disk
contains a literal percent escape, such as `a%20b.bin`, has to be referenced as
`a%2520b.bin`.

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
`computed_flat_normals` identifies a primitive that omitted `NORMAL`,
`degenerate_triangle_normals` identifies how many of its triangles had no area
and received the fallback normal, `skin_influences_truncated` identifies how
many vertices exceeded the prepared four-influence limit after `JOINTS_0/1` and
`WEIGHTS_0/1` were combined, and `skin_vertices_unweighted` identifies how many
vertices carried no skin weight at all.

Use `AssetLoadOptions::with_strict_external_resources(true)` when missing
external buffers should fail loading instead of producing warnings, and
`AssetLoadOptions::with_strict_textures(true)` when missing external images
should fail instead of producing texture warnings. Use
`with_fetch_byte_limit(n)` to cap the combined scene and external-resource bytes
for one load. Both strictness flags and the byte limit affect cache
eligibility, and so do the structural import limits below.

### Import limits

A glTF document is checked against structural limits before any node is
imported, so a small hostile file cannot demand an unbounded amount of work.
Exceeding one fails the load with `AssetError::PolicyViolation`, which reports
the measured value, the limit, and the option that raises it.

| Limit | Default | Option |
|---|---|---|
| Node depth: nodes on the longest root-to-leaf chain (a root has depth 1) | 1,024 | `AssetLoadOptions::with_max_node_depth` |
| Node count: nodes declared by the document | 1,000,000 | `AssetLoadOptions::with_max_node_count` |
| Accessor bytes: bytes the import materializes from accessors | 1 GiB (1,073,741,824) | `AssetLoadOptions::with_max_accessor_bytes` |

The accessor byte budget is what keeps a declared `count` from deciding how
much memory a load takes. A sparse accessor without a `bufferView` is
zero-initialised and backed by no file bytes at all, so a few hundred bytes of
JSON can declare gigabytes. The importer therefore reserves every stream
before it allocates it:

- each decoded stream counts `count` times its in-memory element size, every
  time it is read, so an accessor shared by several primitives counts once per
  primitive;
- data the importer derives counts the same way: generated indices, the list
  indices of strips, fans, and loops, flat normals (one vertex per triangle
  corner in every stream), default line normals, the zero deltas of morph
  targets that omit `POSITION`, and the per-keyframe vectors of morph-weight
  animation;
- with the `meshopt` feature, every decompressed `EXT_meshopt_compression`
  buffer view counts `count` times `byteStride`, because that size is declared
  in JSON too and several views may decode the same compressed bytes. This is
  in addition to the per-view limit of 256 MiB.

A reservation that would exceed the budget fails with the stream's name, its
size, and the running total, and nothing is allocated for it. Hosts that load
untrusted files can lower the budget; hosts that load very large trusted
models raise it.

Buffers are checked before any accessor is read. Every `bufferView` must lie
inside its buffer's declared `byteLength`, and every buffer source (GLB chunk,
external file, or data URI) must provide at least that many bytes, so a view
or accessor can never address bytes that do not exist. The same holds for a
compressed view: it is decoded into its own byte range, which therefore has to
exist before decoding starts.

The node limits apply to the whole document, not only the selected scene. Import,
`SceneAsset::bounds`, `Scene::instantiate`, and
`SceneImport::addressable_node_paths` walk the hierarchy with explicit work
lists, so a raised depth limit costs heap memory rather than call-stack depth.
Other scene traversals still recurse per level, which is why the default depth
stays conservative.

The limits are part of the cache decision: an entry admitted under looser
limits never satisfies a request with stricter ones, while an entry admitted
under stricter limits is reusable. `Assets::reload_scene` validates the
replacement against the limits the original asset was loaded with.

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

## Surface finish

glTF has no way to say how a part was manufactured, so a material states it in
its `extras`, next to scena's other annotations:

```json
{ "name": "body", "extras": { "scena": { "surface_finish": "cast_metal" } } }
```

or, to set more than the kind:

```json
{ "extras": { "scena": { "surface_finish": {
    "kind": "painted_metal", "scale": 1.5, "relief": 0.8, "wear": 0.3,
    "grime": 0.4, "edge_radius_m": 0.002, "axis": [1, 0, 0], "seed": 7
} } } }
```

`kind` is one of `polished_metal`, `satin_metal`, `brushed_metal`,
`machined_metal`, `cast_metal`, `painted_metal`, `powder_coated_metal`,
`molded_plastic`, `clearcoat_plastic`, `rubber` and `fabric`. The other
settings are optional and mean what they do on `SurfaceFinish`; see "Surface
detail" in [Rendering](rendering.md). The import puts the finish on the
`MaterialDesc`, and the path tracer draws it. A finish that names no known
kind, has a setting out of range or carries an unknown setting fails the load
with its `materials[n].extras.scena.surface_finish` path; it is never
silently dropped. A material without the annotation has no finish.

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

### Primitive modes

The renderer draws triangle lists and line lists. Every glTF primitive mode
that describes triangles or lines is imported as one of the two:

| glTF `mode` | Imported as |
|---|---|
| `TRIANGLES` (4, the default) | triangle list, unchanged |
| `TRIANGLE_STRIP` (5) | triangle list: triangle *i* is (*i*, *i*+1, *i*+2) when *i* is even and (*i*, *i*+2, *i*+1) when it is odd, so the whole strip keeps the winding of its first triangle |
| `TRIANGLE_FAN` (6) | triangle list: triangle *i* is (*i*+1, *i*+2, 0) |
| `LINES` (1) | line list, unchanged |
| `LINE_STRIP` (3) | line list: segment *i* is (*i*, *i*+1) |
| `LINE_LOOP` (2) | line list: the strip's segments plus one from the last vertex back to the first |
| `POINTS` (0) | not supported: the load fails with an `AssetError::Parse` that names the mesh and primitive |

The rewrite applies to the index accessor, or to the vertex order of a
primitive without one, so vertices and their attributes are shared exactly as
authored. Strip and fan triangles that repeat an index are the zero-area
triangles exporters use to stitch strips together and are dropped; a primitive
made only of them keeps them. A strip or fan needs at least three vertices and
a line strip or loop at least two; a shorter one fails with the mesh, primitive
and mode. The rewritten index list counts against the accessor byte budget.
`SceneAssetMesh::topology()` and `GeometryDesc::topology()` report the imported
list topology.

### Lights

`KHR_lights_punctual` directional, point, and spot lights instantiate as scene
lights. A node that carries only a light becomes the light node. A node that
also carries a mesh keeps its mesh (or instanced mesh) node and gains the light
as a child with an identity transform, so the light shines from the node's own
frame and follows the node when it is moved or animated. The child is not one
of the node's primitives: morph weights and weight animation never target it,
and it is removed together with the import.

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

Every other stream is decoded with the same rule: the layouts glTF 2.0 and
`KHR_mesh_quantization` define are accepted, and anything else is an
`AssetError::Parse` that names the mesh, primitive, and attribute.

| Stream | Accepted layouts |
|---|---|
| `TEXCOORD_0` | VEC2 of FLOAT or normalized UNSIGNED_BYTE/UNSIGNED_SHORT; with `KHR_mesh_quantization` also BYTE/SHORT (normalized or not) and non-normalized UNSIGNED_BYTE/UNSIGNED_SHORT |
| `COLOR_0` | VEC3 or VEC4 of FLOAT or normalized UNSIGNED_BYTE/UNSIGNED_SHORT |
| `JOINTS_0`, `JOINTS_1` | VEC4 of UNSIGNED_BYTE or UNSIGNED_SHORT |
| `WEIGHTS_0`, `WEIGHTS_1` | VEC4 of FLOAT or normalized UNSIGNED_BYTE/UNSIGNED_SHORT |
| indices | SCALAR of UNSIGNED_BYTE, UNSIGNED_SHORT, or UNSIGNED_INT |
| skin `inverseBindMatrices` | MAT4 of FLOAT |
| animation sampler input | SCALAR of FLOAT |
| animation translation/scale output | VEC3 of FLOAT |
| animation rotation/weights output | FLOAT, or BYTE/UNSIGNED_BYTE/SHORT/UNSIGNED_SHORT decoded as normalized |

A normalized integer is divided by its type maximum; a non-normalized
quantized `TEXCOORD_0` value keeps its integer magnitude, because the
dequantization scale belongs to `KHR_texture_transform`. An accessor that is
read must declare `count` of at least 1, sparse storage must override at least
one element, and a declared `inverseBindMatrices` accessor that cannot be read
is an error rather than an identity fallback.

A failed load never disables the store. Import runs against a private copy of
the asset storage that is committed only after the whole document parsed, so
neither an error nor a panic inside a decoder publishes partial geometry,
materials, or textures, and later calls on the same `Assets` keep working.

Triangle primitives that omit `NORMAL` use glTF flat shading. Scena computes
the geometric face normal and splits indexed geometry into one vertex per
triangle corner so a hard edge never inherits a neighboring face normal. Every
parallel stream—colors, UV0, tangents, morph deltas, joints, and weights—is
split through the same source index. Successful computation is recorded as
`computed_flat_normals` in `scena.asset_load_report.v1` and as an informational
asset-doctor finding.

A triangle without area has no face normal. It also covers no pixel, so it does
not fail the asset: each such triangle receives the fixed fallback normal +Y,
and the primitive records one `degenerate_triangle_normals` warning with the
number of affected triangles and the index of the first one. The fallback only
becomes visible if a morph target or a skin later gives the triangle area;
author `NORMAL` when that shading matters.

Morph import preserves target order even when a target omits POSITION, carries
optional normal and tangent deltas into render preparation, and fans an animated
weight channel out to every renderable primitive of a multi-primitive mesh.
A morphed normal is the authored normal plus every weighted normal delta,
normalized once after the sum, so it does not depend on the order of the
targets; a sum too short to normalize keeps the authored normal.
Normal-map sampling transforms tangent-space values through the morphed tangent
frame before lighting.

Imported animation validation permits the glTF static case of a one-key clip at
time zero while rejecting empty, non-finite, duplicate/non-monotonic, or
shape-mismatched channels. Imported U8, U16, and floating-point skin vectors
must contain finite, non-negative skin weights with a finite sum; vectors with
a positive sum are renormalized to sum to one after decoding. A vertex whose
weights are all zero is bound to no joint: it keeps `[0, 0, 0, 0]`, is not
skinned, and stays at its authored position in the mesh node's space while the
rest of the mesh follows the skeleton. The primitive records one
`skin_vertices_unweighted` warning with the number of such vertices and the
index of the first one. `JOINTS_1` and `WEIGHTS_1` are read
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

A reload leaves the descriptors of the replaced scene in the store, because an
instantiated `Scene` may still draw with them. `Assets::release_unreferenced` frees the ones nothing can reach any more.
It keeps:

- the geometry and default material of every mesh of every cached scene, and
  every material a `KHR_materials_variants` binding of that mesh can select,
  so `Scene::set_active_variant` never swaps in a freed handle;
- every descriptor the caller created or loaded: `Assets::create_geometry`,
  `Assets::create_material`, `Assets::create_environment`, and
  `Assets::load_texture`;
- every texture a retained material samples, including an imported texture that
  only a caller-created material still uses.

Pass scenes that are still instantiated but no longer cached to
`Assets::release_unreferenced_with_scene_roots`; they are rooted the same way.

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
