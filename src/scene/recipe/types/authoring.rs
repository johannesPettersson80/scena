use serde::{Deserialize, Serialize};

use crate::scene::Transform;

use super::{
    default_transform_scale, default_transform_up, default_true, is_default_scale, is_default_up,
    is_false, is_true, is_zero_f64, is_zero_vec3,
};

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
pub struct SceneRecipeInstanceSetV1 {
    pub id: String,
    pub geometry: String,
    pub material: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform: Option<SceneRecipeTransformV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub instances: Vec<SceneRecipeInstanceV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeInstanceV1 {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform: Option<SceneRecipeTransformV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeLabelV1 {
    pub id: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform: Option<SceneRecipeTransformV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub halo: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_px: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeClippingPlaneV1 {
    pub id: String,
    pub normal: [f64; 3],
    pub distance: f64,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub active: bool,
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
    Center {},
    Ground {
        #[serde(default, skip_serializing_if = "is_zero_f64")]
        plane_y: f64,
    },
    FitToSize {
        size: [f64; 3],
    },
    PlaceOn {
        target: String,
        #[serde(default, skip_serializing_if = "is_zero_vec3")]
        offset: [f64; 3],
    },
    AlignToAnchor {
        anchor: String,
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
