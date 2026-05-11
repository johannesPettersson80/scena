use crate::scene::{Quat, Vec3};

use super::AnimationInterpolation;

pub(super) fn sample_vec3(
    times: &[f32],
    values: &[Vec3],
    interpolation: AnimationInterpolation,
    time_seconds: f32,
) -> Option<Vec3> {
    if times.is_empty() || values.is_empty() {
        return None;
    }
    if interpolation == AnimationInterpolation::CubicSpline {
        return sample_cubic_vec3(times, values, time_seconds);
    }
    if time_seconds <= times[0] {
        return values.first().copied();
    }
    if time_seconds >= *times.last()? {
        return values.last().copied();
    }
    for index in 0..times.len().saturating_sub(1) {
        let start = times[index];
        let end = times[index + 1];
        if time_seconds <= end {
            let left = *values.get(index)?;
            let right = *values.get(index + 1)?;
            return Some(match interpolation {
                AnimationInterpolation::Step => left,
                AnimationInterpolation::Linear => {
                    let amount = ((time_seconds - start) / (end - start)).clamp(0.0, 1.0);
                    lerp_vec3(left, right, amount)
                }
                AnimationInterpolation::CubicSpline => unreachable!("handled before loop"),
            });
        }
    }
    values.last().copied()
}

pub(super) fn sample_quat(
    times: &[f32],
    values: &[Quat],
    interpolation: AnimationInterpolation,
    time_seconds: f32,
) -> Option<Quat> {
    if times.is_empty() || values.is_empty() {
        return None;
    }
    if interpolation == AnimationInterpolation::CubicSpline {
        return sample_cubic_quat(times, values, time_seconds);
    }
    if time_seconds <= times[0] {
        return values.first().copied().map(normalize_quat);
    }
    if time_seconds >= *times.last()? {
        return values.last().copied().map(normalize_quat);
    }
    for index in 0..times.len().saturating_sub(1) {
        let start = times[index];
        let end = times[index + 1];
        if time_seconds <= end {
            let left = normalize_quat(*values.get(index)?);
            let right = normalize_quat(*values.get(index + 1)?);
            return Some(match interpolation {
                AnimationInterpolation::Step => left,
                AnimationInterpolation::Linear => {
                    let amount = ((time_seconds - start) / (end - start)).clamp(0.0, 1.0);
                    slerp_quat(left, right, amount)
                }
                AnimationInterpolation::CubicSpline => unreachable!("handled before loop"),
            });
        }
    }
    values.last().copied().map(normalize_quat)
}

pub(super) fn sample_weights(
    times: &[f32],
    values: &[Vec<f32>],
    interpolation: AnimationInterpolation,
    time_seconds: f32,
) -> Option<Vec<f32>> {
    if times.is_empty() || values.is_empty() {
        return None;
    }
    if interpolation == AnimationInterpolation::CubicSpline {
        return sample_cubic_weights(times, values, time_seconds);
    }
    if time_seconds <= times[0] {
        return values.first().cloned();
    }
    if time_seconds >= *times.last()? {
        return values.last().cloned();
    }
    for index in 0..times.len().saturating_sub(1) {
        let start = times[index];
        let end = times[index + 1];
        if time_seconds <= end {
            let left = values.get(index)?;
            let right = values.get(index + 1)?;
            if interpolation == AnimationInterpolation::Step {
                return Some(left.clone());
            }
            let amount = ((time_seconds - start) / (end - start)).clamp(0.0, 1.0);
            return Some(
                left.iter()
                    .zip(right)
                    .map(|(left, right)| left + (right - left) * amount)
                    .collect(),
            );
        }
    }
    values.last().cloned()
}

fn lerp_vec3(left: Vec3, right: Vec3, amount: f32) -> Vec3 {
    Vec3::new(
        left.x + (right.x - left.x) * amount,
        left.y + (right.y - left.y) * amount,
        left.z + (right.z - left.z) * amount,
    )
}

fn sample_cubic_vec3(times: &[f32], values: &[Vec3], time_seconds: f32) -> Option<Vec3> {
    if values.len() < times.len().saturating_mul(3) {
        return None;
    }
    if time_seconds <= times[0] {
        return values.get(1).copied();
    }
    if time_seconds >= *times.last()? {
        return values.get((times.len() - 1) * 3 + 1).copied();
    }
    for index in 0..times.len().saturating_sub(1) {
        let start = times[index];
        let end = times[index + 1];
        if time_seconds <= end {
            let amount = ((time_seconds - start) / (end - start)).clamp(0.0, 1.0);
            return Some(cubic_vec3(
                *values.get(index * 3 + 1)?,
                *values.get(index * 3 + 2)?,
                *values.get((index + 1) * 3)?,
                *values.get((index + 1) * 3 + 1)?,
                end - start,
                amount,
            ));
        }
    }
    values.get((times.len() - 1) * 3 + 1).copied()
}

fn sample_cubic_quat(times: &[f32], values: &[Quat], time_seconds: f32) -> Option<Quat> {
    if values.len() < times.len().saturating_mul(3) {
        return None;
    }
    if time_seconds <= times[0] {
        return values.get(1).copied().map(normalize_quat);
    }
    if time_seconds >= *times.last()? {
        return values
            .get((times.len() - 1) * 3 + 1)
            .copied()
            .map(normalize_quat);
    }
    for index in 0..times.len().saturating_sub(1) {
        let start = times[index];
        let end = times[index + 1];
        if time_seconds <= end {
            let amount = ((time_seconds - start) / (end - start)).clamp(0.0, 1.0);
            return Some(normalize_quat(cubic_quat(
                *values.get(index * 3 + 1)?,
                *values.get(index * 3 + 2)?,
                *values.get((index + 1) * 3)?,
                *values.get((index + 1) * 3 + 1)?,
                end - start,
                amount,
            )));
        }
    }
    values
        .get((times.len() - 1) * 3 + 1)
        .copied()
        .map(normalize_quat)
}

fn sample_cubic_weights(times: &[f32], values: &[Vec<f32>], time_seconds: f32) -> Option<Vec<f32>> {
    if values.len() < times.len().saturating_mul(3) {
        return None;
    }
    if time_seconds <= times[0] {
        return values.get(1).cloned();
    }
    if time_seconds >= *times.last()? {
        return values.get((times.len() - 1) * 3 + 1).cloned();
    }
    for index in 0..times.len().saturating_sub(1) {
        let start = times[index];
        let end = times[index + 1];
        if time_seconds <= end {
            let amount = ((time_seconds - start) / (end - start)).clamp(0.0, 1.0);
            return Some(cubic_weights(
                values.get(index * 3 + 1)?,
                values.get(index * 3 + 2)?,
                values.get((index + 1) * 3)?,
                values.get((index + 1) * 3 + 1)?,
                end - start,
                amount,
            ));
        }
    }
    values.get((times.len() - 1) * 3 + 1).cloned()
}

fn cubic_vec3(
    p0: Vec3,
    out_tangent0: Vec3,
    in_tangent1: Vec3,
    p1: Vec3,
    delta_seconds: f32,
    amount: f32,
) -> Vec3 {
    let components = cubic_components(
        [p0.x, p0.y, p0.z],
        [out_tangent0.x, out_tangent0.y, out_tangent0.z],
        [in_tangent1.x, in_tangent1.y, in_tangent1.z],
        [p1.x, p1.y, p1.z],
        delta_seconds,
        amount,
    );
    Vec3::new(components[0], components[1], components[2])
}

fn cubic_quat(
    p0: Quat,
    out_tangent0: Quat,
    in_tangent1: Quat,
    p1: Quat,
    delta_seconds: f32,
    amount: f32,
) -> Quat {
    let components = cubic_components(
        [p0.x, p0.y, p0.z, p0.w],
        [
            out_tangent0.x,
            out_tangent0.y,
            out_tangent0.z,
            out_tangent0.w,
        ],
        [in_tangent1.x, in_tangent1.y, in_tangent1.z, in_tangent1.w],
        [p1.x, p1.y, p1.z, p1.w],
        delta_seconds,
        amount,
    );
    Quat::from_xyzw(components[0], components[1], components[2], components[3])
}

fn cubic_weights(
    p0: &[f32],
    out_tangent0: &[f32],
    in_tangent1: &[f32],
    p1: &[f32],
    delta_seconds: f32,
    amount: f32,
) -> Vec<f32> {
    p0.iter()
        .zip(out_tangent0)
        .zip(in_tangent1)
        .zip(p1)
        .map(|(((p0, out_tangent0), in_tangent1), p1)| {
            cubic_scalar(*p0, *out_tangent0, *in_tangent1, *p1, delta_seconds, amount)
        })
        .collect()
}

fn cubic_components<const N: usize>(
    p0: [f32; N],
    out_tangent0: [f32; N],
    in_tangent1: [f32; N],
    p1: [f32; N],
    delta_seconds: f32,
    amount: f32,
) -> [f32; N] {
    std::array::from_fn(|index| {
        cubic_scalar(
            p0[index],
            out_tangent0[index],
            in_tangent1[index],
            p1[index],
            delta_seconds,
            amount,
        )
    })
}

fn cubic_scalar(
    p0: f32,
    out_tangent0: f32,
    in_tangent1: f32,
    p1: f32,
    delta_seconds: f32,
    amount: f32,
) -> f32 {
    let t2 = amount * amount;
    let t3 = t2 * amount;
    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + amount;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;
    h00 * p0 + h10 * delta_seconds * out_tangent0 + h01 * p1 + h11 * delta_seconds * in_tangent1
}

fn normalize_quat(value: Quat) -> Quat {
    let length =
        (value.x * value.x + value.y * value.y + value.z * value.z + value.w * value.w).sqrt();
    if length <= f32::EPSILON || !length.is_finite() {
        return Quat::IDENTITY;
    }
    Quat::from_xyzw(value.x / length, value.y / length, value.z / length, value.w / length)
}

fn slerp_quat(left: Quat, right: Quat, amount: f32) -> Quat {
    let mut right = right;
    let mut dot = left.x * right.x + left.y * right.y + left.z * right.z + left.w * right.w;
    if dot < 0.0 {
        dot = -dot;
        right = Quat::from_xyzw(-right.x, -right.y, -right.z, -right.w);
    }
    if dot > 0.9995 {
        return normalize_quat(Quat::from_xyzw(left.x + (right.x - left.x) * amount, left.y + (right.y - left.y) * amount, left.z + (right.z - left.z) * amount, left.w + (right.w - left.w) * amount));
    }
    let theta_0 = dot.acos();
    let theta = theta_0 * amount;
    let sin_theta = theta.sin();
    let sin_theta_0 = theta_0.sin();
    let left_scale = theta.cos() - dot * sin_theta / sin_theta_0;
    let right_scale = sin_theta / sin_theta_0;
    normalize_quat(Quat::from_xyzw(left.x * left_scale + right.x * right_scale, left.y * left_scale + right.y * right_scale, left.z * left_scale + right.z * right_scale, left.w * left_scale + right.w * right_scale))
}
