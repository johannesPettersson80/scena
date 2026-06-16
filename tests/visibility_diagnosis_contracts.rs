#![cfg(feature = "inspection")]

use scena::{
    AlphaMode, Assets, ClippingPlane, ClippingPlaneSet, Color, Diagnostic, DiagnosticCode,
    GeometryDesc, MaterialDesc, RenderIntrospectionOptions, Renderer, RendererStats, Scene,
    VISIBILITY_DIAGNOSIS_SCHEMA_V1, Vec3, VisibilityDiagnosisOptions, VisibilityDiagnosisReportV1,
    capture_rgba8_from_pixels,
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
        draw_calls: 1,
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

    let clipped_but_drawn_stats = RendererStats {
        culled_objects: 438,
        draw_calls: 4,
        ..Default::default()
    };
    let clipped_but_drawn = VisibilityDiagnosisReportV1::from_inspection(
        &inspection,
        clipped_but_drawn_stats,
        None,
        VisibilityDiagnosisOptions::default(),
        true,
    );

    assert!(clipped_but_drawn.ok, "{clipped_but_drawn:#?}");
    assert_no_reason(&clipped_but_drawn, "all_culled");
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

#[test]
fn visibility_diagnosis_classifies_parent_hidden_nan_and_layer_masked_targets() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    let camera = scene.add_default_camera().expect("camera inserts");
    scene
        .set_camera_layer_mask(camera, 0b0001)
        .expect("camera mask updates");
    let parent = scene
        .add_empty(scene.root(), scena::Transform::IDENTITY)
        .expect("parent inserts");
    let child = scene
        .mesh(geometry, material)
        .parent(parent)
        .add()
        .expect("child mesh inserts");
    scene.set_visible(parent, false).expect("parent hides");
    let inspection = scene.inspect_with_assets(&assets).to_schema_report();
    let child_handle = node_handle_for_kind(&inspection, "mesh");
    let renderer = Renderer::headless(64, 64).expect("renderer builds");

    let parent_hidden = renderer.diagnose_visibility(
        &inspection,
        Some(child_handle),
        VisibilityDiagnosisOptions::detail(),
    );
    assert!(!parent_hidden.ok);
    assert_reason(&parent_hidden, "parent_hidden");
    assert_fix(&parent_hidden, "set_visible");
    assert!(
        parent_hidden
            .evidence
            .iter()
            .any(|entry| entry.kind == "visibility_ancestor")
    );

    scene.set_visible(parent, true).expect("parent shows");
    scene
        .set_transform(child, scena::Transform::at(Vec3::new(f32::NAN, 0.0, 0.0)))
        .expect("non-finite transform stores");
    let inspection = scene.inspect_with_assets(&assets).to_schema_report();
    let nan_transform = renderer.diagnose_visibility(
        &inspection,
        Some(child_handle),
        VisibilityDiagnosisOptions::detail(),
    );
    assert!(!nan_transform.ok);
    assert_reason(&nan_transform, "nan_transform");
    assert_fix(&nan_transform, "set_transform");

    scene
        .set_transform(child, scena::Transform::IDENTITY)
        .expect("finite transform restores");
    scene
        .set_layer_mask(child, 0b0010)
        .expect("child mask hides");
    let inspection = scene.inspect_with_assets(&assets).to_schema_report();
    let layer_masked = renderer.diagnose_visibility(
        &inspection,
        Some(child_handle),
        VisibilityDiagnosisOptions::detail(),
    );
    assert!(!layer_masked.ok);
    assert_reason(&layer_masked, "layer_masked");
    assert_fix(&layer_masked, "set_layer_mask");
}

#[test]
fn visibility_diagnosis_walks_subtree_targets() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    scene.add_default_camera().expect("camera inserts");
    let subtree = scene
        .add_empty(scene.root(), scena::Transform::IDENTITY)
        .expect("subtree root inserts");
    let hidden_child = scene
        .mesh(geometry, material)
        .parent(subtree)
        .add()
        .expect("subtree child inserts");
    scene
        .set_visible(hidden_child, false)
        .expect("subtree child hides");
    let inspection = scene.inspect_with_assets(&assets).to_schema_report();
    let hidden_child_handle = node_handle_for_kind(&inspection, "mesh");
    let subtree_handle = inspection
        .nodes
        .iter()
        .find(|node| node.parent == Some(1) && node.kind.eq_ignore_ascii_case("empty"))
        .expect("subtree root appears")
        .handle;
    let renderer = Renderer::headless(64, 64).expect("renderer builds");

    let report = renderer.diagnose_visibility(
        &inspection,
        Some(subtree_handle),
        VisibilityDiagnosisOptions::detail(),
    );

    assert!(!report.ok);
    assert_reason(&report, "node_hidden");
    assert!(
        report
            .reasons
            .iter()
            .any(|reason| reason.affected_handles.contains(&hidden_child_handle)),
        "hidden descendant handle should be affected: {report:#?}"
    );
}

#[test]
fn visibility_diagnosis_classifies_material_and_asset_visibility_failures() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let alpha_zero = assets.create_material(MaterialDesc::unlit(Color::TRANSPARENT));
    let transparent = assets.create_material(
        MaterialDesc::unlit(Color::from_linear_rgba(1.0, 1.0, 1.0, 0.4))
            .with_alpha_mode(AlphaMode::Blend),
    );
    let mut scene = Scene::new();
    let camera = scene.add_default_camera().expect("camera inserts");
    scene
        .mesh(geometry, alpha_zero)
        .add()
        .expect("alpha-zero mesh inserts");
    scene
        .mesh(geometry, transparent)
        .transform(scena::Transform::at(Vec3::new(2.0, 0.0, 0.0)))
        .add()
        .expect("transparent mesh inserts");
    scene
        .frame_all_with_assets(camera, &assets)
        .expect("camera frames material test scene");
    let mut renderer = Renderer::headless(64, 64).expect("renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("scene prepares");
    let inspection = scene.inspect_with_assets(&assets).to_schema_report();

    let alpha_zero_handle = node_handle_for_material_alpha(&inspection, 0.0);
    let alpha_zero_report = renderer.diagnose_visibility(
        &inspection,
        Some(alpha_zero_handle),
        VisibilityDiagnosisOptions::default(),
    );
    assert!(!alpha_zero_report.ok);
    assert_reason(&alpha_zero_report, "alpha_zero");
    assert_fix(&alpha_zero_report, "set_material_alpha");

    let transparent_handle = node_handle_for_alpha_mode(&inspection, "blend");
    let transparent_report = renderer.diagnose_visibility(
        &inspection,
        Some(transparent_handle),
        VisibilityDiagnosisOptions::default(),
    );
    assert!(
        transparent_report.ok,
        "transparent materials are visibility warnings, not hard failures: {transparent_report:#?}"
    );
    assert_reason(&transparent_report, "transparent_material");

    let wrong_assets = Assets::new();
    let _same_geometry_key = wrong_assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let missing_material_inspection = scene.inspect_with_assets(&wrong_assets).to_schema_report();
    let missing_material = renderer.diagnose_visibility(
        &missing_material_inspection,
        Some(alpha_zero_handle),
        VisibilityDiagnosisOptions::detail(),
    );
    assert!(!missing_material.ok);
    assert_reason(&missing_material, "missing_material_upload");

    let empty_assets = Assets::new();
    let missing_geometry_inspection = scene.inspect_with_assets(&empty_assets).to_schema_report();
    let missing_geometry = renderer.diagnose_visibility(
        &missing_geometry_inspection,
        Some(alpha_zero_handle),
        VisibilityDiagnosisOptions::detail(),
    );
    assert!(!missing_geometry.ok);
    assert_reason(&missing_geometry, "missing_geometry");
}

#[test]
fn visibility_diagnosis_maps_camera_clipping_and_backend_diagnostics() {
    let (assets, mut scene, _, _) = diagnostic_scene(true);
    let plane = scene.add_clipping_plane(ClippingPlane::new(Vec3::X, -10.0));
    scene
        .set_clipping_planes(ClippingPlaneSet::new().with_plane(plane))
        .expect("clipping plane activates");
    let inspection = scene.inspect_with_assets(&assets).to_schema_report();
    let handle = mesh_handle(&inspection);
    let diagnostics = vec![
        Diagnostic::warning(
            DiagnosticCode::ObjectsBehindCamera,
            "all mesh bounds are behind the active camera",
            "move or frame the camera",
        ),
        Diagnostic::warning(
            DiagnosticCode::SceneOutsideCameraFrustum,
            "mesh bounds are outside the active camera frustum",
            "frame the scene",
        ),
        Diagnostic::warning(
            DiagnosticCode::ForwardPbrDegraded,
            "forward PBR is degraded on this backend",
            "inspect capabilities",
        ),
    ];

    let report = VisibilityDiagnosisReportV1::from_inspection_with_diagnostics(
        &inspection,
        RendererStats::default(),
        Some(handle),
        VisibilityDiagnosisOptions::detail(),
        true,
        &diagnostics,
    );

    assert!(!report.ok);
    assert_reason(&report, "behind_camera");
    assert_reason(&report, "outside_frustum");
    assert_reason(&report, "clipped_by_active_clipping_plane");
    assert_reason(&report, "backend_capability_degraded");
    assert_fix(&report, "frame_bounds");
    assert_fix(&report, "clear_clipping_planes");
    assert_fix(&report, "inspect_capabilities");
}

#[test]
fn visibility_diagnosis_agrees_with_render_introspection_for_scene_level_failures() {
    let empty_assets = Assets::new();
    let mut empty_scene = Scene::new();
    empty_scene.add_default_camera().expect("camera inserts");
    let mut empty_renderer = Renderer::headless(64, 64).expect("renderer builds");
    empty_renderer
        .prepare_with_assets(&mut empty_scene, &empty_assets)
        .expect("empty scene prepares");
    empty_renderer
        .render_active(&empty_scene)
        .expect("empty scene renders");
    let empty_inspection = empty_scene
        .inspect_with_assets(&empty_assets)
        .to_schema_report();
    let empty_capture = capture_rgba8_from_pixels(
        &empty_scene,
        &empty_renderer,
        Default::default(),
        64,
        64,
        vec![0; 64 * 64 * 4],
    )
    .expect("empty frame captures");
    let empty_introspection = empty_renderer.introspect_capture(
        &empty_capture,
        &empty_inspection,
        RenderIntrospectionOptions::default(),
    );
    let empty_diagnosis = empty_renderer.diagnose_visibility(
        &empty_inspection,
        None,
        VisibilityDiagnosisOptions::default(),
    );
    assert_reason_code(&empty_introspection.reasons, "no_visible_drawables");
    assert_reason(&empty_diagnosis, "no_visible_drawables");

    let (culled_assets, culled_scene, mut culled_renderer, _) =
        diagnostic_scene_with_drawables(true, 1);
    culled_renderer
        .render_active(&culled_scene)
        .expect("culled baseline renders");
    let culled_inspection = culled_scene
        .inspect_with_assets(&culled_assets)
        .to_schema_report();
    let culled_stats = RendererStats {
        culled_objects: 1,
        ..Default::default()
    };
    let culled_capture = capture_rgba8_from_pixels(
        &culled_scene,
        &culled_renderer,
        Default::default(),
        64,
        64,
        vec![0; 64 * 64 * 4],
    )
    .expect("culled frame captures");
    let culled_introspection = scena::RenderIntrospectionReportV1::from_capture(
        &culled_capture,
        &culled_inspection,
        culled_stats,
        RenderIntrospectionOptions::default(),
    );
    let culled_diagnosis = VisibilityDiagnosisReportV1::from_inspection(
        &culled_inspection,
        culled_stats,
        None,
        VisibilityDiagnosisOptions::default(),
        true,
    );
    assert_reason_code(&culled_introspection.reasons, "all_culled");
    assert_reason(&culled_diagnosis, "all_culled");

    let (behind_assets, mut behind_scene, mut behind_renderer, _) =
        diagnostic_scene_at(Vec3::new(0.0, 0.0, 4.0));
    behind_renderer
        .prepare_with_assets(&mut behind_scene, &behind_assets)
        .expect("behind-camera scene prepares");
    behind_renderer
        .render_active(&behind_scene)
        .expect("behind-camera scene renders");
    let behind_inspection = behind_scene
        .inspect_with_assets(&behind_assets)
        .to_schema_report();
    let behind_capture = behind_renderer
        .capture_rgba8(&behind_scene, Default::default())
        .expect("behind-camera scene captures");
    let behind_introspection = behind_renderer.introspect_capture(
        &behind_capture,
        &behind_inspection,
        RenderIntrospectionOptions::default(),
    );
    let behind_diagnosis = behind_renderer.diagnose_visibility(
        &behind_inspection,
        None,
        VisibilityDiagnosisOptions::default(),
    );
    assert_reason_code(&behind_introspection.reasons, "behind_camera");
    assert_reason(&behind_diagnosis, "behind_camera");
}

#[cfg(feature = "scene-host")]
#[test]
fn visibility_diagnosis_accepts_scene_host_import_targets() {
    let mut host = scena::SceneHostCore::headless(96, 96).expect("host builds");
    let import = pollster::block_on(
        host.instantiate_url("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
    )
    .expect("asset imports");
    let roots = host.import_roots(import).expect("import roots resolve");
    assert!(!roots.is_empty());
    host.set_visible(roots[0], false)
        .expect("import root hides through stable handle");
    host.prepare().expect("host prepares");
    let inspection: scena::SceneInspectionReportV1 =
        serde_json::from_str(&host.inspect_json().expect("inspection serializes"))
            .expect("inspection decodes");
    let import_row = inspection
        .imports
        .as_ref()
        .and_then(|imports| imports.iter().find(|entry| entry.handle == import))
        .expect("import inspection row exists");
    assert_eq!(import_row.root_handles, roots);

    let report = VisibilityDiagnosisReportV1::from_inspection(
        &inspection,
        RendererStats::default(),
        Some(import),
        VisibilityDiagnosisOptions::detail(),
        true,
    );

    assert!(!report.ok);
    assert_eq!(report.target.kind, "import");
    assert_eq!(report.target.handle, Some(import));
    assert_reason(&report, "node_hidden");
    assert_fix(&report, "set_visible");
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

fn diagnostic_scene_at(translation: Vec3) -> (Assets, Scene, Renderer, scena::NodeKey) {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    scene.add_default_camera().expect("camera inserts");
    let node = scene
        .mesh(geometry, material)
        .transform(scena::Transform::at(translation))
        .add()
        .expect("mesh node inserts");
    let renderer = Renderer::headless(64, 64).expect("renderer builds");
    (assets, scene, renderer, node)
}

fn mesh_handle(report: &scena::SceneInspectionReportV1) -> u64 {
    report
        .draw_list
        .first()
        .expect("drawable appears in inspection")
        .node
}

fn node_handle_for_kind(report: &scena::SceneInspectionReportV1, kind: &str) -> u64 {
    report
        .nodes
        .iter()
        .find(|node| node.kind.eq_ignore_ascii_case(kind))
        .unwrap_or_else(|| panic!("expected node kind {kind} in {report:#?}"))
        .handle
}

fn node_handle_for_material_alpha(report: &scena::SceneInspectionReportV1, alpha: f32) -> u64 {
    report
        .draw_list
        .iter()
        .find(|draw| {
            draw.material
                .as_ref()
                .is_some_and(|material| (material.base_color.a - alpha).abs() <= 1.0e-6)
        })
        .unwrap_or_else(|| panic!("expected material alpha {alpha} in {report:#?}"))
        .node
}

fn node_handle_for_alpha_mode(report: &scena::SceneInspectionReportV1, alpha_mode: &str) -> u64 {
    report
        .draw_list
        .iter()
        .find(|draw| {
            draw.material
                .as_ref()
                .is_some_and(|material| material.alpha_mode == alpha_mode)
        })
        .unwrap_or_else(|| panic!("expected alpha mode {alpha_mode} in {report:#?}"))
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
    let fix = report
        .fixes
        .iter()
        .find(|fix| fix.action == action)
        .unwrap_or_else(|| panic!("expected fix {action} in {:#?}", report.fixes));
    if action == "frame_bounds" {
        assert!(
            fix.patch.is_some(),
            "frame_bounds fixes must carry a visual_patch camera payload: {fix:#?}"
        );
    }
}

fn assert_reason_code(reasons: &[scena::RenderIntrospectionReasonV1], code: &str) {
    assert!(
        reasons.iter().any(|reason| reason.code == code),
        "expected reason {code} in {reasons:#?}"
    );
}

#[allow(dead_code)]
fn _keep_vec3_in_public_test_surface(_: Vec3) {}
