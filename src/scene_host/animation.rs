use serde::{Deserialize, Serialize};

use super::reporting::{SceneHostAnimationClipV1, SceneHostAnimationInventoryV1};
use super::{SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::AssetFetcher;
use crate::animation::{AnimationLoopMode, AnimationMixerKey};
use crate::diagnostics::AnimationError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SceneHostAnimationLoopMode {
    #[default]
    Once,
    Repeat,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SceneHostAnimationPlayOptions {
    #[serde(default)]
    pub loop_mode: SceneHostAnimationLoopMode,
    #[serde(default = "default_animation_speed")]
    pub speed: f32,
}

impl Default for SceneHostAnimationPlayOptions {
    fn default() -> Self {
        Self {
            loop_mode: SceneHostAnimationLoopMode::Once,
            speed: default_animation_speed(),
        }
    }
}

impl From<SceneHostAnimationLoopMode> for AnimationLoopMode {
    fn from(value: SceneHostAnimationLoopMode) -> Self {
        match value {
            SceneHostAnimationLoopMode::Once => Self::Once,
            SceneHostAnimationLoopMode::Repeat => Self::Repeat,
        }
    }
}

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn animation_inventory_json(&self, import: u64) -> Result<String, SceneHostError> {
        let import = self.resolve_import(import)?;
        let clips = import
            .clips()?
            .iter()
            .filter_map(|clip| {
                Some(SceneHostAnimationClipV1 {
                    name: clip.name()?.to_owned(),
                    duration_seconds: clip.duration_seconds(),
                    channel_count: clip.channels().len(),
                })
            })
            .collect::<Vec<_>>();
        serde_json::to_string(&SceneHostAnimationInventoryV1::new(clips)).map_err(|error| {
            SceneHostError::new(
                SceneHostErrorCode::Inspect,
                format!("animation inventory serialization failed: {error}"),
            )
        })
    }

    pub fn play_animation(
        &mut self,
        import: u64,
        clip_name: &str,
        options: SceneHostAnimationPlayOptions,
    ) -> Result<u64, SceneHostError> {
        validate_speed(options.speed)?;
        let import = self.resolve_import(import)?.clone();
        let mixer = self
            .scene
            .create_animation_mixer(&import, clip_name)
            .map_err(map_animation_error)?;
        self.scene
            .set_animation_loop_mode(mixer, options.loop_mode.into())
            .map_err(map_animation_error)?;
        self.scene
            .set_animation_speed(mixer, options.speed)
            .map_err(map_animation_error)?;
        self.scene
            .play_animation(mixer)
            .map_err(map_animation_error)?;
        Ok(self.animation_handles.insert(mixer))
    }

    pub fn pause_animation(&mut self, handle: u64) -> Result<(), SceneHostError> {
        let mixer = self.resolve_animation_handle(handle)?;
        self.scene
            .pause_animation(mixer)
            .map_err(map_animation_error)
    }

    pub fn stop_animation(&mut self, handle: u64) -> Result<(), SceneHostError> {
        let mixer = self.resolve_animation_handle(handle)?;
        self.scene
            .stop_animation(mixer)
            .map_err(map_animation_error)
    }

    pub fn seek_animation(&mut self, handle: u64, seconds: f64) -> Result<(), SceneHostError> {
        let seconds = validate_time_seconds("seek seconds", seconds)?;
        let mixer = self.resolve_animation_handle(handle)?;
        let duration = self
            .scene
            .animation_mixer(mixer)
            .map_err(map_animation_error)?
            .clip()
            .duration_seconds();
        if seconds > duration {
            return Err(invalid_input(format!(
                "seek seconds must be <= clip duration {duration}, got {seconds}"
            )));
        }
        self.scene
            .seek_animation(mixer, seconds)
            .map_err(map_animation_error)
    }

    pub fn set_animation_speed(&mut self, handle: u64, speed: f64) -> Result<(), SceneHostError> {
        let speed = validate_speed(speed)?;
        let mixer = self.resolve_animation_handle(handle)?;
        self.scene
            .set_animation_speed(mixer, speed)
            .map_err(map_animation_error)
    }

    pub fn animation_clip_for_handle(
        &self,
        handle: u64,
    ) -> Result<&crate::animation::AnimationClip, SceneHostError> {
        let mixer = self.resolve_animation_handle(handle)?;
        Ok(self
            .scene
            .animation_mixer(mixer)
            .map_err(map_animation_error)?
            .clip())
    }

    pub(super) fn animation_timeline_binding(
        &self,
        handle: u64,
    ) -> Result<(f64, AnimationLoopMode), SceneHostError> {
        let mixer = self.resolve_animation_handle(handle)?;
        let mixer = self
            .scene
            .animation_mixer(mixer)
            .map_err(map_animation_error)?;
        Ok((
            f64::from(mixer.clip().duration_seconds()),
            mixer.loop_mode(),
        ))
    }

    pub(super) fn advance_animation(
        &mut self,
        handle: u64,
        delta_seconds: f64,
    ) -> Result<(), SceneHostError> {
        let delta_seconds = validate_time_seconds("animation_time seconds", delta_seconds)?;
        let mixer = self.resolve_animation_handle(handle)?;
        self.scene
            .update_animation(mixer, delta_seconds)
            .map_err(map_animation_error)
    }

    pub fn advance(&mut self, delta_seconds: f64) -> Result<(), SceneHostError> {
        let delta_seconds = validate_time_seconds("advance delta_seconds", delta_seconds)?;
        let mixers = self.animation_handles.values().copied().collect::<Vec<_>>();
        for mixer in mixers {
            self.scene
                .update_animation(mixer, delta_seconds)
                .map_err(map_animation_error)?;
        }
        self.advance_transitions(delta_seconds)
    }

    pub(super) fn resolve_animation_handle(
        &self,
        handle: u64,
    ) -> Result<AnimationMixerKey, SceneHostError> {
        self.animation_handles
            .get(
                handle,
                SceneHostErrorCode::AnimationHandleNotFound,
                SceneHostErrorCode::StaleAnimationHandle,
            )
            .copied()
    }

    pub(super) fn invalidate_stale_animation_handles(&mut self) {
        let stale = self
            .animation_handles
            .entries()
            .filter_map(|(handle, mixer)| {
                self.scene
                    .animation_mixer(*mixer)
                    .map_or(Some(handle), |mixer| mixer.is_stale().then_some(handle))
            })
            .collect::<Vec<_>>();
        for handle in stale {
            let _ = self.animation_handles.remove(
                handle,
                SceneHostErrorCode::AnimationHandleNotFound,
                SceneHostErrorCode::StaleAnimationHandle,
            );
        }
    }
}

fn default_animation_speed() -> f32 {
    1.0
}

fn validate_speed(speed: impl Into<f64>) -> Result<f32, SceneHostError> {
    let speed = speed.into();
    if !speed.is_finite() || speed <= 0.0 || speed > f64::from(f32::MAX) {
        return Err(invalid_input(format!(
            "animation speed must be finite and > 0, got {speed}"
        )));
    }
    Ok(speed as f32)
}

pub(super) fn validate_time_seconds(field: &str, value: f64) -> Result<f32, SceneHostError> {
    if !value.is_finite() || value < 0.0 || value > f64::from(f32::MAX) {
        return Err(invalid_input(format!(
            "{field} must be finite and non-negative, got {value}"
        )));
    }
    Ok(value as f32)
}

pub(super) fn invalid_input(message: impl Into<String>) -> SceneHostError {
    SceneHostError::new(SceneHostErrorCode::InvalidInput, message.into())
}

fn map_animation_error(error: AnimationError) -> SceneHostError {
    match error {
        AnimationError::ClipNotFound { name } => SceneHostError::new(
            SceneHostErrorCode::AnimationClipNotFound,
            format!("animation clip {name} was not found"),
        ),
        AnimationError::InvalidClip { reason } => SceneHostError::new(
            SceneHostErrorCode::InvalidInput,
            format!("animation clip is invalid: {reason}"),
        ),
        AnimationError::MixerNotFound(mixer) => SceneHostError::new(
            SceneHostErrorCode::AnimationHandleNotFound,
            format!("animation mixer {mixer:?} was not found"),
        ),
        AnimationError::StaleMixer(mixer) => SceneHostError::new(
            SceneHostErrorCode::StaleAnimationHandle,
            format!("animation mixer {mixer:?} is stale"),
        ),
    }
}
