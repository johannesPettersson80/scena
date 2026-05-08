use super::super::transforms::rotate_vec3;
use super::{ConnectionRoll, Transform, Vec3};

pub(super) fn roll_transform(
    roll: ConnectionRoll,
    source_current_connector: Transform,
    target_aligned: Transform,
) -> Transform {
    let degrees = match roll {
        ConnectionRoll::MatchTarget => 0.0,
        ConnectionRoll::ExplicitDegrees(degrees) => degrees,
        ConnectionRoll::PreserveSource => {
            preserve_roll_degrees(source_current_connector, target_aligned)
        }
        ConnectionRoll::ChooseNearest { step_degrees } => {
            let preserved = preserve_roll_degrees(source_current_connector, target_aligned);
            (preserved / step_degrees).round() * step_degrees
        }
    };
    if degrees.is_finite() {
        Transform::IDENTITY.rotate_x_deg(degrees)
    } else {
        Transform::IDENTITY
    }
}

fn preserve_roll_degrees(source_current_connector: Transform, target_aligned: Transform) -> f32 {
    let target_forward = rotate_vec3(target_aligned.rotation, Vec3::new(1.0, 0.0, 0.0));
    let target_up = rotate_vec3(target_aligned.rotation, Vec3::new(0.0, 1.0, 0.0));
    let source_up = rotate_vec3(source_current_connector.rotation, Vec3::new(0.0, 1.0, 0.0));
    signed_angle_on_axis_degrees(target_up, source_up, target_forward).unwrap_or(0.0)
}

fn signed_angle_on_axis_degrees(from: Vec3, to: Vec3, axis: Vec3) -> Option<f32> {
    let axis = normalize_vec3(axis)?;
    let from = normalize_vec3(project_onto_plane(from, axis))?;
    let to = normalize_vec3(project_onto_plane(to, axis))?;
    let sin = dot_vec3(axis, cross_vec3(from, to));
    let cos = dot_vec3(from, to).clamp(-1.0, 1.0);
    Some(sin.atan2(cos).to_degrees())
}

fn project_onto_plane(value: Vec3, normal: Vec3) -> Vec3 {
    subtract_vec3(value, scale_vec3(normal, dot_vec3(value, normal)))
}

fn normalize_vec3(value: Vec3) -> Option<Vec3> {
    let length = dot_vec3(value, value).sqrt();
    if length <= f32::EPSILON || !length.is_finite() {
        return None;
    }
    Some(scale_vec3(value, length.recip()))
}

fn subtract_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x - right.x, left.y - right.y, left.z - right.z)
}

fn scale_vec3(value: Vec3, scale: f32) -> Vec3 {
    Vec3::new(value.x * scale, value.y * scale, value.z * scale)
}

fn dot_vec3(left: Vec3, right: Vec3) -> f32 {
    left.x * right.x + left.y * right.y + left.z * right.z
}

fn cross_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(
        left.y * right.z - left.z * right.y,
        left.z * right.x - left.x * right.z,
        left.x * right.y - left.y * right.x,
    )
}
