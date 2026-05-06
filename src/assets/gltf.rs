use std::sync::Arc;

use serde_json::Value as JsonValue;

use crate::diagnostics::AssetError;
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
    extensions_used: Vec<String>,
    extensions_required: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneAssetNode {
    name: Option<String>,
    children: Vec<usize>,
    transform: Transform,
    mesh: Option<SceneAssetMesh>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneAssetMesh {
    geometry: GeometryHandle,
    material: MaterialHandle,
    uses_vertex_colors: bool,
}

impl SceneAsset {
    pub fn empty() -> Self {
        Self {
            inner: Arc::new(SceneAssetData {
                path: AssetPath::from("memory:empty"),
                node_count: 0,
                mesh_count: 0,
                nodes: Vec::new(),
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

        let buffers = parse_buffers(&path, &json)?;
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
        let node_count = nodes.len();
        let mesh_count = meshes.len();
        Ok(Self {
            inner: Arc::new(SceneAssetData {
                path,
                node_count,
                mesh_count,
                nodes,
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

    pub fn extensions_used(&self) -> &[String] {
        &self.inner.extensions_used
    }

    pub fn extensions_required(&self) -> &[String] {
        &self.inner.extensions_required
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

    pub fn mesh(&self) -> Option<SceneAssetMesh> {
        self.mesh
    }
}

impl SceneAssetMesh {
    pub const fn geometry(self) -> GeometryHandle {
        self.geometry
    }

    pub const fn material(self) -> MaterialHandle {
        self.material
    }

    pub const fn uses_vertex_colors(self) -> bool {
        self.uses_vertex_colors
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
