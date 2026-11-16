//! `PlacementFrame` solves placement verbs against the exact world matrix of a
//! node's parent.
//!
//! Under a sheared frame every result is read back through plain matrix
//! products. In the world frame and under a uniformly scaled parent the verbs
//! must return exactly what the free `placement_*_transform` functions and the
//! translation, rotation, and scale arithmetic around them always returned.

use scena::{
    Aabb, Mat4, PlacementFrame, Scene, Transform, Vec3, placement_align_to_feature_transform,
    placement_center_transform, placement_fit_to_size_transform, placement_ground_transform,
    placement_look_at_transform, placement_place_on_feature_transform,
};

const TOLERANCE: f32 = 2.0e-4;

fn bounds() -> Aabb {
    Aabb::new(Vec3::new(-0.3, -0.1, -0.2), Vec3::new(0.5, 0.2, 0.1))
}

fn local() -> Transform {
    Transform::at(Vec3::new(0.1, 0.3, -0.2))
        .rotate_y_deg(-30.0)
        .rotate_x_deg(15.0)
        .with_scale(Vec3::new(1.2, 0.8, 1.5))
}

/// A rotated node under a rotated, non-uniformly scaled node: sheared.
fn sheared_frame() -> PlacementFrame {
    PlacementFrame::WORLD
        .child(
            Transform::at(Vec3::new(0.3, 0.4, -0.2))
                .rotate_y_deg(25.0)
                .with_scale(Vec3::new(2.5, 1.0, 0.6)),
        )
        .child(
            Transform::at(Vec3::new(0.2, 0.1, 0.0))
                .rotate_x_deg(20.0)
                .rotate_z_deg(40.0),
        )
}

fn uniform_parent() -> Transform {
    Transform::at(Vec3::new(0.3, 0.4, -0.2))
        .rotate_y_deg(25.0)
        .rotate_z_deg(-10.0)
        .scale_by(1.7)
}

fn matrix_of(local: Transform) -> Mat4 {
    Mat4::from_scale_rotation_translation(local.scale, local.rotation, local.translation)
}

/// The local transform that gives a child of `parent` the world transform
/// `world`, written as recipe and command-line placement have always
/// written it.
fn trs_local(parent: Transform, world: Transform) -> Transform {
    let inverse_rotation = parent.rotation.normalize().inverse();
    Transform {
        translation: (inverse_rotation * (world.translation - parent.translation)) / parent.scale,
        rotation: (inverse_rotation * world.rotation).normalize(),
        scale: world.scale / parent.scale,
    }
}

/// The Y axis of a frame made perpendicular to its X axis.
fn up_across(frame: Mat4) -> Vec3 {
    let x = frame.transform_vector3(Vec3::X).normalize();
    let y = frame.transform_vector3(Vec3::Y);
    (y - x * y.dot(x)).normalize()
}

#[test]
fn the_sheared_frame_is_sheared() {
    let world = sheared_frame().world_matrix();
    let x = world.transform_vector3(Vec3::X).normalize();
    let y = world.transform_vector3(Vec3::Y).normalize();
    assert!(x.dot(y).abs() > 0.2, "{x:?} {y:?}");
}

#[test]
fn a_scene_node_frame_is_its_world_matrix() {
    let mut scene = Scene::new();
    let chain = [
        Transform::at(Vec3::new(0.3, 0.4, -0.2))
            .rotate_y_deg(25.0)
            .with_scale(Vec3::new(2.5, 1.0, 0.6)),
        Transform::at(Vec3::new(0.2, 0.1, 0.0)).rotate_z_deg(40.0),
    ];
    let rig = scene
        .add_empty(scene.root(), chain[0])
        .expect("rig inserts");
    let arm = scene.add_empty(rig, chain[1]).expect("arm inserts");

    let frame = scene.placement_frame(arm).expect("arm resolves");

    assert_eq!(
        frame.world_matrix(),
        scene.world_matrix(arm).expect("arm resolves")
    );
    assert!(
        frame
            .world_matrix()
            .abs_diff_eq(matrix_of(chain[0]) * matrix_of(chain[1]), 1.0e-5)
    );
    assert!(
        PlacementFrame::WORLD
            .child(chain[0])
            .child(chain[1])
            .world_matrix()
            .abs_diff_eq(frame.world_matrix(), 1.0e-5),
        "a frame built from the same chain is the same frame"
    );
}

#[test]
fn moving_verbs_are_exact_under_a_sheared_frame() {
    let frame = sheared_frame();
    let target = Vec3::new(0.5, 0.25, -0.5);

    let centred = frame
        .center(bounds(), local(), target)
        .expect("frame inverts");
    let centre = frame.world_bounds(bounds(), centred).center();
    assert!((centre - target).length() <= TOLERANCE, "{centre:?}");

    let grounded = frame
        .ground(bounds(), local(), -0.75)
        .expect("frame inverts");
    let lowest = frame.world_bounds(bounds(), grounded).min.y;
    assert!((lowest + 0.75).abs() <= TOLERANCE, "{lowest}");

    for moved in [centred, grounded] {
        assert_eq!(moved.rotation, local().rotation, "moving keeps rotation");
        assert_eq!(moved.scale, local().scale, "moving keeps scale");
    }
}

#[test]
fn fit_to_size_is_exact_under_a_sheared_frame() {
    let frame = sheared_frame();
    let before = frame.world_bounds(bounds(), local());
    let largest_before = (before.max - before.min).max_element();
    assert!(largest_before > 0.4, "the fixture must start too large");

    let fitted = frame
        .fit_to_size(bounds(), local(), None, Some(0.4))
        .expect("frame inverts");

    let after = frame.world_bounds(bounds(), fitted);
    let largest = (after.max - after.min).max_element();
    assert!((largest - 0.4).abs() <= TOLERANCE, "{largest}");
    assert_eq!(fitted.translation, local().translation);
    assert_eq!(fitted.rotation, local().rotation);
}

#[test]
fn look_at_aims_exactly_under_a_sheared_frame() {
    let frame = sheared_frame();
    let target = Vec3::new(-0.6, 1.4, 0.8);

    let turned = frame
        .look_at(local(), target, Vec3::Y)
        .expect("frame inverts");

    let world = frame.child(turned).world_matrix();
    let position = world.transform_point3(Vec3::ZERO);
    let before = frame
        .child(local())
        .world_matrix()
        .transform_point3(Vec3::ZERO);
    assert_eq!(turned.translation, local().translation);
    assert_eq!(turned.scale, local().scale);
    assert!((position - before).length() <= TOLERANCE);
    let forward = world.transform_vector3(Vec3::NEG_Z).normalize();
    assert!(
        (forward - (target - position).normalize()).length() <= TOLERANCE,
        "{forward:?}"
    );
    // The up axis lies in the plane of the viewing direction and the
    // requested up, on the up side.
    let plane_normal = (target - position).cross(Vec3::Y).normalize();
    let up = world.transform_vector3(Vec3::Y).normalize();
    assert!(up.dot(plane_normal).abs() <= TOLERANCE, "{up:?}");
    assert!(up.y > 0.0, "{up:?}");
}

#[test]
fn features_mate_through_their_exact_frames_under_shear() {
    let frame = sheared_frame();
    let feature = Transform::at(Vec3::new(0.2, -0.1, 0.3))
        .rotate_z_deg(35.0)
        .rotate_y_deg(-20.0);
    // The target feature is sheared in the world as well.
    let target = PlacementFrame::WORLD
        .child(
            Transform::at(Vec3::new(1.0, 0.5, -0.3))
                .rotate_x_deg(30.0)
                .with_scale(Vec3::new(0.7, 1.8, 1.0)),
        )
        .child(Transform::at(Vec3::new(-0.2, 0.3, 0.1)).rotate_z_deg(50.0));
    let target_world = target.world_matrix();
    let feature_frame = PlacementFrame::WORLD.child(feature);

    let aligned = frame
        .align_feature(local(), feature_frame, target)
        .expect("frames invert");

    let feature_world = frame.child(aligned).world_matrix() * matrix_of(feature);
    assert!(
        (feature_world.transform_point3(Vec3::ZERO) - target_world.transform_point3(Vec3::ZERO))
            .length()
            <= TOLERANCE
    );
    let x = feature_world.transform_vector3(Vec3::X).normalize();
    let wanted_x = target_world.transform_vector3(Vec3::X).normalize();
    assert!(
        (x - wanted_x).length() <= TOLERANCE,
        "{x:?} vs {wanted_x:?}"
    );
    assert!((up_across(feature_world) - up_across(target_world)).length() <= TOLERANCE);
    assert_eq!(aligned.scale, local().scale, "aligning keeps scale");

    let placed = frame
        .place_feature_on(local(), feature_frame, target)
        .expect("frames invert");
    let feature_world = frame.child(placed).world_matrix() * matrix_of(feature);
    assert!(
        (feature_world.transform_point3(Vec3::ZERO) - target_world.transform_point3(Vec3::ZERO))
            .length()
            <= TOLERANCE
    );
    assert_eq!(placed.rotation, local().rotation, "placing keeps rotation");
    assert_eq!(placed.scale, local().scale, "placing keeps scale");
}

#[test]
fn the_world_frame_returns_what_the_free_functions_return() {
    let frame = PlacementFrame::WORLD;
    let target = Vec3::new(0.5, 0.25, -0.5);
    let feature = Transform::at(Vec3::new(0.2, -0.1, 0.3)).rotate_z_deg(35.0);
    let target_feature = Transform::at(Vec3::new(1.0, 0.5, -0.3)).rotate_x_deg(30.0);
    // A uniformly scaled node, so its rotated feature is not sheared.
    let node = Transform::at(Vec3::new(0.1, 0.3, -0.2))
        .rotate_y_deg(-30.0)
        .scale_by(1.4);

    assert_eq!(
        frame.center(bounds(), local(), target).expect("solves"),
        placement_center_transform(bounds(), local(), target)
    );
    assert_eq!(
        frame.ground(bounds(), local(), -0.75).expect("solves"),
        placement_ground_transform(bounds(), local(), -0.75)
    );
    assert_eq!(
        frame
            .fit_to_size(bounds(), local(), None, Some(0.4))
            .expect("solves"),
        placement_fit_to_size_transform(bounds(), local(), None, Some(0.4)).expect("solves")
    );
    assert_eq!(
        frame.look_at(local(), target, Vec3::Y).expect("solves"),
        placement_look_at_transform(local(), target, Vec3::Y).expect("solves")
    );
    assert_eq!(
        frame
            .align_feature(
                node,
                PlacementFrame::WORLD.child(feature),
                PlacementFrame::WORLD.child(target_feature)
            )
            .expect("solves"),
        placement_align_to_feature_transform(node, feature, target_feature).expect("solves")
    );
    assert_eq!(
        frame
            .place_feature_on(
                local(),
                PlacementFrame::WORLD.child(feature),
                PlacementFrame::WORLD.child(target_feature)
            )
            .expect("solves"),
        placement_place_on_feature_transform(local(), feature, target_feature).expect("solves")
    );
}

#[test]
fn a_uniform_parent_returns_what_the_trs_arithmetic_returns() {
    let parent = uniform_parent();
    let frame = PlacementFrame::WORLD.child(parent);
    let world = Transform::compose(parent, local());
    let target = Vec3::new(0.5, 0.25, -0.5);

    assert_eq!(
        frame.center(bounds(), local(), target).expect("solves"),
        trs_local(parent, placement_center_transform(bounds(), world, target))
    );
    assert_eq!(
        frame.ground(bounds(), local(), -0.75).expect("solves"),
        trs_local(parent, placement_ground_transform(bounds(), world, -0.75))
    );
    assert_eq!(
        frame
            .fit_to_size(bounds(), local(), None, Some(0.4))
            .expect("solves"),
        trs_local(
            parent,
            placement_fit_to_size_transform(bounds(), world, None, Some(0.4)).expect("solves")
        )
    );
    assert_eq!(
        frame.look_at(local(), target, Vec3::Y).expect("solves"),
        trs_local(
            parent,
            placement_look_at_transform(world, target, Vec3::Y).expect("solves")
        )
    );
}

#[test]
fn a_collapsed_frame_is_reported_instead_of_solved() {
    let frame = PlacementFrame::WORLD
        .child(Transform::IDENTITY.with_scale(Vec3::new(2.0, 0.0, 1.0)))
        .child(Transform::IDENTITY.rotate_z_deg(30.0));

    let error = frame
        .center(bounds(), local(), Vec3::ZERO)
        .expect_err("a frame with a zero scale cannot be inverted");

    assert_eq!(error.code, "non_invertible_parent_transform");
}
