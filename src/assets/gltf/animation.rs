//! glTF animation parsing. The `gltf` crate supplies the typed document
//! model; sampler input and output accessors are decoded through the checked
//! accessor reader so an unexpected component type is a structured error
//! rather than an `unreachable!()` inside the upstream channel reader.

use ::gltf::Document;
use ::gltf::accessor::{Accessor, DataType, Dimensions, Item as AccessorItem};
use ::gltf::animation::Interpolation as GltfInterpolation;
use ::gltf::animation::Property as GltfProperty;

use crate::animation::{
    AnimationInterpolation, AnimationOutput, AnimationSourceChannel, AnimationSourceClip,
    AnimationTarget,
};
use crate::assets::AssetPath;
use crate::diagnostics::AssetError;
use crate::scene::Quat;

use super::SceneAssetClip;
use super::buffers::ResolvedGltfBuffers;
use super::typed_accessors::{
    accessor_error, decode_accessor, normalize_i8, normalize_i16, normalize_u8, normalize_u16,
};

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

    let streams = ChannelStreams {
        path,
        buffers,
        clip_index,
        channel_index,
    };
    let input_seconds = streams.read_input_seconds(&sampler.input())?;
    let output_accessor = sampler.output();
    let output = match target_property {
        AnimationTarget::Translation | AnimationTarget::Scale => AnimationOutput::Vec3(
            streams
                .read_output_vec3(&output_accessor)?
                .into_iter()
                .map(crate::scene::Vec3::from_array)
                .collect(),
        ),
        AnimationTarget::Rotation => AnimationOutput::Quat(
            streams
                .read_output_rotations(&output_accessor)?
                .into_iter()
                .map(|values| Quat::from_xyzw(values[0], values[1], values[2], values[3]))
                .collect(),
        ),
        AnimationTarget::Weights => {
            let raw = streams.read_output_weights(&output_accessor)?;
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
                    buffers,
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

/// Decodes one channel's sampler accessors, naming the clip and channel in
/// every error. The output accessor's dimensions are validated by the caller.
struct ChannelStreams<'a> {
    path: &'a AssetPath,
    buffers: &'a ResolvedGltfBuffers,
    clip_index: usize,
    channel_index: usize,
}

impl ChannelStreams<'_> {
    fn label(&self, stream: &str) -> String {
        format!(
            "animation {} channel {} sampler {stream}",
            self.clip_index, self.channel_index
        )
    }

    fn read_input_seconds(&self, accessor: &Accessor<'_>) -> Result<Vec<f32>, AssetError> {
        let label = self.label("input");
        if accessor.data_type() != DataType::F32 || accessor.dimensions() != Dimensions::Scalar {
            return Err(accessor_error(
                self.path,
                &label,
                format!(
                    "must use FLOAT SCALAR keyframe times, found {:?} {:?}",
                    accessor.data_type(),
                    accessor.dimensions()
                ),
            ));
        }
        self.decode(accessor, &label, std::convert::identity::<f32>)
    }

    fn read_output_vec3(&self, accessor: &Accessor<'_>) -> Result<Vec<[f32; 3]>, AssetError> {
        let label = self.label("output");
        if accessor.data_type() != DataType::F32 {
            return Err(accessor_error(
                self.path,
                &label,
                format!(
                    "must use FLOAT components for translation and scale, found {:?}",
                    accessor.data_type()
                ),
            ));
        }
        self.decode(accessor, &label, std::convert::identity::<[f32; 3]>)
    }

    /// Integer rotation keys are normalized per the glTF animation rules.
    fn read_output_rotations(&self, accessor: &Accessor<'_>) -> Result<Vec<[f32; 4]>, AssetError> {
        let label = self.label("output");
        macro_rules! read_quat {
            ($component:ty, $convert:expr) => {
                self.decode(accessor, &label, |value: [$component; 4]| {
                    value.map($convert)
                })
            };
        }
        match accessor.data_type() {
            DataType::F32 => read_quat!(f32, std::convert::identity),
            DataType::I8 => read_quat!(i8, normalize_i8),
            DataType::U8 => read_quat!(u8, normalize_u8),
            DataType::I16 => read_quat!(i16, normalize_i16),
            DataType::U16 => read_quat!(u16, normalize_u16),
            DataType::U32 => Err(self.unsigned_int_output(&label, "rotation")),
        }
    }

    /// Integer morph-weight keys are normalized per the glTF animation rules.
    fn read_output_weights(&self, accessor: &Accessor<'_>) -> Result<Vec<f32>, AssetError> {
        let label = self.label("output");
        match accessor.data_type() {
            DataType::F32 => self.decode(accessor, &label, std::convert::identity::<f32>),
            DataType::I8 => self.decode(accessor, &label, normalize_i8),
            DataType::U8 => self.decode(accessor, &label, normalize_u8),
            DataType::I16 => self.decode(accessor, &label, normalize_i16),
            DataType::U16 => self.decode(accessor, &label, normalize_u16),
            DataType::U32 => Err(self.unsigned_int_output(&label, "weights")),
        }
    }

    fn decode<T: AccessorItem, Out>(
        &self,
        accessor: &Accessor<'_>,
        label: &str,
        convert: impl FnMut(T) -> Out,
    ) -> Result<Vec<Out>, AssetError> {
        decode_accessor(self.path, self.buffers, accessor, label, convert)
    }

    fn unsigned_int_output(&self, label: &str, property: &str) -> AssetError {
        accessor_error(
            self.path,
            label,
            format!(
                "must use FLOAT or normalized BYTE/UNSIGNED_BYTE/SHORT/UNSIGNED_SHORT components for {property}, found UNSIGNED_INT"
            ),
        )
    }
}

struct WeightChannelSpec<'a> {
    path: &'a AssetPath,
    buffers: &'a ResolvedGltfBuffers,
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
    // Each keyframe becomes its own vector; with one morph target that header
    // outweighs the four decoded bytes it holds, so it is reserved explicitly.
    context.buffers.charge_accessor_bytes(
        context.path,
        &format!(
            "animation {} channel {} weight keyframes",
            context.clip_index, context.channel_index
        ),
        raw.len() / chunk_size,
        size_of::<Vec<f32>>(),
    )?;
    Ok(AnimationOutput::Weights(
        raw.chunks_exact(chunk_size).map(<[f32]>::to_vec).collect(),
    ))
}
