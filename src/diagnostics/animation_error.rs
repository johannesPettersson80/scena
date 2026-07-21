use crate::animation::AnimationMixerKey;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnimationError {
    ClipNotFound {
        name: String,
        candidates: Vec<String>,
    },
    InvalidClip {
        reason: String,
    },
    MixerNotFound(AnimationMixerKey),
    StaleMixer(AnimationMixerKey),
}
