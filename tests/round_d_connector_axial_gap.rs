use scena::{ConnectOptions, ConnectorFrame, Scene, Transform, Vec3};

fn assert_vec3_near(actual: Vec3, expected: Vec3) {
    const EPSILON: f32 = 0.0001;
    assert!(
        (actual.x - expected.x).abs() <= EPSILON
            && (actual.y - expected.y).abs() <= EPSILON
            && (actual.z - expected.z).abs() <= EPSILON,
        "expected {actual:?} to be within {EPSILON} of {expected:?}"
    );
}

#[test]
fn connect_options_axial_gap_offsets_along_target_forward_axis() {
    let mut scene = Scene::new();
    let source = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(1.0, 0.0, 0.0)))
        .expect("source inserts");
    let target = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(5.0, 1.0, 0.0)))
        .expect("target inserts");
    let source_connector = ConnectorFrame::new(source, Transform::IDENTITY).named("source");
    let target_connector = ConnectorFrame::new(target, Transform::IDENTITY).named("target");

    let preview = scene
        .connect(
            source_connector,
            target_connector,
            ConnectOptions::default().with_axial_gap(0.4),
        )
        .expect("connector placement with gap solves");

    assert_vec3_near(
        preview.resolved_transform().translation,
        Vec3::new(5.4, 1.0, 0.0),
    );
    assert_vec3_near(
        scene
            .world_transform(source)
            .expect("source world transform exists")
            .translation,
        Vec3::new(5.4, 1.0, 0.0),
    );
}

#[test]
fn axial_gap_sanitizes_invalid_or_negative_values_to_zero() {
    assert_eq!(
        ConnectOptions::default().with_axial_gap(-1.0).axial_gap(),
        0.0
    );
    assert_eq!(
        ConnectOptions::default()
            .with_axial_gap(f32::INFINITY)
            .axial_gap(),
        0.0
    );
}
