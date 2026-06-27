use std::collections::BTreeMap;

use crate::assets::{AssetFetcher, Assets};
use crate::diagnostics::LookupError;
use crate::geometry::{GeometryDesc, GeometryTopology, GeometryVertex};
use crate::material::{Color, MaterialDesc};

use super::{
    AnchorKey, AnnotationAnchor, ConnectorKey, LabelDesc, LabelKey, NodeKey, Scene, Transform, Vec3,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Callout {
    id: String,
    anchor: CalloutAnchor,
    text: String,
    label_offset: Vec3,
    color: Color,
    label_size: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalloutAnchorKind {
    Node,
    World,
    Anchor,
    Connector,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CalloutAnchor {
    Node { node: NodeKey, local_offset: Vec3 },
    World { position: Vec3 },
    Anchor { anchor: AnchorKey },
    Connector { connector: ConnectorKey },
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalloutReport {
    pub id: String,
    pub anchor_id: String,
    pub leader_line_node: NodeKey,
    pub label_node: NodeKey,
    pub label: LabelKey,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SceneCalloutState {
    anchor: CalloutAnchor,
    leader_line_node: NodeKey,
    label_node: NodeKey,
    label: LabelKey,
    text: String,
}

#[derive(Debug, Clone)]
struct ResolvedCalloutAnchor {
    annotation_anchor: AnnotationAnchor,
    parent: NodeKey,
    local_anchor_position: Vec3,
}

impl Callout {
    pub fn node(
        id: impl Into<String>,
        node: NodeKey,
        local_offset: Vec3,
        text: impl Into<String>,
    ) -> Self {
        Self::new(id, CalloutAnchor::Node { node, local_offset }, text)
    }

    pub fn world(id: impl Into<String>, position: Vec3, text: impl Into<String>) -> Self {
        Self::new(id, CalloutAnchor::World { position }, text)
    }

    pub fn anchor(id: impl Into<String>, anchor: AnchorKey, text: impl Into<String>) -> Self {
        Self::new(id, CalloutAnchor::Anchor { anchor }, text)
    }

    pub fn connector(
        id: impl Into<String>,
        connector: ConnectorKey,
        text: impl Into<String>,
    ) -> Self {
        Self::new(id, CalloutAnchor::Connector { connector }, text)
    }

    fn new(id: impl Into<String>, anchor: CalloutAnchor, text: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            anchor,
            text: text.into(),
            label_offset: Vec3::new(0.35, 0.2, 0.0),
            color: Color::CYAN,
            label_size: 14.0,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub const fn target(&self) -> CalloutAnchor {
        self.anchor
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub const fn label_offset(&self) -> Vec3 {
        self.label_offset
    }

    pub const fn color(&self) -> Color {
        self.color
    }

    pub const fn label_size(&self) -> f32 {
        self.label_size
    }

    pub const fn with_label_offset(mut self, offset: Vec3) -> Self {
        self.label_offset = offset;
        self
    }

    pub const fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn with_label_size(mut self, size: f32) -> Self {
        self.label_size = if size.is_finite() && size >= 4.0 {
            size
        } else {
            self.label_size
        };
        self
    }
}

impl CalloutAnchor {
    pub const fn kind(self) -> CalloutAnchorKind {
        match self {
            Self::Node { .. } => CalloutAnchorKind::Node,
            Self::World { .. } => CalloutAnchorKind::World,
            Self::Anchor { .. } => CalloutAnchorKind::Anchor,
            Self::Connector { .. } => CalloutAnchorKind::Connector,
        }
    }

    pub(crate) const fn target_node(self) -> Option<NodeKey> {
        match self {
            Self::Node { node, .. } => Some(node),
            Self::World { .. } => None,
            Self::Anchor { .. } | Self::Connector { .. } => None,
        }
    }
}

impl SceneCalloutState {
    #[cfg(feature = "scene-host")]
    pub const fn leader_line_node(&self) -> NodeKey {
        self.leader_line_node
    }

    #[cfg(feature = "scene-host")]
    pub const fn label_node(&self) -> NodeKey {
        self.label_node
    }

    pub(crate) fn touches_removed_node(
        &self,
        removed: &std::collections::BTreeSet<NodeKey>,
    ) -> bool {
        removed.contains(&self.leader_line_node)
            || removed.contains(&self.label_node)
            || self
                .anchor
                .target_node()
                .is_some_and(|node| removed.contains(&node))
    }
}

impl Scene {
    pub fn add_callout<F: AssetFetcher>(
        &mut self,
        assets: &Assets<F>,
        callout: Callout,
    ) -> Result<CalloutReport, LookupError> {
        validate_callout_id(callout.id())?;
        validate_point(callout.label_offset(), "callout label offset")?;
        if self.callouts.contains_key(callout.id()) {
            self.clear_callout(callout.id());
        }
        let resolved = self.resolve_callout_anchor(&callout)?;
        let label_position = resolved.local_anchor_position + callout.label_offset();
        let leader_end = leader_line_end(resolved.local_anchor_position, label_position);

        self.set_annotation_anchor(resolved.annotation_anchor.clone())?;
        let line_node = self
            .mesh(
                assets.create_geometry(line_geometry(resolved.local_anchor_position, leader_end)),
                assets.create_material(MaterialDesc::line(callout.color(), 1.0)),
            )
            .parent(resolved.parent)
            .transform(Transform::IDENTITY)
            .add()?;
        let (label, label_node) = self.add_label_node(
            resolved.parent,
            LabelDesc::new(callout.text())
                .with_color(callout.color())
                .with_background(Color::BLACK)
                .with_size(callout.label_size()),
            Transform::at(label_position),
        )?;

        let state = SceneCalloutState {
            anchor: callout.target(),
            leader_line_node: line_node,
            label_node,
            label,
            text: callout.text().to_owned(),
        };
        self.callouts.insert(callout.id.clone(), state);

        Ok(CalloutReport {
            id: callout.id.clone(),
            anchor_id: callout.id,
            leader_line_node: line_node,
            label_node,
            label,
            text: callout.text,
        })
    }

    pub fn clear_callout(&mut self, id: &str) -> bool {
        let Some(callout) = self.callouts.remove(id) else {
            return false;
        };
        self.clear_annotation_anchor(id);
        remove_generated_node_if_live(self, callout.leader_line_node);
        remove_generated_node_if_live(self, callout.label_node);
        true
    }

    pub fn callout(&self, id: &str) -> Option<CalloutReport> {
        self.callouts.get(id).map(|callout| callout.to_report(id))
    }

    #[cfg(feature = "scene-host")]
    pub(crate) fn callout_state(&self, id: &str) -> Option<&SceneCalloutState> {
        self.callouts.get(id)
    }

    pub(crate) fn callouts_mut(&mut self) -> &mut BTreeMap<String, SceneCalloutState> {
        &mut self.callouts
    }

    fn resolve_callout_anchor(
        &self,
        callout: &Callout,
    ) -> Result<ResolvedCalloutAnchor, LookupError> {
        match callout.target() {
            CalloutAnchor::Node { node, local_offset } => {
                if !self.nodes.contains_key(node) {
                    return Err(LookupError::NodeNotFound(node));
                }
                validate_point(local_offset, "callout node local offset")?;
                Ok(ResolvedCalloutAnchor {
                    annotation_anchor: AnnotationAnchor::node(callout.id(), node, local_offset),
                    parent: node,
                    local_anchor_position: local_offset,
                })
            }
            CalloutAnchor::World { position } => {
                validate_point(position, "callout world position")?;
                Ok(ResolvedCalloutAnchor {
                    annotation_anchor: AnnotationAnchor::world(callout.id(), position),
                    parent: self.root(),
                    local_anchor_position: position,
                })
            }
            CalloutAnchor::Anchor { anchor } => {
                let frame = self
                    .anchors
                    .get(anchor)
                    .ok_or(LookupError::AnchorNotFound {
                        name: format!("{anchor:?}"),
                    })?;
                if !self.nodes.contains_key(frame.node()) {
                    return Err(LookupError::NodeNotFound(frame.node()));
                }
                let local_position = frame.local_transform().translation;
                Ok(ResolvedCalloutAnchor {
                    annotation_anchor: AnnotationAnchor::node(
                        callout.id(),
                        frame.node(),
                        local_position,
                    ),
                    parent: frame.node(),
                    local_anchor_position: local_position,
                })
            }
            CalloutAnchor::Connector { connector } => {
                let frame =
                    self.connectors
                        .get(connector)
                        .ok_or(LookupError::ConnectorNotFound {
                            name: format!("{connector:?}"),
                        })?;
                if !self.nodes.contains_key(frame.node()) {
                    return Err(LookupError::NodeNotFound(frame.node()));
                }
                let local_position = frame.local_transform().translation;
                Ok(ResolvedCalloutAnchor {
                    annotation_anchor: AnnotationAnchor::node(
                        callout.id(),
                        frame.node(),
                        local_position,
                    ),
                    parent: frame.node(),
                    local_anchor_position: local_position,
                })
            }
        }
    }
}

impl SceneCalloutState {
    fn to_report(&self, id: &str) -> CalloutReport {
        CalloutReport {
            id: id.to_owned(),
            anchor_id: id.to_owned(),
            leader_line_node: self.leader_line_node,
            label_node: self.label_node,
            label: self.label,
            text: self.text.clone(),
        }
    }
}

fn validate_callout_id(id: &str) -> Result<(), LookupError> {
    if id.trim().is_empty() {
        return Err(LookupError::InvalidBounds {
            reason: "callout id must be non-empty",
        });
    }
    Ok(())
}

fn validate_point(point: Vec3, reason: &'static str) -> Result<(), LookupError> {
    if point.x.is_finite() && point.y.is_finite() && point.z.is_finite() {
        return Ok(());
    }
    Err(LookupError::InvalidBounds { reason })
}

fn remove_generated_node_if_live(scene: &mut Scene, node: NodeKey) {
    if scene.node(node).is_some() {
        let _ = scene.remove_node(node);
    }
}

fn line_geometry(start: Vec3, end: Vec3) -> GeometryDesc {
    GeometryDesc::try_new(
        GeometryTopology::Lines,
        vec![
            GeometryVertex {
                position: start,
                normal: Vec3::Y,
            },
            GeometryVertex {
                position: end,
                normal: Vec3::Y,
            },
        ],
        vec![0, 1],
    )
    .expect("callout leader-line geometry is generated as a valid line pair")
}

fn leader_line_end(anchor: Vec3, label_position: Vec3) -> Vec3 {
    let offset = label_position - anchor;
    if offset.length_squared() <= f32::EPSILON {
        return label_position;
    }
    anchor + offset * 0.25
}
