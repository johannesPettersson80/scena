use std::fmt;

use serde::{Deserialize, Serialize};

use crate::scene::{Quat, Transform, Vec3};

use super::super::{
    default_transform_scale, default_transform_up, is_default_scale, is_default_up, is_zero_f64,
    is_zero_vec3,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SceneRecipeTransformV1 {
    Raw {
        #[serde(default, skip_serializing_if = "is_zero_vec3")]
        translation: [f64; 3],
        rotation: [f64; 4],
        #[serde(
            default = "default_transform_scale",
            skip_serializing_if = "is_default_scale"
        )]
        scale: [f64; 3],
    },
    Trs {
        #[serde(default, skip_serializing_if = "is_zero_vec3")]
        translation: [f64; 3],
        /// Intrinsic rotations composed around local X, then Y, then Z.
        #[serde(default, skip_serializing_if = "is_zero_vec3")]
        rotation_degrees: [f64; 3],
        #[serde(
            default = "default_transform_scale",
            skip_serializing_if = "is_default_scale"
        )]
        scale: [f64; 3],
    },
    LookAt {
        eye: [f64; 3],
        target: SceneRecipeLookAtTargetV1,
        #[serde(
            default = "default_transform_up",
            skip_serializing_if = "is_default_up"
        )]
        up: [f64; 3],
    },
    Center {},
    Ground {
        #[serde(default, skip_serializing_if = "is_zero_f64")]
        plane_y: f64,
    },
    FitToSize {
        size: [f64; 3],
    },
    PlaceOn {
        target: String,
        #[serde(default, skip_serializing_if = "is_zero_vec3")]
        offset: [f64; 3],
    },
    AlignToAnchor {
        anchor: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum SceneRecipeLookAtTargetV1 {
    Node(String),
    Position([f64; 3]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneRecipeTransformConversionError {
    NonFinite { field: &'static str },
    InvalidQuaternion,
    PlacementRequiresScene { kind: &'static str },
}

impl fmt::Display for SceneRecipeTransformConversionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite { field } => {
                write!(
                    formatter,
                    "transform {field} must contain finite f32 values"
                )
            }
            Self::InvalidQuaternion => write!(
                formatter,
                "raw transform rotation must be a finite non-zero quaternion"
            ),
            Self::PlacementRequiresScene { kind } => write!(
                formatter,
                "transform kind '{kind}' requires scene bounds or target resolution"
            ),
        }
    }
}

impl std::error::Error for SceneRecipeTransformConversionError {}

impl SceneRecipeTransformV1 {
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Raw { .. } => "raw",
            Self::Trs { .. } => "trs",
            Self::LookAt { .. } => "look_at",
            Self::Center {} => "center",
            Self::Ground { .. } => "ground",
            Self::FitToSize { .. } => "fit_to_size",
            Self::PlaceOn { .. } => "place_on",
            Self::AlignToAnchor { .. } => "align_to_anchor",
        }
    }
}

impl From<Transform> for SceneRecipeTransformV1 {
    fn from(transform: Transform) -> Self {
        Self::Raw {
            translation: transform.translation.to_array().map(f64::from),
            rotation: transform.rotation.to_array().map(f64::from),
            scale: transform.scale.to_array().map(f64::from),
        }
    }
}

impl TryFrom<&SceneRecipeTransformV1> for Transform {
    type Error = SceneRecipeTransformConversionError;

    fn try_from(transform: &SceneRecipeTransformV1) -> Result<Self, Self::Error> {
        match transform {
            SceneRecipeTransformV1::Raw {
                translation,
                rotation,
                scale,
            } => {
                let translation = finite_vec3(*translation, "translation")?;
                let scale = finite_vec3(*scale, "scale")?;
                let rotation_values = finite_vec4(*rotation, "rotation")?;
                let rotation = Quat::from_array(rotation_values);
                let length_squared = rotation.length_squared();
                if !length_squared.is_finite() || length_squared <= f32::EPSILON {
                    return Err(SceneRecipeTransformConversionError::InvalidQuaternion);
                }
                Ok(Transform {
                    translation,
                    rotation: rotation.normalize(),
                    scale,
                })
            }
            SceneRecipeTransformV1::Trs {
                translation,
                rotation_degrees,
                scale,
            } => {
                let translation = finite_vec3(*translation, "translation")?;
                let rotation_degrees = finite_vec3(*rotation_degrees, "rotation_degrees")?;
                let scale = finite_vec3(*scale, "scale")?;
                Ok(Transform::IDENTITY
                    .with_translation(translation)
                    .rotate_x_deg(rotation_degrees.x)
                    .rotate_y_deg(rotation_degrees.y)
                    .rotate_z_deg(rotation_degrees.z)
                    .with_scale(scale))
            }
            other => Err(
                SceneRecipeTransformConversionError::PlacementRequiresScene { kind: other.kind() },
            ),
        }
    }
}

fn finite_vec3(
    values: [f64; 3],
    field: &'static str,
) -> Result<Vec3, SceneRecipeTransformConversionError> {
    let values = values.map(|value| value as f32);
    values
        .iter()
        .all(|value| value.is_finite())
        .then(|| Vec3::from_array(values))
        .ok_or(SceneRecipeTransformConversionError::NonFinite { field })
}

fn finite_vec4(
    values: [f64; 4],
    field: &'static str,
) -> Result<[f32; 4], SceneRecipeTransformConversionError> {
    let values = values.map(|value| value as f32);
    values
        .iter()
        .all(|value| value.is_finite())
        .then_some(values)
        .ok_or(SceneRecipeTransformConversionError::NonFinite { field })
}
