use super::{SceneHostError, SceneHostErrorCode};
use crate::{Quat, Transform, Vec3};

pub(super) const TRANSFORM_COMPONENT_COUNT: usize = 10;
const QUATERNION_NORM_TOLERANCE: f32 = 1.0e-2;

#[cfg(target_arch = "wasm32")]
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
    transform_from_components(
        [translation[0], translation[1], translation[2]],
        [rotation[0], rotation[1], rotation[2], rotation[3]],
        [scale[0], scale[1], scale[2]],
    )
}

pub(super) fn transform_from_component_array(
    components: [f32; TRANSFORM_COMPONENT_COUNT],
) -> Result<Transform, SceneHostError> {
    transform_from_components(
        [components[0], components[1], components[2]],
        [components[3], components[4], components[5], components[6]],
        [components[7], components[8], components[9]],
    )
}

pub(super) fn transform_from_components(
    translation: [f32; 3],
    rotation: [f32; 4],
    scale: [f32; 3],
) -> Result<Transform, SceneHostError> {
    validate_finite_components("translation", &translation)?;
    validate_finite_components("rotation", &rotation)?;
    validate_finite_components("scale", &scale)?;
    let rotation = normalize_scene_host_quaternion(rotation)?;
    Ok(Transform {
        translation: Vec3::new(translation[0], translation[1], translation[2]),
        rotation,
        scale: Vec3::new(scale[0], scale[1], scale[2]),
    })
}

pub(super) fn validate_transform(transform: Transform) -> Result<Transform, SceneHostError> {
    transform_from_components(
        transform.translation.to_array(),
        transform.rotation.to_array(),
        transform.scale.to_array(),
    )
}

#[cfg(target_arch = "wasm32")]
pub(super) fn vec3_array_from_slice(
    field: &str,
    values: &[f32],
) -> Result<[f32; 3], SceneHostError> {
    if values.len() != 3 {
        return Err(invalid_len(field, 3, values.len()));
    }
    let values = [values[0], values[1], values[2]];
    validate_finite_components(field, &values)?;
    Ok(values)
}

fn normalize_scene_host_quaternion(rotation: [f32; 4]) -> Result<Quat, SceneHostError> {
    let length_squared = rotation
        .into_iter()
        .map(|component| component * component)
        .sum::<f32>();
    if !length_squared.is_finite() || length_squared <= f32::EPSILON {
        return Err(invalid_input(
            "rotation quaternion length must be finite and non-zero",
        ));
    }
    let length = length_squared.sqrt();
    if (length - 1.0).abs() > QUATERNION_NORM_TOLERANCE {
        return Err(invalid_input(format!(
            "rotation quaternion length must be within {QUATERNION_NORM_TOLERANCE} of 1.0, got {length}"
        )));
    }
    let inverse_length = length.recip();
    Ok(Quat::from_xyzw(
        rotation[0] * inverse_length,
        rotation[1] * inverse_length,
        rotation[2] * inverse_length,
        rotation[3] * inverse_length,
    ))
}

fn validate_finite_components(field: &str, values: &[f32]) -> Result<(), SceneHostError> {
    if values.iter().all(|value| value.is_finite()) {
        return Ok(());
    }
    Err(invalid_input(format!(
        "{field} must contain only finite values"
    )))
}

#[cfg(target_arch = "wasm32")]
fn invalid_len(field: &str, expected: usize, actual: usize) -> SceneHostError {
    invalid_input(format!(
        "{field} must contain {expected} values, got {actual}"
    ))
}

fn invalid_input(message: impl Into<String>) -> SceneHostError {
    SceneHostError::new(SceneHostErrorCode::InvalidInput, message.into())
}
