use serde::{Deserialize, Serialize};

use crate::scene::recipe::{
    AREA_LIGHT_PRESETS, DIRECTIONAL_LIGHT_PRESETS, POINT_LIGHT_PRESETS, RENDER_PROFILES,
    RENDER_QUALITIES, SCENE_PRESETS, STUDIO_LIGHT_PRESETS, TONEMAPPERS,
};

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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entries: Vec<VocabularyValueV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VocabularyValueV1 {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub deprecated: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub feature_requirements: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capability_requirements: Vec<String>,
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
            vocabulary("placement_verbs", "scene/placement", crate::PLACEMENT_VERBS),
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
            vocabulary(
                "material_presets",
                "material/presets",
                crate::MaterialDesc::PRESET_NAMES,
            ),
            vocabulary(
                "camera_lens_presets",
                "scene/camera",
                crate::PerspectiveCamera::LENS_PRESET_NAMES,
            ),
            vocabulary(
                "framing_presets",
                "scene/framing",
                crate::FramingOptions::PRESET_NAMES,
            ),
            vocabulary_owned(
                "named_colors",
                "material/color",
                crate::Color::NAMED_CONSTANTS
                    .iter()
                    .map(|(name, _)| (*name).to_owned())
                    .collect(),
            ),
            vocabulary_owned(
                "environment_presets",
                "assets/environment_preset",
                crate::EnvironmentPreset::ALL
                    .iter()
                    .map(|preset| preset.recipe_name().to_owned())
                    .collect(),
            ),
            vocabulary(
                "auto_exposure_presets",
                "render/exposure",
                crate::AutoExposureConfig::PRESET_NAMES,
            ),
            vocabulary_with_feature(
                "scene_presets",
                "scene_host/product",
                SCENE_PRESETS,
                "scene-host",
            ),
            vocabulary("render_profiles", "render/settings", RENDER_PROFILES),
            vocabulary("quality_presets", "render/settings", RENDER_QUALITIES),
            vocabulary("tonemappers", "render/output", TONEMAPPERS),
            vocabulary_with_alias(
                "easing_curves",
                "controls/camera_transition",
                crate::TransitionEasing::NAMES,
                "ease_in_out",
                "easeInOut",
            ),
            vocabulary(
                "directional_light_presets",
                "scene/recipe/validation/authoring/lights",
                DIRECTIONAL_LIGHT_PRESETS,
            ),
            vocabulary(
                "point_light_presets",
                "scene/recipe/validation/authoring/lights",
                POINT_LIGHT_PRESETS,
            ),
            vocabulary(
                "area_light_presets",
                "scene/recipe/validation/authoring/lights",
                AREA_LIGHT_PRESETS,
            ),
            vocabulary(
                "studio_light_presets",
                "scene/recipe/validation/authoring/lights",
                STUDIO_LIGHT_PRESETS,
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
    vocabulary_owned(
        name,
        owner,
        values.iter().map(|value| (*value).to_owned()).collect(),
    )
}

fn vocabulary_owned(name: &str, owner: &str, values: Vec<String>) -> VocabularyV1 {
    let entries = values
        .iter()
        .map(|name| VocabularyValueV1 {
            name: name.clone(),
            aliases: Vec::new(),
            deprecated: false,
            feature_requirements: Vec::new(),
            capability_requirements: Vec::new(),
        })
        .collect();
    VocabularyV1 {
        name: name.to_owned(),
        version: 1,
        owner: owner.to_owned(),
        values,
        entries,
    }
}

fn vocabulary_with_feature(
    name: &str,
    owner: &str,
    values: &[&str],
    feature: &str,
) -> VocabularyV1 {
    let mut vocabulary = vocabulary(name, owner, values);
    for entry in &mut vocabulary.entries {
        entry.feature_requirements.push(feature.to_owned());
    }
    vocabulary
}

fn vocabulary_with_alias(
    name: &str,
    owner: &str,
    values: &[&str],
    canonical: &str,
    alias: &str,
) -> VocabularyV1 {
    let mut vocabulary = vocabulary(name, owner, values);
    vocabulary
        .entries
        .iter_mut()
        .find(|entry| entry.name == canonical)
        .expect("canonical vocabulary value exists")
        .aliases
        .push(alias.to_owned());
    vocabulary
}

pub fn validate_vocabulary_report_v1(report: &VocabularyReportV1) -> Vec<String> {
    let expected = vocabulary_report_v1();
    let mut errors = Vec::new();
    for expected_vocabulary in expected.vocabularies {
        let Some(actual) = report
            .vocabularies
            .iter()
            .find(|actual| actual.name == expected_vocabulary.name)
        else {
            errors.push(format!("missing vocabulary '{}'", expected_vocabulary.name));
            continue;
        };
        for expected_value in &expected_vocabulary.values {
            if !actual.values.contains(expected_value) {
                errors.push(format!(
                    "vocabulary '{}' is missing authoritative value '{}'",
                    expected_vocabulary.name, expected_value
                ));
            }
        }
        for actual_value in &actual.values {
            if !expected_vocabulary.values.contains(actual_value) {
                errors.push(format!(
                    "vocabulary '{}' advertises non-authoritative value '{}'",
                    expected_vocabulary.name, actual_value
                ));
            }
        }
        if actual.entries != expected_vocabulary.entries {
            errors.push(format!(
                "vocabulary '{}' value metadata differs from its authoritative registry",
                expected_vocabulary.name
            ));
        }
    }
    for actual in &report.vocabularies {
        if !vocabulary_report_v1()
            .vocabularies
            .iter()
            .any(|expected| expected.name == actual.name)
        {
            errors.push(format!("unknown vocabulary '{}'", actual.name));
        }
    }
    errors
}
