use crate::animation::AnimationTarget;
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
        let unit_scale = self.source_units.meters_per_unit();
        let converted_basis = self
            .source_coordinate_system
            .convert_connector_transform(transform);
        Transform {
            translation: self
                .source_coordinate_system
                .convert_vec3(scale_vec3(transform.translation, unit_scale)),
            rotation: converted_basis.rotation,
            scale: self
                .source_coordinate_system
                .convert_scale(scale_vec3(transform.scale, unit_scale)),
        }
    }

    pub(super) fn convert_animation_vec3(self, target: AnimationTarget, value: Vec3) -> Vec3 {
        let unit_scale = self.source_units.meters_per_unit();
        match target {
            AnimationTarget::Translation => self
                .source_coordinate_system
                .convert_vec3(scale_vec3(value, unit_scale)),
            AnimationTarget::Scale => self
                .source_coordinate_system
                .convert_scale(scale_vec3(value, unit_scale)),
            AnimationTarget::Rotation | AnimationTarget::Weights => value,
        }
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
        multiply_quat(basis, multiply_quat(rotation, inverse_unit_quat(basis)))
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

const fn scale_vec3(value: Vec3, scale: f32) -> Vec3 {
    Vec3::new(value.x * scale, value.y * scale, value.z * scale)
}

fn multiply_quat(left: Quat, right: Quat) -> Quat {
    normalize_quat(Quat::from_xyzw(left.w * right.x + left.x * right.w + left.y * right.z - left.z * right.y, left.w * right.y - left.x * right.z + left.y * right.w + left.z * right.x, left.w * right.z + left.x * right.y - left.y * right.x + left.z * right.w, left.w * right.w - left.x * right.x - left.y * right.y - left.z * right.z))
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
    Quat::from_xyzw(value.x * inverse_length, value.y * inverse_length, value.z * inverse_length, value.w * inverse_length)
}
