#![cfg(feature = "inspection")]

use scena::{
    Assets, Color, GeometryDesc, MaterialDesc, Renderer, RendererStats, Scene,
    VISIBILITY_DIAGNOSIS_SCHEMA_V1, Vec3, VisibilityDiagnosisOptions, VisibilityDiagnosisReportV1,
};

#[test]
fn visibility_diagnosis_classifies_actionable_node_failures() {
    let (assets, mut scene, renderer, node) = diagnostic_scene(true);
    let visible_inspection = scene.inspect_with_assets(&assets).to_schema_report();
    let handle = mesh_handle(&visible_inspection);
    scene.set_visible(node, false).expect("node hides");
    let inspection = scene.inspect_with_assets(&assets).to_schema_report();

    let hidden = renderer.diagnose_visibility(
        &inspection,
        Some(handle),
        VisibilityDiagnosisOptions::default(),
    );
    assert!(!hidden.ok);
    assert_eq!(hidden.schema, VISIBILITY_DIAGNOSIS_SCHEMA_V1);
    assert_reason(&hidden, "node_hidden");
    assert_fix(&hidden, "set_visible");
    assert_eq!(hidden.target.handle, Some(handle));

    scene.set_visible(node, true).expect("node shows");
    scene
        .set_transform(node, scena::Transform::IDENTITY.scale_by(0.0))
        .expect("node scales to zero");
    let inspection = scene.inspect_with_assets(&assets).to_schema_report();
    let zero_scale = renderer.diagnose_visibility(
        &inspection,
        Some(handle),
        VisibilityDiagnosisOptions::detail(),
    );
    assert!(!zero_scale.ok);
    assert_reason(&zero_scale, "zero_scale");
    assert_fix(&zero_scale, "set_transform");
    assert!(!zero_scale.evidence.is_empty());
}

#[test]
fn visibility_diagnosis_classifies_scene_level_failures() {
    let (assets, scene, renderer, _) = diagnostic_scene(false);
    let inspection = scene.inspect_with_assets(&assets).to_schema_report();

    let not_prepared =
        renderer.diagnose_visibility(&inspection, None, VisibilityDiagnosisOptions::default());
    assert!(!not_prepared.ok);
    assert_reason(&not_prepared, "not_prepared");
    assert_fix(&not_prepared, "prepare");

    let stats = RendererStats {
        culled_objects: 2,
        ..Default::default()
    };
    let all_culled = VisibilityDiagnosisReportV1::from_inspection(
        &inspection,
        stats,
        None,
        VisibilityDiagnosisOptions::default(),
        true,
    );
    assert!(!all_culled.ok);
    assert_reason(&all_culled, "all_culled");
    assert_fix(&all_culled, "frame_bounds");
}

#[test]
fn visibility_diagnosis_does_not_treat_partial_culling_as_all_culled() {
    let (assets, scene, _, _) = diagnostic_scene_with_drawables(true, 2);
    let inspection = scene.inspect_with_assets(&assets).to_schema_report();
    let stats = RendererStats {
        culled_objects: 1,
        ..Default::default()
    };

    let partial_cull = VisibilityDiagnosisReportV1::from_inspection(
        &inspection,
        stats,
        None,
        VisibilityDiagnosisOptions::default(),
        true,
    );

    assert!(partial_cull.ok, "{partial_cull:#?}");
    assert_no_reason(&partial_cull, "all_culled");
}

#[test]
fn visibility_diagnosis_reports_stale_handle_and_matches_golden_fixture() {
    let (assets, scene, renderer, _) = diagnostic_scene(true);
    let inspection = scene.inspect_with_assets(&assets).to_schema_report();

    let stale = renderer.diagnose_visibility(
        &inspection,
        Some(999_999),
        VisibilityDiagnosisOptions::default(),
    );
    assert!(!stale.ok);
    assert_reason(&stale, "stale_handle");
    assert!(
        stale
            .reasons
            .iter()
            .any(|reason| !reason.auto_fixable && reason.confidence == "high")
    );

    let fixture: VisibilityDiagnosisReportV1 = serde_json::from_str(include_str!(
        "assets/stable-contracts/visibility_diagnosis.v1.json"
    ))
    .expect("visibility diagnosis fixture deserializes");
    assert_eq!(fixture.schema, VISIBILITY_DIAGNOSIS_SCHEMA_V1);
    assert!(!fixture.ok);
    assert_reason(&fixture, "node_hidden");
    assert_fix(&fixture, "set_visible");
    assert_eq!(
        serde_json::to_value(&fixture).expect("fixture serializes"),
        serde_json::from_str::<serde_json::Value>(include_str!(
            "assets/stable-contracts/visibility_diagnosis.v1.json"
        ))
        .expect("fixture JSON parses")
    );
}

fn diagnostic_scene(prepared: bool) -> (Assets, Scene, Renderer, scena::NodeKey) {
    diagnostic_scene_with_drawables(prepared, 1)
}

fn diagnostic_scene_with_drawables(
    prepared: bool,
    drawable_count: usize,
) -> (Assets, Scene, Renderer, scena::NodeKey) {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    scene.add_default_camera().expect("camera inserts");
    let mut first_node = None;
    for index in 0..drawable_count.max(1) {
        let node = scene
            .mesh(geometry, material)
            .transform(scena::Transform::at(Vec3::new(
                index as f32 * 2.0,
                0.0,
                0.0,
            )))
            .add()
            .expect("mesh node inserts");
        first_node.get_or_insert(node);
    }
    let mut renderer = Renderer::headless(64, 64).expect("renderer builds");
    if prepared {
        renderer
            .prepare_with_assets(&mut scene, &assets)
            .expect("scene prepares");
    }
    (
        assets,
        scene,
        renderer,
        first_node.expect("at least one drawable inserted"),
    )
}

fn mesh_handle(report: &scena::SceneInspectionReportV1) -> u64 {
    report
        .draw_list
        .first()
        .expect("drawable appears in inspection")
        .node
}

fn assert_reason(report: &VisibilityDiagnosisReportV1, code: &str) {
    assert!(
        report.reasons.iter().any(|reason| reason.code == code),
        "expected reason {code} in {:#?}",
        report.reasons
    );
}

fn assert_no_reason(report: &VisibilityDiagnosisReportV1, code: &str) {
    assert!(
        !report.reasons.iter().any(|reason| reason.code == code),
        "did not expect reason {code} in {:#?}",
        report.reasons
    );
}

fn assert_fix(report: &VisibilityDiagnosisReportV1, action: &str) {
    assert!(
        report.fixes.iter().any(|fix| fix.action == action),
        "expected fix {action} in {:#?}",
        report.fixes
    );
}

#[allow(dead_code)]
fn _keep_vec3_in_public_test_surface(_: Vec3) {}
