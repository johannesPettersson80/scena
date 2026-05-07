use super::{CameraKey, NodeKey, NodeKind, Scene, Transform};

#[derive(Debug, Clone, PartialEq)]
pub struct SceneInspectionReport {
    nodes: Vec<SceneNodeInspection>,
    active_camera: Option<CameraKey>,
    visible_drawable_count: usize,
    structure_revision: u64,
    transform_revision: u64,
    interaction_revision: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneNodeInspection {
    node: NodeKey,
    parent: Option<NodeKey>,
    kind: &'static str,
    transform: Transform,
    visible: bool,
    tags: Vec<String>,
    layer_mask: u64,
    render_group: i16,
    helper_on_top: bool,
}

impl Scene {
    pub fn inspect(&self) -> SceneInspectionReport {
        let dirty = self.dirty_state();
        SceneInspectionReport {
            nodes: self
                .nodes
                .iter()
                .map(|(node_key, node)| SceneNodeInspection {
                    node: node_key,
                    parent: node.parent,
                    kind: kind_name(&node.kind),
                    transform: node.transform,
                    visible: node.visible,
                    tags: node.tags.iter().cloned().collect(),
                    layer_mask: node.layer_mask,
                    render_group: node.render_group,
                    helper_on_top: node.helper_on_top,
                })
                .collect(),
            active_camera: self.active_camera,
            visible_drawable_count: self.visible_drawable_count(),
            structure_revision: dirty.structure_revision,
            transform_revision: dirty.transform_revision,
            interaction_revision: dirty.interaction_revision,
        }
    }
}

impl SceneInspectionReport {
    pub fn nodes(&self) -> &[SceneNodeInspection] {
        &self.nodes
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub const fn active_camera(&self) -> Option<CameraKey> {
        self.active_camera
    }

    pub const fn visible_drawable_count(&self) -> usize {
        self.visible_drawable_count
    }

    pub const fn structure_revision(&self) -> u64 {
        self.structure_revision
    }

    pub const fn transform_revision(&self) -> u64 {
        self.transform_revision
    }

    pub const fn interaction_revision(&self) -> u64 {
        self.interaction_revision
    }
}

impl SceneNodeInspection {
    pub const fn node(&self) -> NodeKey {
        self.node
    }

    pub const fn parent(&self) -> Option<NodeKey> {
        self.parent
    }

    pub const fn kind(&self) -> &'static str {
        self.kind
    }

    pub const fn transform(&self) -> Transform {
        self.transform
    }

    pub const fn visible(&self) -> bool {
        self.visible
    }

    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    pub const fn layer_mask(&self) -> u64 {
        self.layer_mask
    }

    pub const fn render_group(&self) -> i16 {
        self.render_group
    }

    pub const fn helper_on_top(&self) -> bool {
        self.helper_on_top
    }
}

const fn kind_name(kind: &NodeKind) -> &'static str {
    match kind {
        NodeKind::Empty => "Empty",
        NodeKind::Renderable(_) => "Renderable",
        NodeKind::Mesh(_) => "Mesh",
        NodeKind::Model(_) => "Model",
        NodeKind::InstanceSet(_) => "InstanceSet",
        NodeKind::Label(_) => "Label",
        NodeKind::Camera(_) => "Camera",
        NodeKind::Light(_) => "Light",
    }
}
