use crate::animation::{
    AnimationChannel, AnimationLoopMode, AnimationMixer, AnimationMixerKey, AnimationPlaybackState,
    AnimationTarget,
};
use crate::diagnostics::AnimationError;

use super::{Scene, SceneImport, Transform};

impl Scene {
    pub fn create_animation_mixer(
        &mut self,
        import: &SceneImport,
        clip_name: &str,
    ) -> Result<AnimationMixerKey, AnimationError> {
        let clip = import
            .clip(clip_name)
            .map_err(|_| AnimationError::ClipNotFound {
                name: clip_name.to_string(),
            })?
            .clip();
        Ok(self
            .animation_mixers
            .insert(AnimationMixer::new(clip, import.live_flag())))
    }

    pub fn animation_mixer(
        &self,
        mixer: AnimationMixerKey,
    ) -> Result<&AnimationMixer, AnimationError> {
        self.animation_mixers
            .get(mixer)
            .ok_or(AnimationError::MixerNotFound(mixer))
    }

    pub fn play_animation(&mut self, mixer: AnimationMixerKey) -> Result<(), AnimationError> {
        self.animation_mixer_mut(mixer)?.play();
        Ok(())
    }

    pub fn pause_animation(&mut self, mixer: AnimationMixerKey) -> Result<(), AnimationError> {
        self.animation_mixer_mut(mixer)?.pause();
        Ok(())
    }

    pub fn stop_animation(&mut self, mixer: AnimationMixerKey) -> Result<(), AnimationError> {
        let clip = {
            let mixer = self.animation_mixer_mut(mixer)?;
            mixer.stop();
            mixer.clip().clone()
        };
        self.apply_animation_clip(&clip, 0.0);
        Ok(())
    }

    pub fn seek_animation(
        &mut self,
        mixer: AnimationMixerKey,
        time_seconds: f32,
    ) -> Result<(), AnimationError> {
        let (clip, time_seconds) = {
            let mixer = self.animation_mixer_mut(mixer)?;
            mixer.seek(time_seconds);
            (mixer.clip().clone(), mixer.time_seconds())
        };
        self.apply_animation_clip(&clip, time_seconds);
        Ok(())
    }

    pub fn set_animation_speed(
        &mut self,
        mixer: AnimationMixerKey,
        speed: f32,
    ) -> Result<(), AnimationError> {
        self.animation_mixer_mut(mixer)?.set_speed(speed);
        Ok(())
    }

    pub fn set_animation_loop_mode(
        &mut self,
        mixer: AnimationMixerKey,
        loop_mode: AnimationLoopMode,
    ) -> Result<(), AnimationError> {
        self.animation_mixer_mut(mixer)?.set_loop_mode(loop_mode);
        Ok(())
    }

    pub fn update_animation(
        &mut self,
        mixer: AnimationMixerKey,
        delta_seconds: f32,
    ) -> Result<(), AnimationError> {
        let (clip, time_seconds, was_playing) = {
            let mixer = self.animation_mixer_mut(mixer)?;
            let was_playing = mixer.state() == AnimationPlaybackState::Playing;
            mixer.advance(delta_seconds);
            (mixer.clip().clone(), mixer.time_seconds(), was_playing)
        };
        if was_playing {
            self.apply_animation_clip(&clip, time_seconds);
        }
        Ok(())
    }

    fn animation_mixer_mut(
        &mut self,
        mixer: AnimationMixerKey,
    ) -> Result<&mut AnimationMixer, AnimationError> {
        let mixer_state = self
            .animation_mixers
            .get_mut(mixer)
            .ok_or(AnimationError::MixerNotFound(mixer))?;
        if mixer_state.is_stale() {
            return Err(AnimationError::StaleMixer(mixer));
        }
        Ok(mixer_state)
    }

    fn apply_animation_clip(&mut self, clip: &crate::animation::AnimationClip, time_seconds: f32) {
        let mut changed = false;
        for channel in clip.channels() {
            changed |= self.apply_animation_channel(channel, time_seconds);
        }
        if changed {
            self.structure_revision = self.structure_revision.saturating_add(1);
        }
    }

    fn apply_animation_channel(&mut self, channel: &AnimationChannel, time_seconds: f32) -> bool {
        let Some(node) = self.nodes.get_mut(channel.target_node()) else {
            return false;
        };
        let before = node.transform;
        let mut transform = before;
        match channel.target() {
            AnimationTarget::Translation => {
                let Some(value) = channel.sample_vec3(time_seconds) else {
                    return false;
                };
                transform.translation = value;
            }
            AnimationTarget::Scale => {
                let Some(value) = channel.sample_vec3(time_seconds) else {
                    return false;
                };
                transform.scale = value;
            }
            AnimationTarget::Rotation => {
                let Some(value) = channel.sample_quat(time_seconds) else {
                    return false;
                };
                transform.rotation = value;
            }
            AnimationTarget::Weights => return false,
        }
        if before == transform {
            return false;
        }
        node.transform = Transform { ..transform };
        true
    }
}
