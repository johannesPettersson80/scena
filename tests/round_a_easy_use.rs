use scena::{Angle, Color, PerspectiveCamera, Quat, Transform, Vec3};

fn assert_close(left: f32, right: f32) {
    assert!(
        (left - right).abs() <= 1.0e-4,
        "expected {left} to be close to {right}"
    );
}

fn assert_color_close(left: Color, right: Color) {
    assert_close(left.r, right.r);
    assert_close(left.g, right.g);
    assert_close(left.b, right.b);
    assert_close(left.a, right.a);
}

#[test]
fn round_a_color_named_constants_and_hex_alias_are_public() {
    assert_eq!(Color::WHITE, Color::from_linear_rgba(1.0, 1.0, 1.0, 1.0));
    assert_eq!(Color::BLACK, Color::from_linear_rgba(0.0, 0.0, 0.0, 1.0));

    assert_color_close(Color::GRAY, Color::from_hex("#808080").unwrap());
    assert_color_close(Color::LIGHT_GRAY, Color::from_hex("#d9dde3").unwrap());
    assert_color_close(Color::DARK_GRAY, Color::from_hex("#30343b").unwrap());
    assert_color_close(Color::CHARCOAL, Color::from_hex("1a1d28").unwrap());
    assert_color_close(Color::STUDIO_BACKDROP, Color::from_hex("#f2f3f5").unwrap());
    assert_color_close(Color::WARM_WHITE, Color::from_hex("#fff4e6").unwrap());
    assert_color_close(Color::COOL_WHITE, Color::from_hex("#eef6ff").unwrap());
    assert_color_close(Color::RED, Color::from_hex("#ff3b30").unwrap());
    assert_color_close(Color::GREEN, Color::from_hex("#34c759").unwrap());
    assert_color_close(Color::BLUE, Color::from_hex("#0a84ff").unwrap());
    assert_color_close(Color::ORANGE, Color::from_hex("#ff9500").unwrap());
    assert_color_close(Color::YELLOW, Color::from_hex("#ffcc00").unwrap());
    assert_color_close(Color::CYAN, Color::from_hex("#32ade6").unwrap());
    assert_color_close(Color::MAGENTA, Color::from_hex("#bf5af2").unwrap());

    assert!(Color::from_hex("#xyzxyz").is_err());
    assert!(Color::from_hex("#1234").is_err());
}

#[test]
fn round_a_color_kelvin_helper_is_clamped_and_ordered() {
    let warm = Color::from_kelvin(2700.0);
    let neutral = Color::from_kelvin(6500.0);

    assert!(
        warm.r > warm.b,
        "2700K should be visibly warmer than blue: {warm:?}"
    );
    assert!(
        neutral.b > warm.b,
        "6500K should have more blue than 2700K: warm={warm:?} neutral={neutral:?}"
    );
    assert_color_close(Color::from_kelvin(1000.0), warm);
    assert_color_close(Color::from_kelvin(f32::NAN), neutral);
}

#[test]
fn round_a_perspective_camera_lens_presets_are_named_degree_surfaces() {
    assert_close(
        PerspectiveCamera::wide_angle().vertical_fov.radians(),
        Angle::from_degrees(84.0).radians(),
    );
    assert_close(
        PerspectiveCamera::standard().vertical_fov.radians(),
        Angle::from_degrees(46.0).radians(),
    );
    assert_close(
        PerspectiveCamera::portrait().vertical_fov.radians(),
        Angle::from_degrees(28.0).radians(),
    );
    assert_close(
        PerspectiveCamera::telephoto().vertical_fov.radians(),
        Angle::from_degrees(18.0).radians(),
    );
    assert_close(
        PerspectiveCamera::standard()
            .with_fov_degrees(60.0)
            .vertical_fov
            .radians(),
        Angle::from_degrees(60.0).radians(),
    );
}

#[test]
fn round_a_transform_looking_at_faces_target_with_requested_up() {
    let transform =
        Transform::at(Vec3::new(0.0, 0.0, 0.0)).looking_at(Vec3::new(0.0, 0.0, -1.0), Vec3::Y);
    assert!(transform.rotation.abs_diff_eq(Quat::IDENTITY, 1.0e-5));

    let rotated = Transform::at(Vec3::ZERO).looking_at(Vec3::new(1.0, 0.0, 0.0), Vec3::Y);
    let forward = rotated.rotation * Vec3::new(0.0, 0.0, -1.0);
    assert!(
        forward.abs_diff_eq(Vec3::X, 1.0e-5),
        "forward vector must face +X, got {forward:?}"
    );
}
