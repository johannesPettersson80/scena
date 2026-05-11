//! Stage C2: glTF animation parsing now uses the `gltf` crate's typed
//! `animation::Channel::reader()` so input/output accessor walking is
//! delegated to the gltf-crate util module (no hand-rolled component
//! reading).

use ::gltf::Document;
use ::gltf::animation::Interpolation as GltfInterpolation;
use ::gltf::animation::Property as GltfProperty;
use ::gltf::animation::util::ReadOutputs;

use crate::animation::{
    AnimationInterpolation, AnimationOutput, AnimationSourceChannel, AnimationSourceClip,
    AnimationTarget,
};
use crate::assets::AssetPath;
use crate::diagnostics::AssetError;
use crate::scene::Quat;

use super::SceneAssetClip;

pub(super) fn parse_gltf_clips(
    path: &AssetPath,
    document: &Document,
    buffers: &[Vec<u8>],
) -> Result<Vec<SceneAssetClip>, AssetError> {
    document
        .animations()
        .map(|animation| {
            let channels = animation
                .channels()
                .map(|channel| parse_channel(path, &channel, buffers))
                .collect::<Result<Vec<_>, _>>()?;
            let duration_seconds = channels
                .iter()
                .flat_map(|channel| channel.input_seconds().iter().copied())
                .fold(0.0_f32, f32::max);
            Ok(SceneAssetClip {
                clip: AnimationSourceClip::new(
                    animation.name().map(str::to_string),
                    channels,
                    duration_seconds,
                ),
            })
        })
        .collect()
}

fn parse_channel(
    path: &AssetPath,
    channel: &::gltf::animation::Channel<'_>,
    buffers: &[Vec<u8>],
) -> Result<AnimationSourceChannel, AssetError> {
    let target = channel.target();
    let target_node = target.node().index();
    let target_property = match target.property() {
        GltfProperty::Translation => AnimationTarget::Translation,
        GltfProperty::Rotation => AnimationTarget::Rotation,
        GltfProperty::Scale => AnimationTarget::Scale,
        GltfProperty::MorphTargetWeights => AnimationTarget::Weights,
    };
    let sampler = channel.sampler();
    let interpolation = match sampler.interpolation() {
        GltfInterpolation::Linear => AnimationInterpolation::Linear,
        GltfInterpolation::Step => AnimationInterpolation::Step,
        GltfInterpolation::CubicSpline => AnimationInterpolation::CubicSpline,
    };

    let reader = channel.reader(|buffer| buffers.get(buffer.index()).map(Vec::as_slice));
    let inputs = reader.read_inputs().ok_or_else(|| AssetError::Parse {
        path: path.as_str().to_string(),
        reason: "animation sampler input accessor failed to resolve".to_string(),
    })?;
    let input_seconds: Vec<f32> = inputs.collect();

    let outputs = reader.read_outputs().ok_or_else(|| AssetError::Parse {
        path: path.as_str().to_string(),
        reason: "animation sampler output accessor failed to resolve".to_string(),
    })?;
    let output = match outputs {
        ReadOutputs::Translations(translations) => AnimationOutput::Vec3(
            translations.map(crate::scene::Vec3::from_array).collect(),
        ),
        ReadOutputs::Scales(scales) => {
            AnimationOutput::Vec3(scales.map(crate::scene::Vec3::from_array).collect())
        }
        ReadOutputs::Rotations(rotations) => AnimationOutput::Quat(
            rotations
                .into_f32()
                .map(|values| Quat::from_xyzw(values[0], values[1], values[2], values[3]))
                .collect(),
        ),
        ReadOutputs::MorphTargetWeights(weights) => {
            let raw: Vec<f32> = weights.into_f32().collect();
            collect_weight_keyframes(path, raw, input_seconds.len(), interpolation)?
        }
    };

    Ok(AnimationSourceChannel::new(
        target_node,
        target_property,
        input_seconds,
        output,
        interpolation,
    ))
}

fn collect_weight_keyframes(
    path: &AssetPath,
    raw: Vec<f32>,
    keyframe_count: usize,
    interpolation: AnimationInterpolation,
) -> Result<AnimationOutput, AssetError> {
    if keyframe_count == 0 {
        return Ok(AnimationOutput::Weights(Vec::new()));
    }
    let stride_factor = match interpolation {
        AnimationInterpolation::CubicSpline => 3,
        AnimationInterpolation::Linear | AnimationInterpolation::Step => 1,
    };
    let denom = keyframe_count.saturating_mul(stride_factor);
    if denom == 0 || !raw.len().is_multiple_of(denom) {
        return Err(AssetError::Parse {
            path: path.as_str().to_string(),
            reason: "animation weights output count is not a multiple of the keyframe count"
                .to_string(),
        });
    }
    let targets_per_keyframe = raw.len() / denom;
    if targets_per_keyframe == 0 {
        return Err(AssetError::Parse {
            path: path.as_str().to_string(),
            reason: "animation weights output declares zero morph targets per keyframe".to_string(),
        });
    }
    let chunk_size = targets_per_keyframe * stride_factor;
    Ok(AnimationOutput::Weights(
        raw.chunks_exact(chunk_size).map(<[f32]>::to_vec).collect(),
    ))
}
