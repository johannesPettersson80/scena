use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::scene::Transform;

pub const SCENE_RECIPE_SCHEMA_V1: &str = "scena.scene_recipe.v1";
pub const SCENE_RECIPE_VALIDATION_SCHEMA_V1: &str = "scena.scene_recipe_validation.v1";
pub const SCENE_RECIPE_BUILD_SCHEMA_V1: &str = "scena.scene_recipe_build.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneRecipeV1 {
    pub schema: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub imports: Vec<SceneRecipeImportV1>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub colors: BTreeMap<String, SceneRecipeColorV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub geometries: Vec<SceneRecipeGeometryV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub materials: Vec<SceneRecipeMaterialV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<SceneRecipeNodeV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cameras: Vec<SceneRecipeCameraV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lights: Vec<SceneRecipeLightV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section_box: Option<SceneRecipeSectionBoxV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub measurements: Vec<SceneRecipeMeasurementV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub callouts: Vec<SceneRecipeCalloutV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exploded_view: Option<SceneRecipeExplodedViewV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture: Option<SceneRecipeCaptureV1>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SceneRecipeColorV1 {
    Hex(String),
    Srgb8 { srgb8: [u8; 3] },
    Linear { linear: [f64; 3] },
    Kelvin { kelvin: f64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeGeometryV1 {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mesh: Option<SceneRecipeMeshV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primitive: Option<SceneRecipePrimitiveV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipePrimitiveV1 {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<Vec<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub segments: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rings: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub divisions: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<[f64; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<[f64; 3]>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub points: Vec<[f64; 3]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeMeshV1 {
    pub topology: String,
    pub positions: Vec<[f64; 3]>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub normals: Vec<[f64; 3]>,
    pub indices: Vec<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub colors: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub uvs: Vec<[f64; 2]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeMaterialV1 {
    pub id: String,
    pub kind: String,
    pub base_color: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metallic: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roughness: Option<f64>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub double_sided: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emissive: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emissive_strength: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alpha_mode: Option<SceneRecipeAlphaModeV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_width_px: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_angle_threshold_degrees: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_color_texture: Option<SceneRecipeTextureSlotV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normal_texture: Option<SceneRecipeTextureSlotV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metallic_roughness_texture: Option<SceneRecipeTextureSlotV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occlusion_texture: Option<SceneRecipeTextureSlotV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emissive_texture: Option<SceneRecipeTextureSlotV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SceneRecipeAlphaModeV1 {
    Opaque,
    Mask { cutoff: f64 },
    Blend,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneRecipeTextureColorSpaceV1 {
    Srgb,
    Linear,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeTextureSlotV1 {
    pub uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_space: Option<SceneRecipeTextureColorSpaceV1>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub optional: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeNodeV1 {
    pub id: String,
    pub geometry: String,
    pub material: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer_mask: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render_group: Option<i16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform: Option<SceneRecipeTransformV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeCameraV1 {
    pub id: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fov_degrees: Option<f64>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub active: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform: Option<SceneRecipeTransformV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeLightV1 {
    pub id: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub illuminance_lux: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intensity_candela: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inner_cone_degrees: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outer_cone_degrees: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform: Option<SceneRecipeTransformV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SceneRecipeTransformV1 {
    Raw {
        #[serde(default, skip_serializing_if = "is_zero_vec3")]
        translation: [f64; 3],
        rotation: [f64; 4],
        #[serde(
            default = "default_transform_scale",
            skip_serializing_if = "is_default_scale"
        )]
        scale: [f64; 3],
    },
    Trs {
        #[serde(default, skip_serializing_if = "is_zero_vec3")]
        translation: [f64; 3],
        #[serde(default, skip_serializing_if = "is_zero_vec3")]
        rotation_degrees: [f64; 3],
        #[serde(
            default = "default_transform_scale",
            skip_serializing_if = "is_default_scale"
        )]
        scale: [f64; 3],
    },
    LookAt {
        eye: [f64; 3],
        target: SceneRecipeLookAtTargetV1,
        #[serde(
            default = "default_transform_up",
            skip_serializing_if = "is_default_up"
        )]
        up: [f64; 3],
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SceneRecipeLookAtTargetV1 {
    Node(String),
    Position([f64; 3]),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneRecipeImportV1 {
    pub id: String,
    pub uri: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub optional: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform: Option<Transform>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_extent: Option<SceneRecipeExpectedExtentV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneRecipeExpectedExtentV1 {
    pub min: f64,
    pub max: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeSectionBoxV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<SceneRecipeTargetV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import: Option<String>,
    #[serde(default)]
    pub margin: f32,
    #[serde(default)]
    pub inverted: bool,
    #[serde(default)]
    pub helper_wireframe: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SceneRecipeTargetV1 {
    Node { id: String },
    Import { id: String },
    World { position: [f32; 3] },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeMeasurementV1 {
    pub id: String,
    pub kind: String,
    pub start: [f32; 3],
    pub end: [f32; 3],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub precision: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeCalloutV1 {
    pub id: String,
    pub text: String,
    pub target: SceneRecipeCalloutTargetV1,
    #[serde(default)]
    pub label_offset: [f32; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SceneRecipeCalloutTargetV1 {
    ImportRoot {
        import: String,
        #[serde(default)]
        local_offset: [f32; 3],
    },
    Node {
        id: String,
        #[serde(default)]
        local_offset: [f32; 3],
    },
    World {
        position: [f32; 3],
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeExplodedViewV1 {
    pub import: String,
    #[serde(default)]
    pub mode: SceneRecipeExplodedViewModeV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axis: Option<[f32; 3]>,
    #[serde(default = "default_exploded_factor")]
    pub factor: f32,
    #[serde(default = "default_exploded_distance")]
    pub distance: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneRecipeExplodedViewModeV1 {
    #[default]
    DirectChildren,
    HierarchyDepth,
    Axis,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneRecipeCaptureV1 {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneRecipeValidationReportV1 {
    pub schema: String,
    pub ok: bool,
    pub diagnostics: Vec<SceneRecipeDiagnosticV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneRecipeBuildV1 {
    pub schema: String,
    pub ok: bool,
    pub imports: Vec<SceneRecipeBuildImportV1>,
    pub nodes: Vec<SceneRecipeBuildTargetV1>,
    pub cameras: Vec<SceneRecipeBuildTargetV1>,
    pub lights: Vec<SceneRecipeBuildTargetV1>,
    pub geometries: Vec<SceneRecipeBuildResourceV1>,
    pub materials: Vec<SceneRecipeBuildResourceV1>,
    pub diagnostics: Vec<SceneRecipeDiagnosticV1>,
    pub skipped: Vec<SceneRecipeBuildSkippedV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneRecipeBuildImportV1 {
    pub id: String,
    pub uri: String,
    pub import_handle: u64,
    pub root_handles: Vec<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_root: Option<u64>,
    pub nodes_by_path: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneRecipeBuildTargetV1 {
    pub id: String,
    pub handle: u64,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneRecipeBuildResourceV1 {
    pub id: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertex_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_count: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneRecipeBuildSkippedV1 {
    pub path: String,
    pub id: String,
    pub kind: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneRecipeDiagnosticV1 {
    pub code: String,
    pub severity: String,
    pub path: String,
    pub message: String,
    pub help: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    #[serde(default)]
    pub auto_fixable: bool,
}

fn default_exploded_factor() -> f32 {
    1.0
}

fn default_exploded_distance() -> f32 {
    1.0
}

fn default_transform_scale() -> [f64; 3] {
    [1.0, 1.0, 1.0]
}

fn default_transform_up() -> [f64; 3] {
    [0.0, 1.0, 0.0]
}

fn is_zero_vec3(value: &[f64; 3]) -> bool {
    *value == [0.0, 0.0, 0.0]
}

fn is_default_scale(value: &[f64; 3]) -> bool {
    *value == default_transform_scale()
}

fn is_default_up(value: &[f64; 3]) -> bool {
    *value == default_transform_up()
}

fn is_false(value: &bool) -> bool {
    !*value
}
