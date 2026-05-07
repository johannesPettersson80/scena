//! glTF animation playback, mixer state, skinning, and morph-target support.

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use slotmap::new_key_type;

use crate::scene::{NodeKey, Quat, Vec3};

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
    clip: AnimationClip,
    state: AnimationPlaybackState,
    time_seconds: f32,
    speed: f32,
    loop_mode: AnimationLoopMode,
    import_live: Arc<AtomicBool>,
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
    pub fn new(
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
        let channels = self
            .channels
            .iter()
            .filter_map(|channel| channel.rebind(&mut map_node, &mut map_vec3))
            .collect();
        AnimationClip::new(key, self.name.clone(), channels, self.duration_seconds)
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

    fn rebind<F, G>(&self, map_node: &mut F, map_vec3: &mut G) -> Option<AnimationChannel>
    where
        F: FnMut(usize) -> Option<NodeKey>,
        G: FnMut(AnimationTarget, Vec3) -> Vec3,
    {
        let output = match &self.output {
            AnimationOutput::Vec3(values) => AnimationOutput::Vec3(
                values
                    .iter()
                    .copied()
                    .map(|value| map_vec3(self.target, value))
                    .collect(),
            ),
            AnimationOutput::Quat(values) => AnimationOutput::Quat(values.clone()),
            AnimationOutput::Weights(values) => AnimationOutput::Weights(values.clone()),
        };
        Some(AnimationChannel::new(
            map_node(self.source_node)?,
            self.target,
            self.input_seconds.clone(),
            output,
            self.interpolation,
        ))
    }
}

impl AnimationMixer {
    pub fn new(clip: AnimationClip, import_live: Arc<AtomicBool>) -> Self {
        Self {
            clip,
            state: AnimationPlaybackState::Stopped,
            time_seconds: 0.0,
            speed: 1.0,
            loop_mode: AnimationLoopMode::Once,
            import_live,
        }
    }

    pub const fn state(&self) -> AnimationPlaybackState {
        self.state
    }

    pub const fn time_seconds(&self) -> f32 {
        self.time_seconds
    }

    pub const fn speed(&self) -> f32 {
        self.speed
    }

    pub const fn loop_mode(&self) -> AnimationLoopMode {
        self.loop_mode
    }

    pub fn clip(&self) -> &AnimationClip {
        &self.clip
    }

    pub(crate) fn is_stale(&self) -> bool {
        !self.import_live.load(Ordering::Acquire)
    }

    pub(crate) fn play(&mut self) {
        self.state = AnimationPlaybackState::Playing;
    }

    pub(crate) fn pause(&mut self) {
        self.state = AnimationPlaybackState::Paused;
    }

    pub(crate) fn stop(&mut self) {
        self.state = AnimationPlaybackState::Stopped;
        self.time_seconds = 0.0;
    }

    pub(crate) fn seek(&mut self, time_seconds: f32) {
        self.time_seconds = self.clamp_or_wrap_time(time_seconds.max(0.0));
    }

    pub(crate) fn set_speed(&mut self, speed: f32) {
        self.speed = if speed.is_finite() { speed } else { 1.0 };
    }

    pub(crate) fn set_loop_mode(&mut self, loop_mode: AnimationLoopMode) {
        self.loop_mode = loop_mode;
        self.time_seconds = self.clamp_or_wrap_time(self.time_seconds);
    }

    pub(crate) fn advance(&mut self, delta_seconds: f32) {
        if self.state != AnimationPlaybackState::Playing {
            return;
        }
        let delta = if delta_seconds.is_finite() {
            delta_seconds.max(0.0)
        } else {
            0.0
        };
        self.time_seconds = self.clamp_or_wrap_time(self.time_seconds + delta * self.speed);
    }

    fn clamp_or_wrap_time(&self, time_seconds: f32) -> f32 {
        let duration = self.clip.duration_seconds.max(0.0);
        if duration <= f32::EPSILON {
            return 0.0;
        }
        match self.loop_mode {
            AnimationLoopMode::Once => time_seconds.clamp(0.0, duration),
            AnimationLoopMode::Repeat => time_seconds.rem_euclid(duration),
        }
    }
}

fn sample_vec3(
    times: &[f32],
    values: &[Vec3],
    interpolation: AnimationInterpolation,
    time_seconds: f32,
) -> Option<Vec3> {
    if times.is_empty() || values.is_empty() {
        return None;
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
                AnimationInterpolation::Linear | AnimationInterpolation::CubicSpline => {
                    let amount = ((time_seconds - start) / (end - start)).clamp(0.0, 1.0);
                    lerp_vec3(left, right, amount)
                }
            });
        }
    }
    values.last().copied()
}

fn sample_quat(
    times: &[f32],
    values: &[Quat],
    interpolation: AnimationInterpolation,
    time_seconds: f32,
) -> Option<Quat> {
    if times.is_empty() || values.is_empty() {
        return None;
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
                AnimationInterpolation::Linear | AnimationInterpolation::CubicSpline => {
                    let amount = ((time_seconds - start) / (end - start)).clamp(0.0, 1.0);
                    slerp_quat(left, right, amount)
                }
            });
        }
    }
    values.last().copied().map(normalize_quat)
}

fn lerp_vec3(left: Vec3, right: Vec3, amount: f32) -> Vec3 {
    Vec3::new(
        left.x + (right.x - left.x) * amount,
        left.y + (right.y - left.y) * amount,
        left.z + (right.z - left.z) * amount,
    )
}

fn normalize_quat(value: Quat) -> Quat {
    let length =
        (value.x * value.x + value.y * value.y + value.z * value.z + value.w * value.w).sqrt();
    if length <= f32::EPSILON || !length.is_finite() {
        return Quat::IDENTITY;
    }
    Quat {
        x: value.x / length,
        y: value.y / length,
        z: value.z / length,
        w: value.w / length,
    }
}

fn slerp_quat(left: Quat, right: Quat, amount: f32) -> Quat {
    let mut right = right;
    let mut dot = left.x * right.x + left.y * right.y + left.z * right.z + left.w * right.w;
    if dot < 0.0 {
        dot = -dot;
        right = Quat {
            x: -right.x,
            y: -right.y,
            z: -right.z,
            w: -right.w,
        };
    }
    if dot > 0.9995 {
        return normalize_quat(Quat {
            x: left.x + (right.x - left.x) * amount,
            y: left.y + (right.y - left.y) * amount,
            z: left.z + (right.z - left.z) * amount,
            w: left.w + (right.w - left.w) * amount,
        });
    }
    let theta_0 = dot.acos();
    let theta = theta_0 * amount;
    let sin_theta = theta.sin();
    let sin_theta_0 = theta_0.sin();
    let left_scale = theta.cos() - dot * sin_theta / sin_theta_0;
    let right_scale = sin_theta / sin_theta_0;
    normalize_quat(Quat {
        x: left.x * left_scale + right.x * right_scale,
        y: left.y * left_scale + right.y * right_scale,
        z: left.z * left_scale + right.z * right_scale,
        w: left.w * left_scale + right.w * right_scale,
    })
}
