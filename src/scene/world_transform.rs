//! Exact world-space placement of a scene node.
//!
//! A parent with non-uniform or negative scale and a rotated child produce a
//! sheared world transform, which translation/rotation/scale cannot represent.
//! The scene therefore resolves every node to an affine matrix and keeps a TRS
//! view next to it for the APIs that still speak [`Transform`].
//!
//! Hierarchies whose composition *is* exactly representable as TRS (uniform
//! ancestor scale, or unrotated children) keep the composed TRS and apply it
//! with the same arithmetic the renderer has always used, so their output is
//! bit-for-bit unchanged. Everything else goes through the matrix.

use glam::{Affine3A, DVec3, Mat3, Mat3A, Vec3A};

use super::math::{Mat4, Quat, Transform, Vec3};

#[cfg(any(test, feature = "scene-host"))]
mod fit;
mod rigid;
#[cfg(feature = "scene-host")]
pub(crate) use fit::fit_local_transform;
pub(crate) use rigid::aim_rotation;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct WorldTransform {
    /// Exact world-from-local affine matrix.
    matrix: Affine3A,
    /// The composed TRS when `trs_exact`, otherwise the decomposition of
    /// `matrix`, which drops shear.
    trs: Transform,
    /// `trs` reproduces `matrix`, so TRS arithmetic may stand in for it.
    trs_exact: bool,
    /// Inverse transpose of the linear part, rescaled so it stays well
    /// conditioned. Only its direction is meaningful; callers normalize.
    normal: Mat3A,
    /// Directions transform by `trs.rotation` alone (exact TRS with uniform
    /// non-negative scale).
    rotation_only: bool,
    /// The linear part has a negative determinant and reverses winding.
    mirrored: bool,
}

/// The inverse of a [`WorldTransform`], for carrying world-space points and
/// directions into a node's local space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LocalFromWorld {
    matrix: Affine3A,
}

impl WorldTransform {
    pub(crate) const IDENTITY: Self = Self {
        matrix: Affine3A::IDENTITY,
        trs: Transform::IDENTITY,
        trs_exact: true,
        normal: Mat3A::IDENTITY,
        rotation_only: true,
        mirrored: false,
    };

    /// A placement described exactly by one TRS transform.
    pub(crate) fn from_trs(trs: Transform) -> Self {
        let matrix = trs_matrix(trs);
        let mirrored = determinant(matrix.matrix3) < 0.0;
        let rotation_only = is_uniform(trs.scale) && trs.scale.x >= 0.0;
        let normal = if rotation_only {
            rotation_scale_matrix(trs.rotation, Vec3::ONE)
        } else {
            normal_matrix(matrix.matrix3, mirrored, trs.rotation)
        };
        Self {
            matrix,
            trs,
            trs_exact: true,
            normal,
            rotation_only,
            mirrored,
        }
    }

    /// A placement described by an arbitrary affine matrix.
    pub(crate) fn from_matrix(matrix: Affine3A) -> Self {
        let mirrored = determinant(matrix.matrix3) < 0.0;
        let trs = decompose(matrix, mirrored);
        Self {
            matrix,
            trs,
            trs_exact: false,
            normal: normal_matrix(matrix.matrix3, mirrored, trs.rotation),
            rotation_only: false,
            mirrored,
        }
    }

    /// Composes this placement with a child's local TRS transform.
    pub(crate) fn child(self, local: Transform) -> Self {
        if self.trs_exact && trs_composition_is_exact(self.trs, local) {
            return Self::from_trs(Transform::compose(self.trs, local));
        }
        Self::from_matrix(self.matrix * trs_matrix(local))
    }

    /// The exact world-from-local matrix.
    pub(crate) const fn matrix(self) -> Affine3A {
        self.matrix
    }

    pub(crate) fn to_mat4(self) -> Mat4 {
        Mat4::from(self.matrix)
    }

    /// The TRS view: exact when [`Self::exact_trs`] is `Some`, otherwise the
    /// decomposition of the matrix with shear dropped.
    pub(crate) const fn trs(self) -> Transform {
        self.trs
    }

    /// The TRS transform when it reproduces the matrix exactly.
    pub(crate) const fn exact_trs(self) -> Option<Transform> {
        if self.trs_exact { Some(self.trs) } else { None }
    }

    /// World-space position of the node origin. Exact for every hierarchy.
    pub(crate) const fn translation(self) -> Vec3 {
        self.trs.translation
    }

    /// Whether the placement reverses triangle winding.
    pub(crate) const fn is_mirrored(self) -> bool {
        self.mirrored
    }

    /// The rotation that alone carries directions into world space, when the
    /// placement is a rotation with uniform non-negative scale.
    pub(crate) const fn uniform_rotation(self) -> Option<Quat> {
        if self.rotation_only {
            Some(self.trs.rotation)
        } else {
            None
        }
    }

    /// Inverse transpose of the linear part, up to a positive factor.
    pub(crate) const fn normal_matrix(self) -> Mat3A {
        self.normal
    }

    /// The same placement with the node origin moved to the world origin.
    pub(crate) fn without_translation(self) -> Self {
        let mut linear = self;
        linear.matrix.translation = Vec3A::ZERO;
        linear.trs.translation = Vec3::ZERO;
        linear
    }

    pub(crate) fn transform_point(self, point: Vec3) -> Vec3 {
        if self.trs_exact {
            rotate_vec3(self.trs.rotation, point * self.trs.scale) + self.trs.translation
        } else {
            self.matrix.transform_point3(point)
        }
    }

    /// Applies the linear part only, so offsets and tangents keep their
    /// length under scale.
    pub(crate) fn transform_vector(self, vector: Vec3) -> Vec3 {
        if self.trs_exact {
            rotate_vec3(self.trs.rotation, vector * self.trs.scale)
        } else {
            self.matrix.transform_vector3(vector)
        }
    }

    /// Carries a surface normal into world space. The result is not unit
    /// length; callers normalize with their own degenerate fallback.
    pub(crate) fn transform_normal(self, normal: Vec3) -> Vec3 {
        if self.rotation_only {
            rotate_vec3(self.trs.rotation, normal)
        } else {
            Vec3::from(self.normal * Vec3A::from(normal))
        }
    }

    /// Carries a world-space surface normal back into local space, up to a
    /// positive factor. Inverts [`Self::transform_normal`].
    pub(crate) fn inverse_transform_normal(self, world_normal: Vec3) -> Vec3 {
        Vec3::from(self.matrix.matrix3.transpose() * Vec3A::from(world_normal))
    }

    /// The inverse placement, or `None` when the matrix is singular.
    pub(crate) fn inverse(self) -> Option<LocalFromWorld> {
        let matrix = self.matrix.as_daffine3();
        let determinant = matrix.matrix3.determinant();
        if !determinant.is_finite() || determinant.abs() <= f64::MIN_POSITIVE {
            return None;
        }
        let inverse = matrix.inverse();
        inverse.is_finite().then(|| LocalFromWorld {
            matrix: Affine3A::from_mat3_translation(
                inverse.matrix3.as_mat3(),
                inverse.translation.as_vec3(),
            ),
        })
    }
}

impl From<Transform> for WorldTransform {
    fn from(trs: Transform) -> Self {
        Self::from_trs(trs)
    }
}

impl LocalFromWorld {
    pub(crate) const fn matrix(self) -> Affine3A {
        self.matrix
    }

    pub(crate) fn transform_point(self, world_point: Vec3) -> Vec3 {
        self.matrix.transform_point3(world_point)
    }

    pub(crate) fn transform_vector(self, world_vector: Vec3) -> Vec3 {
        self.matrix.transform_vector3(world_vector)
    }
}

/// Splits a matrix into translation, rotation, and scale.
///
/// Translation is exact. Scale is the length of each basis column, with the
/// sign of a mirrored matrix carried on X. Rotation keeps the forward (Z) axis
/// exact and the up (Y) axis as close as an orthonormal basis allows, so a
/// camera or light under a sheared parent still faces along its matrix. The
/// result reproduces the matrix only when the matrix has no shear.
fn decompose(matrix: Affine3A, mirrored: bool) -> Transform {
    let x = Vec3::from(matrix.matrix3.x_axis);
    let y = Vec3::from(matrix.matrix3.y_axis);
    let z = Vec3::from(matrix.matrix3.z_axis);
    let sign = if mirrored { -1.0 } else { 1.0 };
    let forward = z
        .try_normalize()
        .or_else(|| (x.cross(y) * sign).try_normalize())
        .unwrap_or(Vec3::Z);
    let right = y
        .cross(forward)
        .try_normalize()
        .or_else(|| (x * sign - forward * (x * sign).dot(forward)).try_normalize())
        .unwrap_or_else(|| forward.any_orthonormal_vector());
    let up = forward.cross(right);
    let rotation = Quat::from_mat3(&Mat3::from_cols(right, up, forward));
    Transform {
        translation: Vec3::from(matrix.translation),
        rotation: normalized_or_identity(rotation),
        scale: Vec3::new(x.length() * sign, y.length(), z.length()),
    }
}

/// TRS composition is exact when the child rotation commutes with the parent
/// scale: the parent scale is uniform, or the child is unrotated.
fn trs_composition_is_exact(parent: Transform, child: Transform) -> bool {
    is_uniform(parent.scale)
        || (child.rotation.x == 0.0 && child.rotation.y == 0.0 && child.rotation.z == 0.0)
}

fn is_uniform(scale: Vec3) -> bool {
    scale.x == scale.y && scale.y == scale.z
}

fn normalized_or_identity(rotation: Quat) -> Quat {
    let length_squared = rotation.length_squared();
    if length_squared <= f32::EPSILON || !length_squared.is_finite() {
        Quat::IDENTITY
    } else {
        rotation * length_squared.sqrt().recip()
    }
}

/// The matrix of one TRS transform. A zero or non-finite rotation is treated
/// as identity and any other rotation is normalized, matching
/// [`Transform::compose`].
fn trs_matrix(trs: Transform) -> Affine3A {
    let linear = rotation_scale_matrix(trs.rotation, trs.scale);
    Affine3A::from_cols(
        linear.x_axis,
        linear.y_axis,
        linear.z_axis,
        Vec3A::from(trs.translation),
    )
}

/// Rotation followed by per-axis scale, as basis columns.
///
/// Kept as the exact expression the renderer has always uploaded as its
/// per-draw matrix, so exact-TRS hierarchies reach the GPU unchanged.
fn rotation_scale_matrix(rotation: Quat, scale: Vec3) -> Mat3A {
    let length_squared = rotation.x * rotation.x
        + rotation.y * rotation.y
        + rotation.z * rotation.z
        + rotation.w * rotation.w;
    let (rx, ry, rz, rw) = if length_squared <= f32::EPSILON || !length_squared.is_finite() {
        (0.0, 0.0, 0.0, 1.0)
    } else {
        let inverse_length = length_squared.sqrt().recip();
        (
            rotation.x * inverse_length,
            rotation.y * inverse_length,
            rotation.z * inverse_length,
            rotation.w * inverse_length,
        )
    };
    let xx = rx * rx;
    let yy = ry * ry;
    let zz = rz * rz;
    let xy = rx * ry;
    let xz = rx * rz;
    let yz = ry * rz;
    let wx = rw * rx;
    let wy = rw * ry;
    let wz = rw * rz;
    Mat3A::from_cols(
        Vec3A::new(
            (1.0 - 2.0 * (yy + zz)) * scale.x,
            (2.0 * (xy + wz)) * scale.x,
            (2.0 * (xz - wy)) * scale.x,
        ),
        Vec3A::new(
            (2.0 * (xy - wz)) * scale.y,
            (1.0 - 2.0 * (xx + zz)) * scale.y,
            (2.0 * (yz + wx)) * scale.y,
        ),
        Vec3A::new(
            (2.0 * (xz + wy)) * scale.z,
            (2.0 * (yz - wx)) * scale.z,
            (1.0 - 2.0 * (xx + yy)) * scale.z,
        ),
    )
}

fn determinant(linear: Mat3A) -> f64 {
    linear
        .x_axis
        .as_dvec3()
        .dot(linear.y_axis.as_dvec3().cross(linear.z_axis.as_dvec3()))
}

/// Inverse transpose of `linear`, up to a positive factor.
///
/// Built from the cofactor matrix so a singular (flattening) matrix still
/// yields the normal of the plane it collapses onto, and evaluated in `f64` so
/// very small scales do not underflow. The determinant's sign is reapplied so
/// mirrored geometry keeps outward-facing normals.
fn normal_matrix(linear: Mat3A, mirrored: bool, fallback_rotation: Quat) -> Mat3A {
    let x = linear.x_axis.as_dvec3();
    let y = linear.y_axis.as_dvec3();
    let z = linear.z_axis.as_dvec3();
    let cofactors: [DVec3; 3] = [y.cross(z), z.cross(x), x.cross(y)];
    let largest = cofactors
        .iter()
        .map(|column| column.length())
        .fold(0.0_f64, f64::max);
    if !largest.is_finite() || largest <= f64::MIN_POSITIVE {
        return Mat3A::from_quat(normalized_or_identity(fallback_rotation));
    }
    let factor = if mirrored {
        -largest.recip()
    } else {
        largest.recip()
    };
    Mat3A::from_cols(
        (cofactors[0] * factor).as_vec3a(),
        (cofactors[1] * factor).as_vec3a(),
        (cofactors[2] * factor).as_vec3a(),
    )
}

/// Rotates `vector` by `rotation`, normalizing the quaternion first and
/// treating a zero or non-finite quaternion as identity.
///
/// This is the arithmetic every transform site in the crate has always used.
/// It is kept verbatim, rather than replaced by `Quat * Vec3`, because the two
/// round differently and exact-TRS hierarchies must not change output.
pub(crate) fn rotate_vec3(rotation: Quat, vector: Vec3) -> Vec3 {
    let length_squared = rotation.x * rotation.x
        + rotation.y * rotation.y
        + rotation.z * rotation.z
        + rotation.w * rotation.w;
    if length_squared <= f32::EPSILON || !length_squared.is_finite() {
        return vector;
    }
    let inverse_length = length_squared.sqrt().recip();
    let qx = rotation.x * inverse_length;
    let qy = rotation.y * inverse_length;
    let qz = rotation.z * inverse_length;
    let qw = rotation.w * inverse_length;
    let tx = 2.0 * (qy * vector.z - qz * vector.y);
    let ty = 2.0 * (qz * vector.x - qx * vector.z);
    let tz = 2.0 * (qx * vector.y - qy * vector.x);
    Vec3::new(
        vector.x + qw * tx + (qy * tz - qz * ty),
        vector.y + qw * ty + (qz * tx - qx * tz),
        vector.z + qw * tz + (qx * ty - qy * tx),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f32 = 1.0e-5;

    fn scaled(scale: Vec3) -> Transform {
        Transform::IDENTITY.with_scale(scale)
    }

    fn rotated_z(degrees: f32) -> Transform {
        Transform::IDENTITY.rotate_z_deg(degrees)
    }

    #[test]
    fn exact_chains_keep_the_composed_trs_and_its_arithmetic() {
        let parent = Transform::at(Vec3::new(1.0, 2.0, 3.0))
            .rotate_y_deg(30.0)
            .with_uniform_scale(2.0);
        let child = Transform::at(Vec3::new(-1.0, 0.5, 0.25))
            .rotate_x_deg(70.0)
            .with_scale(Vec3::new(1.0, 2.0, 3.0));
        let world = WorldTransform::IDENTITY.child(parent).child(child);
        let composed = Transform::compose(Transform::compose(Transform::IDENTITY, parent), child);

        assert_eq!(world.exact_trs(), Some(composed));
        assert_eq!(world.trs(), composed);
        let point = Vec3::new(0.3, -0.7, 1.9);
        assert_eq!(
            world.transform_point(point),
            rotate_vec3(composed.rotation, point * composed.scale) + composed.translation,
            "an exact chain must keep the historical TRS arithmetic bit for bit"
        );
        assert!(
            world
                .matrix()
                .transform_point3(point)
                .abs_diff_eq(world.transform_point(point), TOLERANCE),
            "the matrix and the TRS view must describe the same placement"
        );
    }

    #[test]
    fn a_rotated_child_of_a_non_uniform_parent_is_placed_by_the_matrix() {
        let world = WorldTransform::IDENTITY
            .child(scaled(Vec3::new(2.0, 1.0, 1.0)))
            .child(rotated_z(45.0));
        let diagonal = std::f32::consts::FRAC_1_SQRT_2;

        assert_eq!(world.exact_trs(), None);
        assert!(
            world
                .transform_point(Vec3::X)
                .abs_diff_eq(Vec3::new(2.0 * diagonal, diagonal, 0.0), TOLERANCE)
        );
        assert!(
            world
                .transform_point(Vec3::Y)
                .abs_diff_eq(Vec3::new(-2.0 * diagonal, diagonal, 0.0), TOLERANCE)
        );
        assert!(!world.is_mirrored());
    }

    #[test]
    fn an_unrotated_child_of_a_non_uniform_parent_stays_exact() {
        let world = WorldTransform::IDENTITY
            .child(scaled(Vec3::new(2.0, 1.0, 0.5)).rotate_y_deg(20.0))
            .child(Transform::at(Vec3::new(1.0, 2.0, 3.0)).with_scale(Vec3::new(3.0, 1.0, 1.0)));

        let exact = world
            .exact_trs()
            .expect("no rotation meets the parent scale");
        assert_eq!(exact.scale, Vec3::new(6.0, 1.0, 0.5));
    }

    #[test]
    fn exactness_is_not_recovered_below_a_sheared_node() {
        let sheared = WorldTransform::IDENTITY
            .child(scaled(Vec3::new(2.0, 1.0, 1.0)))
            .child(rotated_z(30.0));
        let below = sheared.child(Transform::at(Vec3::new(0.0, 1.0, 0.0)));

        assert_eq!(below.exact_trs(), None);
        assert!(
            below
                .translation()
                .abs_diff_eq(sheared.transform_point(Vec3::new(0.0, 1.0, 0.0)), TOLERANCE)
        );
    }

    #[test]
    fn normals_follow_the_inverse_transpose() {
        // The plane x + y = c has normal (1, 1, 0). Stretching X by two turns
        // it into x / 2 + y = c, whose normal is (1, 2, 0).
        let stretched = WorldTransform::from_trs(scaled(Vec3::new(2.0, 1.0, 1.0)));
        let normal = stretched
            .transform_normal(Vec3::new(1.0, 1.0, 0.0))
            .normalize();
        assert!(normal.abs_diff_eq(Vec3::new(1.0, 2.0, 0.0).normalize(), TOLERANCE));

        // A sheared hierarchy keeps normals perpendicular to transformed tangents.
        let sheared = WorldTransform::IDENTITY
            .child(scaled(Vec3::new(3.0, 1.0, 0.5)))
            .child(rotated_z(40.0).rotate_x_deg(25.0));
        let (tangent, bitangent) = (Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
        let world_normal = sheared
            .transform_normal(tangent.cross(bitangent))
            .normalize();
        assert!(world_normal.dot(sheared.transform_vector(tangent)).abs() <= TOLERANCE);
        assert!(world_normal.dot(sheared.transform_vector(bitangent)).abs() <= TOLERANCE);
    }

    #[test]
    fn uniform_scale_carries_directions_by_rotation_alone() {
        let transform = Transform::at(Vec3::new(4.0, 5.0, 6.0))
            .rotate_y_deg(35.0)
            .with_uniform_scale(7.0);
        let world = WorldTransform::from_trs(transform);

        assert_eq!(world.uniform_rotation(), Some(transform.rotation));
        assert_eq!(
            world.transform_normal(Vec3::X),
            rotate_vec3(transform.rotation, Vec3::X),
            "uniform scale must keep the historical rotation-only normal bit for bit"
        );
    }

    #[test]
    fn mirrored_placements_report_it_and_keep_normals_outward() {
        let mirrored = WorldTransform::from_trs(scaled(Vec3::new(-1.0, 1.0, 1.0)));
        assert!(mirrored.is_mirrored());
        assert_eq!(mirrored.uniform_rotation(), None);
        // The +X face of a box lands on the -X side and must face -X there.
        assert!(
            mirrored
                .transform_normal(Vec3::X)
                .normalize()
                .abs_diff_eq(Vec3::NEG_X, TOLERANCE)
        );
        assert!(
            mirrored
                .transform_normal(Vec3::Y)
                .normalize()
                .abs_diff_eq(Vec3::Y, TOLERANCE)
        );

        let point_reflection = WorldTransform::from_trs(scaled(Vec3::splat(-2.0)));
        assert!(point_reflection.is_mirrored());
        assert!(
            point_reflection
                .transform_normal(Vec3::Z)
                .normalize()
                .abs_diff_eq(Vec3::NEG_Z, TOLERANCE)
        );

        let double_mirror = mirrored.child(scaled(Vec3::new(1.0, -1.0, 1.0)));
        assert!(!double_mirror.is_mirrored());
    }

    #[test]
    fn a_flattening_scale_keeps_the_normal_of_the_plane_it_collapses_onto() {
        let flattened =
            WorldTransform::from_trs(scaled(Vec3::new(1.0, 1.0, 0.0)).rotate_x_deg(90.0));
        // Local Z is squashed, then turned onto world -Y.
        let normal = flattened
            .transform_normal(Vec3::new(0.3, 0.2, 1.0))
            .normalize();
        assert!(normal.abs_diff_eq(Vec3::NEG_Y, TOLERANCE), "got {normal:?}");
        assert_eq!(flattened.inverse(), None);
    }

    #[test]
    fn inverse_normal_and_inverse_round_trip_a_sheared_placement() {
        let world = WorldTransform::IDENTITY
            .child(Transform::at(Vec3::new(1.0, -2.0, 0.5)).with_scale(Vec3::new(-2.0, 1.0, 3.0)))
            .child(rotated_z(55.0).rotate_y_deg(-20.0));
        let local_from_world = world.inverse().expect("the placement is invertible");
        let point = Vec3::new(0.4, 1.3, -0.8);

        assert!(
            local_from_world
                .transform_point(world.transform_point(point))
                .abs_diff_eq(point, TOLERANCE)
        );
        assert!(
            local_from_world
                .transform_vector(world.transform_vector(point))
                .abs_diff_eq(point, TOLERANCE)
        );
        let normal = Vec3::new(0.2, -0.5, 0.8).normalize();
        let round_trip = world
            .transform_normal(world.inverse_transform_normal(normal))
            .normalize();
        assert!(
            round_trip.abs_diff_eq(normal, TOLERANCE),
            "got {round_trip:?}"
        );
    }

    #[test]
    fn the_decomposition_keeps_position_and_forward_axis_of_a_sheared_matrix() {
        let world = WorldTransform::IDENTITY
            .child(Transform::at(Vec3::new(3.0, 0.0, 0.0)).with_scale(Vec3::new(1.0, 1.0, 4.0)))
            .child(Transform::at(Vec3::new(0.0, 1.0, 2.0)).rotate_x_deg(35.0));
        let decomposed = world.trs();

        assert_eq!(
            decomposed.translation,
            Vec3::from(world.matrix().translation)
        );
        assert!(
            (decomposed.rotation * Vec3::Z)
                .abs_diff_eq(world.transform_vector(Vec3::Z).normalize(), TOLERANCE)
        );
        assert!((decomposed.rotation.length() - 1.0).abs() <= TOLERANCE);
    }

    #[test]
    fn a_shear_free_matrix_decomposes_exactly_with_the_mirror_on_x() {
        // Non-uniform mirrored scale under a half turn has no shear.
        let world = WorldTransform::IDENTITY
            .child(scaled(Vec3::new(2.0, -1.0, 3.0)))
            .child(rotated_z(180.0));
        assert_eq!(world.exact_trs(), None);

        let rebuilt = WorldTransform::from_trs(world.trs());
        assert!(rebuilt.matrix().abs_diff_eq(world.matrix(), TOLERANCE));
        assert!(world.trs().scale.x < 0.0 && world.trs().scale.y > 0.0);
    }

    #[test]
    fn degenerate_rotations_are_treated_as_identity_like_transform_compose() {
        let degenerate = Transform {
            rotation: Quat::from_xyzw(0.0, 0.0, 0.0, 0.0),
            ..Transform::at(Vec3::new(1.0, 2.0, 3.0))
        };
        let world = WorldTransform::from_trs(degenerate);
        assert!(world.matrix().is_finite());
        assert_eq!(world.transform_point(Vec3::X), Vec3::new(2.0, 2.0, 3.0));

        let unnormalized = Transform {
            rotation: Quat::from_xyzw(0.0, 0.0, 2.0, 2.0),
            ..Transform::IDENTITY
        };
        assert!(
            WorldTransform::from_trs(unnormalized)
                .matrix()
                .transform_point3(Vec3::X)
                .abs_diff_eq(Vec3::Y, TOLERANCE)
        );
    }
}
