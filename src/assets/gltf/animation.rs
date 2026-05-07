use serde_json::Value as JsonValue;

use crate::animation::{
    AnimationInterpolation, AnimationOutput, AnimationSourceChannel, AnimationSourceClip,
    AnimationTarget,
};
use crate::assets::AssetPath;
use crate::diagnostics::AssetError;
use crate::scene::Quat;

use super::SceneAssetClip;
use super::accessor::{
    self, GltfAccessor, GltfBufferView, read_f32_accessor, read_vec3_accessor, read_vec4_accessor,
};

pub(super) fn parse_gltf_clips(
    path: &AssetPath,
    json: &JsonValue,
    buffers: &[Vec<u8>],
    buffer_views: &[GltfBufferView],
    accessors: &[GltfAccessor],
) -> Result<Vec<SceneAssetClip>, AssetError> {
    json.get("animations")
        .and_then(JsonValue::as_array)
        .map(|animations| {
            animations
                .iter()
                .map(|animation| parse_animation(path, animation, buffers, buffer_views, accessors))
                .collect()
        })
        .unwrap_or_else(|| Ok(Vec::new()))
}

fn parse_animation(
    path: &AssetPath,
    animation: &JsonValue,
    buffers: &[Vec<u8>],
    buffer_views: &[GltfBufferView],
    accessors: &[GltfAccessor],
) -> Result<SceneAssetClip, AssetError> {
    let samplers = animation
        .get("samplers")
        .and_then(JsonValue::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let channels = animation
        .get("channels")
        .and_then(JsonValue::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .map(|channel| {
            parse_animation_channel(path, channel, samplers, buffers, buffer_views, accessors)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let duration_seconds = channels
        .iter()
        .flat_map(|channel| channel.input_seconds().iter().copied())
        .fold(0.0_f32, f32::max);
    Ok(SceneAssetClip {
        clip: AnimationSourceClip::new(
            animation
                .get("name")
                .and_then(JsonValue::as_str)
                .map(str::to_string),
            channels,
            duration_seconds,
        ),
    })
}

fn parse_animation_channel(
    path: &AssetPath,
    channel: &JsonValue,
    samplers: &[JsonValue],
    buffers: &[Vec<u8>],
    buffer_views: &[GltfBufferView],
    accessors: &[GltfAccessor],
) -> Result<AnimationSourceChannel, AssetError> {
    let sampler = sampler_for_channel(path, channel, samplers)?;
    let input = required_index(path, sampler, "input", "animation sampler is missing input")?;
    let output = required_index(
        path,
        sampler,
        "output",
        "animation sampler is missing output",
    )?;
    let interpolation = parse_interpolation(sampler);
    let target_json = channel
        .get("target")
        .ok_or_else(|| accessor::parse_error(path, "animation channel is missing target"))?;
    let source_node = required_index(
        path,
        target_json,
        "node",
        "animation target is missing node",
    )?;
    let target = parse_target(path, target_json)?;
    let input_seconds = read_f32_accessor(path, input, buffers, buffer_views, accessors)?;
    let output = parse_output(path, target, output, buffers, buffer_views, accessors)?;

    Ok(AnimationSourceChannel::new(
        source_node,
        target,
        input_seconds,
        output,
        interpolation,
    ))
}

fn sampler_for_channel<'a>(
    path: &AssetPath,
    channel: &JsonValue,
    samplers: &'a [JsonValue],
) -> Result<&'a JsonValue, AssetError> {
    let sampler_index = required_index(
        path,
        channel,
        "sampler",
        "animation channel is missing sampler",
    )?;
    samplers
        .get(sampler_index)
        .ok_or_else(|| accessor::parse_error(path, "animation channel references missing sampler"))
}

fn parse_interpolation(sampler: &JsonValue) -> AnimationInterpolation {
    match sampler
        .get("interpolation")
        .and_then(JsonValue::as_str)
        .unwrap_or("LINEAR")
    {
        "STEP" => AnimationInterpolation::Step,
        "CUBICSPLINE" => AnimationInterpolation::CubicSpline,
        _ => AnimationInterpolation::Linear,
    }
}

fn parse_target(path: &AssetPath, target_json: &JsonValue) -> Result<AnimationTarget, AssetError> {
    match target_json.get("path").and_then(JsonValue::as_str) {
        Some("translation") => Ok(AnimationTarget::Translation),
        Some("rotation") => Ok(AnimationTarget::Rotation),
        Some("scale") => Ok(AnimationTarget::Scale),
        Some("weights") => Ok(AnimationTarget::Weights),
        Some(other) => Err(accessor::parse_error(
            path,
            format!("unsupported animation target path {other}"),
        )),
        None => Err(accessor::parse_error(
            path,
            "animation target is missing path",
        )),
    }
}

fn parse_output(
    path: &AssetPath,
    target: AnimationTarget,
    output: usize,
    buffers: &[Vec<u8>],
    buffer_views: &[GltfBufferView],
    accessors: &[GltfAccessor],
) -> Result<AnimationOutput, AssetError> {
    match target {
        AnimationTarget::Translation | AnimationTarget::Scale => Ok(AnimationOutput::Vec3(
            read_vec3_accessor(path, output, buffers, buffer_views, accessors)?,
        )),
        AnimationTarget::Rotation => Ok(AnimationOutput::Quat(
            read_vec4_accessor(path, output, buffers, buffer_views, accessors)?
                .into_iter()
                .map(|values| Quat {
                    x: values[0],
                    y: values[1],
                    z: values[2],
                    w: values[3],
                })
                .collect(),
        )),
        AnimationTarget::Weights => Ok(AnimationOutput::Weights(
            read_f32_accessor(path, output, buffers, buffer_views, accessors)?
                .into_iter()
                .map(|value| vec![value])
                .collect(),
        )),
    }
}

fn required_index(
    path: &AssetPath,
    value: &JsonValue,
    field: &str,
    message: &'static str,
) -> Result<usize, AssetError> {
    value
        .get(field)
        .and_then(JsonValue::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| accessor::parse_error(path, message))
}
