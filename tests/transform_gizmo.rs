use std::f32::consts::FRAC_PI_2;

use scena::{
    Assets, Color, GeometryDesc, GizmoAxis, GizmoConstraint, GizmoMode, GizmoRay, GizmoSpace,
    MaterialDesc, Scene, Transform, TransformGizmo, Vec3,
};

fn assert_vec3_close(actual: Vec3, expected: Vec3) {
    assert!(
        actual.abs_diff_eq(expected, 1.0e-5),
        "expected {expected:?}, got {actual:?}"
    );
}

#[test]
fn transform_gizmo_translates_along_world_axis_from_pointer_rays() {
    let gizmo = TransformGizmo::new(GizmoMode::Translate)
        .with_constraint(GizmoConstraint::Axis(GizmoAxis::X));
    let start = Transform::IDENTITY;
    let start_ray = GizmoRay::new(Vec3::new(1.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0))
        .expect("start ray normalizes");
    let current_ray = GizmoRay::new(Vec3::new(3.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0))
        .expect("current ray normalizes");

    let transformed = gizmo
        .drag_transform(start, start_ray, current_ray)
        .expect("axis drag resolves");

    assert_vec3_close(transformed.translation, Vec3::new(2.0, 0.0, 0.0));
    assert_eq!(transformed.rotation, Transform::IDENTITY.rotation);
    assert_eq!(transformed.scale, Vec3::ONE);
}

#[test]
fn transform_gizmo_translates_on_world_plane_from_pointer_rays() {
    let gizmo = TransformGizmo::new(GizmoMode::Translate)
        .with_constraint(GizmoConstraint::Plane(GizmoAxis::Z));
    let start = Transform::IDENTITY;
    let start_ray = GizmoRay::new(Vec3::new(1.0, 2.0, 5.0), Vec3::new(0.0, 0.0, -1.0))
        .expect("start ray normalizes");
    let current_ray = GizmoRay::new(Vec3::new(3.0, 4.0, 5.0), Vec3::new(0.0, 0.0, -1.0))
        .expect("current ray normalizes");

    let transformed = gizmo
        .drag_transform(start, start_ray, current_ray)
        .expect("plane drag resolves");

    assert_vec3_close(transformed.translation, Vec3::new(2.0, 2.0, 0.0));
}

#[test]
fn transform_gizmo_uses_local_axis_when_requested() {
    let gizmo = TransformGizmo::new(GizmoMode::Translate)
        .with_space(GizmoSpace::Local)
        .with_constraint(GizmoConstraint::Axis(GizmoAxis::Y));
    let start = Transform::IDENTITY.rotate_z_deg(-90.0);
    let start_ray = GizmoRay::new(Vec3::new(1.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0))
        .expect("start ray normalizes");
    let current_ray = GizmoRay::new(Vec3::new(3.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0))
        .expect("current ray normalizes");

    let transformed = gizmo
        .drag_transform(start, start_ray, current_ray)
        .expect("local axis drag resolves");

    assert_vec3_close(transformed.translation, Vec3::new(2.0, 0.0, 0.0));
}

#[test]
fn transform_gizmo_rotates_around_constrained_axis() {
    let gizmo =
        TransformGizmo::new(GizmoMode::Rotate).with_constraint(GizmoConstraint::Axis(GizmoAxis::Z));
    let start = Transform::IDENTITY;
    let start_ray = GizmoRay::new(Vec3::new(1.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0))
        .expect("start ray normalizes");
    let current_ray = GizmoRay::new(Vec3::new(0.0, 1.0, 5.0), Vec3::new(0.0, 0.0, -1.0))
        .expect("current ray normalizes");

    let transformed = gizmo
        .drag_transform(start, start_ray, current_ray)
        .expect("rotation drag resolves");

    let rotated_x = transformed.rotation * Vec3::X;
    assert_vec3_close(rotated_x, Vec3::Y);
}

#[test]
fn transform_gizmo_scales_along_constrained_axis() {
    let gizmo =
        TransformGizmo::new(GizmoMode::Scale).with_constraint(GizmoConstraint::Axis(GizmoAxis::X));
    let start = Transform::IDENTITY;
    let start_ray = GizmoRay::new(Vec3::new(1.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0))
        .expect("start ray normalizes");
    let current_ray = GizmoRay::new(Vec3::new(2.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0))
        .expect("current ray normalizes");

    let transformed = gizmo
        .drag_transform(start, start_ray, current_ray)
        .expect("scale drag resolves");

    assert_vec3_close(transformed.scale, Vec3::new(2.0, 1.0, 1.0));
}

#[test]
fn transform_gizmo_view_aligned_translate_uses_pointer_ray_plane() {
    let gizmo = TransformGizmo::new(GizmoMode::Translate).with_space(GizmoSpace::ViewAligned);
    let start = Transform::IDENTITY;
    let start_ray = GizmoRay::new(Vec3::new(1.0, 2.0, 5.0), Vec3::new(0.0, 0.0, -1.0))
        .expect("start ray normalizes");
    let current_ray = GizmoRay::new(Vec3::new(3.0, 4.0, 5.0), Vec3::new(0.0, 0.0, -1.0))
        .expect("current ray normalizes");

    let transformed = gizmo
        .drag_transform(start, start_ray, current_ray)
        .expect("view plane drag resolves");

    assert_vec3_close(transformed.translation, Vec3::new(2.0, 2.0, 0.0));
}

#[test]
fn transform_gizmo_helper_geometry_is_parented_and_on_top() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    let target = scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(1.0, 2.0, 3.0)))
        .add()
        .expect("target inserts");

    let helpers = TransformGizmo::new(GizmoMode::Translate)
        .add_helpers(&mut scene, &assets, target)
        .expect("helpers insert");

    assert_eq!(helpers.nodes().len(), 3);
    for node in helpers.nodes() {
        assert_eq!(
            scene.node(*node).expect("helper exists").parent(),
            Some(target)
        );
        assert_eq!(scene.helper_on_top(*node), Some(true));
    }
}

#[test]
fn transform_gizmo_invalid_ray_is_rejected() {
    assert!(GizmoRay::new(Vec3::ZERO, Vec3::ZERO).is_none());
    assert!(GizmoRay::new(Vec3::ZERO, Vec3::new(f32::NAN, 0.0, -1.0)).is_none());
}

#[cfg(feature = "scene-host")]
#[test]
fn transform_gizmo_visual_patch_reports_stale_target_handles() {
    use scena::{SceneHostCore, SceneHostErrorCode, VisualPatchResultV1};

    let mut host = SceneHostCore::headless(64, 64).expect("host builds");
    let patch =
        TransformGizmo::new(GizmoMode::Translate).to_visual_patch(999_999, Transform::IDENTITY);

    let result = host.apply_patch(&patch).expect("patch schema is valid");
    assert_eq!(result.applied.transforms, 0);
    assert_eq!(result.failed.len(), 1);
    assert_eq!(result.failed[0].channel, "transforms");
    assert_eq!(result.failed[0].handle, Some(999_999));
    assert_eq!(
        result.failed[0].code,
        SceneHostErrorCode::NodeHandleNotFound
    );

    let json = serde_json::to_string(&result).expect("result serializes");
    let decoded: VisualPatchResultV1 = serde_json::from_str(&json).expect("result decodes");
    assert_eq!(decoded.failed[0].handle, Some(999_999));
}

#[test]
fn transform_gizmo_rotation_keeps_finite_quaternion() {
    let gizmo =
        TransformGizmo::new(GizmoMode::Rotate).with_constraint(GizmoConstraint::Axis(GizmoAxis::Z));
    let start = Transform::IDENTITY;
    let start_ray = GizmoRay::new(Vec3::new(1.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0))
        .expect("start ray normalizes");
    let current_ray = GizmoRay::new(Vec3::new(0.0, -1.0, 5.0), Vec3::new(0.0, 0.0, -1.0))
        .expect("current ray normalizes");

    let transformed = gizmo
        .drag_transform(start, start_ray, current_ray)
        .expect("rotation drag resolves");

    assert!(transformed.rotation.is_finite());
    let rotated_x = transformed.rotation * Vec3::X;
    assert_vec3_close(rotated_x, -Vec3::Y);
    assert!((transformed.rotation.to_axis_angle().1.abs() - FRAC_PI_2).abs() < 1.0e-5);
}
