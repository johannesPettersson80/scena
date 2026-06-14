use std::fs;

use scena::{
    Transform, Vec3, placement_align_to_feature_transform, placement_look_at_transform,
    placement_place_on_feature_transform,
};

#[test]
fn placement_result_golden_fixture_matches_live_schema() {
    let text = fs::read_to_string("tests/assets/stable-contracts/placement_result.v1.json")
        .expect("placement fixture reads");
    let result: scena::ScenePlacementResultV1 =
        serde_json::from_str(&text).expect("placement fixture deserializes");

    assert_eq!(result.schema, scena::SCENE_PLACEMENT_RESULT_SCHEMA_V1);
    assert!(result.ok);
    assert_eq!(
        serde_json::to_value(&result).expect("placement result serializes"),
        serde_json::from_str::<serde_json::Value>(&text).expect("fixture parses")
    );
}

#[test]
fn placement_look_at_orients_negative_z_toward_target() {
    let current = Transform::at(Vec3::new(2.0, 0.0, 0.0)).scale_by(3.0);
    let transform =
        placement_look_at_transform(current, Vec3::new(2.0, 0.0, -4.0), Vec3::Y).unwrap();

    let forward = transform.rotation * Vec3::new(0.0, 0.0, -1.0);
    assert_vec3(forward, Vec3::new(0.0, 0.0, -1.0));
    assert_vec3(transform.translation, current.translation);
    assert_vec3(transform.scale, current.scale);
}

#[test]
fn placement_align_to_feature_matches_source_frame_to_target_frame() {
    let current = Transform::at(Vec3::new(5.0, 0.0, 0.0));
    let source_feature = Transform::at(Vec3::new(1.0, 0.0, 0.0));
    let target_feature = Transform::at(Vec3::new(0.0, 2.0, 0.0)).rotate_z_deg(90.0);

    let transform =
        placement_align_to_feature_transform(current, source_feature, target_feature).unwrap();
    let aligned_feature = Transform::compose(transform, source_feature);

    assert_vec3(aligned_feature.translation, target_feature.translation);
    let aligned_forward = aligned_feature.rotation * Vec3::new(0.0, 0.0, -1.0);
    let target_forward = target_feature.rotation * Vec3::new(0.0, 0.0, -1.0);
    assert_vec3(aligned_forward, target_forward);
}

#[test]
fn placement_place_on_feature_snaps_positions_without_rotating_source() {
    let current = Transform::at(Vec3::new(5.0, 0.0, 0.0)).rotate_y_deg(45.0);
    let source_feature = Transform::at(Vec3::new(1.0, 0.0, 0.0));
    let target_feature = Transform::at(Vec3::new(0.0, 2.0, 0.0)).rotate_z_deg(90.0);

    let transform =
        placement_place_on_feature_transform(current, source_feature, target_feature).unwrap();
    let placed_feature = Transform::compose(transform, source_feature);

    assert_vec3(placed_feature.translation, target_feature.translation);
    assert_eq!(transform.rotation, current.rotation);
    assert_eq!(transform.scale, current.scale);
}

#[test]
fn placement_align_rejects_non_invertible_source_feature() {
    let error = placement_align_to_feature_transform(
        Transform::IDENTITY,
        Transform {
            scale: Vec3::ZERO,
            ..Transform::IDENTITY
        },
        Transform::IDENTITY,
    )
    .unwrap_err();

    assert_eq!(error.code, "non_invertible_feature");
}

fn assert_vec3(actual: Vec3, expected: Vec3) {
    assert!(
        actual.abs_diff_eq(expected, 1.0e-5),
        "expected {expected:?}, got {actual:?}"
    );
}
