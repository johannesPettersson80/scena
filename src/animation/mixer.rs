use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use super::{AnimationClip, AnimationLoopMode, AnimationMixer, AnimationPlaybackState};

impl AnimationMixer {
    pub fn new(clip: AnimationClip, import_live: Arc<AtomicBool>) -> Self {
        Self {
            clip: Arc::new(clip),
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
        self.clip.as_ref()
    }

    pub(crate) fn shared_clip(&self) -> Arc<AnimationClip> {
        Arc::clone(&self.clip)
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
