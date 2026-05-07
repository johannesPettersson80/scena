use serde_json::Value as JsonValue;

use crate::scene::{Quat, Transform, Vec3};

pub(super) fn parse_node_transform(node: &JsonValue) -> Transform {
    if let Some(transform) = node
        .get("matrix")
        .and_then(JsonValue::as_array)
        .and_then(|values| matrix_transform(values))
    {
        return transform;
    }
    Transform {
        translation: vec3_field(node, "translation", Vec3::ZERO),
        rotation: quat_field(node, "rotation", Quat::IDENTITY),
        scale: vec3_field(node, "scale", Vec3::ONE),
    }
}

fn matrix_transform(values: &[JsonValue]) -> Option<Transform> {
    if values.len() != 16 {
        return None;
    }
    let values = values
        .iter()
        .map(|value| value.as_f64().map(|value| value as f32))
        .collect::<Option<Vec<_>>>()?;
    let column_x = Vec3::new(values[0], values[1], values[2]);
    let column_y = Vec3::new(values[4], values[5], values[6]);
    let column_z = Vec3::new(values[8], values[9], values[10]);
    let scale = Vec3::new(
        length_vec3(column_x),
        length_vec3(column_y),
        length_vec3(column_z),
    );
    let rotation_x = scale_or_zero(column_x, scale.x.recip());
    let rotation_y = scale_or_zero(column_y, scale.y.recip());
    let rotation_z = scale_or_zero(column_z, scale.z.recip());
    Some(Transform {
        translation: Vec3::new(values[12], values[13], values[14]),
        rotation: quat_from_rotation_columns(rotation_x, rotation_y, rotation_z),
        scale,
    })
}

fn vec3_field(node: &JsonValue, field: &str, fallback: Vec3) -> Vec3 {
    let Some(values) = node.get(field).and_then(JsonValue::as_array) else {
        return fallback;
    };
    Vec3::new(
        array_f32(values, 0).unwrap_or(fallback.x),
        array_f32(values, 1).unwrap_or(fallback.y),
        array_f32(values, 2).unwrap_or(fallback.z),
    )
}

fn quat_field(node: &JsonValue, field: &str, fallback: Quat) -> Quat {
    let Some(values) = node.get(field).and_then(JsonValue::as_array) else {
        return fallback;
    };
    Quat {
        x: array_f32(values, 0).unwrap_or(fallback.x),
        y: array_f32(values, 1).unwrap_or(fallback.y),
        z: array_f32(values, 2).unwrap_or(fallback.z),
        w: array_f32(values, 3).unwrap_or(fallback.w),
    }
}

fn array_f32(values: &[JsonValue], index: usize) -> Option<f32> {
    values
        .get(index)
        .and_then(JsonValue::as_f64)
        .map(|value| value as f32)
}

fn quat_from_rotation_columns(x: Vec3, y: Vec3, z: Vec3) -> Quat {
    let m00 = x.x;
    let m01 = y.x;
    let m02 = z.x;
    let m10 = x.y;
    let m11 = y.y;
    let m12 = z.y;
    let m20 = x.z;
    let m21 = y.z;
    let m22 = z.z;
    let trace = m00 + m11 + m22;
    let quat = if trace > 0.0 {
        let scale = (trace + 1.0).sqrt() * 2.0;
        Quat {
            w: 0.25 * scale,
            x: (m21 - m12) / scale,
            y: (m02 - m20) / scale,
            z: (m10 - m01) / scale,
        }
    } else if m00 > m11 && m00 > m22 {
        let scale = (1.0 + m00 - m11 - m22).sqrt() * 2.0;
        Quat {
            w: (m21 - m12) / scale,
            x: 0.25 * scale,
            y: (m01 + m10) / scale,
            z: (m02 + m20) / scale,
        }
    } else if m11 > m22 {
        let scale = (1.0 + m11 - m00 - m22).sqrt() * 2.0;
        Quat {
            w: (m02 - m20) / scale,
            x: (m01 + m10) / scale,
            y: 0.25 * scale,
            z: (m12 + m21) / scale,
        }
    } else {
        let scale = (1.0 + m22 - m00 - m11).sqrt() * 2.0;
        Quat {
            w: (m10 - m01) / scale,
            x: (m02 + m20) / scale,
            y: (m12 + m21) / scale,
            z: 0.25 * scale,
        }
    };
    normalize_quat(quat)
}

fn normalize_quat(value: Quat) -> Quat {
    let length =
        (value.x * value.x + value.y * value.y + value.z * value.z + value.w * value.w).sqrt();
    if length <= f32::EPSILON || !length.is_finite() {
        return Quat::IDENTITY;
    }
    Quat {
        x: value.x / length,
        y: value.y / length,
        z: value.z / length,
        w: value.w / length,
    }
}

fn length_vec3(value: Vec3) -> f32 {
    (value.x * value.x + value.y * value.y + value.z * value.z).sqrt()
}

fn scale_or_zero(value: Vec3, factor: f32) -> Vec3 {
    if !factor.is_finite() {
        return Vec3::ZERO;
    }
    Vec3::new(value.x * factor, value.y * factor, value.z * factor)
}
