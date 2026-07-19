use crate::scene::{Quat, Vec3};

use super::AnimationInterpolation;

pub(super) fn sample_vec3(
    times: &[f32],
    values: &[Vec3],
    interpolation: AnimationInterpolation,
    time_seconds: f32,
) -> Option<Vec3> {
    let mut ignored = 0;
    sample_vec3_impl::<false>(times, values, interpolation, time_seconds, &mut ignored)
}

pub(super) fn sample_vec3_profiled(
    times: &[f32],
    values: &[Vec3],
    interpolation: AnimationInterpolation,
    time_seconds: f32,
    intervals_tested: &mut u64,
) -> Option<Vec3> {
    sample_vec3_impl::<true>(times, values, interpolation, time_seconds, intervals_tested)
}

fn sample_vec3_impl<const PROFILE: bool>(
    times: &[f32],
    values: &[Vec3],
    interpolation: AnimationInterpolation,
    time_seconds: f32,
    intervals_tested: &mut u64,
) -> Option<Vec3> {
    if times.is_empty() || values.is_empty() {
        return None;
    }
    if interpolation == AnimationInterpolation::CubicSpline {
        return sample_cubic_vec3::<PROFILE>(times, values, time_seconds, intervals_tested);
    }
    if time_seconds <= times[0] {
        return values.first().copied();
    }
    if time_seconds >= *times.last()? {
        return values.last().copied();
    }
    let index = keyframe_segment::<PROFILE>(times, time_seconds, intervals_tested)?;
    let start = times[index];
    let end = times[index + 1];
    let left = *values.get(index)?;
    let right = *values.get(index + 1)?;
    Some(match interpolation {
        AnimationInterpolation::Step => left,
        AnimationInterpolation::Linear => {
            let amount = ((time_seconds - start) / (end - start)).clamp(0.0, 1.0);
            lerp_vec3(left, right, amount)
        }
        AnimationInterpolation::CubicSpline => unreachable!("handled before lookup"),
    })
}

pub(super) fn sample_quat(
    times: &[f32],
    values: &[Quat],
    interpolation: AnimationInterpolation,
    time_seconds: f32,
) -> Option<Quat> {
    let mut ignored = 0;
    sample_quat_impl::<false>(times, values, interpolation, time_seconds, &mut ignored)
}

pub(super) fn sample_quat_profiled(
    times: &[f32],
    values: &[Quat],
    interpolation: AnimationInterpolation,
    time_seconds: f32,
    intervals_tested: &mut u64,
) -> Option<Quat> {
    sample_quat_impl::<true>(times, values, interpolation, time_seconds, intervals_tested)
}

fn sample_quat_impl<const PROFILE: bool>(
    times: &[f32],
    values: &[Quat],
    interpolation: AnimationInterpolation,
    time_seconds: f32,
    intervals_tested: &mut u64,
) -> Option<Quat> {
    if times.is_empty() || values.is_empty() {
        return None;
    }
    if interpolation == AnimationInterpolation::CubicSpline {
        return sample_cubic_quat::<PROFILE>(times, values, time_seconds, intervals_tested);
    }
    if time_seconds <= times[0] {
        return values.first().copied().map(normalize_quat);
    }
    if time_seconds >= *times.last()? {
        return values.last().copied().map(normalize_quat);
    }
    let index = keyframe_segment::<PROFILE>(times, time_seconds, intervals_tested)?;
    let start = times[index];
    let end = times[index + 1];
    let left = normalize_quat(*values.get(index)?);
    let right = normalize_quat(*values.get(index + 1)?);
    Some(match interpolation {
        AnimationInterpolation::Step => left,
        AnimationInterpolation::Linear => {
            let amount = ((time_seconds - start) / (end - start)).clamp(0.0, 1.0);
            slerp_quat(left, right, amount)
        }
        AnimationInterpolation::CubicSpline => unreachable!("handled before lookup"),
    })
}

pub(super) fn sample_weights(
    times: &[f32],
    values: &[Vec<f32>],
    interpolation: AnimationInterpolation,
    time_seconds: f32,
) -> Option<Vec<f32>> {
    let mut ignored = 0;
    sample_weights_impl::<false>(times, values, interpolation, time_seconds, &mut ignored)
}

pub(super) fn sample_weights_into(
    times: &[f32],
    values: &[Vec<f32>],
    interpolation: AnimationInterpolation,
    time_seconds: f32,
    output: &mut Vec<f32>,
) -> bool {
    let mut ignored = 0;
    sample_weights_into_impl::<false>(
        times,
        values,
        interpolation,
        time_seconds,
        output,
        &mut ignored,
    )
}

pub(super) fn sample_weights_into_profiled(
    times: &[f32],
    values: &[Vec<f32>],
    interpolation: AnimationInterpolation,
    time_seconds: f32,
    output: &mut Vec<f32>,
    intervals_tested: &mut u64,
) -> bool {
    sample_weights_into_impl::<true>(
        times,
        values,
        interpolation,
        time_seconds,
        output,
        intervals_tested,
    )
}

fn sample_weights_impl<const PROFILE: bool>(
    times: &[f32],
    values: &[Vec<f32>],
    interpolation: AnimationInterpolation,
    time_seconds: f32,
    intervals_tested: &mut u64,
) -> Option<Vec<f32>> {
    let mut output = Vec::new();
    sample_weights_into_impl::<PROFILE>(
        times,
        values,
        interpolation,
        time_seconds,
        &mut output,
        intervals_tested,
    )
    .then_some(output)
}

fn sample_weights_into_impl<const PROFILE: bool>(
    times: &[f32],
    values: &[Vec<f32>],
    interpolation: AnimationInterpolation,
    time_seconds: f32,
    output: &mut Vec<f32>,
    intervals_tested: &mut u64,
) -> bool {
    if times.is_empty() || values.is_empty() {
        return false;
    }
    if interpolation == AnimationInterpolation::CubicSpline {
        return sample_cubic_weights_into::<PROFILE>(
            times,
            values,
            time_seconds,
            output,
            intervals_tested,
        );
    }
    if time_seconds <= times[0] {
        return replace_output(output, values.first());
    }
    let Some(last_time) = times.last() else {
        return false;
    };
    if time_seconds >= *last_time {
        return replace_output(output, values.last());
    }
    let Some(index) = keyframe_segment::<PROFILE>(times, time_seconds, intervals_tested) else {
        return false;
    };
    let Some(left) = values.get(index) else {
        return false;
    };
    if interpolation == AnimationInterpolation::Step {
        return replace_output(output, Some(left));
    }
    let Some(right) = values.get(index + 1) else {
        return false;
    };
    let amount =
        ((time_seconds - times[index]) / (times[index + 1] - times[index])).clamp(0.0, 1.0);
    output.clear();
    output.extend(
        left.iter()
            .zip(right)
            .map(|(left, right)| left + (right - left) * amount),
    );
    true
}

fn lerp_vec3(left: Vec3, right: Vec3, amount: f32) -> Vec3 {
    Vec3::new(
        left.x + (right.x - left.x) * amount,
        left.y + (right.y - left.y) * amount,
        left.z + (right.z - left.z) * amount,
    )
}

fn sample_cubic_vec3<const PROFILE: bool>(
    times: &[f32],
    values: &[Vec3],
    time_seconds: f32,
    intervals_tested: &mut u64,
) -> Option<Vec3> {
    if values.len() < times.len().saturating_mul(3) {
        return None;
    }
    if time_seconds <= times[0] {
        return values.get(1).copied();
    }
    if time_seconds >= *times.last()? {
        return values.get((times.len() - 1) * 3 + 1).copied();
    }
    let index = keyframe_segment::<PROFILE>(times, time_seconds, intervals_tested)?;
    let start = times[index];
    let end = times[index + 1];
    let amount = ((time_seconds - start) / (end - start)).clamp(0.0, 1.0);
    Some(cubic_vec3(
        *values.get(index * 3 + 1)?,
        *values.get(index * 3 + 2)?,
        *values.get((index + 1) * 3)?,
        *values.get((index + 1) * 3 + 1)?,
        end - start,
        amount,
    ))
}

fn sample_cubic_quat<const PROFILE: bool>(
    times: &[f32],
    values: &[Quat],
    time_seconds: f32,
    intervals_tested: &mut u64,
) -> Option<Quat> {
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
    let index = keyframe_segment::<PROFILE>(times, time_seconds, intervals_tested)?;
    let start = times[index];
    let end = times[index + 1];
    let amount = ((time_seconds - start) / (end - start)).clamp(0.0, 1.0);
    Some(normalize_quat(cubic_quat(
        *values.get(index * 3 + 1)?,
        *values.get(index * 3 + 2)?,
        *values.get((index + 1) * 3)?,
        *values.get((index + 1) * 3 + 1)?,
        end - start,
        amount,
    )))
}

fn sample_cubic_weights_into<const PROFILE: bool>(
    times: &[f32],
    values: &[Vec<f32>],
    time_seconds: f32,
    output: &mut Vec<f32>,
    intervals_tested: &mut u64,
) -> bool {
    if values.len() < times.len().saturating_mul(3) {
        return false;
    }
    if time_seconds <= times[0] {
        return replace_output(output, values.get(1));
    }
    let Some(last_time) = times.last() else {
        return false;
    };
    if time_seconds >= *last_time {
        return replace_output(output, values.get((times.len() - 1) * 3 + 1));
    }
    let Some(index) = keyframe_segment::<PROFILE>(times, time_seconds, intervals_tested) else {
        return false;
    };
    let start = times[index];
    let end = times[index + 1];
    let amount = ((time_seconds - start) / (end - start)).clamp(0.0, 1.0);
    let (Some(p0), Some(out_tangent0), Some(in_tangent1), Some(p1)) = (
        values.get(index * 3 + 1),
        values.get(index * 3 + 2),
        values.get((index + 1) * 3),
        values.get((index + 1) * 3 + 1),
    ) else {
        return false;
    };
    cubic_weights_into(
        output,
        p0,
        out_tangent0,
        in_tangent1,
        p1,
        end - start,
        amount,
    );
    true
}

#[inline]
fn record_interval<const PROFILE: bool>(intervals_tested: &mut u64) {
    if PROFILE {
        *intervals_tested = intervals_tested.saturating_add(1);
    }
}

fn keyframe_segment<const PROFILE: bool>(
    times: &[f32],
    time_seconds: f32,
    intervals_tested: &mut u64,
) -> Option<usize> {
    if times.len() < 2 {
        return None;
    }
    let mut left = 0;
    let mut right = times.len();
    while left < right {
        record_interval::<PROFILE>(intervals_tested);
        let middle = left + (right - left) / 2;
        if times[middle] <= time_seconds {
            left = middle + 1;
        } else {
            right = middle;
        }
    }
    Some(left.saturating_sub(1).min(times.len() - 2))
}

fn replace_output(output: &mut Vec<f32>, values: Option<&Vec<f32>>) -> bool {
    let Some(values) = values else {
        return false;
    };
    output.clear();
    output.extend_from_slice(values);
    true
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

fn cubic_weights_into(
    output: &mut Vec<f32>,
    p0: &[f32],
    out_tangent0: &[f32],
    in_tangent1: &[f32],
    p1: &[f32],
    delta_seconds: f32,
    amount: f32,
) {
    output.clear();
    output.extend(p0.iter().zip(out_tangent0).zip(in_tangent1).zip(p1).map(
        |(((p0, out_tangent0), in_tangent1), p1)| {
            cubic_scalar(*p0, *out_tangent0, *in_tangent1, *p1, delta_seconds, amount)
        },
    ));
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
    Quat::from_xyzw(
        value.x / length,
        value.y / length,
        value.z / length,
        value.w / length,
    )
}

fn slerp_quat(left: Quat, right: Quat, amount: f32) -> Quat {
    let mut right = right;
    let mut dot = left.x * right.x + left.y * right.y + left.z * right.z + left.w * right.w;
    if dot < 0.0 {
        dot = -dot;
        right = Quat::from_xyzw(-right.x, -right.y, -right.z, -right.w);
    }
    if dot > 0.9995 {
        return normalize_quat(Quat::from_xyzw(
            left.x + (right.x - left.x) * amount,
            left.y + (right.y - left.y) * amount,
            left.z + (right.z - left.z) * amount,
            left.w + (right.w - left.w) * amount,
        ));
    }
    let theta_0 = dot.acos();
    let theta = theta_0 * amount;
    let sin_theta = theta.sin();
    let sin_theta_0 = theta_0.sin();
    let left_scale = theta.cos() - dot * sin_theta / sin_theta_0;
    let right_scale = sin_theta / sin_theta_0;
    normalize_quat(Quat::from_xyzw(
        left.x * left_scale + right.x * right_scale,
        left.y * left_scale + right.y * right_scale,
        left.z * left_scale + right.z * right_scale,
        left.w * left_scale + right.w * right_scale,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_vec3_holds_left_key_until_boundary() {
        let times = [0.0, 1.0];
        let values = [Vec3::ZERO, Vec3::new(2.0, 0.0, 0.0)];
        assert_eq!(
            sample_vec3(&times, &values, AnimationInterpolation::Step, 0.5),
            Some(Vec3::ZERO)
        );
        assert_eq!(
            sample_vec3(&times, &values, AnimationInterpolation::Step, 1.0),
            Some(Vec3::new(2.0, 0.0, 0.0))
        );
    }

    #[test]
    fn linear_quat_slerps_between_distinct_rotations() {
        let times = [0.0, 1.0];
        let values = [
            Quat::IDENTITY,
            Quat::from_xyzw(
                0.0,
                0.0,
                std::f32::consts::FRAC_1_SQRT_2,
                std::f32::consts::FRAC_1_SQRT_2,
            ),
        ];
        let sampled = sample_quat(&times, &values, AnimationInterpolation::Linear, 0.5)
            .expect("midpoint samples");
        assert!(
            sampled.z.abs() > 0.35 && sampled.w < 0.95,
            "slerp midpoint should not stay identity: {sampled:?}"
        );
    }

    #[test]
    fn cubic_vec3_uses_gltf_in_value_out_layout() {
        let times = [0.0, 1.0];
        let values = [
            Vec3::ZERO,
            Vec3::ZERO,
            Vec3::ZERO,
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::ZERO,
        ];
        let sampled = sample_vec3(&times, &values, AnimationInterpolation::CubicSpline, 0.5)
            .expect("cubic midpoint samples");
        assert!(
            (sampled.x - 0.5).abs() < 0.02,
            "zero-tangent Hermite midpoint should land halfway, got {sampled:?}"
        );
    }

    #[test]
    fn pf10_profiled_keyframe_lookup_is_logarithmic_for_linear_and_cubic_channels() {
        const KEY_COUNT: usize = 65_536;
        let times = (0..KEY_COUNT).map(|index| index as f32).collect::<Vec<_>>();
        let vec3_values = times
            .iter()
            .map(|time| Vec3::new(*time, -*time, time * 0.5))
            .collect::<Vec<_>>();
        let mut cubic_values = Vec::with_capacity(KEY_COUNT * 3);
        for value in &vec3_values {
            cubic_values.extend_from_slice(&[Vec3::ZERO, *value, Vec3::ZERO]);
        }

        let mut linear_probes = 0;
        let linear = sample_vec3_profiled(
            &times,
            &vec3_values,
            AnimationInterpolation::Linear,
            48_123.5,
            &mut linear_probes,
        )
        .expect("linear sample");
        let mut cubic_probes = 0;
        let cubic = sample_vec3_profiled(
            &times,
            &cubic_values,
            AnimationInterpolation::CubicSpline,
            48_123.5,
            &mut cubic_probes,
        )
        .expect("cubic sample");

        assert!(
            (linear.x - 48_123.5).abs() < 0.01,
            "linear parity: {linear:?}"
        );
        assert!((cubic.x - 48_123.5).abs() < 0.01, "cubic parity: {cubic:?}");
        assert!(
            linear_probes <= 17,
            "linear lookup must be O(log K), got {linear_probes}"
        );
        assert!(
            cubic_probes <= 17,
            "cubic lookup must be O(log K), got {cubic_probes}"
        );
    }

    #[test]
    fn pf10_weight_sampling_reuses_caller_owned_output_storage() {
        let times = [0.0, 1.0, 2.0];
        let values = vec![vec![0.0, 1.0], vec![0.5, 0.5], vec![1.0, 0.0]];
        let mut output = Vec::with_capacity(8);
        let original_capacity = output.capacity();
        let mut probes = 0;

        assert!(sample_weights_into_profiled(
            &times,
            &values,
            AnimationInterpolation::Linear,
            1.5,
            &mut output,
            &mut probes,
        ));
        assert_eq!(output, [0.75, 0.25]);
        assert_eq!(output.capacity(), original_capacity);
        assert!(probes <= 2);
    }
}
