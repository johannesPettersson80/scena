use scena::{
    ASSET_GEOMETRY_SUMMARY_SCHEMA_V1, AnnotationAnchor, AnnotationProjectionReportV1, Assets,
    Color, ConnectOptions, ConnectionError, ConnectionRequest, ConnectorFrame, DirectionalLight,
    GeometryDesc, LabelDesc, LookupError, MaterialDesc, NodeKind, PerspectiveCamera,
    SCENE_ANNOTATION_PROJECTION_SCHEMA_V1, Scene, SceneAssetGeometrySummary, SourceUnits,
    Transform, Vec3,
};

#[test]
fn transform_compose_matches_trs_parent_child_math() {
    let parent = Transform {
        translation: Vec3::new(1.0, 2.0, 3.0),
        rotation: scena::Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
        scale: Vec3::new(2.0, 3.0, 4.0),
    };
    let child = Transform {
        translation: Vec3::new(0.5, 1.0, -0.25),
        rotation: scena::Quat::from_rotation_x(0.25),
        scale: Vec3::new(5.0, 7.0, 11.0),
    };

    let composed = Transform::compose(parent, child);

    let expected_translation =
        parent.translation + parent.rotation * (child.translation * parent.scale);
    let expected_rotation = (parent.rotation * child.rotation).normalize();
    let expected_scale = parent.scale * child.scale;
    assert!(
        composed
            .translation
            .abs_diff_eq(expected_translation, 1.0e-5)
    );
    assert!(composed.scale.abs_diff_eq(expected_scale, 1.0e-5));
    assert!(
        composed.rotation.dot(expected_rotation).abs() > 1.0 - 1.0e-5,
        "quaternions should encode the same orientation"
    );
}

#[test]
fn transform_multiplication_delegates_to_compose() {
    let parent = Transform::at(Vec3::new(2.0, 0.0, 0.0)).rotate_y_deg(45.0);
    let child = Transform::at(Vec3::new(0.0, 0.0, -3.0)).scale_by(2.0);

    assert_eq!(parent * child, Transform::compose(parent, child));
}

#[test]
fn scene_remove_node_recursively_deletes_subtree_and_owned_resources() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    let parent = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(1.0, 0.0, 0.0)))
        .expect("parent inserts");
    let sibling = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(-1.0, 0.0, 0.0)))
        .expect("sibling inserts");
    let mesh = scene
        .mesh(geometry, material)
        .parent(parent)
        .add()
        .expect("mesh inserts");
    let label = scene
        .add_label(parent, LabelDesc::new("part label"), Transform::IDENTITY)
        .expect("label inserts");
    let instance_set = scene
        .add_instance_set(parent, geometry, material, Transform::IDENTITY)
        .expect("instance set inserts");
    let light_node = scene
        .directional_light(DirectionalLight::default())
        .parent(parent)
        .add()
        .expect("light inserts");
    let NodeKind::Light(light) = *scene.node(light_node).expect("light node exists").kind() else {
        panic!("expected light node kind");
    };
    let camera = scene
        .add_perspective_camera(parent, PerspectiveCamera::default(), Transform::IDENTITY)
        .expect("camera inserts");
    scene
        .set_active_camera(camera)
        .expect("camera can become active");
    let structure_before = scene.dirty_state().structure_revision;
    let transform_before = scene.dirty_state().transform_revision;

    scene.remove_node(parent).expect("subtree removes");

    assert!(scene.node(parent).is_none());
    assert!(scene.node(mesh).is_none());
    assert!(scene.node(light_node).is_none());
    assert!(scene.node(sibling).is_some());
    assert!(
        !scene
            .node(scene.root())
            .expect("root remains")
            .children()
            .contains(&parent)
    );
    assert!(scene.label(label).is_none());
    assert!(scene.instance_set(instance_set).is_none());
    assert!(scene.light(light).is_none());
    assert!(scene.camera(camera).is_none());
    assert_eq!(scene.active_camera(), None);
    assert!(scene.dirty_state().structure_revision > structure_before);
    assert!(scene.dirty_state().transform_revision > transform_before);
}

#[test]
fn scene_remove_node_rejects_root_and_missing_nodes() {
    let mut scene = Scene::new();
    assert!(matches!(
        scene.remove_node(scene.root()),
        Err(LookupError::CannotRemoveRootNode(_))
    ));

    let removed = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("node inserts");
    scene.remove_node(removed).expect("node removes once");
    assert!(matches!(
        scene.remove_node(removed),
        Err(LookupError::NodeNotFound(node)) if node == removed
    ));
}

#[test]
fn scene_remove_node_drops_only_connectors_attached_to_removed_nodes() {
    let mut scene = Scene::new();
    let source = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("source inserts");
    let target = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(2.0, 0.0, 0.0)))
        .expect("target inserts");
    let source_connector = scene
        .add_connector(ConnectorFrame::new(source, Transform::IDENTITY).named("source"))
        .expect("source connector inserts");
    let target_connector = scene
        .add_connector(ConnectorFrame::new(target, Transform::IDENTITY).named("target"))
        .expect("target connector inserts");

    let previews = scene
        .validate_connections(&[ConnectionRequest::new(
            source_connector,
            target_connector,
            ConnectOptions::default(),
        )])
        .expect("connection preview validates before removal");
    assert_eq!(previews[0].connection_line().source(), source);
    assert_eq!(previews[0].connection_line().target(), target);

    scene.remove_node(source).expect("source removes");

    assert!(matches!(
        scene.connector(source_connector),
        Err(ConnectionError::MissingConnector { connector }) if connector == source_connector
    ));
    assert_eq!(
        scene
            .connector(target_connector)
            .expect("target-side connector remains live")
            .node(),
        target
    );
    assert!(matches!(
        scene.validate_connections(&[ConnectionRequest::new(
            target_connector,
            source_connector,
            ConnectOptions::default(),
        )]),
        Err(ConnectionError::MissingConnector { connector }) if connector == source_connector
    ));
}

#[test]
fn scene_set_node_tint_is_per_node_render_state_not_material_mutation() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    let mesh = scene.mesh(geometry, material).add().expect("mesh inserts");
    let tint = Color::from_linear_rgba(1.0, 0.0, 0.0, 0.75);
    let before = scene.dirty_state().structure_revision;

    scene
        .set_node_tint(mesh, Some(tint))
        .expect("node tint sets");

    assert_eq!(
        scene.node_tint(mesh).expect("mesh remains live"),
        Some(tint)
    );
    assert_eq!(
        scene.node(mesh).expect("mesh remains live").tint(),
        Some(tint)
    );
    let NodeKind::Mesh(mesh_node) = *scene.node(mesh).expect("mesh remains live").kind() else {
        panic!("expected mesh node");
    };
    assert_eq!(mesh_node.material(), material);
    assert_eq!(scene.dirty_state().structure_revision, before + 1);

    scene
        .set_node_tint(mesh, Some(tint))
        .expect("same tint is a no-op");
    assert_eq!(scene.dirty_state().structure_revision, before + 1);

    scene.set_node_tint(mesh, None).expect("node tint clears");
    assert_eq!(scene.node_tint(mesh).expect("mesh remains live"), None);
    assert_eq!(scene.dirty_state().structure_revision, before + 2);
}

#[test]
fn scene_annotation_projections_follow_node_anchors_and_cleanup_on_remove() {
    let (mut scene, camera) = Scene::with_default_camera().expect("default camera inserts");
    let marker = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("marker inserts");
    scene
        .set_annotation_anchor(AnnotationAnchor::node("marker-label", marker, Vec3::ZERO))
        .expect("node annotation inserts");

    let centered = scene
        .annotation_projection_report(camera, 100, 100)
        .expect("annotation projects");
    assert_eq!(centered.schema, SCENE_ANNOTATION_PROJECTION_SCHEMA_V1);
    let centered_marker = centered
        .annotations
        .iter()
        .find(|projection| projection.id == "marker-label")
        .expect("marker annotation appears");
    assert_eq!(centered_marker.node_handle, None);
    assert!(centered_marker.visible);
    assert!((centered_marker.x - 50.0).abs() < 1.0);
    assert!((centered_marker.y - 50.0).abs() < 1.0);

    scene
        .set_transform(marker, Transform::at(Vec3::new(0.5, 0.0, 0.0)))
        .expect("marker moves");
    let moved = scene
        .annotation_projection_report(camera, 100, 100)
        .expect("moved annotation projects");
    let moved_marker = moved
        .annotations
        .iter()
        .find(|projection| projection.id == "marker-label")
        .expect("marker annotation still appears");
    assert!(
        moved_marker.x > centered_marker.x,
        "node-anchored projection must follow the node transform"
    );

    let json = serde_json::to_string(&moved).expect("projection report serializes");
    let decoded: AnnotationProjectionReportV1 =
        serde_json::from_str(&json).expect("projection report round-trips");
    assert_eq!(decoded, moved);

    scene.remove_node(marker).expect("marker removes");
    assert!(
        scene
            .annotation_projection_report(camera, 100, 100)
            .expect("projection after removal succeeds")
            .annotations
            .is_empty(),
        "removing a node must drop annotations anchored to it"
    );
}

#[test]
fn scene_geometry_helpers_report_distance_and_world_bounds() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    let left = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(1.0, 0.0, 0.0)))
        .expect("left inserts");
    let right = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(4.0, 0.0, 0.0)))
        .expect("right inserts");
    scene
        .mesh(geometry, material)
        .parent(left)
        .transform(Transform::at(Vec3::new(0.0, 2.0, 0.0)))
        .add()
        .expect("bounded mesh inserts");

    assert_eq!(
        scene
            .world_distance(left, right)
            .expect("distance computes"),
        3.0
    );
    let bounds = scene
        .node_world_bounds(left, &assets)
        .expect("node bounds computes")
        .expect("left subtree has bounds");
    assert_eq!(bounds.min, Vec3::new(0.5, 1.5, -0.5));
    assert_eq!(bounds.max, Vec3::new(1.5, 2.5, 0.5));
}

#[test]
fn scene_asset_geometry_summary_counts_bounds_and_source_metadata() {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/nested_mesh_bounds_scene.gltf"))
            .expect("bounded glTF loads");

    let summary = scene_asset.geometry_summary();

    assert_eq!(summary.schema, ASSET_GEOMETRY_SUMMARY_SCHEMA_V1);
    assert_eq!(summary.node_count, scene_asset.node_count());
    assert_eq!(summary.mesh_count, scene_asset.mesh_count());
    assert_eq!(
        summary.provenance.source_path().as_str(),
        "tests/assets/gltf/nested_mesh_bounds_scene.gltf"
    );
    assert!(summary.provenance.source_sha256().is_some());
    assert_eq!(
        summary.primitive_count,
        scene_asset
            .nodes()
            .iter()
            .map(|node| node.meshes().len())
            .sum::<usize>()
    );
    assert!(summary.primitive_count > 0);
    assert!(summary.bounds.is_some());
    let json = serde_json::to_string(&summary).expect("summary serializes");
    let decoded: SceneAssetGeometrySummary =
        serde_json::from_str(&json).expect("summary deserializes");
    assert_eq!(decoded, summary);

    let anchor_units_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/anchor_units_scene.gltf"))
            .expect("anchor units glTF loads");
    let units_summary = anchor_units_asset.geometry_summary();
    assert_eq!(units_summary.primitive_count, 0);
    assert_eq!(
        units_summary.source_units,
        vec![
            SourceUnits::Millimeters,
            SourceUnits::Inches,
            SourceUnits::Feet
        ]
    );
}
