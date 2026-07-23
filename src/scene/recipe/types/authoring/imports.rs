use serde::{Deserialize, Deserializer, Serialize};

use super::super::{default_true, is_false, is_true};
use super::SceneRecipeTransformV1;

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge_emphasis: Option<SceneRecipeImportEdgeEmphasisV1>,
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
    pub base_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roughness: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metallic: Option<f64>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub double_sided: bool,
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
