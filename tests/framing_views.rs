use scena::{Aabb, FramingOptions, PerspectiveCamera, Scene, Transform, Vec3};

fn unit_bounds() -> Aabb {
    Aabb::new(Vec3::new(-0.5, -0.5, -0.5), Vec3::new(0.5, 0.5, 0.5))
}

fn direction_for(options: FramingOptions) -> Vec3 {
    let width = 800;
    let height = 600;
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default().with_aspect(width as f32 / height as f32),
            Transform::default(),
        )
        .expect("camera inserts");
    let outcome = scene
        .frame_bounds(
            camera,
            unit_bounds(),
            options.fill(0.8).margin_px(16.0).viewport(width, height),
        )
        .expect("framing succeeds");
    (outcome.camera_transform.translation - outcome.target).normalize()
}

fn assert_vec_close(actual: Vec3, expected: Vec3, tolerance: f32) {
    let delta = actual - expected;
    assert!(
        delta.length() <= tolerance,
        "expected {expected:?}, got {actual:?}, delta={delta:?}"
    );
}

#[test]
fn named_cardinal_views_match_world_axes() {
    assert_vec_close(
        direction_for(FramingOptions::new().front()),
        Vec3::new(0.0, 0.0, 1.0),
        1e-5,
    );
    assert_vec_close(
        direction_for(FramingOptions::new().back()),
        Vec3::new(0.0, 0.0, -1.0),
        1e-5,
    );
    assert_vec_close(
        direction_for(FramingOptions::new().right()),
        Vec3::new(1.0, 0.0, 0.0),
        1e-5,
    );
    assert_vec_close(
        direction_for(FramingOptions::new().left()),
        Vec3::new(-1.0, 0.0, 0.0),
        1e-5,
    );
    assert_vec_close(
        direction_for(FramingOptions::new().top()),
        Vec3::new(0.0, 1.0, 0.0),
        1e-5,
    );
    assert_vec_close(
        direction_for(FramingOptions::new().bottom()),
        Vec3::new(0.0, -1.0, 0.0),
        1e-5,
    );
}

#[test]
fn three_quarter_front_views_mirror_across_yz_plane() {
    let right = direction_for(FramingOptions::new().three_quarter_front_right());
    let left = direction_for(FramingOptions::new().three_quarter_front_left());

    assert!((right.x + left.x).abs() <= 1e-5, "{right:?} {left:?}");
    assert!((right.y - left.y).abs() <= 1e-5, "{right:?} {left:?}");
    assert!((right.z - left.z).abs() <= 1e-5, "{right:?} {left:?}");
}

#[test]
fn azimuth_elevation_matches_cardinal_presets() {
    assert_vec_close(
        direction_for(FramingOptions::new().azimuth_elevation(0.0, 0.0)),
        direction_for(FramingOptions::new().front()),
        1e-5,
    );
    assert_vec_close(
        direction_for(FramingOptions::new().azimuth_elevation(90.0, 0.0)),
        direction_for(FramingOptions::new().right()),
        1e-5,
    );
    assert_vec_close(
        direction_for(FramingOptions::new().azimuth_elevation(0.0, 90.0)),
        direction_for(FramingOptions::new().top()),
        1e-5,
    );
    assert_vec_close(
        direction_for(FramingOptions::new().azimuth_elevation(-90.0, 0.0)),
        direction_for(FramingOptions::new().left()),
        1e-5,
    );
    assert_vec_close(
        direction_for(FramingOptions::new().azimuth_elevation(180.0, 0.0)),
        direction_for(FramingOptions::new().back()),
        1e-5,
    );
}

#[test]
fn azimuth_elevation_clamps_elevation_to_vertical_axes() {
    assert_vec_close(
        direction_for(FramingOptions::new().azimuth_elevation(0.0, 120.0)),
        direction_for(FramingOptions::new().top()),
        1e-5,
    );
    assert_vec_close(
        direction_for(FramingOptions::new().azimuth_elevation(0.0, -120.0)),
        direction_for(FramingOptions::new().bottom()),
        1e-5,
    );
}

#[test]
fn azimuth_elevation_decodes_approved_connector_angle() {
    assert_vec_close(
        direction_for(FramingOptions::new().azimuth_elevation(-27.5, 17.8)),
        Vec3::new(-0.4398, 0.3051, 0.8447),
        1e-3,
    );
}

#[test]
fn frame_bounds_right_preset_places_camera_to_right_of_bounds_center() {
    let width = 800;
    let height = 600;
    let bounds = Aabb::new(Vec3::new(2.0, -1.0, -0.5), Vec3::new(4.0, 1.0, 0.5));
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default().with_aspect(width as f32 / height as f32),
            Transform::default(),
        )
        .expect("camera inserts");

    let outcome = scene
        .frame_bounds(
            camera,
            bounds,
            FramingOptions::new()
                .right()
                .fill(0.8)
                .margin_px(16.0)
                .viewport(width, height),
        )
        .expect("right framing succeeds");

    assert_vec_close(outcome.target, bounds.center(), 1e-5);
    assert!(
        outcome.camera_transform.translation.x > outcome.target.x,
        "{outcome:?}"
    );
    assert!(
        (outcome.camera_transform.translation.y - outcome.target.y).abs() <= 1e-5,
        "{outcome:?}"
    );
    assert!(
        (outcome.camera_transform.translation.z - outcome.target.z).abs() <= 1e-5,
        "{outcome:?}"
    );
}
