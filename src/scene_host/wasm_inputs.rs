use super::{SceneHostError, SceneHostErrorCode};
use crate::{Transform, Vec3};

pub(super) fn transform_from_slices(
    translation: &[f32],
    rotation: &[f32],
    scale: &[f32],
) -> Result<Transform, SceneHostError> {
    if translation.len() != 3 {
        return Err(invalid_len("translation", 3, translation.len()));
    }
    if rotation.len() != 4 {
        return Err(invalid_len("rotation", 4, rotation.len()));
    }
    if scale.len() != 3 {
        return Err(invalid_len("scale", 3, scale.len()));
    }
    Ok(transform_from_components(
        [translation[0], translation[1], translation[2]],
        [rotation[0], rotation[1], rotation[2], rotation[3]],
        [scale[0], scale[1], scale[2]],
    ))
}

pub(super) fn transform_from_components(
    translation: [f32; 3],
    rotation: [f32; 4],
    scale: [f32; 3],
) -> Transform {
    Transform {
        translation: Vec3::new(translation[0], translation[1], translation[2]),
        rotation: crate::Quat::from_xyzw(rotation[0], rotation[1], rotation[2], rotation[3]),
        scale: Vec3::new(scale[0], scale[1], scale[2]),
    }
}

pub(super) fn vec3_array_from_slice(
    field: &str,
    values: &[f32],
) -> Result<[f32; 3], SceneHostError> {
    if values.len() != 3 {
        return Err(invalid_len(field, 3, values.len()));
    }
    Ok([values[0], values[1], values[2]])
}

fn invalid_len(field: &str, expected: usize, actual: usize) -> SceneHostError {
    SceneHostError::new(
        SceneHostErrorCode::InvalidInput,
        format!("{field} must contain {expected} values, got {actual}"),
    )
}
