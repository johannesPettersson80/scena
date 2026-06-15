use scena::{Assets, Color, GeometryDesc, MaterialDesc, SceneVisibilitySnapshot, Transform};

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
