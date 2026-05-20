use crate::animation::AnimationMixerKey;

use super::{HeadlessGltfViewer, InteractiveGltfViewer};

impl HeadlessGltfViewer {
    /// Creates and starts a mixer for a named animation clip on the loaded import.
    ///
    /// The returned mixer key remains scene-owned; pass it to
    /// `viewer.scene_mut().update_animation(...)`, loop, speed, pause, seek,
    /// and stop helpers to drive playback explicitly.
    pub fn play_clip(&mut self, name: &str) -> crate::Result<AnimationMixerKey> {
        Ok(self.scene.play_animation_by_name(&self.import, name)?)
    }
}

impl InteractiveGltfViewer {
    /// Creates and starts a mixer for a named animation clip on the loaded import.
    ///
    /// This is viewer sugar over `Scene::play_animation_by_name`; it does not
    /// hide animation updates, prepare, or render inside the call.
    pub fn play_clip(&mut self, name: &str) -> crate::Result<AnimationMixerKey> {
        Ok(self.scene.play_animation_by_name(&self.import, name)?)
    }
}
