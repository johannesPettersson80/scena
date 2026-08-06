use serde::{Deserialize, Deserializer, Serialize};

use super::super::{default_true, is_false, is_true};
use super::{SceneRecipeMaterialImperfectionV1, SceneRecipeMaterialPackV1, SceneRecipeTransformV1};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SceneRecipeImportV1 {
    pub id: String,
    pub uri: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub optional: bool,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_import_transform"
    )]
    pub transform: Option<SceneRecipeTransformV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_extent: Option<SceneRecipeExpectedExtentV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material: Option<SceneRecipeImportMaterialV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub material_bindings: Vec<SceneRecipeImportMaterialBindingV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_emphasis: Option<SceneRecipeImportEdgeEmphasisV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_rounding: Option<SceneRecipeImportEdgeRoundingV1>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ImportTransformCompatibilityV1 {
    Canonical(SceneRecipeTransformV1),
    Legacy(LegacyImportTransformV1),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyImportTransformV1 {
    translation: [f64; 3],
    rotation: [f64; 4],
    scale: [f64; 3],
}

fn deserialize_import_transform<'de, D>(
    deserializer: D,
) -> Result<Option<SceneRecipeTransformV1>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<ImportTransformCompatibilityV1>::deserialize(deserializer).map(|transform| {
        transform.map(|transform| match transform {
            ImportTransformCompatibilityV1::Canonical(transform) => transform,
            ImportTransformCompatibilityV1::Legacy(transform) => SceneRecipeTransformV1::Raw {
                translation: transform.translation,
                rotation: transform.rotation,
                scale: transform.scale,
            },
        })
    })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SceneRecipeExpectedExtentV1 {
    pub min: f64,
    pub max: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeImportMaterialV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material_pack: Option<SceneRecipeMaterialPackV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imperfection: Option<SceneRecipeMaterialImperfectionV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roughness: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metallic: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normal_scale: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occlusion_strength: Option<f64>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub double_sided: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeImportMaterialBindingV1 {
    pub source_material: SceneRecipeSourceMaterialSelectorV1,
    pub material: SceneRecipeImportMaterialV1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeSourceMaterialSelectorV1 {
    pub index: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeImportEdgeEmphasisV1 {
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_width_px: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_angle_threshold_degrees: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeImportEdgeRoundingV1 {
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub enabled: bool,
    #[serde(default = "default_edge_rounding_radius_fraction")]
    pub radius_fraction: f64,
    #[serde(default = "default_edge_rounding_segments")]
    pub segments: u8,
    #[serde(default = "default_edge_rounding_threshold_degrees")]
    pub edge_angle_threshold_degrees: f64,
    #[serde(default = "default_edge_rounding_max_derived_triangles")]
    pub max_derived_triangles: usize,
}

impl Default for SceneRecipeImportEdgeRoundingV1 {
    fn default() -> Self {
        Self {
            enabled: true,
            radius_fraction: default_edge_rounding_radius_fraction(),
            segments: default_edge_rounding_segments(),
            edge_angle_threshold_degrees: default_edge_rounding_threshold_degrees(),
            max_derived_triangles: default_edge_rounding_max_derived_triangles(),
        }
    }
}

impl SceneRecipeImportEdgeRoundingV1 {
    pub fn with_max_derived_triangles(mut self, max_derived_triangles: usize) -> Self {
        self.max_derived_triangles = max_derived_triangles;
        self
    }
}

const fn default_edge_rounding_radius_fraction() -> f64 {
    0.0025
}

const fn default_edge_rounding_segments() -> u8 {
    3
}

const fn default_edge_rounding_threshold_degrees() -> f64 {
    30.0
}

const fn default_edge_rounding_max_derived_triangles() -> usize {
    250_000
}
