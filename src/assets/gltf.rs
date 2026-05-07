use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::Value as JsonValue;

use crate::diagnostics::AssetError;
use crate::geometry::Aabb;
use crate::material::Color;
use crate::scene::{Angle, DirectionalLight, Light, PointLight, Quat, SpotLight, Transform, Vec3};

use self::accessor::{parse_accessors, parse_buffer_views, parse_buffers};
use self::anchors::parse_node_anchors;
use self::glb::{is_glb, parse_glb};
use self::read::{parse_materials, parse_meshes, parse_textures};
use super::{AssetPath, AssetStorage, GeometryHandle, MaterialHandle};

mod accessor;
mod anchors;
mod glb;
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
    light: Option<SceneAssetLight>,
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
    invalid_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneAssetLight {
    light: Light,
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
        Self::from_gltf_json(path, source, None, &BTreeMap::new(), storage)
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
        Self::from_gltf_json(
            path,
            &json,
            binary_chunk.as_deref(),
            &BTreeMap::new(),
            storage,
        )
    }

    pub(super) fn from_gltf_bytes_with_external_buffers(
        path: AssetPath,
        bytes: &[u8],
        external_buffers: &BTreeMap<usize, Vec<u8>>,
        storage: &mut AssetStorage,
    ) -> Result<Self, AssetError> {
        if !is_glb(bytes) {
            let source = std::str::from_utf8(bytes).map_err(|error| AssetError::Parse {
                path: path.as_str().to_string(),
                reason: format!("expected UTF-8 glTF JSON source: {error}"),
            })?;
            return Self::from_gltf_json(path, source, None, external_buffers, storage);
        }

        let (json, binary_chunk) = parse_glb(&path, bytes)?;
        Self::from_gltf_json(
            path,
            &json,
            binary_chunk.as_deref(),
            external_buffers,
            storage,
        )
    }

    pub(super) fn external_buffer_paths(
        path: &AssetPath,
        bytes: &[u8],
    ) -> Result<Vec<(usize, AssetPath)>, AssetError> {
        let json = if is_glb(bytes) {
            parse_glb(path, bytes)?.0
        } else {
            std::str::from_utf8(bytes)
                .map_err(|error| AssetError::Parse {
                    path: path.as_str().to_string(),
                    reason: format!("expected UTF-8 glTF JSON source: {error}"),
                })?
                .to_string()
        };
        let json: JsonValue = serde_json::from_str(&json).map_err(|error| AssetError::Parse {
            path: path.as_str().to_string(),
            reason: error.to_string(),
        })?;
        Ok(json
            .get("buffers")
            .and_then(JsonValue::as_array)
            .map(|buffers| {
                buffers
                    .iter()
                    .enumerate()
                    .filter_map(|(index, buffer)| {
                        let uri = buffer.get("uri").and_then(JsonValue::as_str)?;
                        (!uri.starts_with("data:"))
                            .then(|| (index, resolve_relative_path(path, uri)))
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    fn from_gltf_json(
        path: AssetPath,
        source: &str,
        binary_chunk: Option<&[u8]>,
        external_buffers: &BTreeMap<usize, Vec<u8>>,
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

        let buffers = parse_buffers(&path, &json, binary_chunk, external_buffers)?;
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
        let lights = parse_punctual_lights(&json);
        let nodes = parse_gltf_nodes(&json, &meshes, &lights);
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

    pub fn light(&self) -> Option<SceneAssetLight> {
        self.light
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

    pub(crate) fn invalid_reason(&self) -> Option<&str> {
        self.invalid_reason.as_deref()
    }
}

impl SceneAssetLight {
    pub const fn light(self) -> Light {
        self.light
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

fn resolve_relative_path(base: &AssetPath, uri: &str) -> AssetPath {
    if uri.starts_with("data:") || uri.starts_with('/') || uri.contains("://") {
        return AssetPath::from(uri);
    }
    let Some((directory, _file)) = base.as_str().rsplit_once('/') else {
        return AssetPath::from(uri);
    };
    AssetPath::from(format!("{directory}/{uri}"))
}

fn parse_gltf_nodes(
    json: &JsonValue,
    meshes: &[SceneAssetMesh],
    lights: &[SceneAssetLight],
) -> Vec<SceneAssetNode> {
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
                    light: node
                        .get("extensions")
                        .and_then(|extensions| extensions.get("KHR_lights_punctual"))
                        .and_then(|extension| extension.get("light"))
                        .and_then(JsonValue::as_u64)
                        .and_then(|light| lights.get(light as usize))
                        .copied(),
                    anchors: parse_node_anchors(node),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_punctual_lights(json: &JsonValue) -> Vec<SceneAssetLight> {
    json.get("extensions")
        .and_then(|extensions| extensions.get("KHR_lights_punctual"))
        .and_then(|extension| extension.get("lights"))
        .and_then(JsonValue::as_array)
        .map(|lights| lights.iter().filter_map(parse_punctual_light).collect())
        .unwrap_or_default()
}

fn parse_punctual_light(light: &JsonValue) -> Option<SceneAssetLight> {
    let color = color3_field(light, "color", Color::WHITE);
    let intensity = number_field(light, "intensity").unwrap_or(1.0);
    let range = number_field(light, "range");
    let light = match light.get("type").and_then(JsonValue::as_str)? {
        "directional" => Light::Directional(
            DirectionalLight::default()
                .with_color(color)
                .with_illuminance_lux(intensity),
        ),
        "point" => {
            let mut point = PointLight::default()
                .with_color(color)
                .with_intensity_candela(intensity);
            if let Some(range) = range {
                point = point.with_range(range);
            }
            Light::Point(point)
        }
        "spot" => {
            let spot_json = light.get("spot").unwrap_or(&JsonValue::Null);
            let mut spot = SpotLight::default()
                .with_color(color)
                .with_intensity_candela(intensity)
                .with_inner_cone_angle(Angle::from_radians(
                    number_field(spot_json, "innerConeAngle").unwrap_or(0.0),
                ))
                .with_outer_cone_angle(Angle::from_radians(
                    number_field(spot_json, "outerConeAngle")
                        .unwrap_or(std::f32::consts::FRAC_PI_4),
                ));
            if let Some(range) = range {
                spot = spot.with_range(range);
            }
            Light::Spot(spot)
        }
        _ => return None,
    };
    Some(SceneAssetLight { light })
}

fn number_field(value: &JsonValue, field: &str) -> Option<f32> {
    value
        .get(field)
        .and_then(JsonValue::as_f64)
        .map(|value| value as f32)
}

fn color3_field(value: &JsonValue, field: &str, fallback: Color) -> Color {
    let Some(values) = value.get(field).and_then(JsonValue::as_array) else {
        return fallback;
    };
    Color::from_linear_rgb(
        array_f32(values, 0).unwrap_or(fallback.r),
        array_f32(values, 1).unwrap_or(fallback.g),
        array_f32(values, 2).unwrap_or(fallback.b),
    )
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

pub(super) fn parse_node_transform(node: &JsonValue) -> Transform {
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
