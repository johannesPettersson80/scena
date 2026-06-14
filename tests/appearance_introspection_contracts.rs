#![cfg(feature = "inspection")]

use scena::{
    APPEARANCE_EXPECTATION_SCHEMA_V1, APPEARANCE_INTROSPECTION_SCHEMA_V1, AppearanceExpectationV1,
    AppearanceIntrospectionOptions, AppearanceIntrospectionReportV1, Assets, Color, GeometryDesc,
    MaterialDesc, Renderer, Scene, Transform, Vec3, headless_gltf_viewer,
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
            .any(|reason| reason.code == "color_family_mismatch"
                && reason.target_id == "wrong-swatch"),
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
            .reasons
            .iter()
            .any(|reason| reason.code == "conflicting_variant_expectations"
                && reason.severity == "warning"),
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
fn appearance_introspection_samples_each_target_node_bbox() {
    let (assets, scene, renderer) = two_color_scene();
    let capture = renderer
        .capture_rgba8(&scene, Default::default())
        .expect("two-color frame captures");
    let inspection = scene.inspect_with_assets(&assets).to_schema_report();
    let [left_node, right_node] = sorted_material_nodes(&inspection);
    let expectation: AppearanceExpectationV1 = serde_json::from_value(json!({
        "schema": APPEARANCE_EXPECTATION_SCHEMA_V1,
        "targets": [
            {
                "id": "left-red",
                "node": left_node,
                "color_family": "red",
                "swatch_srgb8": [236, 40, 40]
            },
            {
                "id": "right-blue",
                "node": right_node,
                "color_family": "blue",
                "swatch_srgb8": [40, 80, 236]
            }
        ]
    }))
    .expect("appearance expectation decodes");

    let report = renderer.introspect_appearance(
        &capture,
        &inspection,
        &expectation,
        AppearanceIntrospectionOptions::summary(),
    );

    assert!(report.ok, "{report:#?}");
    let left = target(&report, "left-red");
    let right = target(&report, "right-blue");
    assert_eq!(left.sampled_region.kind, "node_bbox", "{report:#?}");
    assert_eq!(right.sampled_region.kind, "node_bbox", "{report:#?}");
    assert_eq!(left.sampled_color_family, "red", "{report:#?}");
    assert_eq!(right.sampled_color_family, "blue", "{report:#?}");
    assert_ne!(
        left.sampled_color_srgb8, right.sampled_color_srgb8,
        "targets in one frame must not share a frame-global color sample"
    );
}

#[test]
fn appearance_introspection_honors_target_swatch_tolerance() {
    let (assets, scene, renderer) = two_color_scene();
    let capture = renderer
        .capture_rgba8(&scene, Default::default())
        .expect("two-color frame captures");
    let inspection = scene.inspect_with_assets(&assets).to_schema_report();
    let [left_node, _right_node] = sorted_material_nodes(&inspection);
    let expectation: AppearanceExpectationV1 = serde_json::from_value(json!({
        "schema": APPEARANCE_EXPECTATION_SCHEMA_V1,
        "targets": [{
            "id": "strict-red",
            "node": left_node,
            "swatch_srgb8": [255, 0, 0],
            "swatch_tolerance": 0.01
        }]
    }))
    .expect("appearance expectation decodes");

    let report = renderer.introspect_appearance(
        &capture,
        &inspection,
        &expectation,
        AppearanceIntrospectionOptions::summary(),
    );

    assert!(!report.ok, "{report:#?}");
    assert!(
        report
            .reasons
            .iter()
            .any(|reason| reason.code == "swatch_mismatch" && reason.target_id == "strict-red"),
        "{report:#?}"
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

fn two_color_scene() -> (Assets, Scene, Renderer) {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.46, 0.46, 0.18));
    let red = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(236, 40, 40)));
    let blue = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(40, 80, 236)));
    let mut scene = Scene::new();
    let left = scene
        .mesh(geometry, red)
        .transform(Transform::at(Vec3::new(-0.42, 0.0, 0.0)))
        .add()
        .expect("left mesh inserts");
    scene.add_tag(left, "left-red").expect("left tag inserts");
    let right = scene
        .mesh(geometry, blue)
        .transform(Transform::at(Vec3::new(0.42, 0.0, 0.0)))
        .add()
        .expect("right mesh inserts");
    scene
        .add_tag(right, "right-blue")
        .expect("right tag inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    scene
        .frame_all_with_assets(camera, &assets)
        .expect("two-color scene frames");

    let mut renderer = Renderer::headless(160, 96).expect("headless renderer builds");
    renderer.set_background_color(Color::from_srgb_u8(12, 12, 12));
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("two-color scene prepares");
    renderer
        .render_active(&scene)
        .expect("two-color scene renders");
    (assets, scene, renderer)
}

fn sorted_material_nodes(inspection: &scena::SceneInspectionReportV1) -> [u64; 2] {
    let mut draws = inspection
        .draw_list
        .iter()
        .filter(|draw| draw.visible && draw.material.is_some())
        .collect::<Vec<_>>();
    draws.sort_by(|left, right| {
        left.world_transform
            .translation
            .x
            .total_cmp(&right.world_transform.translation.x)
    });
    assert_eq!(draws.len(), 2, "{draws:#?}");
    [draws[0].node, draws[1].node]
}

fn target<'a>(
    report: &'a AppearanceIntrospectionReportV1,
    id: &str,
) -> &'a scena::AppearanceTargetReportV1 {
    report
        .targets
        .iter()
        .find(|target| target.id == id)
        .expect("target exists in appearance report")
}
