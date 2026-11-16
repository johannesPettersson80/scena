//! Connections mate through the exact world matrices of their connectors.
//!
//! A connector under a rotated, non-uniformly scaled ancestor is sheared in
//! the world. By default such a connection is refused as a non-uniform scale
//! risk. With the opt-in, the rigid frames of the two connectors are mated:
//! origin and mating (+X) axis exactly, and the up (+Y) axis once it is made
//! perpendicular to the mating axis. Every expectation here is a plain matrix
//! product read back through `Scene::world_matrix`.

use scena::{
    Assets, ConnectOptions, ConnectionAlignment, ConnectionError, ConnectorFrame, Mat4, NodeKey,
    Scene, Transform, Vec3,
};

const TOLERANCE: f32 = 2.0e-4;

struct Fixture {
    scene: Scene,
    source: ConnectorFrame,
    target: ConnectorFrame,
}

fn source_connector() -> Transform {
    Transform::at(Vec3::new(0.2, -0.1, 0.3))
        .rotate_z_deg(35.0)
        .rotate_y_deg(-20.0)
}

fn target_connector() -> Transform {
    Transform::at(Vec3::new(-0.3, 0.2, 0.1))
        .rotate_y_deg(50.0)
        .rotate_x_deg(25.0)
}

fn sheared_chain() -> [Transform; 2] {
    [
        Transform::at(Vec3::new(0.5, 0.2, -0.4))
            .rotate_y_deg(25.0)
            .with_scale(Vec3::new(2.0, 1.0, 0.6)),
        Transform::at(Vec3::new(0.3, 0.1, 0.2))
            .rotate_x_deg(20.0)
            .rotate_z_deg(40.0),
    ]
}

/// A target connector on a node whose world matrix is sheared, and a source
/// connector on a plain top-level node.
fn sheared_target() -> Fixture {
    let mut scene = Scene::new();
    let chain = sheared_chain();
    let rig = scene
        .add_empty(scene.root(), chain[0])
        .expect("rig inserts");
    let target_node = scene.add_empty(rig, chain[1]).expect("target inserts");
    let source_node = scene
        .add_empty(
            scene.root(),
            Transform::at(Vec3::new(-1.0, 0.5, 0.8)).rotate_x_deg(15.0),
        )
        .expect("source inserts");
    Fixture {
        scene,
        source: ConnectorFrame::new(source_node, source_connector()).named("plug"),
        target: ConnectorFrame::new(target_node, target_connector()).named("socket"),
    }
}

/// A source node that hangs below a sheared parent, and a target connector on
/// a plain top-level node.
fn sheared_source_parent() -> Fixture {
    let mut scene = Scene::new();
    let chain = sheared_chain();
    let rig = scene
        .add_empty(scene.root(), chain[0])
        .expect("rig inserts");
    let arm = scene.add_empty(rig, chain[1]).expect("arm inserts");
    let source_node = scene
        .add_empty(
            arm,
            Transform::at(Vec3::new(0.1, 0.4, -0.2)).rotate_y_deg(10.0),
        )
        .expect("source inserts");
    let target_node = scene
        .add_empty(
            scene.root(),
            Transform::at(Vec3::new(1.2, -0.3, 0.6)).rotate_z_deg(-30.0),
        )
        .expect("target inserts");
    Fixture {
        scene,
        source: ConnectorFrame::new(source_node, source_connector()).named("plug"),
        target: ConnectorFrame::new(target_node, target_connector()).named("socket"),
    }
}

fn opted_in() -> ConnectOptions {
    ConnectOptions::default()
        .allow_non_uniform_scale(true)
        .with_explicit_roll_degrees(0.0)
}

#[test]
fn a_sheared_host_is_a_scale_risk_even_when_its_axes_have_equal_lengths() {
    // Scale 2 on X and 1 on Y under a 45 degree turn about Z gives both
    // sheared axes the length sqrt(2.5); the Z scale matches it.
    let equal_length = 2.5_f32.sqrt();
    let mut scene = Scene::new();
    let rig = scene
        .add_empty(
            scene.root(),
            Transform::IDENTITY.with_scale(Vec3::new(2.0, 1.0, equal_length)),
        )
        .expect("rig inserts");
    let target_node = scene
        .add_empty(rig, Transform::IDENTITY.rotate_z_deg(45.0))
        .expect("target inserts");
    let source_node = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(1.0, 0.0, 0.0)))
        .expect("source inserts");
    let world = scene.world_matrix(target_node).expect("target resolves");
    let lengths = Vec3::new(
        world.transform_vector3(Vec3::X).length(),
        world.transform_vector3(Vec3::Y).length(),
        world.transform_vector3(Vec3::Z).length(),
    );
    assert!(
        (lengths - Vec3::splat(equal_length)).abs().max_element() <= 1.0e-5,
        "the fixture needs equal axis lengths: {lengths:?}"
    );
    assert!(
        world
            .transform_vector3(Vec3::X)
            .dot(world.transform_vector3(Vec3::Y))
            .abs()
            > 1.0,
        "the fixture needs a sheared host"
    );

    let error = scene
        .preview_connection(
            ConnectorFrame::new(source_node, Transform::IDENTITY).named("plug"),
            ConnectorFrame::new(target_node, Transform::IDENTITY).named("socket"),
            ConnectOptions::default(),
        )
        .expect_err("a sheared host cannot be mated rigidly without the opt-in");

    assert_eq!(
        error,
        ConnectionError::NonUniformScaleConnectionRisk { node: target_node }
    );
}

#[test]
fn a_sheared_target_is_refused_by_default() {
    let fixture = sheared_target();
    let target_node = fixture.target.node();

    let error = fixture
        .scene
        .preview_connection(fixture.source, fixture.target, ConnectOptions::default())
        .expect_err("a sheared target is a scale risk by default");

    assert_eq!(
        error,
        ConnectionError::NonUniformScaleConnectionRisk { node: target_node }
    );
}

#[test]
fn an_opted_in_connection_mates_exactly_with_a_sheared_target() {
    let mut fixture = sheared_target();
    assert_sheared(&fixture.scene, fixture.target.node());
    let options = opted_in().with_axial_gap(0.25);

    fixture
        .scene
        .connect(fixture.source.clone(), fixture.target.clone(), options)
        .expect("opted-in connection resolves");

    assert_mated(&fixture, Vec3::new(0.25, 0.0, 0.0), 1.0, 0.0);
}

#[test]
fn an_opted_in_connection_mates_exactly_when_the_source_parent_is_sheared() {
    let mut fixture = sheared_source_parent();
    let parent = fixture
        .scene
        .node(fixture.source.node())
        .and_then(|node| node.parent())
        .expect("source has a parent");
    assert_sheared(&fixture.scene, parent);
    let before = fixture
        .scene
        .node(fixture.source.node())
        .expect("source exists")
        .transform();

    fixture
        .scene
        .connect(fixture.source.clone(), fixture.target.clone(), opted_in())
        .expect("opted-in connection resolves");

    assert_mated(&fixture, Vec3::ZERO, 1.0, 0.0);
    let after = fixture
        .scene
        .node(fixture.source.node())
        .expect("source exists")
        .transform();
    assert_eq!(
        after.scale, before.scale,
        "the source keeps its local scale"
    );
}

#[test]
fn an_opted_in_connection_honours_alignment_and_roll_about_the_exact_mating_axis() {
    let mut fixture = sheared_target();
    let options = ConnectOptions::default()
        .allow_non_uniform_scale(true)
        .with_alignment(ConnectionAlignment::ForwardToBack)
        .with_explicit_roll_degrees(30.0);

    fixture
        .scene
        .connect(fixture.source.clone(), fixture.target.clone(), options)
        .expect("opted-in connection resolves");

    // Facing back turns the mated frame half a turn about its up axis, so
    // the mating axes oppose; the roll then turns about the mating axis.
    assert_mated(&fixture, Vec3::ZERO, -1.0, 30.0);
}

/// An asset whose connector sits on a rotated node under a non-uniformly
/// scaled node, two levels below the asset root. The connection moves the
/// asset root, so the connector frame the solver needs is the one relative to
/// that root, and it is sheared.
fn sheared_connector_asset() -> String {
    let (sin, cos) = (40.0_f32.to_radians() * 0.5).sin_cos();
    let (connector_sin, connector_cos) = (25.0_f32.to_radians() * 0.5).sin_cos();
    serde_json::json!({
        "asset": { "version": "2.0" },
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [
            { "name": "Root", "children": [1] },
            { "name": "Scaled", "scale": [2.0, 1.0, 0.6], "children": [2] },
            {
                "name": "Rotated",
                "translation": [0.2, 0.1, -0.1],
                "rotation": [0.0, 0.0, sin, cos],
                "extras": { "scena": { "connectors": [{
                    "name": "mount",
                    "translation": [0.1, 0.05, 0.0],
                    "rotation": [0.0, connector_sin, 0.0, connector_cos]
                }] } }
            }
        ]
    })
    .to_string()
}

#[test]
fn an_imported_connector_sheared_inside_its_asset_mates_through_its_exact_frame() {
    let assets = Assets::new();
    let asset = pollster::block_on(assets.load_scene_from_bytes(
        "sheared_connector.gltf",
        sheared_connector_asset().as_bytes(),
    ))
    .expect("inline glTF loads");
    let mut scene = Scene::new();
    let import = scene.instantiate(&asset).expect("inline glTF instantiates");
    let connector = import
        .connector("mount")
        .expect("asset declares the connector");
    let target = ConnectorFrame::from_import_connector(connector);
    // The exact frame of the connector in the world: the world matrix of the
    // node that declares it times the connector's own transform.
    let declared = connector.transform();
    let target_world = scene
        .world_matrix(connector.node())
        .expect("connector host resolves")
        * Mat4::from_scale_rotation_translation(
            declared.scale,
            declared.rotation,
            declared.translation,
        );
    let x = target_world.transform_vector3(Vec3::X).normalize();
    let y = target_world.transform_vector3(Vec3::Y).normalize();
    assert!(
        x.dot(y).abs() > 0.2,
        "the fixture proves nothing unless the connector is sheared: {x:?} {y:?}"
    );
    let source_node = scene
        .add_empty(
            scene.root(),
            Transform::at(Vec3::new(-1.0, 0.5, 0.8)).rotate_x_deg(15.0),
        )
        .expect("source inserts");
    let source = ConnectorFrame::new(source_node, source_connector()).named("plug");

    let error = scene
        .preview_connection(source.clone(), target.clone(), ConnectOptions::default())
        .expect_err("a connector sheared inside its asset is a scale risk by default");
    assert_eq!(
        error,
        ConnectionError::NonUniformScaleConnectionRisk {
            node: target.node()
        }
    );

    scene
        .connect(source.clone(), target, opted_in())
        .expect("opted-in connection resolves");

    assert_frames_mated(
        connector_world(&scene, &source),
        target_world,
        Vec3::ZERO,
        1.0,
        0.0,
    );
}

#[test]
fn a_mirrored_sheared_host_is_refused_as_flipped() {
    let mut scene = Scene::new();
    let rig = scene
        .add_empty(
            scene.root(),
            Transform::IDENTITY.with_scale(Vec3::new(-2.0, 1.0, 0.5)),
        )
        .expect("rig inserts");
    let target_node = scene
        .add_empty(rig, Transform::IDENTITY.rotate_z_deg(30.0))
        .expect("target inserts");
    let source_node = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("source inserts");

    let error = scene
        .preview_connection(
            ConnectorFrame::new(source_node, Transform::IDENTITY).named("plug"),
            ConnectorFrame::new(target_node, Transform::IDENTITY).named("socket"),
            ConnectOptions::default().allow_non_uniform_scale(true),
        )
        .expect_err("a mirrored host must not be mated silently");

    assert_eq!(
        error,
        ConnectionError::FlippedConnection {
            connector: None,
            node: Some(target_node)
        }
    );
}

#[test]
fn a_shear_free_connection_still_mates_whole_frames() {
    let mut scene = Scene::new();
    let target_node = scene
        .add_empty(
            scene.root(),
            Transform::at(Vec3::new(0.5, 0.2, -0.4))
                .rotate_y_deg(25.0)
                .scale_by(1.5),
        )
        .expect("target inserts");
    let source_node = scene
        .add_empty(
            scene.root(),
            Transform::at(Vec3::new(-1.0, 0.5, 0.8)).rotate_x_deg(15.0),
        )
        .expect("source inserts");
    let source = ConnectorFrame::new(source_node, source_connector()).named("plug");
    let target = ConnectorFrame::new(target_node, target_connector()).named("socket");

    scene
        .connect(
            source.clone(),
            target.clone(),
            ConnectOptions::default().with_explicit_roll_degrees(0.0),
        )
        .expect("connection resolves");

    let source_frame = connector_world(&scene, &source);
    let target_frame = connector_world(&scene, &target);
    for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
        let reached = source_frame.transform_vector3(axis).normalize();
        let wanted = target_frame.transform_vector3(axis).normalize();
        assert!(
            (reached - wanted).length() <= TOLERANCE,
            "axis {axis:?}: expected {wanted:?}, got {reached:?}"
        );
    }
    assert!(
        (source_frame.transform_point3(Vec3::ZERO) - target_frame.transform_point3(Vec3::ZERO))
            .length()
            <= TOLERANCE
    );
}

fn assert_sheared(scene: &Scene, node: NodeKey) {
    let world = scene.world_matrix(node).expect("node resolves");
    let x = world.transform_vector3(Vec3::X).normalize();
    let y = world.transform_vector3(Vec3::Y).normalize();
    assert!(
        x.dot(y).abs() > 0.2,
        "the fixture proves nothing unless the node is sheared: {x:?} {y:?}"
    );
}

fn connector_world(scene: &Scene, connector: &ConnectorFrame) -> Mat4 {
    let host = scene
        .world_matrix(connector.node())
        .expect("connector host resolves");
    let local = connector.local_transform();
    host * Mat4::from_scale_rotation_translation(local.scale, local.rotation, local.translation)
}

/// The Y axis of a frame made perpendicular to its X axis.
fn up_across(frame: Mat4) -> Vec3 {
    let x = frame.transform_vector3(Vec3::X).normalize();
    let y = frame.transform_vector3(Vec3::Y);
    (y - x * y.dot(x)).normalize()
}

/// Checks the connected source connector against the target connector, both
/// read from exact world matrices: the source origin sits on the target
/// frame's `offset` point, the mating axes are parallel with the given sign,
/// and the source's up axis is the target's turned by `roll_degrees` about
/// the mating axis.
fn assert_mated(fixture: &Fixture, offset: Vec3, mating_sign: f32, roll_degrees: f32) {
    assert_frames_mated(
        connector_world(&fixture.scene, &fixture.source),
        connector_world(&fixture.scene, &fixture.target),
        offset,
        mating_sign,
        roll_degrees,
    );
}

/// [`assert_mated`] for connector frames already carried into the world.
fn assert_frames_mated(
    source: Mat4,
    target: Mat4,
    offset: Vec3,
    mating_sign: f32,
    roll_degrees: f32,
) {
    let origin = source.transform_point3(Vec3::ZERO);
    let wanted_origin = target.transform_point3(offset);
    assert!(
        (origin - wanted_origin).length() <= TOLERANCE,
        "connector origin: expected {wanted_origin:?}, got {origin:?}"
    );

    let mating = source.transform_vector3(Vec3::X).normalize();
    let target_mating = target.transform_vector3(Vec3::X).normalize();
    assert!(
        (mating - target_mating * mating_sign).length() <= TOLERANCE,
        "mating axis: expected {:?}, got {mating:?}",
        target_mating * mating_sign
    );

    // The mated frame's up axis: the target's, turned with the alignment
    // (half a turn about up leaves it alone) and then rolled about the
    // mated X axis.
    let mated_x = target_mating * mating_sign;
    let wanted_up =
        glam::Quat::from_axis_angle(mated_x, roll_degrees.to_radians()) * up_across(target);
    let up = up_across(source);
    assert!(
        (up - wanted_up).length() <= TOLERANCE,
        "up axis: expected {wanted_up:?}, got {up:?}"
    );
}
