//! Stage C2: glTF animation parsing now uses the `gltf` crate's typed
//! `animation::Channel::reader()` so input/output accessor walking is
//! delegated to the gltf-crate util module (no hand-rolled component
//! reading).

use ::gltf::Document;
use ::gltf::accessor::Dimensions;
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
use super::buffers::ResolvedGltfBuffers;

pub(super) fn parse_gltf_clips(
    path: &AssetPath,
    document: &Document,
    buffers: &ResolvedGltfBuffers,
) -> Result<Vec<SceneAssetClip>, AssetError> {
    document
        .animations()
        .enumerate()
        .map(|(clip_index, animation)| {
            let channels = animation
                .channels()
                .enumerate()
                .map(|(channel_index, channel)| {
                    parse_channel(path, clip_index, channel_index, &channel, buffers)
                })
                .collect::<Result<Vec<_>, _>>()?;
            let duration_seconds = channels
                .iter()
                .flat_map(|channel| channel.input_seconds().iter().copied())
                .fold(0.0_f32, f32::max);
            let clip = AnimationSourceClip::imported(
                animation.name().map(str::to_string),
                channels,
                duration_seconds,
            )
            .map_err(|error| AssetError::Parse {
                path: path.as_str().to_string(),
                reason: format!("invalid glTF animation: {error}"),
            })?;
            Ok(SceneAssetClip { clip })
        })
        .collect()
}

fn parse_channel(
    path: &AssetPath,
    clip_index: usize,
    channel_index: usize,
    channel: &::gltf::animation::Channel<'_>,
    buffers: &ResolvedGltfBuffers,
) -> Result<AnimationSourceChannel, AssetError> {
    let target = channel.target();
    let target_node_ref = target.node();
    let target_node = target_node_ref.index();
    let target_property = match target.property() {
        GltfProperty::Translation => AnimationTarget::Translation,
        GltfProperty::Rotation => AnimationTarget::Rotation,
        GltfProperty::Scale => AnimationTarget::Scale,
        GltfProperty::MorphTargetWeights => AnimationTarget::Weights,
    };
    let sampler = channel.sampler();
    let expected_dimensions = match target_property {
        AnimationTarget::Translation | AnimationTarget::Scale => Dimensions::Vec3,
        AnimationTarget::Rotation => Dimensions::Vec4,
        AnimationTarget::Weights => Dimensions::Scalar,
    };
    if sampler.output().dimensions() != expected_dimensions {
        let property = match target_property {
            AnimationTarget::Translation => "translation",
            AnimationTarget::Rotation => "rotation",
            AnimationTarget::Scale => "scale",
            AnimationTarget::Weights => "weights",
        };
        return Err(AssetError::Parse {
            path: path.as_str().to_string(),
            reason: format!(
                "glTF {property} animation output must use {expected_dimensions:?}, found {:?}",
                sampler.output().dimensions(),
            ),
        });
    }
    let interpolation = match sampler.interpolation() {
        GltfInterpolation::Linear => AnimationInterpolation::Linear,
        GltfInterpolation::Step => AnimationInterpolation::Step,
        GltfInterpolation::CubicSpline => AnimationInterpolation::CubicSpline,
    };

    let reader = channel.reader(|buffer| buffers.reader_buffer(buffer.index()));
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
        ReadOutputs::Translations(translations) => {
            AnimationOutput::Vec3(translations.map(crate::scene::Vec3::from_array).collect())
        }
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
            let target_counts = target_node_ref
                .mesh()
                .map(|mesh| {
                    mesh.primitives()
                        .enumerate()
                        .map(|(primitive_index, primitive)| {
                            (primitive_index, primitive.morph_targets().count())
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            collect_weight_keyframes(
                WeightChannelSpec {
                    path,
                    clip_index,
                    channel_index,
                    node_index: target_node,
                    target_counts: &target_counts,
                },
                raw,
                input_seconds.len(),
                interpolation,
            )?
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

struct WeightChannelSpec<'a> {
    path: &'a AssetPath,
    clip_index: usize,
    channel_index: usize,
    node_index: usize,
    target_counts: &'a [(usize, usize)],
}

fn collect_weight_keyframes(
    context: WeightChannelSpec<'_>,
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
            path: context.path.as_str().to_string(),
            reason: "animation weights output count is not a multiple of the keyframe count"
                .to_string(),
        });
    }
    let targets_per_keyframe = raw.len() / denom;
    if targets_per_keyframe == 0 {
        return Err(AssetError::Parse {
            path: context.path.as_str().to_string(),
            reason: "animation weights output declares zero morph targets per keyframe".to_string(),
        });
    }
    for &(primitive_index, expected) in context.target_counts {
        if targets_per_keyframe != expected {
            return Err(AssetError::MorphWeightWidthMismatch {
                path: context.path.as_str().to_string(),
                clip_index: context.clip_index,
                channel_index: context.channel_index,
                node_index: context.node_index,
                primitive_index,
                expected,
                actual: targets_per_keyframe,
            });
        }
    }
    let chunk_size = targets_per_keyframe;
    Ok(AnimationOutput::Weights(
        raw.chunks_exact(chunk_size).map(<[f32]>::to_vec).collect(),
    ))
}
