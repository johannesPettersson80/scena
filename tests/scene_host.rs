#![cfg(all(feature = "scene-host", not(target_arch = "wasm32")))]

use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, FRAC_PI_6};

use scena::{
    ASSET_LOAD_REPORT_SCHEMA_V1, AnnotationProjectionReportV1, AntiAliasing, AssetPath, Assets,
    AutoExposureConfig, Color, GeometryDesc, ImportOptions, MaterialDesc, OrbitControlAction,
    PointerButton, PostBloomConfig, SCENE_HOST_ASSET_IMPORT_SCHEMA_V1,
    SCENE_HOST_SUBTREE_SCHEMA_V1, SceneHostAnimationInventoryV1, SceneHostAnimationLoopMode,
    SceneHostAnimationPlayOptions, SceneHostCameraState, SceneHostCore, SceneHostEasing,
    SceneHostErrorCode, SceneHostSubtreeReportV1, SceneInspectionReportV1,
    ScreenSpaceAmbientOcclusionConfig, Transform, Vec3,
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
fn scene_host_product_studio_visuals_apply_renderable_defaults() {
    let mut host = SceneHostCore::headless(128, 128).expect("host builds");
    assert_eq!(host.renderer().environment(), None);
    let root = host.root_handle();

    host.apply_product_studio_visuals("studio_neutral")
        .expect("product studio visuals apply");
    assert_eq!(
        host.root_handle(),
        root,
        "applying the product studio preset must not replace the SceneHost root handle"
    );
    host.add_empty(
        Some(root),
        Transform::IDENTITY,
        Some("after-studio-root-child"),
    )
    .expect("registered root handle remains usable after applying studio visuals");

    assert!(host.renderer().environment().is_some());
    assert_eq!(host.renderer().background_color(), Color::CHARCOAL);
    assert_eq!(
        host.renderer().auto_exposure(),
        Some(AutoExposureConfig::product_studio())
    );
    assert_eq!(host.renderer().anti_aliasing(), AntiAliasing::Fxaa);
    assert_eq!(host.renderer().bloom(), Some(PostBloomConfig::subtle()));
    assert_eq!(
        host.renderer().screen_space_ambient_occlusion(),
        Some(ScreenSpaceAmbientOcclusionConfig::subtle())
    );
    let report = host.scene().inspect();
    assert_eq!(
        report.light_count(),
        3,
        "SceneHost product studio preset must insert the standard Scena three-point rig"
    );
}

#[test]
fn scene_host_post_processing_setters_update_capability_report() {
    let mut host = SceneHostCore::headless(64, 64).expect("host builds");

    host.set_anti_aliasing("none")
        .expect("anti-aliasing mode sets");
    host.set_bloom_json(Some(
        r#"{"threshold_srgb":128,"intensity":0.4,"radius_px":2}"#,
    ))
    .expect("bloom JSON sets");
    host.set_ambient_occlusion_json(Some(
        r#"{"radius_px":2,"intensity":0.35,"depth_threshold":0.02}"#,
    ))
    .expect("ambient occlusion JSON sets");

    assert_eq!(host.renderer().anti_aliasing(), AntiAliasing::None);
    assert_eq!(
        host.renderer().bloom(),
        Some(PostBloomConfig::new(128, 0.4, 2))
    );
    assert_eq!(
        host.renderer().screen_space_ambient_occlusion(),
        Some(ScreenSpaceAmbientOcclusionConfig::new(2, 0.35, 0.02))
    );
    let report: serde_json::Value =
        serde_json::from_str(&host.capabilities_json().expect("capabilities serialize"))
            .expect("capabilities JSON parses");
    assert_eq!(report["post_processing"]["bloom"], true);
    assert_eq!(
        report["post_processing"]["screen_space_ambient_occlusion"],
        true
    );
    assert_eq!(
        report["post_processing"]["ssao_depth_source"],
        "cpu_depth_frame"
    );

    host.set_bloom_json(None).expect("bloom clears");
    assert_eq!(host.renderer().bloom(), None);
}

#[test]
fn scene_host_product_studio_visuals_reject_unknown_background() {
    let mut host = SceneHostCore::headless(128, 128).expect("host builds");

    let error = host
        .apply_product_studio_visuals("mystery_background")
        .expect_err("unknown background names must fail closed");

    assert_eq!(error.code(), SceneHostErrorCode::InvalidInput);
    assert!(
        error
            .to_string()
            .contains("unsupported SceneHost product studio background mystery_background")
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
    let mut host = SceneHostCore::headless(320, 240).expect("host builds");
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
    let grid_floor = host
        .add_product_grid_floor_under_node(frame)
        .expect("host adds Scena grid floor under imported subtree");
    assert_eq!(grid_floor.len(), 2);
    let floor_bounds = host
        .node_world_bounds(grid_floor[0])
        .expect("grid floor handle resolves")
        .expect("grid floor has bounds");
    assert!(
        floor_bounds.max.x - floor_bounds.min.x > 0.0,
        "grid floor must produce non-empty world bounds"
    );
    for root in roots {
        let report: SceneInspectionReportV1 =
            serde_json::from_str(&host.inspect_json().expect("inspection serializes"))
                .expect("inspection json decodes");
        assert!(matches!(
            report.node_by_handle(root).map(|node| node.parent),
            Some(Some(parent)) if parent == frame
        ));
    }
    host.frame_node_product_view(frame)
        .expect("host frames GLB subtree with product-view preset");
    let product_camera = host.get_camera();
    assert!(
        (product_camera.yaw_radians - FRAC_PI_4).abs() < 1.0e-6,
        "product-view SceneHost framing must use Scena's three-quarter front-right yaw"
    );
    assert!(
        (product_camera.pitch_radians - FRAC_PI_6).abs() < 1.0e-6,
        "product-view SceneHost framing must use Scena's three-quarter front-right elevation"
    );
    host.frame_node_with_preset(frame, "operator_review_default")
        .expect("host frames GLB subtree with operator-review preset");
    let operator_camera = host.get_camera();
    assert!(
        (operator_camera.yaw_radians - 35.0_f32.to_radians()).abs() < 1.0e-6,
        "operator-review SceneHost framing must use the standing-eye-level yaw"
    );
    assert!(
        (operator_camera.pitch_radians - 14.0_f32.to_radians()).abs() < 1.0e-6,
        "operator-review SceneHost framing must use the standing-eye-level pitch"
    );
    assert_ne!(
        operator_camera, product_camera,
        "operator_review_default must not alias product_viewer_default"
    );
    host.frame_node_with_preset(frame, "cell_overview")
        .expect("host frames GLB subtree with named cell-overview preset");
    let overview_camera = host.get_camera();
    assert!(
        overview_camera.pitch_radians > FRAC_PI_2 - 0.03,
        "cell-overview SceneHost framing must use Scena's named top view, got pitch {}",
        overview_camera.pitch_radians
    );
    assert_ne!(overview_camera, product_camera);
    assert_ne!(overview_camera, operator_camera);
    let error = host
        .frame_node_with_preset(frame, "mystery_preset")
        .expect_err("unknown camera preset must fail closed");
    assert_eq!(error.code(), SceneHostErrorCode::InvalidInput);
    assert!(
        error
            .to_string()
            .contains("unsupported SceneHost camera preset mystery_preset")
    );
}

#[test]
fn scene_host_instanced_url_routes_root_handle_mutations_and_inspection() {
    let mut host = SceneHostCore::headless(128, 128).expect("host builds");
    let roots = pollster::block_on(host.instantiate_url_instanced(
        AssetPath::from("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
        3,
    ))
    .expect("instanced asset creates host roots");

    assert_eq!(roots.len(), 3);
    assert!(roots.iter().all(|handle| *handle != host.root_handle()));

    host.set_transforms(&[
        (roots[0], Transform::at(Vec3::new(-0.4, 0.0, 0.0))),
        (roots[1], Transform::at(Vec3::ZERO)),
        (roots[2], Transform::at(Vec3::new(0.4, 0.0, 0.0))),
    ])
    .expect("instance root transform batch applies");
    host.set_visible(roots[1], false)
        .expect("middle instance root hides");
    let translucent_error = host
        .set_node_tint(roots[0], Some(Color::from_linear_rgba(1.0, 0.0, 0.0, 0.5)))
        .expect_err("translucent per-instance tint is rejected");
    assert_eq!(translucent_error.code(), SceneHostErrorCode::InvalidInput);
    host.set_node_tint(roots[2], Some(Color::from_linear_rgba(0.0, 1.0, 0.0, 1.0)))
        .expect("opaque instance tint applies");

    let parent_error = host
        .add_empty(Some(roots[0]), Transform::IDENTITY, Some("bad-parent"))
        .expect_err("instance roots are not scene-graph parents");
    assert!(matches!(
        parent_error.code(),
        SceneHostErrorCode::NodeHandleNotFound | SceneHostErrorCode::StaleNodeHandle
    ));

    let report: serde_json::Value =
        serde_json::from_str(&host.inspect_json().expect("inspection serializes"))
            .expect("inspection decodes");
    let instance_sets = report
        .get("instance_sets")
        .and_then(|value| value.as_array())
        .expect("inspection exposes additive instance_sets field");
    for root in &roots {
        let binding = instance_sets
            .iter()
            .find(|binding| binding["root_handle"].as_u64() == Some(*root))
            .expect("instance root appears in inspection");
        assert!(
            binding["entries"]
                .as_array()
                .expect("entries are an array")
                .iter()
                .all(|entry| entry["set_node"].as_u64().is_some()
                    && entry["instance_id"].as_u64().is_some()),
            "each instance entry must expose the set node handle and source instance id"
        );
    }
    let hidden = instance_sets
        .iter()
        .find(|binding| binding["root_handle"].as_u64() == Some(roots[1]))
        .expect("hidden middle root appears");
    assert_eq!(hidden["visible"].as_bool(), Some(false));
    assert!(
        instance_sets
            .iter()
            .find(|binding| binding["root_handle"].as_u64() == Some(roots[2]))
            .and_then(|binding| binding["tint"].as_object())
            .is_some(),
        "opaque instance tint must be visible in inspection"
    );

    host.remove_node(roots[1])
        .expect("removing an instance root removes only its instance entries");
    let report: serde_json::Value =
        serde_json::from_str(&host.inspect_json().expect("inspection serializes"))
            .expect("inspection decodes");
    let instance_sets = report
        .get("instance_sets")
        .and_then(|value| value.as_array())
        .expect("inspection exposes additive instance_sets field");
    assert!(
        instance_sets
            .iter()
            .all(|binding| binding["root_handle"].as_u64() != Some(roots[1])),
        "removed instance root handle must leave the host binding table"
    );
}

#[test]
fn scene_host_pick_resolves_instanced_drawable_to_instance_root_handle() {
    let mut host = SceneHostCore::headless(128, 128).expect("host builds");
    let roots = pollster::block_on(host.instantiate_url_instanced(
        AssetPath::from("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
        1,
    ))
    .expect("instanced asset creates host roots");

    host.frame_all().expect("instance scene frames");
    host.prepare().expect("instance scene prepares");
    host.render().expect("instance scene renders");

    assert_eq!(
        host.pick(64.0, 64.0).expect("instance pick runs"),
        Some(roots[0]),
        "picking an instanced drawable must return the SceneHost instance-root handle"
    );
}

#[test]
fn scene_host_transform_components_validate_and_batch_atomically() {
    let mut host = SceneHostCore::headless(64, 64).expect("host builds");
    let root = host.root_handle();
    let first = host
        .add_empty(Some(root), Transform::IDENTITY, Some("first"))
        .expect("first node inserts");
    let second = host
        .add_empty(
            Some(root),
            Transform::at(Vec3::new(0.0, 2.0, 0.0)),
            Some("second"),
        )
        .expect("second node inserts");

    host.set_transform_components(
        first,
        [1.0, 2.0, 3.0],
        [0.0, 0.0, 0.0, 1.001],
        [1.0, 0.0, -1.0],
    )
    .expect("near-unit quaternion is accepted and normalized");
    let first_report: SceneInspectionReportV1 =
        serde_json::from_str(&host.inspect_json().expect("inspection serializes"))
            .expect("inspection decodes");
    let normalized = first_report
        .node_by_handle(first)
        .expect("first node appears")
        .local_transform;
    assert!((normalized.rotation.length() - 1.0).abs() < 1.0e-6);
    assert_eq!(normalized.scale, Vec3::new(1.0, 0.0, -1.0));

    let before_invalid_single = normalized;
    let error = host
        .set_transform_components(
            first,
            [f32::NAN, 0.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
        )
        .expect_err("nonfinite translation is rejected");
    assert_eq!(error.code(), SceneHostErrorCode::InvalidInput);
    let report: SceneInspectionReportV1 =
        serde_json::from_str(&host.inspect_json().expect("inspection serializes"))
            .expect("inspection decodes");
    assert_eq!(
        report
            .node_by_handle(first)
            .expect("first node appears")
            .local_transform,
        before_invalid_single,
        "invalid single transform input must not mutate the node"
    );

    let before_batch = report.clone();
    let error = host
        .set_transforms_components(&[
            (first, [5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0]),
            (
                second,
                [0.0, 6.0, 0.0, 0.0, f32::INFINITY, 0.0, 1.0, 1.0, 1.0, 1.0],
            ),
        ])
        .expect_err("nonfinite rotation is rejected before batch mutation");
    assert_eq!(error.code(), SceneHostErrorCode::InvalidInput);
    let after_invalid_batch: SceneInspectionReportV1 =
        serde_json::from_str(&host.inspect_json().expect("inspection serializes"))
            .expect("inspection decodes");
    assert_eq!(
        after_invalid_batch.nodes, before_batch.nodes,
        "invalid batched typed transforms must not partially mutate the scene"
    );

    host.set_transforms_components(&[
        (first, [5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0]),
        (second, [0.0, 6.0, 0.0, 0.0, 0.0, 0.0, 0.999, 1.0, 2.0, 1.0]),
    ])
    .expect("valid typed transform batch applies");
    let report: SceneInspectionReportV1 =
        serde_json::from_str(&host.inspect_json().expect("inspection serializes"))
            .expect("inspection decodes");
    assert_eq!(
        report
            .node_by_handle(first)
            .expect("first node appears")
            .local_transform
            .translation,
        Vec3::new(5.0, 0.0, 0.0)
    );
    let second_rotation = report
        .node_by_handle(second)
        .expect("second node appears")
        .local_transform
        .rotation;
    assert!((second_rotation.length() - 1.0).abs() < 1.0e-6);

    let error = host
        .set_transform_components(
            first,
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.5],
            [1.0, 1.0, 1.0],
        )
        .expect_err("non-unit quaternion is rejected");
    assert_eq!(error.code(), SceneHostErrorCode::InvalidInput);
}

#[test]
fn scene_host_visibility_hides_subtrees_from_rendering_picking_and_inspection() {
    let mut host = SceneHostCore::headless(128, 128).expect("host builds");
    let root = host.root_handle();
    let frame = host
        .add_empty(Some(root), Transform::IDENTITY, Some("visibility-frame"))
        .expect("frame inserts");
    let import = pollster::block_on(host.instantiate_url_under(
        frame,
        AssetPath::from("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
    ))
    .expect("asset instantiates");
    let mesh = host
        .node_handle(import, "ColoredTriangle")
        .expect("mesh resolves");

    host.frame_node(mesh).expect("mesh frames");
    host.prepare().expect("visible scene prepares");
    host.render().expect("visible scene renders");
    assert_eq!(
        host.pick(64.0, 64.0).expect("visible pick runs"),
        Some(mesh),
        "visible mesh should be pickable before hiding its parent"
    );

    host.set_visible(frame, false).expect("frame hides");
    host.prepare().expect("hidden scene prepares");
    host.render().expect("hidden scene renders");
    assert_eq!(
        host.pick(64.0, 64.0).expect("hidden pick runs"),
        None,
        "hidden subtree must not be pickable"
    );
    let report: SceneInspectionReportV1 =
        serde_json::from_str(&host.inspect_json().expect("inspection serializes"))
            .expect("inspection decodes");
    assert!(!report.node_by_handle(frame).expect("frame appears").visible);
    assert!(
        report.draw_list.iter().all(|draw| draw.node != mesh),
        "hidden subtree must not appear in the draw list"
    );
}

#[test]
fn scene_host_subtree_query_is_stable_and_batch_tint_respects_exclusions() {
    let mut host = SceneHostCore::headless(64, 64).expect("host builds");
    let root = host.root_handle();
    let parent = host
        .add_empty(Some(root), Transform::IDENTITY, Some("parent"))
        .expect("parent inserts");
    let first = host
        .add_empty(Some(parent), Transform::IDENTITY, Some("first"))
        .expect("first child inserts");
    host.set_tag(first, "shared").expect("shared tag sets");
    let grandchild = host
        .add_empty(Some(first), Transform::IDENTITY, Some("grandchild"))
        .expect("grandchild inserts");
    let excluded = host
        .add_empty(Some(parent), Transform::IDENTITY, Some("excluded"))
        .expect("excluded child inserts");
    let excluded_child = host
        .add_empty(Some(excluded), Transform::IDENTITY, Some("excluded-child"))
        .expect("excluded grandchild inserts");

    let subtree: SceneHostSubtreeReportV1 =
        serde_json::from_str(&host.subtree_nodes_json(parent).expect("subtree serializes"))
            .expect("subtree report decodes");
    assert_eq!(subtree.schema, SCENE_HOST_SUBTREE_SCHEMA_V1);
    assert_eq!(
        subtree
            .nodes
            .iter()
            .map(|node| node.handle)
            .collect::<Vec<_>>(),
        vec![parent, first, grandchild, excluded, excluded_child],
        "subtree order must be deterministic pre-order"
    );
    assert_eq!(
        subtree
            .nodes
            .iter()
            .find(|node| node.handle == first)
            .expect("first node appears")
            .tags,
        vec!["first".to_owned(), "shared".to_owned()]
    );
    assert!(subtree.nodes.iter().all(|node| node.name.is_none()));

    let tint = Color::from_linear_rgba(0.1, 0.2, 0.3, 1.0);
    host.set_subtree_tint(parent, Some(tint), &[excluded])
        .expect("subtree tint applies");
    let report: SceneInspectionReportV1 =
        serde_json::from_str(&host.inspect_json().expect("inspection serializes"))
            .expect("inspection decodes");
    for handle in [parent, first, grandchild] {
        assert_eq!(
            report.node_by_handle(handle).expect("node appears").tint,
            Some(tint),
            "included subtree node {handle} receives tint"
        );
    }
    for handle in [excluded, excluded_child] {
        assert_eq!(
            report.node_by_handle(handle).expect("node appears").tint,
            None,
            "excluded node {handle} and its subtree keep existing tint"
        );
    }

    host.set_subtree_tint(parent, None, &[excluded])
        .expect("subtree tint clears");
    let report: SceneInspectionReportV1 =
        serde_json::from_str(&host.inspect_json().expect("inspection serializes"))
            .expect("inspection decodes");
    assert!(report.nodes.iter().all(|node| node.tint.is_none()));
}

#[test]
fn scene_host_subtree_tint_cancels_active_tint_transitions_for_touched_nodes() {
    let mut host = SceneHostCore::headless(64, 64).expect("host builds");
    let root = host.root_handle();
    let parent = host
        .add_empty(Some(root), Transform::IDENTITY, Some("parent"))
        .expect("parent inserts");
    let child = host
        .add_empty(Some(parent), Transform::IDENTITY, Some("child"))
        .expect("child inserts");

    host.set_node_tint_eased(
        child,
        Some(Color::from_linear_rgba(0.0, 1.0, 0.0, 1.0)),
        2.0,
        SceneHostEasing::Linear,
    )
    .expect("tint transition starts");

    let subtree_tint = Color::from_linear_rgba(0.8, 0.1, 0.2, 1.0);
    host.set_subtree_tint(parent, Some(subtree_tint), &[])
        .expect("subtree tint applies");
    host.advance(1.0)
        .expect("advance after subtree tint remains valid");

    assert_eq!(
        host_node_tint(&host, child),
        Some(subtree_tint),
        "subtree tint is a direct write and cancels active per-node fades"
    );
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
fn scene_host_animation_inventory_and_playback_controls_drive_imported_clip() {
    let mut host = SceneHostCore::headless(64, 64).expect("host builds");
    let import = pollster::block_on(host.instantiate_url(AssetPath::from(
        "tests/assets/gltf/animated_triangle_scene.glb",
    )))
    .expect("animated asset instantiates");
    let animated = host
        .node_handle_by_name(import, "AnimatedTriangle")
        .expect("animated node resolves");

    let inventory: SceneHostAnimationInventoryV1 = serde_json::from_str(
        &host
            .animation_inventory_json(import)
            .expect("inventory serializes"),
    )
    .expect("inventory decodes");
    assert_eq!(
        inventory.schema,
        scena::SCENE_HOST_ANIMATION_INVENTORY_SCHEMA_V1
    );
    assert!(
        inventory.clips.iter().any(|clip| {
            clip.name == "MoveTriangle" && clip.duration_seconds == 1.0 && clip.channel_count == 1
        }),
        "inventory must expose named clip duration and channel count: {inventory:?}"
    );

    let initial = host_node_translation(&host, animated);
    let mixer = host
        .play_animation(
            import,
            "MoveTriangle",
            SceneHostAnimationPlayOptions {
                loop_mode: SceneHostAnimationLoopMode::Once,
                speed: 1.0,
            },
        )
        .expect("animation starts");
    host.advance(0.5).expect("animation advances");
    let halfway = host_node_translation(&host, animated);
    assert!(
        !halfway.abs_diff_eq(initial, 1.0e-5),
        "advance must move the animated node"
    );

    host.pause_animation(mixer).expect("animation pauses");
    host.advance(0.25)
        .expect("paused animation advance is accepted");
    assert_vec3_near(host_node_translation(&host, animated), halfway);

    host.seek_animation(mixer, 1.0)
        .expect("seek samples clip end");
    let end = host_node_translation(&host, animated);
    assert!(
        !end.abs_diff_eq(halfway, 1.0e-5),
        "seek to clip end must land on a distinct sampled pose"
    );
    host.stop_animation(mixer)
        .expect("stop snaps to clip start");
    let stopped = host_node_translation(&host, animated);
    assert!(
        !stopped.abs_diff_eq(end, 1.0e-5),
        "stop must restore the clip start pose"
    );

    let repeat = host
        .play_animation(
            import,
            "MoveTriangle",
            SceneHostAnimationPlayOptions {
                loop_mode: SceneHostAnimationLoopMode::Repeat,
                speed: 2.0,
            },
        )
        .expect("repeat animation starts");
    host.advance(0.75)
        .expect("repeat animation advances with speed");
    let looped = host_node_translation(&host, animated);
    host.seek_animation(repeat, 0.5)
        .expect("seek to wrapped equivalent time succeeds");
    assert_vec3_near(host_node_translation(&host, animated), looped);
}

#[test]
fn scene_host_animation_boundary_rejects_invalid_public_inputs() {
    let mut host = SceneHostCore::headless(64, 64).expect("host builds");
    let import = pollster::block_on(host.instantiate_url(AssetPath::from(
        "tests/assets/gltf/animated_triangle_scene.glb",
    )))
    .expect("animated asset instantiates");

    let bad_speed = host
        .play_animation(
            import,
            "MoveTriangle",
            SceneHostAnimationPlayOptions {
                loop_mode: SceneHostAnimationLoopMode::Once,
                speed: 0.0,
            },
        )
        .expect_err("zero speed is rejected at the host boundary");
    assert_eq!(bad_speed.code(), SceneHostErrorCode::InvalidInput);

    let unknown = host
        .play_animation(
            import,
            "MissingClip",
            SceneHostAnimationPlayOptions::default(),
        )
        .expect_err("unknown clip is a typed error");
    assert_eq!(unknown.code(), SceneHostErrorCode::AnimationClipNotFound);

    let mixer = host
        .play_animation(
            import,
            "MoveTriangle",
            SceneHostAnimationPlayOptions::default(),
        )
        .expect("valid animation starts");
    for delta in [-0.1, f64::NAN] {
        let error = host
            .advance(delta)
            .expect_err("invalid advance delta is rejected");
        assert_eq!(error.code(), SceneHostErrorCode::InvalidInput);
    }
    for speed in [0.0, -1.0, f64::INFINITY] {
        let error = host
            .set_animation_speed(mixer, speed)
            .expect_err("invalid speed is rejected");
        assert_eq!(error.code(), SceneHostErrorCode::InvalidInput);
    }
    for seek in [-0.1, 1.1, f64::NAN] {
        let error = host
            .seek_animation(mixer, seek)
            .expect_err("invalid seek is rejected");
        assert_eq!(error.code(), SceneHostErrorCode::InvalidInput);
    }

    host.remove_import(import).expect("import removes");
    let stale = host
        .advance(0.1)
        .expect_err("stale mixer handle is typed after import removal");
    assert_eq!(stale.code(), SceneHostErrorCode::StaleAnimationHandle);
}

#[test]
fn scene_host_eased_transform_matches_curve_retargets_and_direct_set_cancels() {
    let mut host = SceneHostCore::headless(64, 64).expect("host builds");
    let node = host
        .add_empty(Some(host.root_handle()), Transform::IDENTITY, Some("eased"))
        .expect("node inserts");

    host.set_transform_eased(
        node,
        Transform::at(Vec3::new(8.0, 0.0, 0.0)),
        2.0,
        SceneHostEasing::EaseInOut,
    )
    .expect("eased transform starts");
    host.advance(1.0).expect("transition advances halfway");
    assert_vec3_near(host_node_translation(&host, node), Vec3::new(4.0, 0.0, 0.0));

    let before_retarget = host_node_translation(&host, node);
    host.set_transform_eased(
        node,
        Transform::at(Vec3::new(10.0, 0.0, 0.0)),
        2.0,
        SceneHostEasing::Linear,
    )
    .expect("retarget starts from current interpolated pose");
    assert_vec3_near(host_node_translation(&host, node), before_retarget);
    host.advance(0.1).expect("retarget advances one small step");
    let after_retarget = host_node_translation(&host, node);
    assert!(
        (after_retarget.x - before_retarget.x).abs() < 0.31,
        "retarget must not jump by more than the next linear transition step"
    );

    host.set_transform(node, Transform::IDENTITY)
        .expect("direct transform cancels transition");
    host.advance(1.0)
        .expect("cancelled transition no longer applies");
    assert_vec3_near(host_node_translation(&host, node), Vec3::ZERO);
}

#[test]
fn scene_host_eased_tint_interpolates_and_rejects_translucent_targets() {
    let mut host = SceneHostCore::headless(64, 64).expect("host builds");
    let import = pollster::block_on(host.instantiate_url(AssetPath::from(
        "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
    )))
    .expect("asset instantiates");
    let mesh = host
        .node_handle(import, "ColoredTriangle")
        .expect("mesh resolves");

    host.set_node_tint_eased(
        mesh,
        Some(Color::from_linear_rgba(0.0, 1.0, 0.0, 1.0)),
        2.0,
        SceneHostEasing::Linear,
    )
    .expect("eased tint starts");
    host.advance(1.0).expect("tint transition advances halfway");
    let halfway = host_node_tint(&host, mesh).expect("halfway tint is explicit");
    assert_color_near(halfway, Color::from_linear_rgba(0.5, 1.0, 0.5, 1.0));

    let translucent = host
        .set_node_tint_eased(
            mesh,
            Some(Color::from_linear_rgba(1.0, 0.0, 0.0, 0.5)),
            0.5,
            SceneHostEasing::Linear,
        )
        .expect_err("translucent tint transitions are rejected");
    assert_eq!(translucent.code(), SceneHostErrorCode::InvalidInput);

    host.set_node_tint_eased(mesh, None, 1.0, SceneHostEasing::Linear)
        .expect("clear tint transition starts");
    host.advance(1.0).expect("clear tint transition finishes");
    assert_eq!(host_node_tint(&host, mesh), None);
}

#[test]
fn scene_host_advance_applies_mixers_before_transitions_so_transition_wins() {
    let mut host = SceneHostCore::headless(64, 64).expect("host builds");
    let import = pollster::block_on(host.instantiate_url(AssetPath::from(
        "tests/assets/gltf/animated_triangle_scene.glb",
    )))
    .expect("animated asset instantiates");
    let animated = host
        .node_handle_by_name(import, "AnimatedTriangle")
        .expect("animated node resolves");
    let start = host_node_translation(&host, animated);
    let target = Vec3::new(start.x + 6.0, start.y, start.z);

    host.play_animation(
        import,
        "MoveTriangle",
        SceneHostAnimationPlayOptions::default(),
    )
    .expect("animation starts");
    host.set_transform_eased(
        animated,
        Transform::at(target),
        1.0,
        SceneHostEasing::Linear,
    )
    .expect("transition starts on same node as mixer");
    host.advance(0.5)
        .expect("advance applies mixer then transition");

    assert_vec3_near(
        host_node_translation(&host, animated),
        start.lerp(target, 0.5),
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

fn host_node_translation(host: &SceneHostCore, handle: u64) -> Vec3 {
    host_report(host)
        .node_by_handle(handle)
        .expect("node appears in host inspection")
        .local_transform
        .translation
}

fn host_node_tint(host: &SceneHostCore, handle: u64) -> Option<Color> {
    host_report(host)
        .node_by_handle(handle)
        .expect("node appears in host inspection")
        .tint
}

fn host_report(host: &SceneHostCore) -> SceneInspectionReportV1 {
    serde_json::from_str(&host.inspect_json().expect("inspection serializes"))
        .expect("inspection decodes")
}

fn assert_vec3_near(actual: Vec3, expected: Vec3) {
    assert!(
        actual.abs_diff_eq(expected, 1.0e-5),
        "expected {expected:?}, got {actual:?}"
    );
}

fn assert_color_near(actual: Color, expected: Color) {
    let close = |left: f32, right: f32| (left - right).abs() < 1.0e-5;
    assert!(
        close(actual.r, expected.r)
            && close(actual.g, expected.g)
            && close(actual.b, expected.b)
            && close(actual.a, expected.a),
        "expected {expected:?}, got {actual:?}"
    );
}
