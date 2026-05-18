use crate::Transform;

pub(super) fn smoothstep(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    value * value * (3.0 - 2.0 * value)
}

pub(super) fn lerp_transform(start: Transform, end: Transform, amount: f32) -> Transform {
    Transform {
        translation: start.translation.lerp(end.translation, amount),
        rotation: start.rotation.slerp(end.rotation, amount),
        scale: start.scale.lerp(end.scale, amount),
    }
}
