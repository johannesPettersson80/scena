use serde::{Deserialize, Serialize};

pub const VOCABULARY_SCHEMA_V1: &str = "scena.vocab.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VocabularyReportV1 {
    pub schema: String,
    pub vocabularies: Vec<VocabularyV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VocabularyV1 {
    pub name: String,
    pub version: u32,
    pub owner: String,
    pub values: Vec<String>,
}

pub fn vocabulary_report_v1() -> VocabularyReportV1 {
    VocabularyReportV1 {
        schema: VOCABULARY_SCHEMA_V1.to_owned(),
        vocabularies: vec![
            vocabulary(
                "render_backends",
                "render/backend_selection",
                &["cpu", "headless_gpu", "native_surface", "webgpu", "webgl2"],
            ),
            vocabulary(
                "recipe_material_kinds",
                "scene/recipe/types/authoring",
                &[
                    "unlit",
                    "pbr_metallic_roughness",
                    "line",
                    "wireframe",
                    "edge",
                ],
            ),
            vocabulary(
                "placement_verbs",
                "scene/recipe/placement",
                &[
                    "center",
                    "ground",
                    "fit_to_size",
                    "look_at",
                    "align_to_anchor",
                    "place_on",
                ],
            ),
            vocabulary(
                "alpha_modes",
                "scene/recipe/types/authoring",
                &["opaque", "mask", "blend"],
            ),
            vocabulary(
                "texture_color_spaces",
                "scene/recipe/types/authoring",
                &["srgb", "linear"],
            ),
            vocabulary(
                "camera_kinds",
                "scene/recipe/types/authoring",
                &["perspective", "orthographic"],
            ),
            vocabulary(
                "light_kinds",
                "scene/recipe/types/authoring",
                &[
                    "directional",
                    "point",
                    "spot",
                    "area",
                    "ambient",
                    "hemisphere",
                ],
            ),
        ],
    }
}

pub fn vocabulary_v1(name: &str) -> Option<VocabularyV1> {
    vocabulary_report_v1()
        .vocabularies
        .into_iter()
        .find(|vocabulary| vocabulary.name == name)
}

fn vocabulary(name: &str, owner: &str, values: &[&str]) -> VocabularyV1 {
    VocabularyV1 {
        name: name.to_owned(),
        version: 1,
        owner: owner.to_owned(),
        values: values.iter().map(|value| (*value).to_owned()).collect(),
    }
}
