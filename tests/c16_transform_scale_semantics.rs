use scena::{Transform, Vec3};

#[test]
fn scale_by_composes_multiplicatively_without_resetting_other_components() {
    let transform = Transform::IDENTITY
        .with_scale(Vec3::new(2.0, 3.0, 4.0))
        .with_translation(Vec3::new(5.0, 6.0, 7.0))
        .rotate_y_deg(90.0)
        .scale_by(0.5)
        .scale_by(4.0);

    assert!(
        transform
            .scale
            .abs_diff_eq(Vec3::new(4.0, 6.0, 8.0), 1.0e-6)
    );
    assert_eq!(transform.translation, Vec3::new(5.0, 6.0, 7.0));
    let forward = transform.rotation * Vec3::NEG_Z;
    assert!(forward.abs_diff_eq(Vec3::NEG_X, 1.0e-5));
}

#[test]
fn scale_setters_and_composition_are_order_explicit() {
    let replace_after_compose = Transform::IDENTITY
        .with_scale(Vec3::new(2.0, 3.0, 4.0))
        .scale_by(2.0)
        .with_uniform_scale(3.0);
    let compose_after_replace = Transform::IDENTITY
        .with_scale(Vec3::new(2.0, 3.0, 4.0))
        .with_uniform_scale(3.0)
        .scale_by(2.0);

    assert_eq!(replace_after_compose.scale, Vec3::splat(3.0));
    assert_eq!(compose_after_replace.scale, Vec3::splat(6.0));
}

#[test]
fn rotation_helpers_and_scale_by_each_compose_in_call_order() {
    let transform = Transform::IDENTITY
        .rotate_x_deg(90.0)
        .rotate_y_deg(90.0)
        .scale_by(2.0)
        .scale_by(3.0);
    let expected_rotation = Transform::IDENTITY
        .rotate_x_deg(90.0)
        .rotate_y_deg(90.0)
        .rotation;

    assert!((transform.rotation.dot(expected_rotation).abs() - 1.0).abs() <= 1.0e-5);
    assert_eq!(transform.scale, Vec3::splat(6.0));
}
