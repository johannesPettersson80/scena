//! glTF animation playback, mixer state, skinning, and morph-target support.

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use slotmap::new_key_type;

use crate::diagnostics::AnimationError;
use crate::scene::{NodeKey, Quat, Vec3};

mod sampling;
use self::sampling::{
    sample_quat, sample_quat_profiled, sample_vec3, sample_vec3_profiled, sample_weights,
    sample_weights_into, sample_weights_into_profiled,
};
mod validation;
use self::validation::{validate_clip, validate_imported_clip, validate_imported_source_clip};
mod mixer;

new_key_type! {
    pub struct AnimationMixerKey;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AnimationClipKey(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationPlaybackState {
    Playing,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationLoopMode {
    Once,
    Repeat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationTarget {
    Translation,
    Rotation,
    Scale,
    Weights,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationInterpolation {
    Linear,
    Step,
    CubicSpline,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimationClip {
    key: AnimationClipKey,
    name: Option<String>,
    channels: Vec<AnimationChannel>,
    duration_seconds: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimationSourceClip {
    name: Option<String>,
    channels: Vec<AnimationSourceChannel>,
    duration_seconds: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimationChannel {
    target_node: NodeKey,
    target: AnimationTarget,
    input_seconds: Vec<f32>,
    output: AnimationOutput,
    interpolation: AnimationInterpolation,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimationSourceChannel {
    source_node: usize,
    target: AnimationTarget,
    input_seconds: Vec<f32>,
    output: AnimationOutput,
    interpolation: AnimationInterpolation,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AnimationOutput {
    Vec3(Vec<Vec3>),
    Quat(Vec<Quat>),
    Weights(Vec<Vec<f32>>),
}

#[derive(Debug, Clone)]
pub struct AnimationMixer {
    clip: Arc<AnimationClip>,
    state: AnimationPlaybackState,
    time_seconds: f32,
    speed: f32,
    loop_mode: AnimationLoopMode,
    import_live: Arc<AtomicBool>,
}

/// Deterministic work and payload-copy counters from one animation update.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AnimationUpdateMetrics {
    pub channels_scanned: u64,
    pub keyframe_intervals_tested: u64,
    pub weight_values_written: u64,
    pub weight_bytes_written: u64,
    pub clip_clone_bytes: u64,
}

impl AnimationClipKey {
    pub(crate) fn fresh() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl AnimationClip {
    pub fn authored(
        name: Option<String>,
        channels: Vec<AnimationChannel>,
        duration_seconds: f32,
    ) -> Result<Self, AnimationError> {
        Self::new(AnimationClipKey::fresh(), name, channels, duration_seconds)
    }

    pub fn new(
        key: AnimationClipKey,
        name: Option<String>,
        channels: Vec<AnimationChannel>,
        duration_seconds: f32,
    ) -> Result<Self, AnimationError> {
        validate_clip(&channels, duration_seconds)?;
        Ok(Self {
            key,
            name,
            channels,
            duration_seconds,
        })
    }

    pub(crate) fn new_unchecked(
        key: AnimationClipKey,
        name: Option<String>,
        channels: Vec<AnimationChannel>,
        duration_seconds: f32,
    ) -> Self {
        Self {
            key,
            name,
            channels,
            duration_seconds,
        }
    }

    fn imported(
        key: AnimationClipKey,
        name: Option<String>,
        channels: Vec<AnimationChannel>,
        duration_seconds: f32,
    ) -> Result<Self, AnimationError> {
        validate_imported_clip(&channels, duration_seconds)?;
        Ok(Self {
            key,
            name,
            channels,
            duration_seconds,
        })
    }

    pub const fn key(&self) -> AnimationClipKey {
        self.key
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn channels(&self) -> &[AnimationChannel] {
        &self.channels
    }

    pub const fn duration_seconds(&self) -> f32 {
        self.duration_seconds
    }
}

impl AnimationSourceClip {
    pub fn new(
        name: Option<String>,
        channels: Vec<AnimationSourceChannel>,
        duration_seconds: f32,
    ) -> Self {
        Self {
            name,
            channels,
            duration_seconds,
        }
    }

    pub(crate) fn imported(
        name: Option<String>,
        channels: Vec<AnimationSourceChannel>,
        duration_seconds: f32,
    ) -> Result<Self, AnimationError> {
        validate_imported_source_clip(&channels, duration_seconds)?;
        Ok(Self {
            name,
            channels,
            duration_seconds,
        })
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn channels(&self) -> &[AnimationSourceChannel] {
        &self.channels
    }

    pub const fn duration_seconds(&self) -> f32 {
        self.duration_seconds
    }

    pub fn rebind<F, G>(
        &self,
        key: AnimationClipKey,
        mut map_node: F,
        mut map_vec3: G,
    ) -> AnimationClip
    where
        F: FnMut(usize) -> Option<NodeKey>,
        G: FnMut(AnimationTarget, Vec3) -> Vec3,
    {
        let mut keep_rotation = |_: AnimationInterpolation, _: usize, value: Quat| value;
        let channels = self
            .channels
            .iter()
            .filter_map(|channel| channel.rebind(&mut map_node, &mut map_vec3, &mut keep_rotation))
            .collect();
        AnimationClip::new_unchecked(key, self.name.clone(), channels, self.duration_seconds)
    }

    pub(crate) fn rebind_imported_many<F, G, H>(
        &self,
        key: AnimationClipKey,
        mut map_nodes: F,
        mut map_vec3: G,
        mut map_quat: H,
    ) -> Result<AnimationClip, AnimationError>
    where
        F: FnMut(usize, AnimationTarget) -> Vec<NodeKey>,
        G: FnMut(AnimationTarget, Vec3) -> Vec3,
        H: FnMut(AnimationInterpolation, usize, Quat) -> Quat,
    {
        let mut channels = Vec::new();
        for source in &self.channels {
            let output = source.rebound_output(&mut map_vec3, &mut map_quat);
            for target_node in map_nodes(source.source_node, source.target) {
                channels.push(AnimationChannel::new(
                    target_node,
                    source.target,
                    source.input_seconds.clone(),
                    output.clone(),
                    source.interpolation,
                ));
            }
        }
        AnimationClip::imported(key, self.name.clone(), channels, self.duration_seconds)
    }
}

impl AnimationChannel {
    pub fn new(
        target_node: NodeKey,
        target: AnimationTarget,
        input_seconds: Vec<f32>,
        output: AnimationOutput,
        interpolation: AnimationInterpolation,
    ) -> Self {
        Self {
            target_node,
            target,
            input_seconds,
            output,
            interpolation,
        }
    }

    pub const fn target_node(&self) -> NodeKey {
        self.target_node
    }

    pub const fn target(&self) -> AnimationTarget {
        self.target
    }

    pub fn sample_vec3(&self, time_seconds: f32) -> Option<Vec3> {
        let AnimationOutput::Vec3(values) = &self.output else {
            return None;
        };
        sample_vec3(
            &self.input_seconds,
            values,
            self.interpolation,
            time_seconds,
        )
    }

    pub(crate) fn sample_vec3_profiled(
        &self,
        time_seconds: f32,
        intervals_tested: &mut u64,
    ) -> Option<Vec3> {
        let AnimationOutput::Vec3(values) = &self.output else {
            return None;
        };
        sample_vec3_profiled(
            &self.input_seconds,
            values,
            self.interpolation,
            time_seconds,
            intervals_tested,
        )
    }

    pub fn sample_quat(&self, time_seconds: f32) -> Option<Quat> {
        let AnimationOutput::Quat(values) = &self.output else {
            return None;
        };
        sample_quat(
            &self.input_seconds,
            values,
            self.interpolation,
            time_seconds,
        )
    }

    pub(crate) fn sample_quat_profiled(
        &self,
        time_seconds: f32,
        intervals_tested: &mut u64,
    ) -> Option<Quat> {
        let AnimationOutput::Quat(values) = &self.output else {
            return None;
        };
        sample_quat_profiled(
            &self.input_seconds,
            values,
            self.interpolation,
            time_seconds,
            intervals_tested,
        )
    }

    pub fn sample_weights(&self, time_seconds: f32) -> Option<Vec<f32>> {
        let AnimationOutput::Weights(values) = &self.output else {
            return None;
        };
        sample_weights(
            &self.input_seconds,
            values,
            self.interpolation,
            time_seconds,
        )
    }

    pub(crate) fn sample_weights_into_profiled(
        &self,
        time_seconds: f32,
        output: &mut Vec<f32>,
        intervals_tested: &mut u64,
    ) -> bool {
        let AnimationOutput::Weights(values) = &self.output else {
            return false;
        };
        sample_weights_into_profiled(
            &self.input_seconds,
            values,
            self.interpolation,
            time_seconds,
            output,
            intervals_tested,
        )
    }

    pub(crate) fn sample_weights_into(&self, time_seconds: f32, output: &mut Vec<f32>) -> bool {
        let AnimationOutput::Weights(values) = &self.output else {
            return false;
        };
        sample_weights_into(
            &self.input_seconds,
            values,
            self.interpolation,
            time_seconds,
            output,
        )
    }
}

impl AnimationSourceChannel {
    pub fn new(
        source_node: usize,
        target: AnimationTarget,
        input_seconds: Vec<f32>,
        output: AnimationOutput,
        interpolation: AnimationInterpolation,
    ) -> Self {
        Self {
            source_node,
            target,
            input_seconds,
            output,
            interpolation,
        }
    }

    pub const fn source_node(&self) -> usize {
        self.source_node
    }

    pub fn input_seconds(&self) -> &[f32] {
        &self.input_seconds
    }

    fn rebind<F, G, H>(
        &self,
        map_node: &mut F,
        map_vec3: &mut G,
        map_quat: &mut H,
    ) -> Option<AnimationChannel>
    where
        F: FnMut(usize) -> Option<NodeKey>,
        G: FnMut(AnimationTarget, Vec3) -> Vec3,
        H: FnMut(AnimationInterpolation, usize, Quat) -> Quat,
    {
        let output = self.rebound_output(map_vec3, map_quat);
        Some(AnimationChannel::new(
            map_node(self.source_node)?,
            self.target,
            self.input_seconds.clone(),
            output,
            self.interpolation,
        ))
    }

    fn rebound_output<G, H>(&self, map_vec3: &mut G, map_quat: &mut H) -> AnimationOutput
    where
        G: FnMut(AnimationTarget, Vec3) -> Vec3,
        H: FnMut(AnimationInterpolation, usize, Quat) -> Quat,
    {
        match &self.output {
            AnimationOutput::Vec3(values) => AnimationOutput::Vec3(
                values
                    .iter()
                    .copied()
                    .map(|value| map_vec3(self.target, value))
                    .collect(),
            ),
            AnimationOutput::Quat(values) => AnimationOutput::Quat(
                values
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(index, value)| map_quat(self.interpolation, index, value))
                    .collect(),
            ),
            AnimationOutput::Weights(values) => AnimationOutput::Weights(values.clone()),
        }
    }
}
