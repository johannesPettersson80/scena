use std::sync::Arc;

use serde_json::Value as JsonValue;

use crate::diagnostics::AssetError;
use crate::geometry::Aabb;
use crate::scene::{Quat, Transform, Vec3};

use self::accessor::{parse_accessors, parse_buffer_views, parse_buffers};
use self::read::{parse_materials, parse_meshes, parse_textures};
use super::{AssetPath, AssetStorage, GeometryHandle, MaterialHandle};

mod accessor;
mod read;

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
    clips: Vec<SceneAssetClip>,
    extensions_used: Vec<String>,
    extensions_required: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneAssetNode {
    name: Option<String>,
    children: Vec<usize>,
    transform: Transform,
    mesh: Option<SceneAssetMesh>,
    anchors: Vec<SceneAssetAnchor>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneAssetMesh {
    geometry: GeometryHandle,
    material: MaterialHandle,
    bounds: Aabb,
    uses_vertex_colors: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneAssetAnchor {
    name: String,
    transform: Transform,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneAssetClip {
    name: Option<String>,
}

impl SceneAsset {
    pub fn empty() -> Self {
        Self {
            inner: Arc::new(SceneAssetData {
                path: AssetPath::from("memory:empty"),
                node_count: 0,
                mesh_count: 0,
                nodes: Vec::new(),
                clips: Vec::new(),
                extensions_used: Vec::new(),
                extensions_required: Vec::new(),
            }),
        }
    }

    pub(super) fn from_gltf_source(
        path: AssetPath,
        source: &str,
        storage: &mut AssetStorage,
    ) -> Result<Self, AssetError> {
        Self::from_gltf_json(path, source, None, storage)
    }

    pub(super) fn from_gltf_bytes(
        path: AssetPath,
        bytes: &[u8],
        storage: &mut AssetStorage,
    ) -> Result<Self, AssetError> {
        if !is_glb(bytes) {
            let source = std::str::from_utf8(bytes).map_err(|error| AssetError::Parse {
                path: path.as_str().to_string(),
                reason: format!("expected UTF-8 glTF JSON source: {error}"),
            })?;
            return Self::from_gltf_source(path, source, storage);
        }

        let (json, binary_chunk) = parse_glb(&path, bytes)?;
        Self::from_gltf_json(path, &json, binary_chunk.as_deref(), storage)
    }

    fn from_gltf_json(
        path: AssetPath,
        source: &str,
        binary_chunk: Option<&[u8]>,
        storage: &mut AssetStorage,
    ) -> Result<Self, AssetError> {
        let json: JsonValue = serde_json::from_str(source).map_err(|error| AssetError::Parse {
            path: path.as_str().to_string(),
            reason: error.to_string(),
        })?;
        validate_gltf_version(&path, &json)?;
        let extensions_used = string_array_field(&json, "extensionsUsed");
        let extensions_required = string_array_field(&json, "extensionsRequired");
        for extension in &extensions_required {
            if !is_v1_required_gltf_extension(extension) {
                return Err(AssetError::UnsupportedRequiredExtension {
                    path: path.as_str().to_string(),
                    extension: extension.clone(),
                });
            }
        }

        let buffers = parse_buffers(&path, &json, binary_chunk)?;
        let buffer_views = parse_buffer_views(&path, &json)?;
        let accessors = parse_accessors(&path, &json)?;
        let textures = parse_textures(&path, &json, storage);
        let materials = parse_materials(&json, storage, &textures);
        let meshes = parse_meshes(
            &path,
            &json,
            &buffers,
            &buffer_views,
            &accessors,
            &materials,
            storage,
        )?;
        let nodes = parse_gltf_nodes(&json, &meshes);
        let clips = parse_gltf_clips(&json);
        let node_count = nodes.len();
        let mesh_count = meshes.len();
        Ok(Self {
            inner: Arc::new(SceneAssetData {
                path,
                node_count,
                mesh_count,
                nodes,
                clips,
                extensions_used,
                extensions_required,
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

    pub fn clips(&self) -> &[SceneAssetClip] {
        &self.inner.clips
    }

    pub fn extensions_used(&self) -> &[String] {
        &self.inner.extensions_used
    }

    pub fn extensions_required(&self) -> &[String] {
        &self.inner.extensions_required
    }
}

fn is_glb(bytes: &[u8]) -> bool {
    bytes.starts_with(&GLB_MAGIC.to_le_bytes())
}

fn parse_glb(path: &AssetPath, bytes: &[u8]) -> Result<(String, Option<Vec<u8>>), AssetError> {
    if bytes.len() < GLB_HEADER_LEN {
        return Err(glb_error(path, "GLB file is shorter than its header"));
    }
    let magic = read_u32_le(path, bytes, 0)?;
    let version = read_u32_le(path, bytes, 4)?;
    let length = read_u32_le(path, bytes, 8)? as usize;
    if magic != GLB_MAGIC {
        return Err(glb_error(path, "invalid GLB magic"));
    }
    if version != 2 {
        return Err(glb_error(path, "expected GLB version 2"));
    }
    if length > bytes.len() {
        return Err(glb_error(path, "GLB declared length exceeds fetched bytes"));
    }

    let mut offset = GLB_HEADER_LEN;
    let mut json = None;
    let mut binary = None;
    while offset + GLB_CHUNK_HEADER_LEN <= length {
        let chunk_length = read_u32_le(path, bytes, offset)? as usize;
        let chunk_type = read_u32_le(path, bytes, offset + 4)?;
        offset += GLB_CHUNK_HEADER_LEN;
        let end = offset
            .checked_add(chunk_length)
            .ok_or_else(|| glb_error(path, "GLB chunk length overflow"))?;
        if end > length {
            return Err(glb_error(path, "GLB chunk exceeds declared length"));
        }
        let chunk = &bytes[offset..end];
        match chunk_type {
            GLB_JSON_CHUNK => {
                json = Some(
                    std::str::from_utf8(chunk).map_err(|error| AssetError::Parse {
                        path: path.as_str().to_string(),
                        reason: format!("invalid GLB JSON chunk UTF-8: {error}"),
                    })?,
                );
            }
            GLB_BIN_CHUNK => {
                binary = Some(chunk.to_vec());
            }
            _ => {}
        }
        offset = end;
    }

    let json = json.ok_or_else(|| glb_error(path, "GLB is missing JSON chunk"))?;
    Ok((json.to_string(), binary))
}

fn read_u32_le(path: &AssetPath, bytes: &[u8], offset: usize) -> Result<u32, AssetError> {
    let chunk = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| glb_error(path, "unexpected end of GLB while reading u32"))?;
    Ok(u32::from_le_bytes(
        chunk.try_into().expect("slice length checked above"),
    ))
}

fn glb_error(path: &AssetPath, reason: impl Into<String>) -> AssetError {
    AssetError::Parse {
        path: path.as_str().to_string(),
        reason: reason.into(),
    }
}

const GLB_MAGIC: u32 = 0x4654_6C67;
const GLB_JSON_CHUNK: u32 = 0x4E4F_534A;
const GLB_BIN_CHUNK: u32 = 0x004E_4942;
const GLB_HEADER_LEN: usize = 12;
const GLB_CHUNK_HEADER_LEN: usize = 8;

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

    pub fn mesh(&self) -> Option<SceneAssetMesh> {
        self.mesh
    }

    pub fn anchors(&self) -> &[SceneAssetAnchor] {
        &self.anchors
    }
}

impl SceneAssetMesh {
    pub const fn geometry(self) -> GeometryHandle {
        self.geometry
    }

    pub const fn material(self) -> MaterialHandle {
        self.material
    }

    pub const fn bounds(self) -> Aabb {
        self.bounds
    }

    pub const fn uses_vertex_colors(self) -> bool {
        self.uses_vertex_colors
    }
}

impl SceneAssetAnchor {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn transform(&self) -> Transform {
        self.transform
    }
}

impl SceneAssetClip {
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

fn validate_gltf_version(path: &AssetPath, json: &JsonValue) -> Result<(), AssetError> {
    let version = json
        .get("asset")
        .and_then(|asset| asset.get("version"))
        .and_then(JsonValue::as_str);
    if version == Some("2.0") {
        Ok(())
    } else {
        Err(AssetError::Parse {
            path: path.as_str().to_string(),
            reason: "expected glTF asset.version \"2.0\"".to_string(),
        })
    }
}

fn string_array_field(json: &JsonValue, field: &str) -> Vec<String> {
    json.get(field)
        .and_then(JsonValue::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(JsonValue::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn parse_gltf_nodes(json: &JsonValue, meshes: &[SceneAssetMesh]) -> Vec<SceneAssetNode> {
    json.get("nodes")
        .and_then(JsonValue::as_array)
        .map(|nodes| {
            nodes
                .iter()
                .map(|node| SceneAssetNode {
                    name: node
                        .get("name")
                        .and_then(JsonValue::as_str)
                        .map(str::to_string),
                    children: node
                        .get("children")
                        .and_then(JsonValue::as_array)
                        .map(|children| {
                            children
                                .iter()
                                .filter_map(JsonValue::as_u64)
                                .map(|child| child as usize)
                                .collect()
                        })
                        .unwrap_or_default(),
                    transform: parse_node_transform(node),
                    mesh: node
                        .get("mesh")
                        .and_then(JsonValue::as_u64)
                        .and_then(|mesh| meshes.get(mesh as usize))
                        .copied(),
                    anchors: parse_node_anchors(node),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_node_anchors(node: &JsonValue) -> Vec<SceneAssetAnchor> {
    node.get("extras")
        .and_then(|extras| extras.get("scena"))
        .and_then(|scena| scena.get("anchors"))
        .and_then(JsonValue::as_array)
        .map(|anchors| {
            anchors
                .iter()
                .filter_map(|anchor| {
                    let name = anchor.get("name").and_then(JsonValue::as_str)?;
                    Some(SceneAssetAnchor {
                        name: name.to_string(),
                        transform: parse_node_transform(anchor),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_gltf_clips(json: &JsonValue) -> Vec<SceneAssetClip> {
    json.get("animations")
        .and_then(JsonValue::as_array)
        .map(|animations| {
            animations
                .iter()
                .map(|animation| SceneAssetClip {
                    name: animation
                        .get("name")
                        .and_then(JsonValue::as_str)
                        .map(str::to_string),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_node_transform(node: &JsonValue) -> Transform {
    Transform {
        translation: vec3_field(node, "translation", Vec3::ZERO),
        rotation: quat_field(node, "rotation", Quat::IDENTITY),
        scale: vec3_field(node, "scale", Vec3::ONE),
    }
}

fn vec3_field(node: &JsonValue, field: &str, fallback: Vec3) -> Vec3 {
    let Some(values) = node.get(field).and_then(JsonValue::as_array) else {
        return fallback;
    };
    Vec3::new(
        array_f32(values, 0).unwrap_or(fallback.x),
        array_f32(values, 1).unwrap_or(fallback.y),
        array_f32(values, 2).unwrap_or(fallback.z),
    )
}

fn quat_field(node: &JsonValue, field: &str, fallback: Quat) -> Quat {
    let Some(values) = node.get(field).and_then(JsonValue::as_array) else {
        return fallback;
    };
    Quat {
        x: array_f32(values, 0).unwrap_or(fallback.x),
        y: array_f32(values, 1).unwrap_or(fallback.y),
        z: array_f32(values, 2).unwrap_or(fallback.z),
        w: array_f32(values, 3).unwrap_or(fallback.w),
    }
}

fn array_f32(values: &[JsonValue], index: usize) -> Option<f32> {
    values
        .get(index)
        .and_then(JsonValue::as_f64)
        .map(|value| value as f32)
}

fn is_v1_required_gltf_extension(extension: &str) -> bool {
    matches!(
        extension,
        "KHR_lights_punctual"
            | "KHR_materials_unlit"
            | "KHR_materials_emissive_strength"
            | "KHR_texture_transform"
            | "KHR_mesh_quantization"
    )
}
