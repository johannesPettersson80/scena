use super::{
    AnimationChannel, AnimationInterpolation, AnimationOutput, AnimationSourceChannel,
    AnimationTarget,
};
use crate::diagnostics::AnimationError;
use crate::scene::{Quat, Vec3};

pub(super) fn validate_clip(
    channels: &[AnimationChannel],
    duration_seconds: f32,
) -> Result<(), AnimationError> {
    if !duration_seconds.is_finite() || duration_seconds <= 0.0 {
        return Err(AnimationError::InvalidClip {
            reason: "duration_seconds must be finite and positive".to_owned(),
        });
    }
    if channels.is_empty() {
        return Err(AnimationError::InvalidClip {
            reason: "clip must contain at least one channel".to_owned(),
        });
    }
    for (channel_index, channel) in channels.iter().enumerate() {
        validate_channel(channel_index, channel, duration_seconds)?;
    }
    Ok(())
}

pub(super) fn validate_imported_clip(
    channels: &[AnimationChannel],
    duration_seconds: f32,
) -> Result<(), AnimationError> {
    validate_imported_header(channels.is_empty(), duration_seconds)?;
    for (channel_index, channel) in channels.iter().enumerate() {
        validate_channel(channel_index, channel, duration_seconds)?;
    }
    Ok(())
}

pub(super) fn validate_imported_source_clip(
    channels: &[AnimationSourceChannel],
    duration_seconds: f32,
) -> Result<(), AnimationError> {
    validate_imported_header(channels.is_empty(), duration_seconds)?;
    for (channel_index, channel) in channels.iter().enumerate() {
        validate_channel_fields(
            channel_index,
            channel.target,
            &channel.input_seconds,
            &channel.output,
            channel.interpolation,
            duration_seconds,
        )?;
    }
    Ok(())
}

fn validate_imported_header(
    channels_are_empty: bool,
    duration_seconds: f32,
) -> Result<(), AnimationError> {
    if !duration_seconds.is_finite() || duration_seconds < 0.0 {
        return Err(AnimationError::InvalidClip {
            reason: "imported duration_seconds must be finite and non-negative".to_owned(),
        });
    }
    if channels_are_empty {
        return Err(AnimationError::InvalidClip {
            reason: "imported clip must contain at least one channel".to_owned(),
        });
    }
    Ok(())
}

fn validate_channel(
    channel_index: usize,
    channel: &AnimationChannel,
    duration_seconds: f32,
) -> Result<(), AnimationError> {
    validate_channel_fields(
        channel_index,
        channel.target,
        &channel.input_seconds,
        &channel.output,
        channel.interpolation,
        duration_seconds,
    )
}

fn validate_channel_fields(
    channel_index: usize,
    target: AnimationTarget,
    input_seconds: &[f32],
    output: &AnimationOutput,
    interpolation: AnimationInterpolation,
    duration_seconds: f32,
) -> Result<(), AnimationError> {
    validate_times(channel_index, input_seconds, duration_seconds)?;
    let expected_values = if interpolation == AnimationInterpolation::CubicSpline {
        input_seconds.len().saturating_mul(3)
    } else {
        input_seconds.len()
    };
    validate_output_type(channel_index, target, output)?;
    validate_output_values(channel_index, output, expected_values)
}

fn validate_times(
    channel_index: usize,
    input_seconds: &[f32],
    duration_seconds: f32,
) -> Result<(), AnimationError> {
    if input_seconds.is_empty() {
        return Err(invalid_channel(channel_index, "times must not be empty"));
    }
    let mut previous = None;
    for (time_index, time) in input_seconds.iter().copied().enumerate() {
        if !time.is_finite() || time < 0.0 {
            return Err(invalid_channel(
                channel_index,
                format!("time[{time_index}] must be finite and non-negative"),
            ));
        }
        if time > duration_seconds {
            return Err(invalid_channel(
                channel_index,
                format!("time[{time_index}] exceeds clip duration"),
            ));
        }
        if previous.is_some_and(|previous| time <= previous) {
            return Err(invalid_channel(
                channel_index,
                format!("time[{time_index}] must be strictly increasing"),
            ));
        }
        previous = Some(time);
    }
    Ok(())
}

fn validate_output_type(
    channel_index: usize,
    target: AnimationTarget,
    output: &AnimationOutput,
) -> Result<(), AnimationError> {
    let valid = matches!(
        (target, output),
        (
            AnimationTarget::Translation | AnimationTarget::Scale,
            AnimationOutput::Vec3(_)
        ) | (AnimationTarget::Rotation, AnimationOutput::Quat(_))
            | (AnimationTarget::Weights, AnimationOutput::Weights(_))
    );
    if valid {
        return Ok(());
    }
    let reason = match target {
        AnimationTarget::Translation => "translation channel output must use VEC3 values",
        AnimationTarget::Scale => "scale channel output must use VEC3 values",
        AnimationTarget::Rotation => "rotation channel output must use VEC4 quaternion values",
        AnimationTarget::Weights => "weights channel output must use scalar morph-target values",
    };
    Err(invalid_channel(channel_index, reason))
}

fn validate_output_values(
    channel_index: usize,
    output: &AnimationOutput,
    expected_values: usize,
) -> Result<(), AnimationError> {
    match output {
        AnimationOutput::Vec3(values) => {
            validate_value_count(channel_index, "Vec3", values.len(), expected_values)?;
            if let Some(index) = values.iter().position(|value| !vec3_is_finite(*value)) {
                return Err(invalid_channel(
                    channel_index,
                    format!("Vec3 output[{index}] must be finite"),
                ));
            }
        }
        AnimationOutput::Quat(values) => {
            validate_value_count(channel_index, "Quat", values.len(), expected_values)?;
            if let Some(index) = values.iter().position(|value| !quat_is_finite(*value)) {
                return Err(invalid_channel(
                    channel_index,
                    format!("Quat output[{index}] must be finite"),
                ));
            }
        }
        AnimationOutput::Weights(values) => {
            validate_value_count(channel_index, "Weights", values.len(), expected_values)?;
            validate_weight_values(channel_index, values)?;
        }
    }
    Ok(())
}

fn validate_value_count(
    channel_index: usize,
    label: &str,
    actual: usize,
    expected: usize,
) -> Result<(), AnimationError> {
    if actual == expected {
        return Ok(());
    }
    Err(invalid_channel(
        channel_index,
        format!("{label} output length must be {expected}"),
    ))
}

fn validate_weight_values(channel_index: usize, values: &[Vec<f32>]) -> Result<(), AnimationError> {
    let Some(width) = values.first().map(Vec::len).filter(|width| *width > 0) else {
        return Err(invalid_channel(
            channel_index,
            "weights output must contain at least one morph target",
        ));
    };
    for (index, value) in values.iter().enumerate() {
        if value.len() != width {
            return Err(invalid_channel(
                channel_index,
                format!("weights output[{index}] has inconsistent width"),
            ));
        }
        if value.iter().any(|component| !component.is_finite()) {
            return Err(invalid_channel(
                channel_index,
                format!("weights output[{index}] must be finite"),
            ));
        }
    }
    Ok(())
}

fn invalid_channel(channel_index: usize, reason: impl Into<String>) -> AnimationError {
    AnimationError::InvalidClip {
        reason: format!("channel {channel_index}: {}", reason.into()),
    }
}

fn quat_is_finite(value: Quat) -> bool {
    value.x.is_finite() && value.y.is_finite() && value.z.is_finite() && value.w.is_finite()
}

fn vec3_is_finite(value: Vec3) -> bool {
    value.x.is_finite() && value.y.is_finite() && value.z.is_finite()
}
