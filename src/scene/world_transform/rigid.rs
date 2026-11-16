//! Rigid frames and TRS fits for placements that shear.
//!
//! A world placement is an affine matrix. Some consumers need more structure
//! than that: a connector mates through a rigid frame, and a placement verb
//! has to hand back a child's local translation, rotation, and scale. Under a
//! rotated, non-uniformly scaled ancestor neither exists exactly, so each
//! helper here states which parts of the matrix it keeps exact and which it
//! lets go, instead of leaving that to a general-purpose decomposition.

use glam::Mat3;

use super::{Quat, Vec3, WorldTransform};

/// Cosine between two matrix axes above which a placement counts as sheared.
const SHEAR_COSINE_TOLERANCE: f32 = 1.0e-5;

/// An orthonormal frame read from an exact placement.
///
/// The origin and the X axis are those of the matrix. The Y axis is the
/// matrix Y axis with its component along X removed, and Z completes a
/// right-handed basis. The skew of Y and Z, which a rigid frame cannot hold,
/// is what is dropped.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RigidFrame {
    pub(crate) origin: Vec3,
    pub(crate) rotation: Quat,
    /// Length of each matrix axis: the scale the rigid frame leaves out.
    pub(crate) axis_lengths: Vec3,
}

impl RigidFrame {
    pub(crate) fn x_axis(self) -> Vec3 {
        self.rotation * Vec3::X
    }

    pub(crate) fn y_axis(self) -> Vec3 {
        self.rotation * Vec3::Y
    }
}

impl WorldTransform {
    /// Composes this placement with a child placement that may be sheared.
    pub(crate) fn then(self, child: Self) -> Self {
        match child.exact_trs() {
            Some(local) => self.child(local),
            None => Self::from_matrix(self.matrix * child.matrix),
        }
    }

    /// Whether the three matrix axes are mutually perpendicular, so the
    /// placement is a rotation with per-axis scale and nothing else.
    pub(crate) fn is_shear_free(self) -> bool {
        let Some([x, y, z]) = self.unit_axes() else {
            return false;
        };
        x.dot(y).abs() <= SHEAR_COSINE_TOLERANCE
            && x.dot(z).abs() <= SHEAR_COSINE_TOLERANCE
            && y.dot(z).abs() <= SHEAR_COSINE_TOLERANCE
    }

    /// Whether every matrix axis has the same length.
    pub(crate) fn has_uniform_axis_lengths(self) -> bool {
        let lengths = self.axis_lengths();
        let largest = lengths.max_element();
        largest.is_finite() && largest - lengths.min_element() <= 1.0e-5 * largest.max(1.0)
    }

    /// Length of each matrix axis.
    pub(crate) fn axis_lengths(self) -> Vec3 {
        Vec3::new(
            self.matrix.matrix3.x_axis.length(),
            self.matrix.matrix3.y_axis.length(),
            self.matrix.matrix3.z_axis.length(),
        )
    }

    /// The rigid frame of this placement, keeping its origin and X axis
    /// exact. `None` when an axis collapses or X and Y are parallel.
    pub(crate) fn rigid_frame(self) -> Option<RigidFrame> {
        let x_axis = Vec3::from(self.matrix.matrix3.x_axis).try_normalize()?;
        let y = Vec3::from(self.matrix.matrix3.y_axis);
        let y_axis = (y - x_axis * y.dot(x_axis)).try_normalize()?;
        let z_axis = x_axis.cross(y_axis);
        let rotation = Quat::from_mat3(&Mat3::from_cols(x_axis, y_axis, z_axis)).normalize();
        let axis_lengths = self.axis_lengths();
        (rotation.is_finite() && axis_lengths.min_element() > f32::EPSILON).then_some(RigidFrame {
            origin: self.translation(),
            rotation,
            axis_lengths,
        })
    }

    fn unit_axes(self) -> Option<[Vec3; 3]> {
        Some([
            Vec3::from(self.matrix.matrix3.x_axis).try_normalize()?,
            Vec3::from(self.matrix.matrix3.y_axis).try_normalize()?,
            Vec3::from(self.matrix.matrix3.z_axis).try_normalize()?,
        ])
    }
}

/// Finds the rotation `r` of a child of `parent` that aims two of the child's
/// local directions in world space.
///
/// `parent * r` carries `local_x` exactly onto the direction `world_x`, and
/// carries `local_y` into the half-plane that `world_x` and `world_y` span, on
/// the `world_y` side. Those are the three degrees of freedom a rotation has,
/// so both hold exactly for every invertible parent, sheared or not. Under a
/// shear-free, uniformly scaled parent the result is the ordinary rigid
/// alignment of the two frames.
///
/// Returns `None` when the parent cannot be inverted or either pair of
/// directions is parallel or zero.
pub(crate) fn aim_rotation(
    parent: WorldTransform,
    local_x: Vec3,
    local_y: Vec3,
    world_x: Vec3,
    world_y: Vec3,
) -> Option<Quat> {
    let local_from_world = parent.inverse()?;
    let world_x = world_x.try_normalize()?;
    let world_y = (world_y - world_x * world_y.dot(world_x)).try_normalize()?;
    let world_z = world_x.cross(world_y);
    let from = local_x.try_normalize()?;
    let onto = local_from_world.transform_vector(world_x).try_normalize()?;
    let aim = Quat::from_rotation_arc(from, onto);

    // Any further turn about `onto` keeps the first direction aimed. Find the
    // one that brings the second direction into the requested half-plane.
    let second = aim * local_y;
    let across = (second - onto * second.dot(onto)).try_normalize()?;
    let along_cos = parent.transform_vector(across);
    let along_sin = parent.transform_vector(onto.cross(across));
    let in_plane = |vector: Vec3| (vector.dot(world_y), vector.dot(world_z));
    let (cos_y, cos_z) = in_plane(along_cos);
    let (sin_y, sin_z) = in_plane(along_sin);
    let mut roll = (-cos_z).atan2(sin_z);
    if roll.cos() * cos_y + roll.sin() * sin_y < 0.0 {
        roll += std::f32::consts::PI;
    }
    let rotation = (Quat::from_axis_angle(onto, roll) * aim).normalize();
    rotation.is_finite().then_some(rotation)
}

#[cfg(test)]
mod tests {
    use super::super::Transform;
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
    fn the_fixture_is_sheared() {
        let parent = sheared_parent();
        assert!(parent.exact_trs().is_none());
        assert!(!parent.is_shear_free());
        assert!(WorldTransform::from(Transform::IDENTITY.rotate_x_deg(30.0)).is_shear_free());
    }

    #[test]
    fn rigid_frame_keeps_the_origin_and_the_x_axis_of_a_sheared_placement() {
        let placement = sheared_parent();
        let frame = placement.rigid_frame().expect("placement has a frame");

        assert_eq!(frame.origin, placement.translation());
        assert_parallel(
            frame.x_axis(),
            placement.transform_vector(Vec3::X),
            "x axis",
        );
        // Y stays in the plane the matrix X and Y axes span, on the Y side.
        let matrix_y = placement.transform_vector(Vec3::Y);
        let plane_normal = placement
            .transform_vector(Vec3::X)
            .cross(matrix_y)
            .normalize();
        assert!(frame.y_axis().dot(plane_normal).abs() <= 1.0e-5);
        assert!(frame.y_axis().dot(matrix_y) > 0.0);
        assert!(frame.x_axis().dot(frame.y_axis()).abs() <= 1.0e-6);
    }

    #[test]
    fn aim_rotation_aims_both_directions_exactly_under_a_sheared_parent() {
        let parent = sheared_parent();
        let local_x = Vec3::new(0.3, 0.9, -0.2);
        let local_y = Vec3::new(-0.5, 0.1, 0.8);
        let world_x = Vec3::new(0.2, -0.7, 0.4);
        let world_y = Vec3::new(0.9, 0.3, 0.1);

        let rotation =
            aim_rotation(parent, local_x, local_y, world_x, world_y).expect("aim is solvable");

        let reached_x = parent.transform_vector(rotation * local_x);
        let reached_y = parent.transform_vector(rotation * local_y);
        assert_parallel(reached_x, world_x, "first direction");
        let plane_normal = world_x.cross(world_y).normalize();
        assert!(
            reached_y.normalize().dot(plane_normal).abs() <= 2.0e-5,
            "the second direction must lie in the requested plane: {reached_y:?}"
        );
        let world_y_across = world_y - world_x * world_y.dot(world_x) / world_x.length_squared();
        assert!(reached_y.dot(world_y_across) > 0.0);
    }

    #[test]
    fn aim_rotation_is_the_rigid_alignment_under_a_uniform_parent() {
        let parent = WorldTransform::from(
            Transform::at(Vec3::new(1.0, 2.0, 3.0))
                .rotate_x_deg(35.0)
                .with_uniform_scale(2.0),
        );
        let target = Quat::from_euler(glam::EulerRot::XYZ, 0.4, -0.9, 1.3);

        let rotation = aim_rotation(parent, Vec3::X, Vec3::Y, target * Vec3::X, target * Vec3::Y)
            .expect("aim is solvable");

        let world_rotation = parent.trs().rotation * rotation;
        assert!(world_rotation.dot(target).abs() >= 1.0 - 1.0e-5);
    }
}
