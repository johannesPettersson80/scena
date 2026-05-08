use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::import::{ImportAnchor, ImportConnector};
use super::transforms::compose_transform;
use super::{
    AnchorFrame, ConnectorKey, NodeKey, Scene, SourceCoordinateSystem, SourceUnits, Transform, Vec3,
};

mod error;
mod imports;
mod locks;
mod metadata;
mod options;
mod parenting;
mod roll;
mod scale;
mod validation;
pub use error::ConnectionError;
pub use metadata::{ConnectorMetadata, ConnectorPolarity, ConnectorRollPolicy};
pub use options::{
    ConnectOptions, ConnectionAlignment, ConnectionParenting, ConnectionRequest, ConnectionRoll,
};
use parenting::node_is_descendant_of;
use roll::roll_transform;
use scale::preserve_source_scale;
use validation::{
    inverse_transform, validate_connector_handedness, validate_connector_host_prepared,
    validate_connector_kinds, validate_connector_live, validate_connector_source_metadata,
    validate_connector_transform, validate_node_transform, validate_transform_scale,
};

#[derive(Debug, Clone)]
pub struct ConnectorFrame {
    node: NodeKey,
    local_transform: Transform,
    name: Option<String>,
    kind: Option<String>,
    allowed_mates: BTreeSet<String>,
    tags: BTreeSet<String>,
    snap_tolerance: Option<f32>,
    clearance_hint: Option<f32>,
    roll_policy: ConnectorRollPolicy,
    polarity: Option<ConnectorPolarity>,
    metadata: Option<ConnectorMetadata>,
    source_units: SourceUnits,
    source_coordinate_system: SourceCoordinateSystem,
    import_live: Option<Arc<AtomicBool>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectionPreview {
    source: ConnectorFrame,
    target: ConnectorFrame,
    resolved_transform: Transform,
    resolved_parent: Option<NodeKey>,
    connection_line: ConnectionLineOverlay,
    warnings: Vec<ConnectionWarning>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConnectionLineOverlay {
    source: NodeKey,
    target: NodeKey,
    start: Vec3,
    end: Vec3,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionWarning {
    SourceMoved,
}

impl ConnectorFrame {
    pub fn new(node: NodeKey, local_transform: Transform) -> Self {
        Self {
            node,
            local_transform,
            name: None,
            kind: None,
            allowed_mates: BTreeSet::new(),
            tags: BTreeSet::new(),
            snap_tolerance: None,
            clearance_hint: None,
            roll_policy: ConnectorRollPolicy::Preserve,
            polarity: None,
            metadata: None,
            source_units: SourceUnits::Meters,
            source_coordinate_system: SourceCoordinateSystem::GltfYUpRightHanded,
            import_live: None,
        }
    }

    pub fn from_import_anchor(anchor: &ImportAnchor) -> Self {
        let mut connector = Self::new(anchor.placement_node(), anchor.connection_transform())
            .named(anchor.name())
            .with_source_metadata(anchor.source_units(), anchor.source_coordinate_system());
        connector.import_live = Some(anchor.live_flag());
        connector
    }

    pub fn from_import_connector(connector: &ImportConnector) -> Self {
        let mut frame = Self::new(connector.placement_node(), connector.connection_transform())
            .named(connector.name())
            .with_source_metadata(
                connector.source_units(),
                connector.source_coordinate_system(),
            );
        if let Some(kind) = connector.kind() {
            frame = frame.with_kind(kind);
        }
        for allowed_mate in connector.allowed_mates() {
            frame = frame.with_allowed_mate(allowed_mate);
        }
        for tag in connector.tags() {
            frame = frame.with_tag(tag);
        }
        if let Some(tolerance) = connector.snap_tolerance() {
            frame = frame.with_snap_tolerance(tolerance);
        }
        if let Some(clearance) = connector.clearance_hint() {
            frame = frame.with_clearance_hint(clearance);
        }
        frame = frame.with_roll_policy(connector.roll_policy());
        if let Some(polarity) = connector.polarity() {
            frame = frame.with_polarity(polarity);
        }
        if let Some(metadata) = connector.metadata() {
            frame = frame.with_metadata(metadata.clone());
        }
        frame.import_live = Some(connector.live_flag());
        frame
    }

    pub fn from_anchor_frame(anchor: &AnchorFrame) -> Self {
        let mut frame = Self::new(anchor.node(), anchor.local_transform())
            .with_source_metadata(anchor.source_units(), anchor.source_coordinate_system());
        if let Some(name) = anchor.name() {
            frame = frame.named(name);
        }
        frame.import_live = anchor.live_flag();
        frame
    }

    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = Some(kind.into());
        self
    }

    pub fn with_allowed_mate(mut self, kind: impl Into<String>) -> Self {
        self.allowed_mates.insert(kind.into());
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.insert(tag.into());
        self
    }

    pub fn with_snap_tolerance(mut self, tolerance: f32) -> Self {
        self.snap_tolerance = Some(tolerance.max(0.0));
        self
    }

    pub fn with_clearance_hint(mut self, clearance: f32) -> Self {
        self.clearance_hint = Some(clearance.max(0.0));
        self
    }

    pub const fn with_roll_policy(mut self, policy: ConnectorRollPolicy) -> Self {
        self.roll_policy = policy;
        self
    }

    pub const fn with_polarity(mut self, polarity: ConnectorPolarity) -> Self {
        self.polarity = Some(polarity);
        self
    }

    pub fn with_metadata(mut self, metadata: ConnectorMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    fn with_source_metadata(
        mut self,
        units: SourceUnits,
        coordinate_system: SourceCoordinateSystem,
    ) -> Self {
        self.source_units = units;
        self.source_coordinate_system = coordinate_system;
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

    pub fn kind(&self) -> Option<&str> {
        self.kind.as_deref()
    }

    pub fn allowed_mates(&self) -> Vec<&str> {
        self.allowed_mates.iter().map(String::as_str).collect()
    }

    pub fn tags(&self) -> &BTreeSet<String> {
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

    pub const fn source_units(&self) -> SourceUnits {
        self.source_units
    }

    pub const fn source_coordinate_system(&self) -> SourceCoordinateSystem {
        self.source_coordinate_system
    }

    fn is_live(&self) -> bool {
        match &self.import_live {
            Some(flag) => flag.load(Ordering::Acquire),
            None => true,
        }
    }
}

impl ConnectionPreview {
    pub const fn source(&self) -> &ConnectorFrame {
        &self.source
    }

    pub const fn target(&self) -> &ConnectorFrame {
        &self.target
    }

    pub const fn resolved_transform(&self) -> Transform {
        self.resolved_transform
    }

    pub const fn resolved_parent(&self) -> Option<NodeKey> {
        self.resolved_parent
    }

    pub fn warnings(&self) -> &[ConnectionWarning] {
        &self.warnings
    }

    pub const fn connection_line(&self) -> ConnectionLineOverlay {
        self.connection_line
    }
}

impl ConnectionLineOverlay {
    pub const fn source(self) -> NodeKey {
        self.source
    }

    pub const fn target(self) -> NodeKey {
        self.target
    }

    pub const fn start(self) -> Vec3 {
        self.start
    }

    pub const fn end(self) -> Vec3 {
        self.end
    }
}

impl Scene {
    pub fn add_connector(
        &mut self,
        connector: ConnectorFrame,
    ) -> Result<ConnectorKey, ConnectionError> {
        if !self.nodes.contains_key(connector.node) {
            return Err(ConnectionError::NodeNotFound(connector.node));
        }
        validate_connector_live(&connector, None)?;
        let key = self.connectors.insert(connector);
        self.structure_revision = self.structure_revision.saturating_add(1);
        Ok(key)
    }

    pub fn connector(&self, connector: ConnectorKey) -> Result<&ConnectorFrame, ConnectionError> {
        let frame = self
            .connectors
            .get(connector)
            .ok_or(ConnectionError::MissingConnector { connector })?;
        validate_connector_live(frame, Some(connector))?;
        if !self.nodes.contains_key(frame.node) {
            return Err(ConnectionError::NodeNotFound(frame.node));
        }
        Ok(frame)
    }

    pub fn connector_named(&self, name: &str) -> Result<ConnectorKey, ConnectionError> {
        let matches = self
            .connectors
            .iter()
            .filter(|(_, connector)| connector.name() == Some(name))
            .map(|(key, _)| key)
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [] => Err(ConnectionError::MissingConnectorName {
                name: name.to_string(),
            }),
            [connector] => {
                self.connector(*connector)?;
                Ok(*connector)
            }
            _ => Err(ConnectionError::AmbiguousConnector {
                name: name.to_string(),
                matches,
            }),
        }
    }

    pub fn validate_connections(
        &self,
        requests: &[ConnectionRequest],
    ) -> Result<Vec<ConnectionPreview>, ConnectionError> {
        requests
            .iter()
            .map(|request| {
                let source = self.connector(request.source())?.clone();
                let target = self.connector(request.target())?.clone();
                self.preview_connection(source, target, request.options())
            })
            .collect()
    }

    pub fn connect_by_key(
        &mut self,
        source: ConnectorKey,
        target: ConnectorKey,
        options: ConnectOptions,
    ) -> Result<ConnectionPreview, ConnectionError> {
        let source_frame = self.connector(source)?.clone();
        let target_frame = self.connector(target)?.clone();
        self.connect(source_frame, target_frame, options)
    }

    pub fn preview_connection(
        &self,
        source: ConnectorFrame,
        target: ConnectorFrame,
        options: ConnectOptions,
    ) -> Result<ConnectionPreview, ConnectionError> {
        validate_connector_live(&source, None)?;
        validate_connector_live(&target, None)?;
        validate_connector_handedness(&source)?;
        validate_connector_handedness(&target)?;
        validate_connector_source_metadata(&source, &target)?;
        validate_connector_kinds(&source, &target)?;
        let source_node = self
            .nodes
            .get(source.node)
            .ok_or(ConnectionError::NodeNotFound(source.node))?;
        let target_node = self
            .nodes
            .get(target.node)
            .ok_or(ConnectionError::NodeNotFound(target.node))?;
        validate_connector_host_prepared(&source, &source_node.kind)?;
        validate_connector_host_prepared(&target, &target_node.kind)?;
        if self.connection_locked_nodes.contains(&source.node) {
            return Err(ConnectionError::ConnectionWouldMoveLockedNode { node: source.node });
        }

        validate_connector_transform(&source, options)?;
        validate_connector_transform(&target, options)?;
        validate_node_transform(source.node, source_node.transform, options)?;
        let target_world = self
            .world_transform(target.node)
            .ok_or(ConnectionError::NodeNotFound(target.node))?;
        let source_current_world = self
            .world_transform(source.node)
            .ok_or(ConnectionError::NodeNotFound(source.node))?;
        validate_transform_scale(target.node, target_world, options)?;
        let resolved_parent = match options.parenting() {
            ConnectionParenting::PreserveSourceParent => source_node.parent(),
            ConnectionParenting::ReparentSourceToTargetParent => target_node.parent(),
        };
        if let Some(parent) = resolved_parent {
            if !self.nodes.contains_key(parent) {
                return Err(ConnectionError::NodeNotFound(parent));
            }
            if node_is_descendant_of(self, parent, source.node) {
                return Err(ConnectionError::ConnectionWouldCreateCycle {
                    source: source.node,
                    parent,
                });
            }
        }
        let source_parent_world = resolved_parent
            .and_then(|parent| self.world_transform(parent))
            .unwrap_or(Transform::IDENTITY);
        if let Some(parent) = resolved_parent {
            validate_transform_scale(parent, source_parent_world, options)?;
        }

        let target_connector_world = compose_transform(target_world, target.local_transform);
        let target_offset = compose_transform(target_connector_world, options.mate_offset);
        let target_aligned = compose_transform(target_offset, options.alignment_transform());
        let source_current_connector =
            compose_transform(source_current_world, source.local_transform);
        let target_mated = compose_transform(
            target_aligned,
            roll_transform(options.roll(), source_current_connector, target_aligned),
        );
        let source_connector_inverse =
            inverse_transform(source.local_transform).ok_or_else(|| {
                ConnectionError::DegenerateConnectorFrame {
                    connector: source.name.clone(),
                }
            })?;
        let desired_source_world = preserve_source_scale(
            compose_transform(target_mated, source_connector_inverse),
            source_current_world.scale,
            source.local_transform,
            target_mated.translation,
        );
        let parent_inverse = inverse_transform(source_parent_world).ok_or_else(|| {
            ConnectionError::DegenerateConnectorFrame {
                connector: source.name.clone(),
            }
        })?;
        let resolved_transform = compose_transform(parent_inverse, desired_source_world);

        Ok(ConnectionPreview {
            connection_line: ConnectionLineOverlay {
                source: source.node,
                target: target.node,
                start: source_current_connector.translation,
                end: target_mated.translation,
            },
            source,
            target,
            resolved_transform,
            resolved_parent,
            warnings: vec![ConnectionWarning::SourceMoved],
        })
    }

    pub fn connect(
        &mut self,
        source: ConnectorFrame,
        target: ConnectorFrame,
        options: ConnectOptions,
    ) -> Result<ConnectionPreview, ConnectionError> {
        let preview = self.preview_connection(source, target, options)?;
        self.reparent_for_connection(preview.source.node, preview.resolved_parent)?;
        self.set_transform(preview.source.node, preview.resolved_transform)
            .map_err(|_| ConnectionError::NodeNotFound(preview.source.node))?;
        Ok(preview)
    }

    fn reparent_for_connection(
        &mut self,
        node: NodeKey,
        new_parent: Option<NodeKey>,
    ) -> Result<(), ConnectionError> {
        let current_parent = self
            .nodes
            .get(node)
            .ok_or(ConnectionError::NodeNotFound(node))?
            .parent();
        if current_parent == new_parent {
            return Ok(());
        }
        let Some(new_parent) = new_parent else {
            return Err(ConnectionError::NodeNotFound(node));
        };
        if !self.nodes.contains_key(new_parent) {
            return Err(ConnectionError::NodeNotFound(new_parent));
        }
        if node_is_descendant_of(self, new_parent, node) {
            return Err(ConnectionError::ConnectionWouldCreateCycle {
                source: node,
                parent: new_parent,
            });
        }
        if let Some(old_parent) = current_parent
            && let Some(parent_node) = self.nodes.get_mut(old_parent)
        {
            parent_node.children.retain(|child| *child != node);
        }
        self.nodes
            .get_mut(node)
            .ok_or(ConnectionError::NodeNotFound(node))?
            .parent = Some(new_parent);
        if !self.nodes[new_parent].children.contains(&node) {
            self.nodes[new_parent].children.push(node);
        }
        self.structure_revision = self.structure_revision.saturating_add(1);
        self.transform_revision = self.transform_revision.saturating_add(1);
        Ok(())
    }
}

impl PartialEq for ConnectorFrame {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node
            && self.local_transform == other.local_transform
            && self.name == other.name
            && self.kind == other.kind
            && self.allowed_mates == other.allowed_mates
            && self.tags == other.tags
            && self.snap_tolerance == other.snap_tolerance
            && self.clearance_hint == other.clearance_hint
            && self.roll_policy == other.roll_policy
            && self.polarity == other.polarity
            && self.metadata == other.metadata
            && self.source_units == other.source_units
            && self.source_coordinate_system == other.source_coordinate_system
    }
}
