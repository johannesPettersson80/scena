#![cfg(feature = "inspection")]

use scena::{
    APPEARANCE_EXPECTATION_SCHEMA_V1, APPEARANCE_INTROSPECTION_SCHEMA_V1, AppearanceExpectationV1,
    AppearanceIntrospectionOptions, AppearanceIntrospectionReportV1, headless_gltf_viewer,
};
use serde_json::json;

const VARIANT_ASSET: &str = "tests/assets/gltf/material_variants_scene.gltf";

#[test]
fn appearance_introspection_verifies_first_time_variant_color_without_golden_image() {
    let mut viewer = pollster::block_on(
        headless_gltf_viewer(VARIANT_ASSET)
            .size(96, 72)
            .with_default_light()
            .build(),
    )
    .expect("variant viewer builds");
    viewer
        .set_active_material_variant(Some("noon"))
        .expect("noon variant applies");
    viewer
        .render_next_frame()
        .expect("variant viewer renders after applying variant");
    let capture = viewer.capture().expect("variant frame captures");
    let inspection = viewer
        .scene()
        .inspect_with_assets(viewer.assets())
        .to_schema_report();
    let expectation: AppearanceExpectationV1 = serde_json::from_value(json!({
        "schema": APPEARANCE_EXPECTATION_SCHEMA_V1,
        "targets": [{
            "id": "variant-swatch",
            "variant": "noon",
            "color_family": "green",
            "swatch_srgb8": [0, 255, 0],
            "require_source_material": true,
            "alpha_mode": "opaque"
        }]
    }))
    .expect("appearance expectation decodes");

    let report = AppearanceIntrospectionReportV1::from_capture(
        &capture,
        &inspection,
        &expectation,
        AppearanceIntrospectionOptions::summary()
            .with_active_material_variant(viewer.active_material_variant())
            .with_available_material_variants(viewer.material_variants().to_vec()),
    );

    assert_eq!(report.schema, APPEARANCE_INTROSPECTION_SCHEMA_V1);
    assert!(report.ok, "{report:#?}");
    assert_eq!(report.active_variant.as_deref(), Some("noon"));
    assert_eq!(report.targets[0].sampled_color_family, "green");
    assert_eq!(
        report.targets[0]
            .material
            .as_ref()
            .expect("matched target carries material")
            .source
            .kind,
        "source_material"
    );
    assert!(report.reasons.is_empty());
}

#[test]
fn appearance_introspection_fails_closed_for_wrong_color_missing_variant_and_fallback() {
    let mut viewer = pollster::block_on(
        headless_gltf_viewer(VARIANT_ASSET)
            .size(96, 72)
            .with_default_light()
            .build(),
    )
    .expect("variant viewer builds");
    viewer
        .set_active_material_variant(Some("noon"))
        .expect("noon variant applies");
    viewer
        .render_next_frame()
        .expect("variant viewer renders after applying variant");
    let capture = viewer.capture().expect("variant frame captures");
    let inspection = viewer
        .scene()
        .inspect_with_assets(viewer.assets())
        .to_schema_report();
    let expectation: AppearanceExpectationV1 = serde_json::from_value(json!({
        "schema": APPEARANCE_EXPECTATION_SCHEMA_V1,
        "targets": [
            {
                "id": "wrong-swatch",
                "variant": "noon",
                "color_family": "blue",
                "swatch_srgb8": [0, 0, 255],
                "require_source_material": true
            },
            {
                "id": "missing-variant",
                "variant": "sunset",
                "color_family": "green"
            }
        ]
    }))
    .expect("appearance expectation decodes");

    let report = AppearanceIntrospectionReportV1::from_capture(
        &capture,
        &inspection,
        &expectation,
        AppearanceIntrospectionOptions::summary()
            .with_active_material_variant(viewer.active_material_variant())
            .with_available_material_variants(viewer.material_variants().to_vec()),
    );

    assert!(!report.ok);
    assert!(
        report
            .reasons
            .iter()
            .any(|reason| reason.code == "color_mismatch" && reason.target_id == "wrong-swatch"),
        "{report:#?}"
    );
    assert!(
        report
            .reasons
            .iter()
            .any(|reason| reason.code == "variant_missing"
                && reason.target_id == "missing-variant"),
        "{report:#?}"
    );
    assert!(
        report
            .fixes
            .iter()
            .any(|fix| fix.action == "inspect_material_assignment")
    );
}

#[test]
fn appearance_introspection_golden_fixtures_match_live_schema() {
    let expectation: AppearanceExpectationV1 = serde_json::from_str(include_str!(
        "assets/stable-contracts/appearance_expectation.v1.json"
    ))
    .expect("appearance expectation fixture decodes");
    assert_eq!(expectation.schema, APPEARANCE_EXPECTATION_SCHEMA_V1);
    assert_eq!(expectation.targets[0].id, "primary-finish");

    let report: AppearanceIntrospectionReportV1 = serde_json::from_str(include_str!(
        "assets/stable-contracts/appearance_introspection.v1.json"
    ))
    .expect("appearance report fixture decodes");
    assert_eq!(report.schema, APPEARANCE_INTROSPECTION_SCHEMA_V1);
    assert!(!report.ok);
    assert!(
        report
            .reasons
            .iter()
            .any(|reason| reason.code == "generated_fallback")
    );
}
