use std::collections::BTreeSet;

use scena::{
    AutoExposureConfig, Color, EnvironmentPreset, FramingOptions, MaterialDesc, PerspectiveCamera,
    VocabularyReportV1, vocabulary_report_v1,
};

#[test]
fn every_authoritative_preset_registry_is_machine_discoverable() {
    let report = vocabulary_report_v1();
    assert_values(&report, "material_presets", MaterialDesc::PRESET_NAMES);
    assert_values(
        &report,
        "camera_lens_presets",
        PerspectiveCamera::LENS_PRESET_NAMES,
    );
    assert_values(&report, "framing_presets", FramingOptions::PRESET_NAMES);
    let colors = Color::NAMED_CONSTANTS
        .iter()
        .map(|(name, _)| *name)
        .collect::<Vec<_>>();
    assert_values(&report, "named_colors", &colors);
    let environments = EnvironmentPreset::ALL
        .iter()
        .map(|preset| preset.recipe_name())
        .collect::<Vec<_>>();
    assert_values(&report, "environment_presets", &environments);
    assert_values(
        &report,
        "auto_exposure_presets",
        AutoExposureConfig::PRESET_NAMES,
    );
    assert_values(
        &report,
        "scene_presets",
        &["product_studio", "cad_studio", "industrial_studio"],
    );
    assert_values(
        &report,
        "render_profiles",
        &["auto", "quality", "balanced", "compatibility", "industrial"],
    );
    assert_values(&report, "quality_presets", &["low", "medium", "high"]);
    assert_values(&report, "tonemappers", &["standard", "aces", "pbr_neutral"]);
    assert_values(&report, "easing_curves", &["linear", "ease_in_out"]);
    assert_values(
        &report,
        "directional_light_presets",
        &["sun", "key", "fill", "rim"],
    );
    assert_values(
        &report,
        "point_light_presets",
        &["softbox", "bulb_warm", "bulb_cool"],
    );
    assert_values(&report, "area_light_presets", &["softbox"]);
    assert_values(&report, "studio_light_presets", &["studio_rig"]);

    for vocabulary in &report.vocabularies {
        assert!(
            !vocabulary.owner.is_empty(),
            "{} has no owner",
            vocabulary.name
        );
        assert_eq!(
            vocabulary.values.iter().cloned().collect::<BTreeSet<_>>(),
            vocabulary
                .entries
                .iter()
                .map(|entry| entry.name.clone())
                .collect::<BTreeSet<_>>(),
            "{} metadata does not cover every value",
            vocabulary.name
        );
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_preset_vocabulary_matches_scene_host_registry() {
    let scenes = scena::SceneSetupPreset::ALL
        .iter()
        .map(|preset| preset.recipe_name())
        .collect::<Vec<_>>();
    assert_values(&vocabulary_report_v1(), "scene_presets", &scenes);
}

#[test]
fn omitted_preset_mutation_is_rejected() {
    let mut report = vocabulary_report_v1();
    let materials = report
        .vocabularies
        .iter_mut()
        .find(|vocabulary| vocabulary.name == "material_presets")
        .expect("material vocabulary exists");
    materials.values.retain(|value| value != "chrome");
    materials.entries.retain(|entry| entry.name != "chrome");
    let errors = scena::validate_vocabulary_report_v1(&report);
    assert!(
        errors
            .iter()
            .any(|error| error.contains("material_presets") && error.contains("chrome")),
        "known-bad omission was not rejected: {errors:?}"
    );
}

fn assert_values(report: &VocabularyReportV1, name: &str, expected: &[&str]) {
    let vocabulary = report
        .vocabularies
        .iter()
        .find(|vocabulary| vocabulary.name == name)
        .unwrap_or_else(|| panic!("missing vocabulary {name}"));
    assert_eq!(
        vocabulary
            .values
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        expected.iter().copied().collect::<BTreeSet<_>>(),
        "{name} drifted from its authoritative registry"
    );
}
