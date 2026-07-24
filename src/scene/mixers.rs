use std::sync::{Arc, atomic::AtomicBool};

use crate::animation::{
    AnimationChannel, AnimationLoopMode, AnimationMixer, AnimationMixerKey, AnimationPlaybackState,
    AnimationTarget, AnimationUpdateMetrics,
};
use crate::diagnostics::AnimationError;

use super::{Scene, SceneImport, Transform};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppliedAnimationChange {
    None,
    Transform,
}

impl AppliedAnimationChange {
    const fn transform_changed(self) -> bool {
        matches!(self, Self::Transform)
    }
}

impl Scene {
    /// Creates a paused mixer for a named imported animation clip.
    ///
    /// Returns the mixer key without starting playback. For the one-call "play
    /// this now" path, use [`Self::play_animation_by_name`].
    pub fn create_animation_mixer(
        &mut self,
        import: &SceneImport,
        clip_name: &str,
    ) -> Result<AnimationMixerKey, AnimationError> {
        let clip = import
            .clip(clip_name)
            .map_err(|error| match error {
                crate::LookupError::ClipNotFound { name, candidates } => {
                    AnimationError::ClipNotFound { name, candidates }
                }
                _ => AnimationError::ClipNotFound {
                    name: clip_name.to_string(),
                    candidates: Vec::new(),
                },
            })?
            .clip();
        Ok(self
            .animation_mixers
            .insert(AnimationMixer::new(clip, import.live_flag())))
    }

    /// Creates and starts a mixer for a named imported animation clip.
    ///
    /// This is the one-call path for "play this clip now". The returned
    /// mixer key can still be passed to [`Self::update_animation`],
    /// [`Self::set_animation_loop_mode`], [`Self::set_animation_speed`],
    /// pause, seek, and stop helpers.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use scena::{Assets, Scene};
    /// # async fn example() -> scena::Result<()> {
    /// let assets = Assets::new();
    /// let model = assets.load_scene("machine.glb").await?;
    /// let mut scene = Scene::new();
    /// let import = scene.instantiate(&model)?;
    ///
    /// let mixer = scene.play_animation_by_name(&import, "idle")?;
    /// scene.update_animation(mixer, 1.0 / 60.0)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn play_animation_by_name(
        &mut self,
        import: &SceneImport,
        clip_name: &str,
    ) -> Result<AnimationMixerKey, AnimationError> {
        let mixer = self.create_animation_mixer(import, clip_name)?;
        self.play_animation(mixer)?;
        Ok(mixer)
    }

    /// Creates a paused mixer for a caller-authored animation clip.
    ///
    /// Authored clips are not tied to a glTF import lifecycle, so the mixer is
    /// not invalidated by import replacement. The host still owns ticking by
    /// calling [`Self::update_animation`] or explicit sampling with
    /// [`Self::seek_animation`].
    pub fn create_authored_animation_mixer(
        &mut self,
        clip: crate::animation::AnimationClip,
    ) -> Result<AnimationMixerKey, AnimationError> {
        validate_authored_clip(&clip)?;
        Ok(self
            .animation_mixers
            .insert(AnimationMixer::new(clip, Arc::new(AtomicBool::new(true)))))
    }

    /// Creates and starts a mixer for a caller-authored animation clip.
    pub fn play_authored_animation(
        &mut self,
        clip: crate::animation::AnimationClip,
    ) -> Result<AnimationMixerKey, AnimationError> {
        let mixer = self.create_authored_animation_mixer(clip)?;
        self.play_animation(mixer)?;
        Ok(mixer)
    }

    /// Borrows the mixer state for a given key.
    pub fn animation_mixer(
        &self,
        mixer: AnimationMixerKey,
    ) -> Result<&AnimationMixer, AnimationError> {
        self.animation_mixers
            .get(mixer)
            .ok_or(AnimationError::MixerNotFound(mixer))
    }

    /// Starts the mixer; resumes from the current time if it was paused.
    pub fn play_animation(&mut self, mixer: AnimationMixerKey) -> Result<(), AnimationError> {
        self.animation_mixer_mut(mixer)?.play();
        Ok(())
    }

    /// Pauses the mixer at its current time. The next
    /// [`Self::play_animation`] resumes from the same time.
    pub fn pause_animation(&mut self, mixer: AnimationMixerKey) -> Result<(), AnimationError> {
        self.animation_mixer_mut(mixer)?.pause();
        Ok(())
    }

    /// Stops the mixer and snaps the animated nodes back to time zero.
    pub fn stop_animation(&mut self, mixer: AnimationMixerKey) -> Result<(), AnimationError> {
        let clip = {
            let mixer = self.animation_mixer_mut(mixer)?;
            mixer.stop();
            mixer.shared_clip()
        };
        self.apply_animation_clip(&clip, 0.0)
    }

    /// Seeks the mixer to a specific time in seconds and applies the resulting
    /// pose to the animated nodes.
    pub fn seek_animation(
        &mut self,
        mixer: AnimationMixerKey,
        time_seconds: f32,
    ) -> Result<(), AnimationError> {
        let (clip, time_seconds) = {
            let mixer = self.animation_mixer_mut(mixer)?;
            mixer.seek(time_seconds);
            (mixer.shared_clip(), mixer.time_seconds())
        };
        self.apply_animation_clip(&clip, time_seconds)
    }

    /// Sets the playback speed multiplier. `1.0` is real-time; negative values
    /// play the clip in reverse.
    pub fn set_animation_speed(
        &mut self,
        mixer: AnimationMixerKey,
        speed: f32,
    ) -> Result<(), AnimationError> {
        self.animation_mixer_mut(mixer)?.set_speed(speed);
        Ok(())
    }

    /// Sets the loop mode (loop, once, ping-pong).
    pub fn set_animation_loop_mode(
        &mut self,
        mixer: AnimationMixerKey,
        loop_mode: AnimationLoopMode,
    ) -> Result<(), AnimationError> {
        self.animation_mixer_mut(mixer)?.set_loop_mode(loop_mode);
        Ok(())
    }

    /// Advances the mixer by `delta_seconds` and applies the resulting pose to
    /// animated nodes. Hosts are expected to call this once per frame for
    /// every active mixer.
    pub fn update_animation(
        &mut self,
        mixer: AnimationMixerKey,
        delta_seconds: f32,
    ) -> Result<(), AnimationError> {
        self.update_animation_impl::<false>(mixer, delta_seconds)
            .map(|_| ())
    }

    /// Advances one mixer and returns deterministic counters for the animation
    /// sampling and payload-copy work performed by this update.
    pub fn update_animation_profiled(
        &mut self,
        mixer: AnimationMixerKey,
        delta_seconds: f32,
    ) -> Result<AnimationUpdateMetrics, AnimationError> {
        self.update_animation_impl::<true>(mixer, delta_seconds)
    }

    fn update_animation_impl<const PROFILE: bool>(
        &mut self,
        mixer: AnimationMixerKey,
        delta_seconds: f32,
    ) -> Result<AnimationUpdateMetrics, AnimationError> {
        let mut metrics = AnimationUpdateMetrics::default();
        let (clip, time_seconds, was_playing) = {
            let mixer = self.animation_mixer_mut(mixer)?;
            let was_playing = mixer.state() == AnimationPlaybackState::Playing;
            mixer.advance(delta_seconds);
            (mixer.shared_clip(), mixer.time_seconds(), was_playing)
        };
        if was_playing {
            self.apply_animation_clip_impl::<PROFILE>(&clip, time_seconds, &mut metrics)?;
        }
        Ok(metrics)
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

    fn apply_animation_clip(
        &mut self,
        clip: &crate::animation::AnimationClip,
        time_seconds: f32,
    ) -> Result<(), AnimationError> {
        let mut ignored = AnimationUpdateMetrics::default();
        self.apply_animation_clip_impl::<false>(clip, time_seconds, &mut ignored)
    }

    fn apply_animation_clip_impl<const PROFILE: bool>(
        &mut self,
        clip: &crate::animation::AnimationClip,
        time_seconds: f32,
        metrics: &mut AnimationUpdateMetrics,
    ) -> Result<(), AnimationError> {
        let mut transform_changed = false;
        for channel in clip.channels() {
            if PROFILE {
                metrics.channels_scanned = metrics.channels_scanned.saturating_add(1);
            }
            transform_changed |= self
                .apply_animation_channel::<PROFILE>(channel, time_seconds, metrics)?
                .transform_changed();
        }
        if transform_changed {
            self.transform_revision = self.transform_revision.saturating_add(1);
        }
        Ok(())
    }

    fn apply_animation_channel<const PROFILE: bool>(
        &mut self,
        channel: &AnimationChannel,
        time_seconds: f32,
        metrics: &mut AnimationUpdateMetrics,
    ) -> Result<AppliedAnimationChange, AnimationError> {
        if channel.target() == AnimationTarget::Weights {
            let node = channel.target_node();
            let mut weights = self.morph_weights.remove(&node).unwrap_or_default();
            let sampled = if PROFILE {
                channel.sample_weights_into_profiled(
                    time_seconds,
                    &mut weights,
                    &mut metrics.keyframe_intervals_tested,
                )
            } else {
                channel.sample_weights_into(time_seconds, &mut weights)
            };
            if !sampled {
                if !weights.is_empty() {
                    self.morph_weights.insert(node, weights);
                }
                return Ok(AppliedAnimationChange::None);
            }
            if PROFILE {
                metrics.weight_values_written = metrics
                    .weight_values_written
                    .saturating_add(weights.len() as u64);
                metrics.weight_bytes_written = metrics.weight_bytes_written.saturating_add(
                    (weights.len() as u64).saturating_mul(std::mem::size_of::<f32>() as u64),
                );
            }
            self.set_morph_weights_unchecked(node, weights);
            return Ok(AppliedAnimationChange::None);
        }

        let Some(node) = self.nodes.get_mut(channel.target_node()) else {
            return Ok(AppliedAnimationChange::None);
        };
        let before = node.transform;
        let mut transform = before;
        match channel.target() {
            AnimationTarget::Translation => {
                let value = if PROFILE {
                    channel
                        .sample_vec3_profiled(time_seconds, &mut metrics.keyframe_intervals_tested)
                } else {
                    channel.sample_vec3(time_seconds)
                };
                let Some(value) = value else {
                    return Ok(AppliedAnimationChange::None);
                };
                transform.translation = value;
            }
            AnimationTarget::Scale => {
                let value = if PROFILE {
                    channel
                        .sample_vec3_profiled(time_seconds, &mut metrics.keyframe_intervals_tested)
                } else {
                    channel.sample_vec3(time_seconds)
                };
                let Some(value) = value else {
                    return Ok(AppliedAnimationChange::None);
                };
                transform.scale = value;
            }
            AnimationTarget::Rotation => {
                let value = if PROFILE {
                    channel
                        .sample_quat_profiled(time_seconds, &mut metrics.keyframe_intervals_tested)
                } else {
                    channel.sample_quat(time_seconds)
                };
                let Some(value) = value else {
                    return Ok(AppliedAnimationChange::None);
                };
                transform.rotation = value;
            }
            AnimationTarget::Weights => unreachable!("weights handled before mutable node borrow"),
        }
        if !transform.translation.is_finite()
            || !transform.rotation.is_finite()
            || !transform.scale.is_finite()
        {
            return Err(AnimationError::InvalidClip {
                reason: format!(
                    "sampled {:?} channel produced a non-finite transform at {time_seconds} seconds",
                    channel.target()
                ),
            });
        }
        if before == transform {
            return Ok(AppliedAnimationChange::None);
        }
        node.transform = Transform { ..transform };
        Ok(AppliedAnimationChange::Transform)
    }
}

fn validate_authored_clip(clip: &crate::animation::AnimationClip) -> Result<(), AnimationError> {
    if !clip.duration_seconds().is_finite() || clip.duration_seconds() <= 0.0 {
        return Err(AnimationError::InvalidClip {
            reason: "duration_seconds must be finite and positive".to_owned(),
        });
    }
    if clip.channels().is_empty() {
        return Err(AnimationError::InvalidClip {
            reason: "clip must contain at least one channel".to_owned(),
        });
    }
    Ok(())
}

#[cfg(test)]
impl Scene {
    pub(crate) fn insert_animation_mixer_for_test(
        &mut self,
        clip: crate::animation::AnimationClip,
    ) -> AnimationMixerKey {
        self.animation_mixers
            .insert(AnimationMixer::new(clip, Arc::new(AtomicBool::new(true))))
    }
}

#[cfg(test)]
mod tests {
    use crate::animation::{
        AnimationChannel, AnimationClip, AnimationClipKey, AnimationInterpolation, AnimationOutput,
        AnimationTarget,
    };
    use crate::scene::{Scene, Transform, Vec3};

    #[test]
    fn transform_only_animation_updates_transform_revision_without_structure_dirtying() {
        let mut scene = Scene::new();
        let node = scene
            .add_empty(scene.root(), Transform::IDENTITY)
            .expect("animated node inserts");
        let mixer = scene.insert_animation_mixer_for_test(translation_clip(node));
        scene.play_animation(mixer).expect("mixer starts");
        let before = scene.dirty_state();

        for expected_transform_revision in 1..=3 {
            let frame_before = scene.dirty_state();
            scene
                .update_animation(mixer, 0.25)
                .expect("animation frame updates");
            let frame_after = scene.dirty_state();
            assert_eq!(
                frame_after.structure_revision, frame_before.structure_revision,
                "transform-only animation frames must not dirty scene structure"
            );
            assert_eq!(
                frame_after.transform_revision,
                frame_before.transform_revision + 1,
                "changed transform animation frames must bump transform revision exactly once"
            );
            assert_eq!(
                frame_after.transform_revision,
                before.transform_revision + expected_transform_revision,
            );
        }

        let after = scene.dirty_state();
        assert_eq!(
            after.structure_revision, before.structure_revision,
            "transform-only animation playback must preserve structure revision across frames"
        );
    }

    #[test]
    fn morph_weight_animation_keeps_structural_revision_semantics() {
        let mut scene = Scene::new();
        let node = scene
            .add_empty(scene.root(), Transform::IDENTITY)
            .expect("morph node inserts");
        scene.set_initial_morph_weights(node, &[0.0]);
        let mixer = scene.insert_animation_mixer_for_test(weight_clip(node));
        let before = scene.dirty_state();

        scene
            .seek_animation(mixer, 1.0)
            .expect("morph weight seek applies");
        let after = scene.dirty_state();

        assert_eq!(
            after.transform_revision, before.transform_revision,
            "morph weight animation must not masquerade as a transform-only update"
        );
        assert!(
            after.structure_revision > before.structure_revision,
            "morph weight animation remains structural because vertex deformation changes"
        );
    }

    #[test]
    fn finite_authored_keys_cannot_overflow_into_a_runtime_transform() {
        let mut scene = Scene::new();
        let node = scene
            .add_empty(scene.root(), Transform::IDENTITY)
            .expect("animated node inserts");
        let clip = AnimationClip::new(
            AnimationClipKey::fresh(),
            Some("OverflowGuard".to_owned()),
            vec![AnimationChannel::new(
                node,
                AnimationTarget::Translation,
                vec![0.0, 10.0],
                AnimationOutput::Vec3(vec![
                    Vec3::ZERO,
                    Vec3::ZERO,
                    Vec3::new(f32::MAX, 0.0, 0.0),
                    Vec3::new(-f32::MAX, 0.0, 0.0),
                    Vec3::ZERO,
                    Vec3::ZERO,
                ]),
                AnimationInterpolation::CubicSpline,
            )],
            10.0,
        )
        .expect("all authored key and tangent values are finite");
        let mixer = scene.insert_animation_mixer_for_test(clip);
        let before = scene.node(node).unwrap().transform();

        let error = scene
            .seek_animation(mixer, 5.0)
            .expect_err("overflowed sample must fail rather than poison the node");
        assert!(error.to_string().contains("non-finite transform"));
        assert_eq!(scene.node(node).unwrap().transform(), before);
    }

    fn translation_clip(node: crate::scene::NodeKey) -> AnimationClip {
        AnimationClip::new(
            AnimationClipKey::fresh(),
            Some("MoveX".to_string()),
            vec![AnimationChannel::new(
                node,
                AnimationTarget::Translation,
                vec![0.0, 1.0],
                AnimationOutput::Vec3(vec![Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0)]),
                AnimationInterpolation::Linear,
            )],
            1.0,
        )
        .expect("test translation clip is valid")
    }

    fn weight_clip(node: crate::scene::NodeKey) -> AnimationClip {
        AnimationClip::new(
            AnimationClipKey::fresh(),
            Some("Morph".to_string()),
            vec![AnimationChannel::new(
                node,
                AnimationTarget::Weights,
                vec![0.0, 1.0],
                AnimationOutput::Weights(vec![vec![0.0], vec![1.0]]),
                AnimationInterpolation::Linear,
            )],
            1.0,
        )
        .expect("test weight clip is valid")
    }
}
