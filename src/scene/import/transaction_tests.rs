use crate::assets::{Assets, SceneAsset};
use crate::geometry::Primitive;
use crate::scene::{
    AnchorFrame, CameraKey, ConnectionError, ConnectorFrame, NodeKey, PerspectiveCamera, Scene,
    Transform, Vec3,
};
use crate::{LookupError, Renderer};

#[derive(Debug, Clone, PartialEq, Eq)]
struct SceneFingerprint {
    nodes: usize,
    cameras: usize,
    lights: usize,
    instance_sets: usize,
    particle_sets: usize,
    animation_mixers: usize,
    labels: usize,
    anchors: usize,
    retired_anchors: usize,
    annotations: usize,
    callouts: usize,
    measurements: usize,
    connectors: usize,
    retired_connectors: usize,
    bounded_nodes: usize,
    morph_nodes: usize,
    skin_nodes: usize,
    root_children: Vec<NodeKey>,
    structure_revision: u64,
    transform_revision: u64,
    appearance_revision: u64,
    visibility_revision: u64,
}

fn fingerprint(scene: &Scene) -> SceneFingerprint {
    SceneFingerprint {
        nodes: scene.nodes.len(),
        cameras: scene.cameras.len(),
        lights: scene.lights.len(),
        instance_sets: scene.instance_sets.len(),
        particle_sets: scene.particle_sets.len(),
        animation_mixers: scene.animation_mixers.len(),
        labels: scene.labels.len(),
        anchors: scene.anchors.len(),
        retired_anchors: scene.retired_anchors.len(),
        annotations: scene.annotations.len(),
        callouts: scene.callouts.len(),
        measurements: scene.measurements.len(),
        connectors: scene.connectors.len(),
        retired_connectors: scene.retired_connectors.len(),
        bounded_nodes: scene.node_bounds.len(),
        morph_nodes: scene.morph_weights.len(),
        skin_nodes: scene.skin_bindings.len(),
        root_children: scene.nodes[scene.root].children.clone(),
        structure_revision: scene.structure_revision,
        transform_revision: scene.transform_revision,
        appearance_revision: scene.appearance_revision,
        visibility_revision: scene.visibility_revision,
    }
}

fn minimal_asset() -> SceneAsset {
    let assets = Assets::new();
    pollster::block_on(assets.load_scene("tests/assets/gltf/minimal_scene.gltf"))
        .expect("minimal scene asset loads")
}

fn anchor_asset() -> SceneAsset {
    let assets = Assets::new();
    pollster::block_on(assets.load_scene("tests/assets/gltf/anchor_debug_scene.gltf"))
        .expect("anchor scene asset loads")
}

fn scene_with_visible_baseline() -> (Scene, super::SceneImport, Renderer, CameraKey) {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 2.0)),
        )
        .expect("camera inserts");
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::IDENTITY,
        )
        .expect("baseline triangle inserts");
    let import = scene
        .instantiate(&minimal_asset())
        .expect("baseline import instantiates");
    let mut renderer = Renderer::headless(16, 16).expect("renderer builds");
    renderer.prepare(&mut scene).expect("baseline prepares");
    (scene, import, renderer, camera)
}

#[test]
fn repeated_replace_import_is_bounded_and_removes_every_old_root() {
    let asset = minimal_asset();
    let (mut scene, mut current, mut renderer, camera) = scene_with_visible_baseline();
    let baseline = renderer
        .render(&scene, camera)
        .expect("baseline render succeeds");
    assert!(
        baseline.draw_calls > 0,
        "the bounded replacement proof must observe real draws"
    );
    let expected_draw_calls = baseline.draw_calls;
    let expected_pixels = renderer.frame_rgba8().to_vec();
    let expected = fingerprint(&scene);

    for iteration in 0..256 {
        let old = current.clone();
        let old_roots = old.roots().to_vec();
        current = scene
            .replace_import(&old, &asset)
            .unwrap_or_else(|error| panic!("replacement {iteration} succeeds: {error}"));

        assert!(matches!(old.node("Root"), Err(LookupError::StaleImport)));
        for root in old_roots {
            assert!(
                scene.node(root).is_none(),
                "replacement {iteration} must remove old root {root:?}"
            );
        }
        assert_eq!(
            fingerprint(&scene).nodes,
            expected.nodes,
            "replacement {iteration} must keep node count bounded"
        );
        assert_eq!(
            scene.nodes[scene.root].children.len(),
            expected.root_children.len(),
            "replacement {iteration} must keep root count bounded"
        );
        renderer
            .prepare(&mut scene)
            .unwrap_or_else(|error| panic!("replacement {iteration} prepares: {error}"));
        let rendered = renderer
            .render(&scene, camera)
            .unwrap_or_else(|error| panic!("replacement {iteration} renders: {error}"));
        assert_eq!(
            rendered.draw_calls, expected_draw_calls,
            "replacement {iteration} must keep draw count bounded"
        );
        assert_eq!(
            renderer.frame_rgba8(),
            expected_pixels,
            "replacement {iteration} must retain the same visible output"
        );
        assert!(
            current.node("Root").is_ok(),
            "replacement {iteration} must leave exactly the new import live"
        );
    }
}

#[test]
fn failed_create_is_exact_noop_at_every_late_failure_family() {
    type InjectFailure = fn(&mut SceneAsset);
    let cases: [(&str, InjectFailure); 4] = [
        ("child-link", |asset| {
            asset.inject_invalid_child_for_transaction_test(1, 99)
        }),
        ("anchor", |asset| {
            asset.inject_invalid_anchor_for_transaction_test(1, "injected anchor failure")
        }),
        ("connector", |asset| {
            asset.inject_invalid_connector_for_transaction_test(1, "injected connector failure")
        }),
        ("skin", |asset| {
            asset.inject_invalid_skin_for_transaction_test(1, 99)
        }),
    ];

    for (name, inject) in cases {
        let (mut scene, _old_import, mut renderer, camera) = scene_with_visible_baseline();
        renderer
            .render(&scene, camera)
            .expect("baseline render succeeds");
        let before_scene = fingerprint(&scene);
        let before_pixels = renderer.frame_rgba8().to_vec();
        let mut invalid = minimal_asset();
        inject(&mut invalid);

        scene
            .instantiate(&invalid)
            .expect_err("injected create failure must surface");

        assert_eq!(
            fingerprint(&scene),
            before_scene,
            "{name} create is a no-op"
        );
        renderer
            .render(&scene, camera)
            .unwrap_or_else(|error| panic!("{name} create must not dirty renderer: {error}"));
        assert_eq!(
            renderer.frame_rgba8(),
            before_pixels,
            "{name} output remains"
        );
    }
}

#[test]
fn prevalidation_rejects_cycles_and_multiple_parents_without_scene_mutation() {
    let mut scene = Scene::new();
    let before = fingerprint(&scene);

    let mut cycle = minimal_asset();
    cycle.inject_invalid_child_for_transaction_test(1, 0);
    assert!(matches!(
        scene.instantiate(&cycle),
        Err(crate::InstantiateError::CyclicNodeGraph { .. })
    ));
    assert_eq!(fingerprint(&scene), before);

    let mut multiple_parents = minimal_asset();
    multiple_parents.inject_additional_root_for_transaction_test();
    multiple_parents.inject_invalid_child_for_transaction_test(2, 1);
    assert!(matches!(
        scene.instantiate(&multiple_parents),
        Err(crate::InstantiateError::MultipleNodeParents { node: 1, .. })
    ));
    assert_eq!(fingerprint(&scene), before);
}

#[test]
fn remove_import_is_atomic_for_multiple_roots_and_rejects_foreign_scene() {
    let mut asset = minimal_asset();
    asset.inject_additional_root_for_transaction_test();
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&asset)
        .expect("multi-root import succeeds");
    assert_eq!(import.roots().len(), 2);
    let roots = import.roots().to_vec();
    let before_structure = scene.structure_revision;

    let mut foreign = Scene::new();
    assert_eq!(
        foreign.remove_import(&import),
        Err(LookupError::ImportFromDifferentScene)
    );
    assert!(
        import.node("Root").is_ok(),
        "foreign rejection keeps import live"
    );

    scene.remove_import(&import).expect("owned import removes");
    assert!(matches!(import.node("Root"), Err(LookupError::StaleImport)));
    assert!(roots.into_iter().all(|root| scene.node(root).is_none()));
    assert_eq!(
        scene.structure_revision,
        before_structure.saturating_add(1),
        "all roots are removed in one revision boundary"
    );
}

#[test]
fn failed_replace_is_exact_noop_at_every_late_failure_family() {
    type InjectFailure = fn(&mut SceneAsset);
    let cases: [(&str, InjectFailure); 4] = [
        ("child-link", |asset| {
            asset.inject_invalid_child_for_transaction_test(1, 99)
        }),
        ("anchor", |asset| {
            asset.inject_invalid_anchor_for_transaction_test(1, "injected anchor failure")
        }),
        ("connector", |asset| {
            asset.inject_invalid_connector_for_transaction_test(1, "injected connector failure")
        }),
        ("skin", |asset| {
            asset.inject_invalid_skin_for_transaction_test(1, 99)
        }),
    ];

    for (name, inject) in cases {
        let (mut scene, old_import, mut renderer, camera) = scene_with_visible_baseline();
        renderer
            .render(&scene, camera)
            .expect("baseline render succeeds");
        let before_scene = fingerprint(&scene);
        let before_pixels = renderer.frame_rgba8().to_vec();
        let old_root = old_import.node("Root").expect("old import is live");
        let mut invalid = minimal_asset();
        inject(&mut invalid);

        scene
            .replace_import(&old_import, &invalid)
            .expect_err("injected replacement failure must surface");

        assert_eq!(
            fingerprint(&scene),
            before_scene,
            "{name} failure must restore every scene-owned registry and revision"
        );
        assert_eq!(
            old_import.node("Root"),
            Ok(old_root),
            "{name} failure must leave the old import live"
        );
        assert!(scene.node(old_root).is_some(), "{name} old root remains");
        renderer
            .render(&scene, camera)
            .unwrap_or_else(|error| panic!("{name} failure must not dirty renderer: {error}"));
        assert_eq!(
            renderer.frame_rgba8(),
            before_pixels,
            "{name} output remains visible"
        );
    }
}

#[test]
fn replacement_uses_fresh_runtime_overrides_and_commits_one_revision_boundary() {
    let asset = minimal_asset();
    let mut scene = Scene::new();
    let old = scene.instantiate(&asset).expect("old import succeeds");
    let old_root = old.node("Root").expect("old root exists");
    scene
        .set_visible(old_root, false)
        .expect("user-authored override applies");
    let before_structure = scene.structure_revision;

    let replacement = scene
        .replace_import(&old, &asset)
        .expect("replacement succeeds");
    let new_root = replacement.node("Root").expect("new root exists");

    assert_eq!(scene.visible(new_root), Some(true));
    assert_eq!(scene.node(old_root), None);
    assert_eq!(
        scene.structure_revision,
        before_structure.saturating_add(1),
        "replacement commits one observable structural revision"
    );
}

#[test]
fn removed_import_anchor_and_connector_handles_remain_stale_without_live_registry_growth() {
    let asset = anchor_asset();
    let mut scene = Scene::new();
    let mut current = scene.instantiate(&asset).expect("first import succeeds");

    for iteration in 0..64 {
        let imported_anchor = current
            .anchor("inspection")
            .unwrap_or_else(|error| panic!("iteration {iteration} anchor resolves: {error}"));
        let anchor = scene
            .add_anchor(AnchorFrame::from_import_anchor(imported_anchor))
            .unwrap_or_else(|error| panic!("iteration {iteration} anchor registers: {error}"));
        let connector = scene
            .add_connector(ConnectorFrame::from_import_anchor(imported_anchor))
            .unwrap_or_else(|error| panic!("iteration {iteration} connector registers: {error}"));

        current = scene
            .replace_import(&current, &asset)
            .unwrap_or_else(|error| panic!("iteration {iteration} replacement succeeds: {error}"));

        assert!(matches!(
            scene.anchor(anchor),
            Err(ConnectionError::StaleAnchorHandle { anchor: stale, name })
                if stale == Some(anchor) && name.as_deref() == Some("inspection")
        ));
        assert!(matches!(
            scene.connector(connector),
            Err(ConnectionError::StaleConnectorHandle { connector: stale, name })
                if stale == Some(connector) && name.as_deref() == Some("inspection")
        ));
        assert_eq!(
            scene.anchors.len(),
            0,
            "iteration {iteration} anchors bounded"
        );
        assert!(
            scene.retired_anchors.len() <= 1,
            "iteration {iteration} retired anchor slots bounded"
        );
        assert_eq!(
            scene.connectors.len(),
            0,
            "iteration {iteration} connectors bounded"
        );
        assert!(
            scene.retired_connectors.len() <= 1,
            "iteration {iteration} retired connector slots bounded"
        );
    }
}

#[test]
fn direct_anchor_and_connector_removal_remains_missing_not_import_stale() {
    let mut scene = Scene::new();
    let node = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("direct host inserts");
    let anchor = scene
        .add_anchor(AnchorFrame::new(node, Transform::IDENTITY).named("direct"))
        .expect("direct anchor registers");
    let connector = scene
        .add_connector(ConnectorFrame::new(node, Transform::IDENTITY).named("direct"))
        .expect("direct connector registers");

    scene.remove_node(node).expect("direct host removes");

    assert_eq!(
        scene.anchor(anchor),
        Err(ConnectionError::MissingAnchor { anchor })
    );
    assert_eq!(
        scene.connector(connector),
        Err(ConnectionError::MissingConnector { connector })
    );
}
