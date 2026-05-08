use super::{
    ConnectorMetadata, ConnectorPolarity, ConnectorRollPolicy, ImportAnchor,
    ImportAnchorDebugMetadata, ImportClip, ImportConnector, ImportPivot, SceneImport,
    SourceCoordinateSystem, SourceUnits,
};
use crate::animation::AnimationClipKey;
use crate::scene::{NodeKey, Transform};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

impl SceneImport {
    pub(crate) fn live_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.live)
    }
}

impl ImportAnchor {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn node(&self) -> NodeKey {
        self.node
    }

    pub const fn placement_node(&self) -> NodeKey {
        self.placement_node
    }

    pub const fn transform(&self) -> Transform {
        self.transform
    }

    pub fn tags(&self) -> &std::collections::BTreeSet<String> {
        &self.tags
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub const fn source_units(&self) -> SourceUnits {
        self.source_units
    }

    pub const fn source_coordinate_system(&self) -> SourceCoordinateSystem {
        self.source_coordinate_system
    }

    pub(crate) fn connection_transform(&self) -> Transform {
        self.placement_transform
    }

    pub(crate) fn live_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.live)
    }
}

impl ImportConnector {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn kind(&self) -> Option<&str> {
        self.kind.as_deref()
    }

    pub fn allowed_mates(&self) -> Vec<&str> {
        self.allowed_mates.iter().map(String::as_str).collect()
    }

    pub fn tags(&self) -> &std::collections::BTreeSet<String> {
        &self.tags
    }

    pub const fn snap_tolerance(&self) -> Option<f32> {
        self.snap_tolerance
    }

    pub const fn clearance_hint(&self) -> Option<f32> {
        self.clearance_hint
    }

    pub const fn roll_policy(&self) -> ConnectorRollPolicy {
        self.roll_policy
    }

    pub const fn polarity(&self) -> Option<ConnectorPolarity> {
        self.polarity
    }

    pub const fn metadata(&self) -> Option<&ConnectorMetadata> {
        self.metadata.as_ref()
    }

    pub const fn node(&self) -> NodeKey {
        self.node
    }

    pub const fn placement_node(&self) -> NodeKey {
        self.placement_node
    }

    pub const fn transform(&self) -> Transform {
        self.transform
    }

    pub const fn source_units(&self) -> SourceUnits {
        self.source_units
    }

    pub const fn source_coordinate_system(&self) -> SourceCoordinateSystem {
        self.source_coordinate_system
    }

    pub(crate) fn connection_transform(&self) -> Transform {
        self.placement_transform
    }

    pub(crate) fn live_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.live)
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
