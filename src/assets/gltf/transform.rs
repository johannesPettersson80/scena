//! Strict glTF node and scena marker transform conversion.

use ::gltf::scene::Transform as GltfTransform;
use glam::Mat4;
use serde_json::Value as JsonValue;

use crate::scene::{Quat, Transform, Vec3};

pub(super) fn from_gltf_transform(transform: GltfTransform) -> Transform {
    let (translation, rotation, scale) = transform.decomposed();
    Transform {
        translation: Vec3::from_array(translation),
        rotation: normalize_quat(Quat::from_xyzw(
            rotation[0],
            rotation[1],
            rotation[2],
            rotation[3],
        )),
        scale: Vec3::from_array(scale),
    }
}

/// Parse the node-style transform embedded in a scena anchor or connector.
///
/// Unlike ordinary glTF nodes, these values live in untyped `extras`, so the
/// upstream glTF validator cannot protect the renderer from malformed values.
/// Every accepted marker transform is therefore finite, invertible, and an
/// exact TRS decomposition. Errors include the full extras path supplied by the
/// caller.
pub(super) fn parse_marker_transform(
    marker: &JsonValue,
    marker_path: &str,
) -> Result<Transform, String> {
    if marker.get("matrix").is_some() {
        for field in ["translation", "rotation", "scale", "forward", "up"] {
            if marker.get(field).is_some() {
                return Err(format!(
                    "{marker_path}.matrix cannot be combined with {marker_path}.{field}"
                ));
            }
        }
        return parse_matrix(marker, marker_path);
    }

    let translation = parse_vec3(marker, "translation", marker_path)?.unwrap_or(Vec3::ZERO);
    let scale = parse_vec3(marker, "scale", marker_path)?.unwrap_or(Vec3::ONE);
    if scale.abs().min_element() <= f32::EPSILON {
        return Err(format!(
            "{marker_path}.scale components must be finite and nonzero"
        ));
    }

    let has_rotation = marker.get("rotation").is_some();
    let has_forward = marker.get("forward").is_some();
    let has_up = marker.get("up").is_some();
    if has_rotation && (has_forward || has_up) {
        return Err(format!(
            "{marker_path}.rotation cannot be combined with {marker_path}.forward or {marker_path}.up"
        ));
    }
    if has_forward != has_up {
        let missing = if has_forward { "up" } else { "forward" };
        return Err(format!(
            "{marker_path}.{missing} is required when the paired basis vector is present"
        ));
    }

    let rotation = if has_rotation {
        parse_rotation(marker, marker_path)?
    } else if has_forward {
        parse_basis_rotation(marker, marker_path)?
    } else {
        Quat::IDENTITY
    };

    Ok(Transform {
        translation,
        rotation,
        scale,
    })
}

fn parse_matrix(marker: &JsonValue, marker_path: &str) -> Result<Transform, String> {
    let field_path = format!("{marker_path}.matrix");
    let values = marker
        .get("matrix")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| format!("{field_path} must be an array of 16 finite numbers"))?;
    if values.len() != 16 {
        return Err(format!(
            "{field_path} must contain exactly 16 finite numbers"
        ));
    }
    let mut raw = [0.0_f32; 16];
    for (index, value) in values.iter().enumerate() {
        raw[index] = finite_f32(value)
            .ok_or_else(|| format!("{field_path}[{index}] must be a finite number"))?;
    }
    if raw[3].abs() > 1.0e-6
        || raw[7].abs() > 1.0e-6
        || raw[11].abs() > 1.0e-6
        || (raw[15] - 1.0).abs() > 1.0e-6
    {
        return Err(format!(
            "{field_path} must be an affine transform with final row [0, 0, 0, 1]"
        ));
    }

    let matrix = Mat4::from_cols_array(&raw);
    let (scale, rotation, translation) = matrix.to_scale_rotation_translation();
    if !translation.is_finite() || !scale.is_finite() || !rotation.is_finite() {
        return Err(format!("{field_path} must have a finite TRS decomposition"));
    }
    if scale.abs().min_element() <= f32::EPSILON {
        return Err(format!(
            "{field_path} must have a decomposable nonzero scale"
        ));
    }
    let recomposed = Mat4::from_scale_rotation_translation(scale, rotation, translation);
    let actual = matrix.to_cols_array();
    let expected = recomposed.to_cols_array();
    let magnitude = actual
        .iter()
        .fold(1.0_f32, |maximum, value| maximum.max(value.abs()));
    if actual
        .iter()
        .zip(expected)
        .any(|(actual, expected)| (actual - expected).abs() > magnitude * 1.0e-5)
    {
        return Err(format!(
            "{field_path} must be decomposable as translation, normalized rotation, and scale without shear"
        ));
    }

    Ok(Transform {
        translation,
        rotation,
        scale,
    })
}

fn parse_rotation(marker: &JsonValue, marker_path: &str) -> Result<Quat, String> {
    let field_path = format!("{marker_path}.rotation");
    let values = marker
        .get("rotation")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| format!("{field_path} must be an array of four finite numbers"))?;
    if values.len() != 4 {
        return Err(format!(
            "{field_path} must contain exactly four finite numbers"
        ));
    }
    let mut raw = [0.0_f32; 4];
    for (index, value) in values.iter().enumerate() {
        raw[index] = finite_f32(value)
            .ok_or_else(|| format!("{field_path}[{index}] must be a finite number"))?;
    }
    let rotation = Quat::from_array(raw);
    let length = rotation.length();
    if !length.is_finite() || length <= f32::EPSILON {
        return Err(format!("{field_path} must be a finite nonzero quaternion"));
    }
    if (length - 1.0).abs() > 1.0e-3 {
        return Err(format!("{field_path} quaternion must be normalized"));
    }
    Ok(rotation.normalize())
}

fn parse_basis_rotation(marker: &JsonValue, marker_path: &str) -> Result<Quat, String> {
    let forward_path = format!("{marker_path}.forward");
    let up_path = format!("{marker_path}.up");
    let forward =
        parse_vec3(marker, "forward", marker_path)?.expect("paired basis presence was checked");
    let authored_up =
        parse_vec3(marker, "up", marker_path)?.expect("paired basis presence was checked");
    let forward = forward
        .try_normalize()
        .ok_or_else(|| format!("{forward_path} must be a finite nonzero vector"))?;
    let authored_up = authored_up
        .try_normalize()
        .ok_or_else(|| format!("{up_path} must be a finite nonzero vector"))?;
    let right = forward
        .cross(authored_up)
        .try_normalize()
        .ok_or_else(|| format!("{up_path} must not be parallel to {forward_path}"))?;
    let up = right
        .cross(forward)
        .try_normalize()
        .ok_or_else(|| format!("{up_path} must form a nondegenerate basis with {forward_path}"))?;
    let rotation = Quat::from_mat3(&glam::Mat3::from_cols(forward, up, right));
    if !rotation.is_finite() || rotation.length_squared() <= f32::EPSILON {
        return Err(format!(
            "{marker_path}.forward/up must produce a finite quaternion"
        ));
    }
    Ok(rotation.normalize())
}

fn parse_vec3(marker: &JsonValue, field: &str, marker_path: &str) -> Result<Option<Vec3>, String> {
    let Some(value) = marker.get(field) else {
        return Ok(None);
    };
    let field_path = format!("{marker_path}.{field}");
    let values = value
        .as_array()
        .ok_or_else(|| format!("{field_path} must be an array of three finite numbers"))?;
    if values.len() != 3 {
        return Err(format!(
            "{field_path} must contain exactly three finite numbers"
        ));
    }
    let mut raw = [0.0_f32; 3];
    for (index, value) in values.iter().enumerate() {
        raw[index] = finite_f32(value)
            .ok_or_else(|| format!("{field_path}[{index}] must be a finite number"))?;
    }
    Ok(Some(Vec3::from_array(raw)))
}

fn finite_f32(value: &JsonValue) -> Option<f32> {
    value
        .as_f64()
        .map(|value| value as f32)
        .filter(|value| value.is_finite())
}

fn normalize_quat(value: Quat) -> Quat {
    let length_sq = value.length_squared();
    if length_sq <= f32::EPSILON || !length_sq.is_finite() {
        return Quat::IDENTITY;
    }
    value.normalize()
}
