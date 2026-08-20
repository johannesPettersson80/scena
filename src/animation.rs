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
use self::validation::{
    validate_clip, validate_imported_clip, validate_imported_source_clip, validate_source_clip,
};
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
    #[deprecated(
        since = "1.10.0",
        note = "use AnimationSourceClip::try_new so invalid authored keyframes return AnimationError"
    )]
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

    pub fn try_new(
        name: Option<String>,
        channels: Vec<AnimationSourceChannel>,
        duration_seconds: f32,
    ) -> Result<Self, AnimationError> {
        validate_source_clip(&channels, duration_seconds)?;
        Ok(Self {
            name,
            channels,
            duration_seconds,
        })
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

    /// Rebinds this source clip, panicking if the result is invalid.
    ///
    /// # Panics
    ///
    /// Panics when `map_vec3` produces a non-finite value, when
    /// `duration_seconds` is not finite and positive, or whenever
    /// [`Self::try_rebind`] would otherwise return an
    /// [`AnimationError`] — that is, on any input a host could supply at
    /// runtime.
    ///
    /// This is a retained pre-1.10.0 compatibility wrapper. It is kept rather
    /// than removed because deleting it would be a breaking change, and it
    /// cannot be made non-panicking without changing its return type. Use
    /// [`Self::try_rebind`], which returns the error instead; this wrapper is
    /// scheduled for removal in the next major release.
    #[deprecated(
        since = "1.10.0",
        note = "use AnimationSourceClip::try_rebind so invalid rebound values return AnimationError"
    )]
    pub fn rebind<F, G>(&self, key: AnimationClipKey, map_node: F, map_vec3: G) -> AnimationClip
    where
        F: FnMut(usize) -> Option<NodeKey>,
        G: FnMut(AnimationTarget, Vec3) -> Vec3,
    {
        self.try_rebind(key, map_node, map_vec3).expect(
            "deprecated AnimationSourceClip::rebind received an invalid clip; use try_rebind",
        )
    }

    pub fn try_rebind<F, G>(
        &self,
        key: AnimationClipKey,
        mut map_node: F,
        mut map_vec3: G,
    ) -> Result<AnimationClip, AnimationError>
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
        AnimationClip::new(key, self.name.clone(), channels, self.duration_seconds)
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

#[cfg(test)]
mod validation_tests {
    use super::*;

    fn source_channel(
        times: Vec<f32>,
        output: AnimationOutput,
        interpolation: AnimationInterpolation,
    ) -> AnimationSourceChannel {
        AnimationSourceChannel::new(
            0,
            AnimationTarget::Translation,
            times,
            output,
            interpolation,
        )
    }

    fn channel_for(
        target: AnimationTarget,
        times: Vec<f32>,
        output: AnimationOutput,
        interpolation: AnimationInterpolation,
    ) -> AnimationSourceChannel {
        AnimationSourceChannel::new(7, target, times, output, interpolation)
    }

    fn assert_invalid(channel: AnimationSourceChannel, duration: f32, expected: &str) {
        let error = AnimationSourceClip::try_new(None, vec![channel], duration)
            .expect_err("invalid source clip must fail closed");
        let message = error.to_string();
        assert!(
            message.contains(expected),
            "expected '{expected}' in '{message}'"
        );
    }

    /// R05: the retained deprecated `rebind` wrapper is a panic surface.
    /// Its contract is now documented rather than silently relied upon, and
    /// pinned here so a future refactor cannot quietly change it.
    #[test]
    #[should_panic(expected = "use try_rebind")]
    fn deprecated_rebind_panics_where_try_rebind_returns_an_error() {
        let clip = AnimationSourceClip::try_new(
            None,
            vec![source_channel(
                vec![0.0, 1.0],
                AnimationOutput::Vec3(vec![Vec3::ZERO, Vec3::ZERO]),
                AnimationInterpolation::Linear,
            )],
            1.0,
        )
        .expect("valid source clip builds");
        let key = AnimationClipKey::fresh();

        // `try_rebind` reports the poisoned value as an error...
        let error = clip.try_rebind(
            key,
            |_| Some(NodeKey::default()),
            |_, _| Vec3::new(f32::NAN, 0.0, 0.0),
        );
        assert!(
            error.is_err(),
            "try_rebind must reject a non-finite rebound value"
        );

        // ...while the deprecated wrapper panics on the same input.
        #[allow(deprecated)]
        let _ = clip.rebind(
            key,
            |_| Some(NodeKey::default()),
            |_, _| Vec3::new(f32::NAN, 0.0, 0.0),
        );
    }

    #[test]
    fn authored_source_clips_validate_before_rebinding() {
        let invalid = AnimationSourceClip::try_new(
            Some("poisoned".to_owned()),
            vec![source_channel(
                vec![0.0, 1.0],
                AnimationOutput::Vec3(vec![Vec3::ZERO, Vec3::new(f32::NAN, 0.0, 0.0)]),
                AnimationInterpolation::Linear,
            )],
            1.0,
        );
        assert!(
            invalid.is_err(),
            "non-finite authored output must fail closed"
        );

        let source = AnimationSourceClip::try_new(
            Some("move".to_owned()),
            vec![source_channel(
                vec![0.0, 1.0],
                AnimationOutput::Vec3(vec![Vec3::ZERO, Vec3::ONE]),
                AnimationInterpolation::Linear,
            )],
            1.0,
        )
        .expect("valid source clip");
        let rebound = source.try_rebind(AnimationClipKey::fresh(), |_| None, |_, value| value);
        assert!(
            rebound.is_err(),
            "rebinding away every channel must not create an unchecked empty clip"
        );
    }

    #[test]
    fn authored_source_clip_validation_covers_time_value_shape_and_duration_matrix() {
        let valid_output = || AnimationOutput::Vec3(vec![Vec3::ZERO, Vec3::ONE]);
        for (times, expected) in [
            (vec![0.0, f32::NAN], "time[1] must be finite"),
            (vec![0.0, f32::INFINITY], "time[1] must be finite"),
            (vec![0.5, 0.25], "strictly increasing"),
            (vec![0.5, 0.5], "strictly increasing"),
        ] {
            assert_invalid(
                source_channel(times, valid_output(), AnimationInterpolation::Linear),
                1.0,
                expected,
            );
        }
        for duration in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert_invalid(
                source_channel(
                    vec![0.0, 1.0],
                    valid_output(),
                    AnimationInterpolation::Linear,
                ),
                duration,
                "duration_seconds must be finite and positive",
            );
        }
        assert_invalid(
            source_channel(
                vec![0.0, 1.0],
                AnimationOutput::Vec3(vec![Vec3::ZERO]),
                AnimationInterpolation::Linear,
            ),
            1.0,
            "output length must be 2",
        );
        assert_invalid(
            source_channel(
                vec![0.0, 1.0],
                AnimationOutput::Vec3(vec![Vec3::ZERO; 5]),
                AnimationInterpolation::CubicSpline,
            ),
            1.0,
            "output length must be 6",
        );
        assert_invalid(
            source_channel(
                vec![0.0, 1.0],
                AnimationOutput::Vec3(vec![Vec3::ZERO, Vec3::new(f32::INFINITY, 0.0, 0.0)]),
                AnimationInterpolation::Linear,
            ),
            1.0,
            "output[1] must be finite",
        );
        assert_invalid(
            channel_for(
                AnimationTarget::Rotation,
                vec![0.0],
                AnimationOutput::Vec3(vec![Vec3::ZERO]),
                AnimationInterpolation::Step,
            ),
            1.0,
            "rotation channel output must use VEC4",
        );
        assert_invalid(
            channel_for(
                AnimationTarget::Weights,
                vec![0.0, 1.0],
                AnimationOutput::Weights(vec![vec![0.0, 1.0], vec![1.0]]),
                AnimationInterpolation::Linear,
            ),
            1.0,
            "inconsistent width",
        );
        assert_invalid(
            channel_for(
                AnimationTarget::Weights,
                vec![0.0],
                AnimationOutput::Weights(vec![vec![f32::NAN]]),
                AnimationInterpolation::Step,
            ),
            1.0,
            "must be finite",
        );
    }

    #[test]
    fn rebind_revalidates_mapped_values() {
        let source = AnimationSourceClip::try_new(
            Some("mapped".to_owned()),
            vec![source_channel(
                vec![0.0, 1.0],
                AnimationOutput::Vec3(vec![Vec3::ZERO, Vec3::ONE]),
                AnimationInterpolation::Linear,
            )],
            1.0,
        )
        .unwrap();
        let error = source
            .try_rebind(
                AnimationClipKey::fresh(),
                |_| Some(NodeKey::default()),
                |_, _| Vec3::new(f32::NAN, 0.0, 0.0),
            )
            .expect_err("mapped non-finite values must be revalidated");
        assert!(error.to_string().contains("must be finite"));
    }
}
