//! Stage C2: node-transform conversion now delegates to the `gltf`
//! crate's typed `scene::Transform::decomposed()`, which handles both
//! TRS-decomposed and 4×4-matrix node transforms uniformly. Scena's
//! basis-rotation extension (forward/up extras) is preserved.

use ::gltf::scene::Transform as GltfTransform;
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

/// Node-style transform parsed from JSON (used by anchors and
/// connectors, whose embedded TRS lives inside `extras`).
pub(super) fn parse_node_transform(node: &JsonValue) -> Transform {
    if let Some(values) = node.get("matrix").and_then(JsonValue::as_array)
        && let Some(transform) = matrix_transform(values)
    {
        return transform;
    }
    Transform {
        translation: vec3_field(node, "translation", Vec3::ZERO),
        rotation: quat_field(node, "rotation")
            .or_else(|| basis_rotation(node))
            .unwrap_or(Quat::IDENTITY),
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
    let mut matrix = [[0.0_f32; 4]; 4];
    for (column, chunk) in matrix.iter_mut().zip(values.chunks_exact(4)) {
        column[0] = chunk[0];
        column[1] = chunk[1];
        column[2] = chunk[2];
        column[3] = chunk[3];
    }
    Some(from_gltf_transform(GltfTransform::Matrix { matrix }))
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

fn quat_field(node: &JsonValue, field: &str) -> Option<Quat> {
    let values = node.get(field).and_then(JsonValue::as_array)?;
    Some(normalize_quat(Quat::from_xyzw(
        array_f32(values, 0)?,
        array_f32(values, 1)?,
        array_f32(values, 2)?,
        array_f32(values, 3)?,
    )))
}

fn array_f32(values: &[JsonValue], index: usize) -> Option<f32> {
    values
        .get(index)
        .and_then(JsonValue::as_f64)
        .map(|value| value as f32)
}

fn basis_rotation(node: &JsonValue) -> Option<Quat> {
    let forward = optional_vec3_field(node, "forward")?.try_normalize()?;
    let authored_up = optional_vec3_field(node, "up")?.try_normalize()?;
    let right = forward.cross(authored_up).try_normalize()?;
    let up = right.cross(forward).try_normalize()?;
    Some(Quat::from_mat3(&glam::Mat3::from_cols(forward, up, right)))
}

fn optional_vec3_field(node: &JsonValue, field: &str) -> Option<Vec3> {
    let values = node.get(field).and_then(JsonValue::as_array)?;
    Some(Vec3::new(
        array_f32(values, 0)?,
        array_f32(values, 1)?,
        array_f32(values, 2)?,
    ))
}

fn normalize_quat(value: Quat) -> Quat {
    let length_sq = value.length_squared();
    if length_sq <= f32::EPSILON || !length_sq.is_finite() {
        return Quat::IDENTITY;
    }
    value.normalize()
}
