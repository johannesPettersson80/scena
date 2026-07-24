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
fn replacement_preserves_root_visibility_and_commits_one_revision_boundary() {
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

    assert_eq!(scene.visible(new_root), Some(false));
    assert_eq!(scene.node(old_root), None);
    assert_eq!(
        scene.structure_revision,
        before_structure.saturating_add(1),
        "replacement commits one observable structural revision"
    );
}

#[test]
fn replacement_preserves_host_parent_and_root_placement() {
    let asset = minimal_asset();
    let mut scene = Scene::new();
    let host = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(4.0, 0.0, 0.0)))
        .expect("host parent inserts");
    let old = scene
        .instantiate_under(host, &asset, super::ImportOptions::gltf_default())
        .expect("nested import instantiates");
    let old_root = old.node("Root").expect("old root exists");
    let placement = Transform::at(Vec3::new(1.0, 2.0, 3.0)).rotate_y_deg(25.0);
    scene
        .set_transform(old_root, placement)
        .expect("host-owned root placement applies");

    let replacement = scene
        .replace_import(&old, &asset)
        .expect("nested replacement succeeds");
    let new_root = replacement.node("Root").expect("new root exists");

    assert_eq!(
        scene.node(new_root).and_then(|node| node.parent()),
        Some(host)
    );
    assert_eq!(
        scene.node(new_root).map(|node| node.transform()),
        Some(placement)
    );
    assert!(scene.node(old_root).is_none());
}

#[test]
fn replacement_parent_and_placement_are_pinned_by_semantic_aov_pixels() {
    let asset = minimal_asset();
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 2.0)),
        )
        .expect("semantic proof camera inserts");
    let host = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(-0.3, 0.0, 0.0)))
        .expect("semantic proof host inserts");
    let old = scene
        .instantiate_under(host, &asset, super::ImportOptions::gltf_default())
        .expect("nested import instantiates");
    let old_root = old.node("Root").expect("old root resolves");
    let placement = Transform::at(Vec3::new(0.5, 0.0, 0.0));
    scene
        .set_transform(old_root, placement)
        .expect("old root placement applies");
    scene
        .add_renderable(
            old_root,
            vec![Primitive::unlit_triangle()],
            Transform::IDENTITY,
        )
        .expect("old proof geometry inserts");
    let mut renderer = Renderer::headless(64, 64).expect("semantic renderer builds");
    renderer.prepare(&mut scene).expect("old scene prepares");
    let before = renderer
        .semantic_aov_raw(&scene, camera)
        .expect("old semantic AOV captures");

    let replacement = scene
        .replace_import(&old, &asset)
        .expect("nested replacement succeeds");
    let new_root = replacement.node("Root").expect("new root resolves");
    scene
        .add_renderable(
            new_root,
            vec![Primitive::unlit_triangle()],
            Transform::IDENTITY,
        )
        .expect("replacement proof geometry inserts");
    renderer
        .prepare(&mut scene)
        .expect("replacement scene prepares");
    let after = renderer
        .semantic_aov_raw(&scene, camera)
        .expect("replacement semantic AOV captures");

    let before_mask = before
        .id_indices
        .iter()
        .map(|index| *index != 0)
        .collect::<Vec<_>>();
    let after_mask = after
        .id_indices
        .iter()
        .map(|index| *index != 0)
        .collect::<Vec<_>>();
    assert_eq!(
        after_mask, before_mask,
        "replacement must preserve the rendered semantic footprint"
    );
    assert_eq!(
        after.depth_meters, before.depth_meters,
        "replacement must preserve exact semantic depth"
    );
    assert_eq!(
        after.world_normals, before.world_normals,
        "replacement must preserve exact semantic normals"
    );

    scene
        .set_transform(new_root, Transform::IDENTITY)
        .expect("known-bad root-placement mutation applies");
    renderer
        .prepare(&mut scene)
        .expect("known-bad mutation prepares");
    let mutated = renderer
        .semantic_aov_raw(&scene, camera)
        .expect("known-bad semantic AOV captures");
    let mutated_mask = mutated
        .id_indices
        .iter()
        .map(|index| *index != 0)
        .collect::<Vec<_>>();
    assert_ne!(
        mutated_mask, before_mask,
        "the semantic oracle must reject the old lost-root-placement behavior"
    );
}

#[test]
fn replacement_preserves_multiple_root_parents_locals_and_host_visibility() {
    let mut asset = minimal_asset();
    asset.inject_additional_root_for_transaction_test();
    let mut scene = Scene::new();
    let host_a = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(4.0, 0.0, 0.0)))
        .unwrap();
    let host_b = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(-4.0, 0.0, 0.0)))
        .unwrap();
    let old = scene
        .instantiate_under(host_a, &asset, super::ImportOptions::gltf_default())
        .unwrap();
    let first = old.node("Root").unwrap();
    let second = old.node("SecondRoot").unwrap();
    let first_local = Transform::at(Vec3::new(1.0, 2.0, 3.0)).rotate_y_deg(20.0);
    let second_local = Transform::at(Vec3::new(-2.0, 1.0, 0.5)).scale_by(1.5);
    scene.set_transform(first, first_local).unwrap();
    scene.set_transform(second, second_local).unwrap();
    super::load::reparent_replacement_root(&mut scene, second, host_b);
    scene.set_visible(first, false).unwrap();
    scene.add_tag(first, "host-owned").unwrap();

    let replacement = scene.replace_import(&old, &asset).unwrap();
    let new_first = replacement.node("Root").unwrap();
    let new_second = replacement.node("SecondRoot").unwrap();
    assert_eq!(
        scene.node(new_first).and_then(|node| node.parent()),
        Some(host_a)
    );
    assert_eq!(
        scene.node(new_second).and_then(|node| node.parent()),
        Some(host_b)
    );
    assert_eq!(
        scene.node(new_first).map(|node| node.transform()),
        Some(first_local)
    );
    assert_eq!(
        scene.node(new_second).map(|node| node.transform()),
        Some(second_local)
    );
    assert_eq!(scene.visible(new_first), Some(false));
    assert_eq!(scene.visible(new_second), Some(true));
    assert!(scene.has_tag(new_first, "host-owned"));
}

#[test]
fn replacement_maps_renamed_roots_by_ordinal_and_discards_removed_roots() {
    let mut old_asset = minimal_asset();
    old_asset.inject_additional_root_for_transaction_test();
    let mut replacement_asset = minimal_asset();
    replacement_asset.rename_root_for_transaction_test(0, "RenamedRoot");
    let mut scene = Scene::new();
    let host = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(4.0, 0.0, 0.0)))
        .unwrap();
    let old = scene
        .instantiate_under(host, &old_asset, super::ImportOptions::gltf_default())
        .unwrap();
    let old_first = old.node("Root").unwrap();
    let old_second = old.node("SecondRoot").unwrap();
    let placement = Transform::at(Vec3::new(1.0, 2.0, 3.0)).rotate_z_deg(15.0);
    scene.set_transform(old_first, placement).unwrap();
    scene.set_visible(old_first, false).unwrap();

    let replacement = scene.replace_import(&old, &replacement_asset).unwrap();
    let renamed = replacement.node("RenamedRoot").unwrap();
    assert_eq!(
        scene.node(renamed).and_then(|node| node.parent()),
        Some(host)
    );
    assert_eq!(
        scene.node(renamed).map(|node| node.transform()),
        Some(placement)
    );
    assert_eq!(scene.visible(renamed), Some(false));
    assert!(scene.node(old_first).is_none());
    assert!(scene.node(old_second).is_none());
    assert_eq!(replacement.roots().len(), 1);
}

#[test]
fn replacement_attaches_added_roots_to_first_prior_host_parent() {
    let old_asset = minimal_asset();
    let mut replacement_asset = minimal_asset();
    replacement_asset.inject_additional_root_for_transaction_test();
    let mut scene = Scene::new();
    let host = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(-4.0, 0.0, 0.0)))
        .unwrap();
    let old = scene
        .instantiate_under(host, &old_asset, super::ImportOptions::gltf_default())
        .unwrap();

    let replacement = scene.replace_import(&old, &replacement_asset).unwrap();
    let first = replacement.node("Root").unwrap();
    let added = replacement.node("SecondRoot").unwrap();
    assert_eq!(scene.node(first).and_then(|node| node.parent()), Some(host));
    assert_eq!(scene.node(added).and_then(|node| node.parent()), Some(host));
    assert_eq!(replacement.roots(), &[first, added]);
}

#[test]
fn removed_import_anchor_and_connector_handles_remain_stale_without_live_registry_growth() {
    let asset = anchor_asset();
    let mut scene = Scene::new();
    let mut current = scene.instantiate(&asset).expect("first import succeeds");
    let mut retired = Vec::new();

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
        retired.push((anchor, connector));
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
            scene.retired_anchors.len() <= 64,
            "iteration {iteration} retired anchor generations bounded"
        );
        assert_eq!(
            scene.connectors.len(),
            0,
            "iteration {iteration} connectors bounded"
        );
        assert!(
            scene.retired_connectors.len() <= 64,
            "iteration {iteration} retired connector generations bounded"
        );
    }

    for (anchor, connector) in retired {
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
