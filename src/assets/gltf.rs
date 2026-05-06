use std::sync::Arc;

use serde_json::Value as JsonValue;

use crate::diagnostics::AssetError;

use super::AssetPath;

#[derive(Debug, Clone)]
pub struct SceneAsset {
    inner: Arc<SceneAssetData>,
}

#[derive(Debug, Clone, PartialEq)]
struct SceneAssetData {
    path: AssetPath,
    node_count: usize,
    nodes: Vec<SceneAssetNode>,
    extensions_used: Vec<String>,
    extensions_required: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneAssetNode {
    name: Option<String>,
    children: Vec<usize>,
}

impl SceneAsset {
    pub fn empty() -> Self {
        Self {
            inner: Arc::new(SceneAssetData {
                path: AssetPath::from("memory:empty"),
                node_count: 0,
                nodes: Vec::new(),
                extensions_used: Vec::new(),
                extensions_required: Vec::new(),
            }),
        }
    }

    pub(super) fn from_gltf_source(path: AssetPath, source: &str) -> Result<Self, AssetError> {
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

        let nodes = parse_gltf_nodes(&json);
        let node_count = nodes.len();
        Ok(Self {
            inner: Arc::new(SceneAssetData {
                path,
                node_count,
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

fn parse_gltf_nodes(json: &JsonValue) -> Vec<SceneAssetNode> {
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
                })
                .collect()
        })
        .unwrap_or_default()
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
