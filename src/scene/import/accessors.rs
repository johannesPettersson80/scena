use super::{ImportAnchor, ImportAnchorDebugMetadata, ImportClip, ImportPivot};
use crate::animation::AnimationClipKey;
use crate::scene::{NodeKey, Transform};

impl ImportAnchor {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn node(&self) -> NodeKey {
        self.node
    }

    pub const fn transform(&self) -> Transform {
        self.transform
    }
}

impl ImportAnchorDebugMetadata {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn node(&self) -> NodeKey {
        self.node
    }

    pub const fn transform(&self) -> Transform {
        self.transform
    }

    pub const fn is_anchor(&self) -> bool {
        true
    }
}

impl From<&ImportAnchor> for ImportAnchorDebugMetadata {
    fn from(anchor: &ImportAnchor) -> Self {
        Self {
            name: anchor.name.clone(),
            node: anchor.node,
            transform: anchor.transform,
        }
    }
}

impl ImportClip {
    pub const fn key(&self) -> AnimationClipKey {
        self.clip.key()
    }

    pub fn name(&self) -> Option<&str> {
        self.clip.name()
    }

    pub fn channels(&self) -> &[crate::animation::AnimationChannel] {
        self.clip.channels()
    }

    pub const fn duration_seconds(&self) -> f32 {
        self.clip.duration_seconds()
    }

    pub(crate) fn clip(&self) -> crate::animation::AnimationClip {
        self.clip.clone()
    }
}

impl ImportPivot {
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub const fn node(&self) -> NodeKey {
        self.node
    }

    pub const fn transform(&self) -> Transform {
        self.transform
    }
}
