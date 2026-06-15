use scena::{
    Assets, Color, GeometryDesc, InspectionHelperKind, MaterialDesc, SceneVisibilitySnapshot,
    Transform, Vec3,
};

#[test]
fn isolate_and_restore_preserve_prior_visibility() {
    let mut scene = scena::Scene::new();
    let keep = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("keep inserts");
    let unrelated = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("unrelated inserts");
    let already_hidden = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("hidden inserts");
    scene
        .set_visible(already_hidden, false)
        .expect("precondition hides node");

    let snapshot = scene.isolate([keep]).expect("isolate succeeds");

    assert!(scene.visible(scene.root()).expect("root remains visible"));
    assert!(scene.visible(keep).expect("kept node remains visible"));
    assert!(!scene.visible(unrelated).expect("unrelated node hides"));
    assert!(
        !scene
            .visible(already_hidden)
            .expect("already-hidden node stays hidden")
    );

    scene
        .restore_visibility(&snapshot)
        .expect("visibility snapshot restores");

    assert!(scene.visible(keep).expect("kept node restores"));
    assert!(scene.visible(unrelated).expect("unrelated node restores"));
    assert!(
        !scene
            .visible(already_hidden)
            .expect("original hidden node remains hidden")
    );
}

#[test]
fn show_hide_toggle_and_show_only_report_changed_nodes() {
    let mut scene = scena::Scene::new();
    let a = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("a inserts");
    let b = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("b inserts");

    scene.hide(b).expect("hide succeeds");
    assert!(!scene.visible(b).expect("b exists"));
    assert!(
        scene
            .toggle_visibility(b)
            .expect("toggle returns new state")
    );
    assert!(scene.visible(b).expect("b shows after toggle"));
    scene.show(b).expect("show succeeds");
    assert!(scene.visible(b).expect("b shows"));

    let snapshot = scene.show_only([a]).expect("show_only succeeds");
    assert_eq!(snapshot.len(), 1, "only b changed visibility");
    assert!(scene.visible(a).expect("a remains visible"));
    assert!(!scene.visible(b).expect("b hides"));
}

#[test]
fn ghost_subtree_tints_nodes_without_mutating_material_descriptors() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material_desc = MaterialDesc::pbr_metallic_roughness(Color::RED, 0.0, 0.5);
    let material = assets.create_material(material_desc.clone());
    let mut scene = scena::Scene::new();
    let parent = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("parent inserts");
    let mesh = scene
        .mesh(geometry, material)
        .parent(parent)
        .add()
        .expect("mesh inserts");

    let snapshot = scene.ghost(parent, 0.35).expect("ghost succeeds");

    assert_eq!(
        snapshot.len(),
        2,
        "parent and child tint states are captured"
    );
    assert_eq!(
        scene.node_tint(mesh).expect("mesh tint reads"),
        Some(Color::from_linear_rgba(1.0, 1.0, 1.0, 0.35))
    );
    assert_eq!(
        assets.material(material).expect("material exists"),
        material_desc,
        "ghosting must not mutate source material descriptors"
    );

    scene.restore_tints(&snapshot).expect("tints restore");
    assert_eq!(scene.node_tint(mesh).expect("mesh tint reads"), None);
}

#[test]
fn visibility_snapshot_is_empty_for_noop_restore() {
    let mut scene = scena::Scene::new();
    let snapshot = SceneVisibilitySnapshot::default();
    scene
        .restore_visibility(&snapshot)
        .expect("empty snapshot restore is a no-op");
}

#[test]
fn fit_selection_frames_union_of_selected_subtrees() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = scena::Scene::new();
    let left = scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(-2.0, 0.0, 0.0)))
        .add()
        .expect("left mesh inserts");
    let right = scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(2.0, 0.0, 0.0)))
        .add()
        .expect("right mesh inserts");
    let camera = scene.add_default_camera().expect("camera inserts");

    let framed_bounds = scene
        .fit_selection_with_assets(camera, [left, right], &assets)
        .expect("selection frames");

    assert_eq!(framed_bounds.min, Vec3::new(-2.5, -0.5, -0.5));
    assert_eq!(framed_bounds.max, Vec3::new(2.5, 0.5, 0.5));
    let camera_node = scene.camera_node(camera).expect("camera node resolves");
    let camera_world = scene
        .world_transform(camera_node)
        .expect("camera world transform resolves");
    assert_eq!(camera_world.translation.x, framed_bounds.center().x);
    assert_eq!(camera_world.translation.y, framed_bounds.center().y);
    assert!(camera_world.translation.z > framed_bounds.max.z);
}

#[test]
fn bounding_box_overlay_and_axes_triad_are_reported_as_helpers() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = scena::Scene::new();
    let target = scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(1.0, 0.0, 0.0)))
        .add()
        .expect("target mesh inserts");

    let bounds = scene
        .add_bounding_box_overlay(&assets, target)
        .expect("bounds overlay inserts");
    let world_axes = scene
        .add_world_axes_triad(&assets, 0.75)
        .expect("world axes insert");
    let local_axes = scene
        .add_local_axes_triad(&assets, target, 0.5)
        .expect("local axes insert");

    assert_eq!(bounds.kind, InspectionHelperKind::BoundingBox);
    assert_eq!(bounds.target, Some(target));
    assert_eq!(
        bounds.bounds.expect("bounds reported").center(),
        Vec3::new(1.0, 0.0, 0.0)
    );
    assert_eq!(world_axes.kind, InspectionHelperKind::WorldAxesTriad);
    assert_eq!(world_axes.target, None);
    assert_eq!(local_axes.kind, InspectionHelperKind::LocalAxesTriad);
    assert_eq!(local_axes.target, Some(target));
    assert_eq!(
        scene
            .node(local_axes.node)
            .expect("local axes node exists")
            .parent(),
        Some(target),
        "local axes triad must follow the target node"
    );

    for helper in [bounds.node, world_axes.node, local_axes.node] {
        assert_eq!(scene.helper_on_top(helper), Some(true));
        assert!(scene.has_tag(helper, "scena:inspection:helper"));
    }

    let report = scene.inspection_toolkit_report();
    assert_eq!(report.helper_nodes.len(), 3);
    assert!(
        report
            .helper_nodes
            .iter()
            .any(|helper| helper.kind == InspectionHelperKind::BoundingBox
                && helper.target == Some(target))
    );
}

#[test]
fn inspection_report_records_active_isolate_and_ghost_state() {
    let mut scene = scena::Scene::new();
    let keep = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("keep inserts");
    let ghosted = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("ghost inserts");
    let ghost_child = scene
        .add_empty(ghosted, Transform::IDENTITY)
        .expect("ghost child inserts");
    let _unrelated = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("unrelated inserts");

    let visibility = scene.isolate([keep]).expect("isolate succeeds");
    scene.ghost(ghosted, 0.25).expect("ghost succeeds");

    let report = scene.inspection_toolkit_report();
    assert_eq!(report.isolated_nodes, vec![keep]);
    assert_eq!(report.hidden_by_isolate_count, 3);
    assert!(report.ghosted_nodes.contains(&ghosted));
    assert!(
        report.ghosted_nodes.contains(&ghost_child),
        "ghosted subtree report includes every tinted node"
    );

    scene
        .restore_visibility(&visibility)
        .expect("isolate restores");
    let restored = scene.inspection_toolkit_report();
    assert!(restored.isolated_nodes.is_empty());
    assert_eq!(restored.hidden_by_isolate_count, 0);
}
