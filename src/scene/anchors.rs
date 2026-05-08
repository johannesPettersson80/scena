use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::geometry::Aabb;

use super::import::ImportAnchor;
use super::{
    AnchorKey, ConnectionError, NodeKey, Scene, SourceCoordinateSystem, SourceUnits, Transform,
};

#[derive(Debug, Clone)]
pub struct AnchorFrame {
    node: NodeKey,
    local_transform: Transform,
    name: Option<String>,
    source_units: SourceUnits,
    source_coordinate_system: SourceCoordinateSystem,
    bounds_hint: Option<Aabb>,
    tags: BTreeSet<String>,
    label: Option<String>,
    import_live: Option<Arc<AtomicBool>>,
}

impl PartialEq for AnchorFrame {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node
            && self.local_transform == other.local_transform
            && self.name == other.name
            && self.source_units == other.source_units
            && self.source_coordinate_system == other.source_coordinate_system
            && self.bounds_hint == other.bounds_hint
            && self.tags == other.tags
            && self.label == other.label
    }
}

impl AnchorFrame {
    pub fn new(node: NodeKey, local_transform: Transform) -> Self {
        Self {
            node,
            local_transform,
            name: None,
            source_units: SourceUnits::Meters,
            source_coordinate_system: SourceCoordinateSystem::GltfYUpRightHanded,
            bounds_hint: None,
            tags: BTreeSet::new(),
            label: None,
            import_live: None,
        }
    }

    pub fn from_import_anchor(anchor: &ImportAnchor) -> Self {
        let mut frame = Self::new(anchor.placement_node(), anchor.connection_transform())
            .named(anchor.name())
            .with_source_units(anchor.source_units())
            .with_source_coordinate_system(anchor.source_coordinate_system());
        for tag in anchor.tags() {
            frame = frame.with_tag(tag);
        }
        if let Some(label) = anchor.label() {
            frame = frame.with_label(label);
        }
        frame.import_live = Some(anchor.live_flag());
        frame
    }

    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_source_units(mut self, units: SourceUnits) -> Self {
        self.source_units = units;
        self
    }

    pub fn with_source_coordinate_system(
        mut self,
        coordinate_system: SourceCoordinateSystem,
    ) -> Self {
        self.source_coordinate_system = coordinate_system;
        self
    }

    pub fn with_bounds_hint(mut self, bounds: Aabb) -> Self {
        self.bounds_hint = Some(bounds);
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.insert(tag.into());
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub const fn node(&self) -> NodeKey {
        self.node
    }

    pub const fn local_transform(&self) -> Transform {
        self.local_transform
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub const fn source_units(&self) -> SourceUnits {
        self.source_units
    }

    pub const fn source_coordinate_system(&self) -> SourceCoordinateSystem {
        self.source_coordinate_system
    }

    pub const fn bounds_hint(&self) -> Option<Aabb> {
        self.bounds_hint
    }

    pub fn tags(&self) -> &BTreeSet<String> {
        &self.tags
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub(super) fn live_flag(&self) -> Option<Arc<AtomicBool>> {
        self.import_live.as_ref().map(Arc::clone)
    }

    fn is_live(&self) -> bool {
        match &self.import_live {
            Some(flag) => flag.load(Ordering::Acquire),
            None => true,
        }
    }
}

impl Scene {
    pub fn add_anchor(&mut self, anchor: AnchorFrame) -> Result<AnchorKey, ConnectionError> {
        if !self.nodes.contains_key(anchor.node) {
            return Err(ConnectionError::NodeNotFound(anchor.node));
        }
        validate_anchor_live(&anchor, None)?;
        let key = self.anchors.insert(anchor);
        self.structure_revision = self.structure_revision.saturating_add(1);
        Ok(key)
    }

    pub fn anchor(&self, anchor: AnchorKey) -> Result<&AnchorFrame, ConnectionError> {
        let frame = self
            .anchors
            .get(anchor)
            .ok_or(ConnectionError::MissingAnchor { anchor })?;
        validate_anchor_live(frame, Some(anchor))?;
        if !self.nodes.contains_key(frame.node) {
            return Err(ConnectionError::NodeNotFound(frame.node));
        }
        Ok(frame)
    }

    pub fn anchor_named(&self, name: &str) -> Result<AnchorKey, ConnectionError> {
        let matches = self
            .anchors
            .iter()
            .filter(|(_, anchor)| anchor.name() == Some(name))
            .map(|(key, _)| key)
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [] => Err(ConnectionError::MissingAnchorName {
                name: name.to_string(),
            }),
            [anchor] => {
                self.anchor(*anchor)?;
                Ok(*anchor)
            }
            _ => Err(ConnectionError::AmbiguousAnchor {
                name: name.to_string(),
                matches,
            }),
        }
    }
}

fn validate_anchor_live(
    anchor: &AnchorFrame,
    key: Option<AnchorKey>,
) -> Result<(), ConnectionError> {
    if anchor.is_live() {
        Ok(())
    } else {
        Err(ConnectionError::StaleAnchorHandle {
            anchor: key,
            name: anchor.name.clone(),
        })
    }
}
