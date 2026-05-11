//! Stage C2: glTF import now uses the canonical `gltf` crate as its
//! parser. Scena retains the typed public surface — `SceneAsset`,
//! `SceneAssetNode`, `SceneAssetMesh`, `SceneAssetSkin`,
//! `SceneAssetClip`, `SceneAssetLight`, `SceneAssetAnchor`,
//! `SceneAssetConnector`, `MaterialVariantBinding` — but every JSON
//! walk + accessor read is delegated to the gltf crate. Scena-specific
//! extras (anchors, connectors) are still parsed by scena since they
//! live in the freeform `extras` slot.

use std::collections::BTreeMap;
use std::sync::Arc;

use ::gltf::Gltf;
use ::gltf::buffer::Source as BufferSource;

use crate::animation::{AnimationSourceChannel, AnimationSourceClip};
use crate::diagnostics::AssetError;
use crate::geometry::Aabb;
use crate::scene::{Light, Transform};

pub use self::anchors::SceneAssetAnchor;
use self::anchors::parse_node_anchors;
use self::animation::parse_gltf_clips;
pub use self::connectors::SceneAssetConnector;
use self::connectors::parse_node_connectors;
pub use self::extensions::{GltfDecoderPolicy, GltfExtensionDiagnostic, GltfExtensionStatus};
use self::extensions::{collect_extension_diagnostics, is_v1_required_gltf_extension};
use self::external::{external_buffer_paths, external_image_paths, resolve_relative_path};
use self::lights::parse_punctual_lights;
use self::materials::parse_materials;
pub use self::material_variants::MaterialVariantBinding;
use self::meshes::parse_meshes;
pub use self::skins::SceneAssetSkin;
use self::skins::parse_skins;
use self::textures::parse_textures;
use self::transform::from_gltf_transform;
use super::{AssetPath, AssetStorage, GeometryHandle, MaterialHandle};

mod anchors;
mod animation;
mod connectors;
mod extensions;
mod external;
mod lights;
mod material_variants;
mod materials;
mod meshes;
mod skins;
mod textures;
mod transform;

#[derive(Debug, Clone)]
pub struct SceneAsset {
    inner: Arc<SceneAssetData>,
}

#[derive(Debug, Clone, PartialEq)]
struct SceneAssetData {
    path: AssetPath,
    node_count: usize,
    mesh_count: usize,
    nodes: Vec<SceneAssetNode>,
    skins: Vec<SceneAssetSkin>,
    clips: Vec<SceneAssetClip>,
    extensions_used: Vec<String>,
    extensions_required: Vec<String>,
    extension_diagnostics: Vec<GltfExtensionDiagnostic>,
    material_variants: Vec<String>,
    retained_source_bytes: Option<Arc<[u8]>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneAssetNode {
    name: Option<String>,
    children: Vec<usize>,
    transform: Transform,
    meshes: Vec<SceneAssetMesh>,
    skin: Option<usize>,
    light: Option<SceneAssetLight>,
    anchors: Vec<SceneAssetAnchor>,
    connectors: Vec<SceneAssetConnector>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneAssetMesh {
    geometry: GeometryHandle,
    material: MaterialHandle,
    bounds: Aabb,
    uses_vertex_colors: bool,
    morph_weights: Vec<f32>,
    material_variant_bindings: Vec<MaterialVariantBinding>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneAssetLight {
    light: Light,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneAssetClip {
    clip: AnimationSourceClip,
}

impl SceneAsset {
    pub fn empty() -> Self {
        Self {
            inner: Arc::new(SceneAssetData {
                path: AssetPath::from("memory:empty"),
                node_count: 0,
                mesh_count: 0,
                nodes: Vec::new(),
                skins: Vec::new(),
                clips: Vec::new(),
                extensions_used: Vec::new(),
                extensions_required: Vec::new(),
                extension_diagnostics: Vec::new(),
                material_variants: Vec::new(),
                retained_source_bytes: None,
            }),
        }
    }

    pub(super) fn from_gltf_bytes(
        path: AssetPath,
        bytes: &[u8],
        storage: &mut AssetStorage,
    ) -> Result<Self, AssetError> {
        Self::from_gltf_bytes_with_external_resources(
            path,
            bytes,
            &BTreeMap::new(),
            &BTreeMap::new(),
            storage,
        )
    }

    pub(super) fn from_gltf_bytes_with_external_resources(
        path: AssetPath,
        bytes: &[u8],
        external_buffers: &BTreeMap<usize, Vec<u8>>,
        external_images: &BTreeMap<AssetPath, Vec<u8>>,
        storage: &mut AssetStorage,
    ) -> Result<Self, AssetError> {
        let gltf = open_gltf_with_massage(&path, bytes)?;
        let blob = gltf.blob.clone();
        Self::from_gltf_document(&path, &gltf, blob.as_deref(), external_buffers, external_images, storage)
    }

    pub(super) fn external_buffer_paths(
        path: &AssetPath,
        bytes: &[u8],
    ) -> Result<Vec<(usize, AssetPath)>, AssetError> {
        external_buffer_paths(path, bytes)
    }

    pub(super) fn external_image_paths(
        path: &AssetPath,
        bytes: &[u8],
    ) -> Result<Vec<AssetPath>, AssetError> {
        external_image_paths(path, bytes)
    }

    fn from_gltf_document(
        path: &AssetPath,
        gltf: &Gltf,
        binary_chunk: Option<&[u8]>,
        external_buffers: &BTreeMap<usize, Vec<u8>>,
        external_images: &BTreeMap<AssetPath, Vec<u8>>,
        storage: &mut AssetStorage,
    ) -> Result<Self, AssetError> {
        validate_gltf_version(path, gltf)?;
        let extensions_used: Vec<String> = gltf
            .document
            .extensions_used()
            .map(str::to_string)
            .collect();
        let extensions_required: Vec<String> = gltf
            .document
            .extensions_required()
            .map(str::to_string)
            .collect();
        for extension in &extensions_required {
            if !is_v1_required_gltf_extension(extension) {
                return Err(AssetError::UnsupportedRequiredExtension {
                    path: path.as_str().to_string(),
                    extension: extension.clone(),
                });
            }
        }
        let extension_diagnostics = collect_extension_diagnostics(&extensions_used);
        let material_variants =
            material_variants::parse_material_variant_names(&gltf.document);

        let buffers = resolve_buffers(path, gltf, binary_chunk, external_buffers)?;
        let textures = parse_textures(path, &gltf.document, &buffers, external_images, storage);
        let materials = parse_materials(path, &gltf.document, storage, &textures)?;
        let meshes = parse_meshes(path, &gltf.document, &buffers, &materials, storage)?;
        let skins = parse_skins(path, &gltf.document, &buffers)?;
        let lights = parse_punctual_lights(&gltf.document);
        let nodes = parse_gltf_nodes(&gltf.document, &meshes, &lights);
        let clips = parse_gltf_clips(path, &gltf.document, &buffers)?;
        let node_count = nodes.len();
        let mesh_count = meshes.iter().map(Vec::len).sum();
        Ok(Self {
            inner: Arc::new(SceneAssetData {
                path: path.clone(),
                node_count,
                mesh_count,
                nodes,
                skins,
                clips,
                extensions_used,
                extensions_required,
                extension_diagnostics,
                material_variants,
                retained_source_bytes: None,
            }),
        })
    }

    pub fn path(&self) -> &AssetPath {
        &self.inner.path
    }

    pub fn node_count(&self) -> usize {
        self.inner.node_count
    }

    pub fn mesh_count(&self) -> usize {
        self.inner.mesh_count
    }

    pub fn nodes(&self) -> &[SceneAssetNode] {
        &self.inner.nodes
    }

    pub fn skins(&self) -> &[SceneAssetSkin] {
        &self.inner.skins
    }

    pub fn clips(&self) -> &[SceneAssetClip] {
        &self.inner.clips
    }

    pub fn extensions_used(&self) -> &[String] {
        &self.inner.extensions_used
    }

    pub fn extensions_required(&self) -> &[String] {
        &self.inner.extensions_required
    }

    pub fn extension_diagnostics(&self) -> &[GltfExtensionDiagnostic] {
        &self.inner.extension_diagnostics
    }

    /// Variant names declared by KHR_materials_variants in declaration
    /// order; empty when the extension is absent (Phase 2B step 1).
    pub fn material_variants(&self) -> &[String] {
        &self.inner.material_variants
    }

    pub fn retained_source_bytes_len(&self) -> Option<usize> {
        self.inner
            .retained_source_bytes
            .as_ref()
            .map(|bytes| bytes.len())
    }

    pub(super) fn retained_source_bytes(&self) -> Option<&[u8]> {
        self.inner.retained_source_bytes.as_deref()
    }

    pub(super) fn with_retained_source_bytes(mut self, bytes: &[u8]) -> Self {
        Arc::make_mut(&mut self.inner).retained_source_bytes =
            Some(Arc::<[u8]>::from(bytes.to_vec()));
        self
    }
}

impl PartialEq for SceneAsset {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner) || self.inner.path == other.inner.path
    }
}

impl Eq for SceneAsset {}

impl SceneAssetNode {
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn children(&self) -> &[usize] {
        &self.children
    }

    pub fn transform(&self) -> Transform {
        self.transform
    }

    pub fn mesh(&self) -> Option<&SceneAssetMesh> {
        self.meshes.first()
    }

    pub fn meshes(&self) -> &[SceneAssetMesh] {
        &self.meshes
    }

    pub const fn skin(&self) -> Option<usize> {
        self.skin
    }

    pub fn light(&self) -> Option<SceneAssetLight> {
        self.light
    }

    pub fn anchors(&self) -> &[SceneAssetAnchor] {
        &self.anchors
    }

    pub fn connectors(&self) -> &[SceneAssetConnector] {
        &self.connectors
    }
}

impl SceneAssetMesh {
    pub const fn geometry(&self) -> GeometryHandle {
        self.geometry
    }

    pub const fn material(&self) -> MaterialHandle {
        self.material
    }

    pub const fn bounds(&self) -> Aabb {
        self.bounds
    }

    pub const fn uses_vertex_colors(&self) -> bool {
        self.uses_vertex_colors
    }

    pub fn morph_weights(&self) -> &[f32] {
        &self.morph_weights
    }

    pub fn material_variant_bindings(&self) -> &[MaterialVariantBinding] {
        &self.material_variant_bindings
    }
}

impl SceneAssetLight {
    pub const fn light(self) -> Light {
        self.light
    }
}

impl SceneAssetClip {
    pub fn name(&self) -> Option<&str> {
        self.clip.name()
    }

    pub fn channels(&self) -> &[AnimationSourceChannel] {
        self.clip.channels()
    }

    pub const fn duration_seconds(&self) -> f32 {
        self.clip.duration_seconds()
    }

    pub(crate) fn clip(&self) -> &AnimationSourceClip {
        &self.clip
    }
}

fn validate_gltf_version(path: &AssetPath, gltf: &Gltf) -> Result<(), AssetError> {
    if gltf.document.as_json().asset.version == "2.0" {
        Ok(())
    } else {
        Err(AssetError::Parse {
            path: path.as_str().to_string(),
            reason: "expected glTF asset.version \"2.0\"".to_string(),
        })
    }
}

fn resolve_buffers(
    path: &AssetPath,
    gltf: &Gltf,
    binary_chunk: Option<&[u8]>,
    external_buffers: &BTreeMap<usize, Vec<u8>>,
) -> Result<Vec<Vec<u8>>, AssetError> {
    gltf.document
        .buffers()
        .map(|buffer| {
            let byte_length = buffer.length();
            match buffer.source() {
                BufferSource::Bin => {
                    let bin = binary_chunk.ok_or_else(|| AssetError::Parse {
                        path: path.as_str().to_string(),
                        reason: "glTF buffer without uri requires a GLB binary chunk".to_string(),
                    })?;
                    Ok(bin
                        .get(..byte_length)
                        .ok_or_else(|| AssetError::Parse {
                            path: path.as_str().to_string(),
                            reason: "GLB binary chunk is shorter than buffer byteLength".to_string(),
                        })?
                        .to_vec())
                }
                BufferSource::Uri(uri) => {
                    if uri.starts_with("data:") {
                        let (_, encoded) =
                            uri.split_once(";base64,").ok_or_else(|| AssetError::Parse {
                                path: path.as_str().to_string(),
                                reason:
                                    "only embedded base64 glTF buffers are supported in this loader slice"
                                        .to_string(),
                            })?;
                        use base64::Engine;
                        base64::engine::general_purpose::STANDARD
                            .decode(encoded)
                            .map_err(|error| AssetError::Parse {
                                path: path.as_str().to_string(),
                                reason: format!("invalid embedded buffer base64: {error}"),
                            })
                    } else {
                        let _resolved = resolve_relative_path(path, uri);
                        let bytes = external_buffers
                            .get(&buffer.index())
                            .ok_or_else(|| AssetError::Parse {
                                path: path.as_str().to_string(),
                                reason: "external glTF buffer was not fetched".to_string(),
                            })?;
                        bytes
                            .get(..byte_length)
                            .map(<[u8]>::to_vec)
                            .ok_or_else(|| AssetError::Parse {
                                path: path.as_str().to_string(),
                                reason: "external glTF buffer is shorter than byteLength"
                                    .to_string(),
                            })
                    }
                }
            }
        })
        .collect()
}

fn parse_gltf_nodes(
    document: &::gltf::Document,
    meshes: &[Vec<SceneAssetMesh>],
    lights: &[SceneAssetLight],
) -> Vec<SceneAssetNode> {
    document
        .nodes()
        .map(|node| SceneAssetNode {
            name: node.name().map(str::to_string),
            children: node.children().map(|child| child.index()).collect(),
            transform: from_gltf_transform(node.transform()),
            meshes: node
                .mesh()
                .and_then(|mesh| meshes.get(mesh.index()))
                .cloned()
                .unwrap_or_default(),
            skin: node.skin().map(|skin| skin.index()),
            light: node
                .light()
                .and_then(|light| lights.get(light.index()).copied()),
            anchors: parse_node_anchors(&node),
            connectors: parse_node_connectors(&node),
        })
        .collect()
}

/// Helper exposed to anchor/connector parsers: convert the `gltf` crate's
/// `Extras = Option<Box<RawValue>>` into a `serde_json::Value` so the
/// existing scena-specific JSON-walking validators can inspect it.
pub(super) fn extras_to_value(
    extras: &::gltf::json::Extras,
) -> Option<serde_json::Value> {
    let raw = extras.as_ref()?;
    serde_json::from_str(raw.get()).ok()
}

pub(super) fn has_glb_magic(bytes: &[u8]) -> bool {
    bytes.starts_with(&0x4654_6C67_u32.to_le_bytes())
}

/// Parse glTF bytes (JSON or GLB) into a `Gltf` value, applying scena's
/// lenient JSON pre-massage for JSON inputs. Stage C2: scena keeps the
/// pre-existing tolerance for animations that omit `channels`/`samplers`
/// arrays — the gltf crate's strict serde derive would otherwise reject
/// those fixtures even though the spec considers them malformed.
pub(super) fn open_gltf_with_massage(path: &AssetPath, bytes: &[u8]) -> Result<Gltf, AssetError> {
    let parse = |slice: &[u8]| {
        Gltf::from_slice_without_validation(slice).map_err(|error| AssetError::Parse {
            path: path.as_str().to_string(),
            reason: error.to_string(),
        })
    };
    if has_glb_magic(bytes) {
        return parse(bytes);
    }
    if let Some(massaged) = massage_json_for_gltf_crate(bytes) {
        return parse(&massaged);
    }
    parse(bytes)
}

/// Pre-process JSON-form glTF to add empty `channels: []` and
/// `samplers: []` arrays to any animation entry that omits them. The
/// previous scena parser tolerated those omissions (a clip with just a
/// `name` produced an empty `SceneAssetClip`); the gltf crate's
/// strict serde derive rejects them. Returns `None` when no change is
/// needed.
pub(super) fn massage_json_for_gltf_crate(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let mut changed = false;

    if let Some(animations) = value.get_mut("animations").and_then(|v| v.as_array_mut()) {
        for animation in animations.iter_mut() {
            let Some(object) = animation.as_object_mut() else {
                continue;
            };
            if !object.contains_key("channels") {
                object.insert("channels".to_string(), serde_json::Value::Array(Vec::new()));
                changed = true;
            }
            if !object.contains_key("samplers") {
                object.insert("samplers".to_string(), serde_json::Value::Array(Vec::new()));
                changed = true;
            }
        }
    }

    // `KHR_materials_variants` requires `variants: [...]`. Scena
    // tolerates an empty extension block so a fixture that only
    // declares the extension surfaces as zero variants.
    if let Some(ext) = value
        .get_mut("extensions")
        .and_then(|v| v.get_mut("KHR_materials_variants"))
        .and_then(|v| v.as_object_mut())
        && !ext.contains_key("variants")
    {
        ext.insert("variants".to_string(), serde_json::Value::Array(Vec::new()));
        changed = true;
    }

    if changed {
        serde_json::to_vec(&value).ok()
    } else {
        None
    }
}
