use serde::{Deserialize, Serialize};

mod imports;
mod transform;

use super::expectations::SceneRecipeTargetRegionV1;
use super::overlays::SceneRecipeTargetV1;
use super::{default_true, is_false, is_true};

pub use imports::{
    SceneRecipeExpectedExtentV1, SceneRecipeImportEdgeEmphasisV1, SceneRecipeImportMaterialV1,
    SceneRecipeImportV1,
};
pub use transform::{
    SceneRecipeLookAtTargetV1, SceneRecipeTransformConversionError, SceneRecipeTransformV1,
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
    pub major_radius: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minor_radius: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bevel: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fillet: Option<f64>,
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
pub struct SceneRecipeMorphV1 {
    pub id: String,
    pub source_geometry: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<SceneRecipeMorphTargetV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeMorphTargetV1 {
    pub position_deltas: Vec<[f64; 3]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeSkinV1 {
    pub id: String,
    pub source_geometry: String,
    pub joints: Vec<[usize; 4]>,
    pub weights: Vec<[f64; 4]>,
}

impl SceneRecipeSkinV1 {
    pub fn influence_indices(&self) -> &[[usize; 4]] {
        &self.joints
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeMaterialV1 {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_color: Option<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clearcoat_factor: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clearcoat_roughness_factor: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clearcoat_normal_scale: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clearcoat_texture: Option<SceneRecipeTextureSlotV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clearcoat_roughness_texture: Option<SceneRecipeTextureSlotV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clearcoat_normal_texture: Option<SceneRecipeTextureSlotV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sheen_color_factor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sheen_roughness_factor: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sheen_color_texture: Option<SceneRecipeTextureSlotV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sheen_roughness_texture: Option<SceneRecipeTextureSlotV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anisotropy_strength_factor: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anisotropy_rotation_radians: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anisotropy_texture: Option<SceneRecipeTextureSlotV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iridescence_factor: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iridescence_ior: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iridescence_thickness_minimum_nm: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iridescence_thickness_maximum_nm: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iridescence_texture: Option<SceneRecipeTextureSlotV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iridescence_thickness_texture: Option<SceneRecipeTextureSlotV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dispersion_factor: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transmission_factor: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ior: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thickness_factor: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attenuation_distance: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attenuation_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transmission_texture: Option<SceneRecipeTextureSlotV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thickness_texture: Option<SceneRecipeTextureSlotV1>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material: Option<String>,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lods: Vec<SceneRecipeNodeLodV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub morph_weights: Vec<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skin_binding: Option<SceneRecipeNodeSkinBindingV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeNodeLodV1 {
    pub geometry: String,
    pub max_screen_fraction: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeNodeSkinBindingV1 {
    pub joints: Vec<String>,
    pub inverse_bind_matrices: Vec<[f64; 16]>,
}

impl SceneRecipeNodeSkinBindingV1 {
    pub fn binding_nodes(&self) -> &[String] {
        &self.joints
    }
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
pub struct SceneRecipeParticleSetV1 {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform: Option<SceneRecipeTransformV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer_mask: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render_group: Option<i16>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub particles: Vec<SceneRecipeParticleV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeParticleV1 {
    pub id: String,
    pub position: [f64; 3],
    pub color: String,
    pub size_px: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation_degrees: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeFontV1 {
    pub id: String,
    pub uri: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub optional: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeLabelV1 {
    pub id: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
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
pub struct SceneRecipeAnimationV1 {
    pub id: String,
    pub duration: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub channels: Vec<SceneRecipeAnimationChannelV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeAnimationChannelV1 {
    pub target: SceneRecipeTargetV1,
    pub path: String,
    #[serde(default = "default_animation_interpolation")]
    pub interpolation: String,
    pub times: Vec<f64>,
    pub values: Vec<Vec<f64>>,
}

fn default_animation_interpolation() -> String {
    "linear".to_owned()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeCameraV1 {
    pub id: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fov_degrees: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lens: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub framing: Option<SceneRecipeCameraFramingV1>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub active: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform: Option<SceneRecipeTransformV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeCameraFramingV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_px: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_region: Option<SceneRecipeTargetRegionV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeLightV1 {
    pub id: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub illuminance_lux: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intensity_candela: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub luminous_flux_lumens: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inner_cone_degrees: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outer_cone_degrees: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform: Option<SceneRecipeTransformV1>,
}
