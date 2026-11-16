//! The closest a translation, rotation, and scale child can come to a
//! desired world placement under a parent that may shear.

use glam::Affine3A;

use super::rigid::aim_rotation;
use super::{Transform, Vec3, WorldTransform};

/// The local transform whose placement under `parent` comes as close to
/// `desired` as translation, rotation, and scale allow.
///
/// The origin, the direction of the X axis, and the length of every axis are
/// exact. The Y axis lies in the plane `desired` spans with its X and Y axes.
/// When `parent^-1 * desired` has no shear this reproduces `desired` exactly;
/// otherwise the skew of Y and Z is what a TRS child cannot carry.
///
/// Returns `None` when the parent cannot be inverted or `desired` collapses.
pub(crate) fn fit_local_transform(
    parent: WorldTransform,
    desired: WorldTransform,
) -> Option<Transform> {
    let local_from_world = parent.inverse()?;
    let desired_matrix: Affine3A = desired.matrix();
    let desired_x = Vec3::from(desired_matrix.matrix3.x_axis);
    let desired_y = Vec3::from(desired_matrix.matrix3.y_axis);
    let rotation = aim_rotation(parent, Vec3::X, Vec3::Y, desired_x, desired_y)?;
    let lengths = desired.axis_lengths();
    let axis_scale = |axis: Vec3, length: f32| {
        let reached = parent.transform_vector(rotation * axis).length();
        (reached > f32::EPSILON).then(|| length / reached)
    };
    // A rotation cannot mirror, so a handedness the parent does not already
    // supply is carried by the Z scale, which no kept axis depends on.
    let mirror = if parent.is_mirrored() == desired.is_mirrored() {
        1.0
    } else {
        -1.0
    };
    let fitted = Transform {
        translation: local_from_world.transform_point(desired.translation()),
        rotation,
        scale: Vec3::new(
            axis_scale(Vec3::X, lengths.x)?,
            axis_scale(Vec3::Y, lengths.y)?,
            axis_scale(Vec3::Z, lengths.z)? * mirror,
        ),
    };
    (fitted.translation.is_finite() && fitted.scale.is_finite()).then_some(fitted)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sheared_parent() -> WorldTransform {
        WorldTransform::IDENTITY
            .child(
                Transform::at(Vec3::new(0.5, -1.0, 2.0))
                    .rotate_y_deg(25.0)
                    .with_scale(Vec3::new(2.5, 1.0, 0.6)),
            )
            .child(Transform::at(Vec3::new(0.2, 0.3, -0.1)).rotate_z_deg(40.0))
    }

    fn assert_parallel(actual: Vec3, expected: Vec3, what: &str) {
        let actual = actual.normalize();
        let expected = expected.normalize();
        assert!(
            (actual - expected).length() <= 2.0e-5,
            "{what}: expected {expected:?}, got {actual:?}"
        );
    }

    #[test]
    fn fit_reproduces_a_desired_placement_that_a_trs_child_can_reach() {
        let parent = sheared_parent();
        let local = Transform::at(Vec3::new(0.4, -0.2, 0.7))
            .rotate_x_deg(50.0)
            .rotate_y_deg(-20.0)
            .with_scale(Vec3::new(0.5, 1.5, 2.0));
        let desired = parent.child(local);

        let fitted = fit_local_transform(parent, desired).expect("fit is solvable");
        let reached = parent.child(fitted).to_mat4();

        assert!(
            reached.abs_diff_eq(desired.to_mat4(), 2.0e-5),
            "expected {:?}, got {reached:?}",
            desired.to_mat4()
        );
    }

    #[test]
    fn fit_keeps_origin_x_direction_and_axis_lengths_when_shear_cannot_be_matched() {
        let parent = sheared_parent();
        let desired = WorldTransform::IDENTITY
            .child(Transform::IDENTITY.with_scale(Vec3::new(1.0, 3.0, 0.5)))
            .child(Transform::at(Vec3::new(1.0, 0.5, -0.5)).rotate_z_deg(55.0));
        assert!(!desired.is_shear_free());

        let fitted = fit_local_transform(parent, desired).expect("fit is solvable");
        let reached = parent.child(fitted);

        assert!((reached.translation() - desired.translation()).length() <= 2.0e-5);
        assert_parallel(
            reached.transform_vector(Vec3::X),
            desired.transform_vector(Vec3::X),
            "x axis",
        );
        assert!(
            (reached.axis_lengths() - desired.axis_lengths())
                .abs()
                .max_element()
                <= 2.0e-5
        );
        let plane_normal = desired
            .transform_vector(Vec3::X)
            .cross(desired.transform_vector(Vec3::Y))
            .normalize();
        assert!(
            reached
                .transform_vector(Vec3::Y)
                .normalize()
                .dot(plane_normal)
                .abs()
                <= 2.0e-5
        );
    }
}
