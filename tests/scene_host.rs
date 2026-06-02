#![cfg(all(feature = "scene-host", not(target_arch = "wasm32")))]

use scena::{
    ASSET_LOAD_REPORT_SCHEMA_V1, AnnotationProjectionReportV1, AssetPath, Assets, Color,
    GeometryDesc, ImportOptions, MaterialDesc, OrbitControlAction, PointerButton,
    SCENE_HOST_ASSET_IMPORT_SCHEMA_V1, SceneHostCameraState, SceneHostCore, SceneHostErrorCode,
    SceneInspectionReportV1, Transform, Vec3,
};

#[test]
fn scene_instantiate_under_parents_import_roots_under_requested_node() {
    let assets = Assets::new();
    let scene_asset = pollster::block_on(
        assets.load_scene("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
    )
    .expect("mesh glTF loads");
    let mut scene = scena::Scene::new();
    let parent = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(1.0, 0.0, 0.0)))
        .expect("parent inserts");

    let import = scene
        .instantiate_under(parent, &scene_asset, ImportOptions::gltf_default())
        .expect("scene instantiates under parent");
    let imported = import
        .node("ColoredTriangle")
        .expect("imported node remains queryable");

    assert_eq!(
        scene.node(imported).expect("imported node exists").parent(),
        Some(parent)
    );
    assert_eq!(
        scene
            .world_transform(imported)
            .expect("imported node has world transform")
            .translation,
        Vec3::new(1.0, 0.0, 0.0)
    );
}

#[test]
fn scene_set_transforms_batches_changed_nodes_into_one_revision_bump() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = scena::Scene::new();
    let a = scene
        .mesh(geometry, material)
        .add()
        .expect("first mesh inserts");
    let b = scene
        .mesh(geometry, material)
        .add()
        .expect("second mesh inserts");
    let before = scene.dirty_state().transform_revision;

    scene
        .set_transforms(&[
            (a, Transform::at(Vec3::new(1.0, 0.0, 0.0))),
            (b, Transform::at(Vec3::new(0.0, 2.0, 0.0))),
        ])
        .expect("batch transform update succeeds");

    assert_eq!(scene.dirty_state().transform_revision, before + 1);
    assert_eq!(
        scene
            .node(a)
            .expect("first mesh exists")
            .transform()
            .translation,
        Vec3::new(1.0, 0.0, 0.0)
    );
    assert_eq!(
        scene
            .node(b)
            .expect("second mesh exists")
            .transform()
            .translation,
        Vec3::new(0.0, 2.0, 0.0)
    );
}

#[test]
fn scene_host_core_constructs_poses_inspects_picks_and_frames_with_one_handle_namespace() {
    let mut host = SceneHostCore::headless(128, 128).expect("host builds");
    let root = host.root_handle();
    let left_frame = host
        .add_empty(
            Some(root),
            Transform::at(Vec3::new(-0.75, 0.0, 0.0)),
            Some("frame:left"),
        )
        .expect("left frame inserts");
    let right_frame = host
        .add_empty(
            Some(root),
            Transform::at(Vec3::new(0.75, 0.0, 0.0)),
            Some("frame:right"),
        )
        .expect("right frame inserts");
    host.set_tag(left_frame, "part-frame")
        .expect("left tag inserts");
    assert!(host.find_by_tag("part-frame").contains(&left_frame));
    host.clear_tag(left_frame, "part-frame")
        .expect("left tag removes");
    assert!(!host.find_by_tag("part-frame").contains(&left_frame));

    let left_import = pollster::block_on(host.instantiate_url_under(
        left_frame,
        AssetPath::from("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
    ))
    .expect("left asset instantiates");
    let right_import = pollster::block_on(host.instantiate_url_under(
        right_frame,
        AssetPath::from("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
    ))
    .expect("right asset instantiates");
    let left_mesh = host
        .node_handle(left_import, "ColoredTriangle")
        .expect("left import path resolves");
    let right_mesh = host
        .node_handle(right_import, "ColoredTriangle")
        .expect("right import path resolves");
    assert!(
        host.import_roots(left_import)
            .expect("left import roots resolve")
            .contains(&left_mesh)
    );

    host.set_transforms(&[
        (left_mesh, Transform::at(Vec3::new(0.0, 1.5, 0.0))),
        (right_mesh, Transform::at(Vec3::new(0.0, -1.5, 0.0))),
    ])
    .expect("batch pose updates");

    let report: SceneInspectionReportV1 =
        serde_json::from_str(&host.inspect_json().expect("inspection serializes"))
            .expect("inspection json decodes");
    let posed = report
        .node_by_handle(left_mesh)
        .expect("posed host handle appears in inspection");
    assert_eq!(posed.handle, left_mesh);
    assert_eq!(posed.parent, Some(left_frame));
    assert_eq!(posed.local_transform.translation, Vec3::new(0.0, 1.5, 0.0));
    assert_eq!(
        posed.world_transform.translation,
        Vec3::new(-0.75, 1.5, 0.0)
    );
    assert!(
        report.draw_list.iter().any(|draw| draw.node == left_mesh),
        "draw list must use the same host node handle namespace"
    );
    assert_eq!(
        host.node_handle_from_inspection(left_mesh)
            .expect("inspection handle validates"),
        left_mesh
    );

    host.frame_node(left_mesh).expect("host frames posed node");
    host.prepare().expect("host prepares");
    host.render().expect("host renders");
    let pixels = host.read_pixels();
    assert_eq!(pixels.len(), 128 * 128 * 4);
    assert_eq!(
        host.pick(64.0, 64.0).expect("css-pixel pick runs"),
        Some(left_mesh)
    );
    host.frame_all().expect("host frames all nodes");
}

#[test]
fn scene_host_core_instantiates_glb_bytes_under_host_frame() {
    let mut host = SceneHostCore::headless(64, 64).expect("host builds");
    let frame = host
        .add_empty(
            None,
            Transform::at(Vec3::new(2.0, 0.0, 0.0)),
            Some("glb-frame"),
        )
        .expect("frame inserts");
    let bytes = std::fs::read("tests/assets/gltf/load_unit.glb").expect("fixture bytes load");

    let import = pollster::block_on(host.instantiate_glb_under(frame, bytes.as_slice()))
        .expect("glb bytes instantiate under frame");
    let roots = host.import_roots(import).expect("import roots resolve");

    assert!(!roots.is_empty());
    for root in roots {
        let report: SceneInspectionReportV1 =
            serde_json::from_str(&host.inspect_json().expect("inspection serializes"))
                .expect("inspection json decodes");
        assert!(matches!(
            report.node_by_handle(root).map(|node| node.parent),
            Some(Some(parent)) if parent == frame
        ));
    }
}

#[test]
fn scene_host_url_instantiation_returns_asset_load_report_json() {
    let mut host = SceneHostCore::headless(64, 64).expect("host builds");

    let json = pollster::block_on(host.instantiate_url_with_report_json(AssetPath::from(
        "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
    )))
    .expect("asset instantiates and reports load metadata");
    let value: serde_json::Value = serde_json::from_str(&json).expect("host import report is JSON");

    assert_eq!(value["schema"], SCENE_HOST_ASSET_IMPORT_SCHEMA_V1);
    assert_eq!(
        value["asset_load_report"]["schema"],
        ASSET_LOAD_REPORT_SCHEMA_V1
    );
    assert_eq!(value["asset_load_report"]["cache_hit"], false);
    assert_eq!(value["asset_load_report"]["geometry"]["primitive_count"], 1);
    let import = value["import"].as_u64().expect("import handle is u64");
    assert!(
        !host
            .import_roots(import)
            .expect("reported import handle resolves")
            .is_empty()
    );
}

#[test]
fn scene_host_core_rejects_missing_and_stale_handles_with_structured_errors() {
    let mut host = SceneHostCore::headless(64, 64).expect("host builds");
    let root = host.root_handle();
    let stale_root = root + (1_u64 << 32);
    let missing_slot = (1_u64 << 32) + 65_535;

    let stale_transform = host
        .set_transform(stale_root, Transform::IDENTITY)
        .expect_err("generation mismatch is stale");
    assert_eq!(stale_transform.code(), SceneHostErrorCode::StaleNodeHandle);

    let missing_transform = host
        .set_transform(missing_slot, Transform::IDENTITY)
        .expect_err("missing slot is not found");
    assert_eq!(
        missing_transform.code(),
        SceneHostErrorCode::NodeHandleNotFound
    );

    let stale_inspection = host
        .node_handle_from_inspection(stale_root)
        .expect_err("inspection handle validates host table generation");
    assert_eq!(stale_inspection.code(), SceneHostErrorCode::StaleNodeHandle);

    let missing_import = host
        .node_handle(root, "anything")
        .expect_err("node handle cannot be used as import handle");
    assert_eq!(
        missing_import.code(),
        SceneHostErrorCode::ImportHandleNotFound
    );
}

#[test]
fn scene_host_remove_node_invalidates_removed_subtree_handles() {
    let mut host = SceneHostCore::headless(64, 64).expect("host builds");
    let root = host.root_handle();
    let parent = host
        .add_empty(Some(root), Transform::IDENTITY, Some("remove-parent"))
        .expect("parent inserts");
    let child = host
        .add_empty(Some(parent), Transform::IDENTITY, Some("remove-child"))
        .expect("child inserts");

    host.remove_node(parent).expect("subtree removes");

    let parent_error = host
        .set_transform(parent, Transform::IDENTITY)
        .expect_err("removed parent handle is stale");
    assert_eq!(parent_error.code(), SceneHostErrorCode::StaleNodeHandle);
    let child_error = host
        .set_transform(child, Transform::IDENTITY)
        .expect_err("removed child handle is stale");
    assert_eq!(child_error.code(), SceneHostErrorCode::StaleNodeHandle);
    assert!(!host.find_by_tag("remove-parent").contains(&parent));
    assert!(!host.find_by_tag("remove-child").contains(&child));
    assert!(
        host.set_transform(root, Transform::IDENTITY).is_ok(),
        "unremoved root handle remains live"
    );
}

#[test]
fn scene_host_remove_import_invalidates_import_and_node_handles() {
    let mut host = SceneHostCore::headless(64, 64).expect("host builds");
    let import = pollster::block_on(host.instantiate_url(AssetPath::from(
        "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
    )))
    .expect("asset instantiates");
    let mesh = host
        .node_handle(import, "ColoredTriangle")
        .expect("import node resolves");

    host.remove_import(import).expect("import removes");

    let import_error = host
        .import_roots(import)
        .expect_err("removed import handle is stale");
    assert_eq!(import_error.code(), SceneHostErrorCode::StaleImportHandle);
    let node_error = host
        .set_transform(mesh, Transform::IDENTITY)
        .expect_err("removed import node handle is stale");
    assert_eq!(node_error.code(), SceneHostErrorCode::StaleNodeHandle);
}

#[test]
fn scene_host_set_node_tint_appears_in_inspection_and_clears() {
    let mut host = SceneHostCore::headless(64, 64).expect("host builds");
    let import = pollster::block_on(host.instantiate_url(AssetPath::from(
        "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
    )))
    .expect("asset instantiates");
    let mesh = host
        .node_handle(import, "ColoredTriangle")
        .expect("import node resolves");
    let tint = Color::from_linear_rgba(0.0, 1.0, 0.0, 0.5);

    host.set_node_tint(mesh, Some(tint))
        .expect("host tint sets");

    let report: SceneInspectionReportV1 =
        serde_json::from_str(&host.inspect_json().expect("inspection serializes"))
            .expect("inspection json decodes");
    assert_eq!(
        report
            .node_by_handle(mesh)
            .expect("mesh appears in inspection")
            .tint,
        Some(tint)
    );

    host.set_node_tint(mesh, None).expect("host tint clears");
    let report: SceneInspectionReportV1 =
        serde_json::from_str(&host.inspect_json().expect("inspection serializes"))
            .expect("inspection json decodes");
    assert_eq!(
        report
            .node_by_handle(mesh)
            .expect("mesh appears in inspection")
            .tint,
        None
    );
}

#[test]
fn scene_host_annotations_bounds_and_distance_use_host_handles() {
    let mut host = SceneHostCore::headless(120, 80).expect("host builds");
    let root = host.root_handle();
    let left = host
        .add_empty(
            Some(root),
            Transform::at(Vec3::new(-0.5, 0.0, 0.0)),
            Some("left"),
        )
        .expect("left frame inserts");
    let right = host
        .add_empty(
            Some(root),
            Transform::at(Vec3::new(0.5, 0.0, 0.0)),
            Some("right"),
        )
        .expect("right frame inserts");
    let import = pollster::block_on(host.instantiate_url_under(
        left,
        AssetPath::from("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
    ))
    .expect("asset instantiates under left frame");
    let mesh = host
        .node_handle(import, "ColoredTriangle")
        .expect("mesh resolves");

    host.set_node_annotation("left-label", left, [0.0, 0.0, 0.0])
        .expect("node annotation sets");
    host.set_world_annotation("origin-label", [0.0, 0.0, 0.0])
        .expect("world annotation sets");

    let projections: AnnotationProjectionReportV1 = serde_json::from_str(
        &host
            .annotation_projections_json()
            .expect("projections serialize"),
    )
    .expect("projection report decodes");
    let left_projection = projections
        .annotations
        .iter()
        .find(|projection| projection.id == "left-label")
        .expect("left annotation appears");
    let origin_projection = projections
        .annotations
        .iter()
        .find(|projection| projection.id == "origin-label")
        .expect("origin annotation appears");
    assert_eq!(left_projection.node_handle, Some(left));
    assert_eq!(origin_projection.node_handle, None);
    assert!(left_projection.visible);
    assert!(origin_projection.visible);
    assert!(
        left_projection.x < origin_projection.x,
        "CSS-pixel projection should reflect the host node transform"
    );
    assert_eq!(
        host.world_distance(left, right)
            .expect("host distance computes"),
        1.0
    );
    assert!(
        host.node_world_bounds(mesh)
            .expect("host node bounds computes")
            .is_some()
    );
    assert!(
        host.node_world_bounds_json(mesh)
            .expect("host node bounds serializes")
            .contains("\"min\"")
    );

    assert!(host.clear_annotation("origin-label"));
    let projections: AnnotationProjectionReportV1 = serde_json::from_str(
        &host
            .annotation_projections_json()
            .expect("projections serialize"),
    )
    .expect("projection report decodes");
    assert!(
        projections
            .annotations
            .iter()
            .all(|projection| projection.id != "origin-label")
    );
}

#[test]
fn scene_host_camera_viewpoint_round_trips_and_rejects_invalid_state() {
    let mut host = SceneHostCore::headless(128, 128).expect("host builds");
    let viewpoint = SceneHostCameraState {
        target: Vec3::new(1.0, 2.0, 3.0),
        distance: 6.0,
        yaw_radians: 0.35,
        pitch_radians: -0.2,
    };

    host.set_camera(viewpoint)
        .expect("scripted camera viewpoint applies");

    let actual = host.camera_state();
    assert_eq!(actual, viewpoint);
    let json: SceneHostCameraState =
        serde_json::from_str(&host.camera_json().expect("camera serializes"))
            .expect("camera JSON decodes");
    assert_eq!(json, viewpoint);

    let camera = host
        .scene()
        .active_camera()
        .expect("host has an active camera");
    let camera_node = host
        .scene()
        .camera_node(camera)
        .expect("camera node exists");
    let camera_transform = host
        .scene()
        .world_transform(camera_node)
        .expect("camera has a world transform");
    assert!(
        !camera_transform
            .translation
            .abs_diff_eq(Vec3::ZERO, f32::EPSILON),
        "set_camera must apply the saved viewpoint to the scene camera"
    );

    let invalid = SceneHostCameraState {
        distance: 0.0,
        ..viewpoint
    };
    let error = host
        .set_camera(invalid)
        .expect_err("non-positive distance is rejected");
    assert_eq!(error.code(), SceneHostErrorCode::InvalidInput);
}

#[test]
fn scene_host_camera_pointer_and_wheel_inputs_use_orbit_controls_without_rendering() {
    let mut host = SceneHostCore::headless(128, 128).expect("host builds");
    host.set_camera(SceneHostCameraState {
        target: Vec3::ZERO,
        distance: 4.0,
        yaw_radians: 0.0,
        pitch_radians: 0.0,
    })
    .expect("initial camera viewpoint applies");
    let initial = host.camera_state();
    let before_input_revision = host.scene().dirty_state().transform_revision;

    assert_eq!(
        host.camera_pointer_down(32.0, 32.0, PointerButton::Primary)
            .expect("primary pointer starts orbit"),
        OrbitControlAction::BeginOrbit
    );
    assert_eq!(
        host.scene().dirty_state().transform_revision,
        before_input_revision,
        "pointer down records input state but does not render or mutate the camera"
    );

    assert_eq!(
        host.camera_pointer_move(52.0, 44.0, 20.0, 12.0)
            .expect("primary drag orbits"),
        OrbitControlAction::Orbit
    );
    let orbit_state = host.camera_state();
    assert!(orbit_state.yaw_radians > initial.yaw_radians);
    assert!(orbit_state.pitch_radians > initial.pitch_radians);
    assert_eq!(
        host.camera_pointer_up(52.0, 44.0)
            .expect("pointer release ends input"),
        OrbitControlAction::End
    );

    let distance_before_wheel = host.camera_state().distance;
    assert_eq!(
        host.camera_wheel(52.0, 44.0, -1.0)
            .expect("wheel dolly applies"),
        OrbitControlAction::Zoom
    );
    assert!(host.camera_state().distance < distance_before_wheel);

    let target_before_pan = host.camera_state().target;
    assert_eq!(
        host.camera_pointer_down(52.0, 44.0, PointerButton::Secondary)
            .expect("secondary pointer starts pan"),
        OrbitControlAction::Pan
    );
    assert_eq!(
        host.camera_pointer_move(62.0, 36.0, 10.0, -8.0)
            .expect("secondary drag pans"),
        OrbitControlAction::Pan
    );
    assert_ne!(host.camera_state().target, target_before_pan);
}
