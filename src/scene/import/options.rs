use crate::animation::{AnimationInterpolation, AnimationTarget};
use crate::scene::{Angle, Quat, Transform, Vec3};

use super::{ImportOptions, SourceCoordinateSystem, SourceUnits};

impl ImportOptions {
    pub const fn gltf_default() -> Self {
        Self {
            source_units: SourceUnits::Meters,
            source_coordinate_system: SourceCoordinateSystem::GltfYUpRightHanded,
        }
    }

    pub const fn source_units(self) -> SourceUnits {
        self.source_units
    }

    pub const fn with_source_units(mut self, units: SourceUnits) -> Self {
        self.source_units = units;
        self
    }

    pub const fn source_coordinate_system(self) -> SourceCoordinateSystem {
        self.source_coordinate_system
    }

    pub const fn with_source_coordinate_system(
        mut self,
        coordinate_system: SourceCoordinateSystem,
    ) -> Self {
        self.source_coordinate_system = coordinate_system;
        self
    }

    pub(super) fn convert_transform(self, transform: Transform) -> Transform {
        let converted_basis = self
            .source_coordinate_system
            .convert_connector_transform(transform);
        Transform {
            translation: self
                .source_coordinate_system
                .convert_vec3(transform.translation),
            rotation: converted_basis.rotation,
            scale: self.source_coordinate_system.convert_scale(transform.scale),
        }
    }

    pub(super) fn convert_animation_vec3(self, target: AnimationTarget, value: Vec3) -> Vec3 {
        match target {
            AnimationTarget::Translation => self.source_coordinate_system.convert_vec3(value),
            AnimationTarget::Scale => self.source_coordinate_system.convert_scale(value),
            AnimationTarget::Rotation | AnimationTarget::Weights => value,
        }
    }

    pub(super) fn convert_animation_rotation(
        self,
        interpolation: AnimationInterpolation,
        output_index: usize,
        value: Quat,
    ) -> Quat {
        let is_cubic_tangent =
            interpolation == AnimationInterpolation::CubicSpline && output_index % 3 != 1;
        if is_cubic_tangent {
            self.source_coordinate_system
                .convert_rotation_derivative(value)
        } else {
            self.source_coordinate_system.convert_rotation(value)
        }
    }

    pub(super) fn unit_root_transform(self) -> Option<Transform> {
        (self.source_units != SourceUnits::Meters)
            .then(|| Transform::IDENTITY.scale_by(self.source_units.meters_per_unit()))
    }
}

impl SourceUnits {
    pub const fn meters_per_unit(self) -> f32 {
        match self {
            Self::Meters => 1.0,
            Self::Centimeters => 0.01,
            Self::Millimeters => 0.001,
            Self::Inches => 0.0254,
            Self::Feet => 0.3048,
        }
    }
}

impl SourceCoordinateSystem {
    pub const fn convert_position(self, value: Vec3) -> Vec3 {
        self.convert_vec3(value)
    }

    pub const fn convert_scale_vector(self, value: Vec3) -> Vec3 {
        self.convert_scale(value)
    }

    pub fn convert_connector_transform(self, transform: Transform) -> Transform {
        if self.has_negative_determinant() {
            return transform;
        }
        Transform {
            translation: self.convert_vec3(transform.translation),
            rotation: self.convert_rotation(transform.rotation),
            scale: self.convert_scale(transform.scale),
        }
    }

    pub const fn has_negative_determinant(self) -> bool {
        matches!(self, Self::YUpLeftHanded | Self::ZUpLeftHanded)
    }

    pub const fn is_left_handed(self) -> bool {
        self.has_negative_determinant()
    }

    const fn convert_vec3(self, value: Vec3) -> Vec3 {
        match self {
            Self::GltfYUpRightHanded => value,
            Self::YUpLeftHanded => Vec3::new(value.x, value.y, -value.z),
            Self::ZUpRightHanded => Vec3::new(value.x, value.z, -value.y),
            Self::ZUpLeftHanded => Vec3::new(value.x, value.z, value.y),
        }
    }

    const fn convert_scale(self, value: Vec3) -> Vec3 {
        match self {
            Self::GltfYUpRightHanded | Self::YUpLeftHanded => value,
            Self::ZUpRightHanded | Self::ZUpLeftHanded => Vec3::new(value.x, value.z, value.y),
        }
    }

    fn convert_rotation(self, rotation: Quat) -> Quat {
        let Some(basis) = self.basis_rotation() else {
            return rotation;
        };
        normalize_quat(conjugate_quat(basis, rotation))
    }

    fn convert_rotation_derivative(self, derivative: Quat) -> Quat {
        let Some(basis) = self.basis_rotation() else {
            return derivative;
        };
        conjugate_quat(basis, derivative)
    }

    fn basis_rotation(self) -> Option<Quat> {
        match self {
            Self::GltfYUpRightHanded => None,
            Self::YUpLeftHanded | Self::ZUpLeftHanded => None,
            Self::ZUpRightHanded => Some(Quat::from_axis_angle(
                Vec3::new(1.0, 0.0, 0.0),
                Angle::from_degrees(-90.0).radians(),
            )),
        }
    }
}

fn conjugate_quat(basis: Quat, value: Quat) -> Quat {
    multiply_quat_raw(basis, multiply_quat_raw(value, inverse_unit_quat(basis)))
}

fn multiply_quat_raw(left: Quat, right: Quat) -> Quat {
    Quat::from_xyzw(
        left.w * right.x + left.x * right.w + left.y * right.z - left.z * right.y,
        left.w * right.y - left.x * right.z + left.y * right.w + left.z * right.x,
        left.w * right.z + left.x * right.y - left.y * right.x + left.z * right.w,
        left.w * right.w - left.x * right.x - left.y * right.y - left.z * right.z,
    )
}

fn inverse_unit_quat(rotation: Quat) -> Quat {
    Quat::from_xyzw(-rotation.x, -rotation.y, -rotation.z, rotation.w)
}

fn normalize_quat(value: Quat) -> Quat {
    let length_squared =
        value.x * value.x + value.y * value.y + value.z * value.z + value.w * value.w;
    if length_squared <= f32::EPSILON || !length_squared.is_finite() {
        return Quat::IDENTITY;
    }
    let inverse_length = length_squared.sqrt().recip();
    Quat::from_xyzw(
        value.x * inverse_length,
        value.y * inverse_length,
        value.z * inverse_length,
        value.w * inverse_length,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::{
        AnimationClipKey, AnimationInterpolation, AnimationOutput, AnimationSourceChannel,
        AnimationSourceClip,
    };
    use crate::scene::NodeKey;
    use slotmap::Key;

    #[test]
    fn z_up_rotation_animation_uses_the_static_transform_basis() {
        let options = ImportOptions::gltf_default()
            .with_source_coordinate_system(SourceCoordinateSystem::ZUpRightHanded);
        let source_rotation = Transform::IDENTITY.rotate_z_deg(90.0).rotation;
        let clip = AnimationSourceClip::try_new(
            Some("z-up-rotation".to_owned()),
            vec![AnimationSourceChannel::new(
                0,
                AnimationTarget::Rotation,
                vec![0.0, 1.0],
                AnimationOutput::Quat(vec![Quat::IDENTITY, source_rotation]),
                AnimationInterpolation::Linear,
            )],
            1.0,
        )
        .expect("source animation is valid");
        let rebound = clip
            .rebind_imported_many(
                AnimationClipKey::fresh(),
                |_, _| vec![NodeKey::null()],
                |target, value| options.convert_animation_vec3(target, value),
                |interpolation, index, value| {
                    options.convert_animation_rotation(interpolation, index, value)
                },
            )
            .expect("converted animation remains valid");
        let actual = rebound.channels()[0]
            .sample_quat(1.0)
            .expect("rotation key samples");
        let expected = options
            .convert_transform(Transform {
                rotation: source_rotation,
                ..Transform::IDENTITY
            })
            .rotation;

        assert_same_orientation(actual, expected);
    }

    #[test]
    fn z_up_cubic_rotation_converts_derivative_tangents_without_normalizing_them() {
        let options = ImportOptions::gltf_default()
            .with_source_coordinate_system(SourceCoordinateSystem::ZUpRightHanded);
        let end = Transform::IDENTITY.rotate_z_deg(90.0).rotation;
        let source = AnimationSourceClip::try_new(
            Some("z-up-cubic-rotation".to_owned()),
            vec![AnimationSourceChannel::new(
                0,
                AnimationTarget::Rotation,
                vec![0.0, 1.0],
                AnimationOutput::Quat(vec![
                    Quat::from_xyzw(0.0, 0.0, 0.0, 0.0),
                    Quat::IDENTITY,
                    Quat::from_xyzw(0.0, 0.0, 0.5, 0.0),
                    Quat::from_xyzw(0.0, 0.0, 0.5, 0.0),
                    end,
                    Quat::from_xyzw(0.0, 0.0, 0.0, 0.0),
                ]),
                AnimationInterpolation::CubicSpline,
            )],
            1.0,
        )
        .expect("source animation is valid");
        let source_rebound = source
            .try_rebind(
                AnimationClipKey::fresh(),
                |_| Some(NodeKey::null()),
                |_, value| value,
            )
            .expect("source animation remains valid");
        let converted = source
            .rebind_imported_many(
                AnimationClipKey::fresh(),
                |_, _| vec![NodeKey::null()],
                |target, value| options.convert_animation_vec3(target, value),
                |interpolation, index, value| {
                    options.convert_animation_rotation(interpolation, index, value)
                },
            )
            .expect("converted cubic animation remains valid");

        for time in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let source_sample = source_rebound.channels()[0]
                .sample_quat(time)
                .expect("source cubic rotation samples");
            let expected = options
                .convert_transform(Transform {
                    rotation: source_sample,
                    ..Transform::IDENTITY
                })
                .rotation;
            let actual = converted.channels()[0]
                .sample_quat(time)
                .expect("converted cubic rotation samples");
            assert_same_orientation(actual, expected);
        }

        let tangent = options.convert_animation_rotation(
            AnimationInterpolation::CubicSpline,
            2,
            Quat::from_xyzw(0.0, 0.0, 0.5, 0.0),
        );
        let norm = (tangent.x * tangent.x
            + tangent.y * tangent.y
            + tangent.z * tangent.z
            + tangent.w * tangent.w)
            .sqrt();
        assert!(
            (norm - 0.5).abs() <= 0.0001,
            "tangent norm changed: {tangent:?}"
        );
        assert!(
            tangent.y.abs() >= 0.4999,
            "Z derivative must map to Y: {tangent:?}"
        );
    }

    fn assert_same_orientation(actual: Quat, expected: Quat) {
        let dot = actual.x * expected.x
            + actual.y * expected.y
            + actual.z * expected.z
            + actual.w * expected.w;
        assert!(
            dot.abs() >= 0.9999,
            "quaternion orientations differ: actual={actual:?} expected={expected:?} dot={dot}"
        );
    }
}
