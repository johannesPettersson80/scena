use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::geometry::{Aabb, GeometryTopology};
use crate::material::Color;

use super::super::{CameraKey, InstanceId, NodeKey, Transform, Vec3};
use super::SceneInspectionReport;

pub const SCENE_INSPECTION_SCHEMA_V1: &str = "scena.scene_inspection.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneInspectionReportV1 {
    pub schema: String,
    pub nodes: Vec<SceneNodeInspectionV1>,
    pub draw_list: Vec<SceneDrawInspectionV1>,
    pub camera_frustums: Vec<SceneCameraFrustumInspectionV1>,
    pub normal_overlays: Vec<SceneNormalInspectionV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_sets: Option<Vec<SceneHostInstanceSetInspectionV1>>,
    pub active_camera: Option<u64>,
    pub counts: SceneInspectionCountsV1,
    pub revisions: SceneInspectionRevisionsV1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneHostInstanceSetInspectionV1 {
    pub root_handle: u64,
    pub visible: bool,
    pub tint: Option<Color>,
    pub root_transform: Transform,
    pub entries: Vec<SceneHostInstanceEntryInspectionV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneHostInstanceEntryInspectionV1 {
    pub set_node: Option<u64>,
    pub instance_id: u64,
    pub local_transform: Transform,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneInspectionCountsV1 {
    pub visible_drawable: usize,
    pub cameras: usize,
    pub lights: usize,
    pub anchors: usize,
    pub connectors: usize,
    pub bounded_nodes: usize,
    pub clipping_planes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneInspectionRevisionsV1 {
    pub structure: u64,
    pub transform: u64,
    #[serde(default)]
    pub appearance: u64,
    pub interaction: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneNodeInspectionV1 {
    pub handle: u64,
    pub parent: Option<u64>,
    pub kind: String,
    pub tags: Vec<String>,
    pub local_transform: Transform,
    pub world_transform: Transform,
    pub visible: bool,
    pub bounds: Option<Aabb>,
    pub layer_mask: u64,
    pub render_group: i16,
    pub helper_on_top: bool,
    #[serde(default)]
    pub tint: Option<Color>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SceneDrawInspectionV1 {
    pub node: u64,
    #[serde(default)]
    pub instance: Option<u64>,
    pub topology: GeometryTopology,
    pub primitive_count: usize,
    pub vertex_count: usize,
    pub index_count: usize,
    pub local_bounds: Aabb,
    pub world_transform: Transform,
    pub visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SceneCameraFrustumInspectionV1 {
    pub node: u64,
    pub near: f32,
    pub far: f32,
    pub corners: [Vec3; 8],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneNormalInspectionV1 {
    pub node: u64,
    #[serde(default)]
    pub instance: Option<u64>,
    pub length: f32,
    pub segments: Vec<[Vec3; 2]>,
}

impl SceneInspectionReport {
    pub fn to_schema_report(&self) -> SceneInspectionReportV1 {
        self.to_schema_report_with_node_handles(&BTreeMap::new())
    }

    pub fn to_schema_report_with_node_handles(
        &self,
        supplied_node_handles: &BTreeMap<NodeKey, u64>,
    ) -> SceneInspectionReportV1 {
        let node_handles = self.schema_node_handles(supplied_node_handles);
        let active_camera = self
            .active_camera
            .and_then(|camera| self.node_for_camera(camera))
            .and_then(|node| node_handles.get(&node).copied());

        SceneInspectionReportV1 {
            schema: SCENE_INSPECTION_SCHEMA_V1.to_owned(),
            nodes: self
                .report_local_node_order()
                .into_iter()
                .filter_map(|node| {
                    let source = self.nodes.iter().find(|candidate| candidate.node == node)?;
                    Some(SceneNodeInspectionV1 {
                        handle: node_handles[&source.node],
                        parent: source
                            .parent
                            .and_then(|parent| node_handles.get(&parent).copied()),
                        kind: source.kind.to_owned(),
                        tags: source.tags.clone(),
                        local_transform: source.transform,
                        world_transform: source.world_transform,
                        visible: source.visible,
                        bounds: source.bounds,
                        layer_mask: source.layer_mask,
                        render_group: source.render_group,
                        helper_on_top: source.helper_on_top,
                        tint: source.tint,
                    })
                })
                .collect(),
            draw_list: self
                .draw_list
                .iter()
                .filter_map(|draw| {
                    Some(SceneDrawInspectionV1 {
                        node: *node_handles.get(&draw.node)?,
                        instance: draw.instance.map(InstanceId::as_u64),
                        topology: draw.topology,
                        primitive_count: draw.primitive_count,
                        vertex_count: draw.vertex_count,
                        index_count: draw.index_count,
                        local_bounds: draw.local_bounds,
                        world_transform: draw.world_transform,
                        visible: draw.visible,
                    })
                })
                .collect(),
            camera_frustums: self
                .camera_frustums
                .iter()
                .filter_map(|frustum| {
                    Some(SceneCameraFrustumInspectionV1 {
                        node: *node_handles.get(&frustum.node)?,
                        near: frustum.near,
                        far: frustum.far,
                        corners: frustum.corners,
                    })
                })
                .collect(),
            normal_overlays: self
                .normal_overlays
                .iter()
                .filter_map(|overlay| {
                    Some(SceneNormalInspectionV1 {
                        node: *node_handles.get(&overlay.node)?,
                        instance: overlay.instance.map(InstanceId::as_u64),
                        length: overlay.length,
                        segments: overlay.segments.clone(),
                    })
                })
                .collect(),
            instance_sets: None,
            active_camera,
            counts: SceneInspectionCountsV1 {
                visible_drawable: self.visible_drawable_count,
                cameras: self.camera_count,
                lights: self.light_count,
                anchors: self.anchor_count,
                connectors: self.connector_count,
                bounded_nodes: self.bounded_node_count,
                clipping_planes: self.clipping_plane_count,
            },
            revisions: SceneInspectionRevisionsV1 {
                structure: self.structure_revision,
                transform: self.transform_revision,
                appearance: self.appearance_revision,
                interaction: self.interaction_revision,
            },
        }
    }

    pub fn to_schema_json(&self) -> serde_json::Value {
        serde_json::to_value(self.to_schema_report())
            .expect("scene inspection schema contains only serializable fields")
    }

    fn schema_node_handles(
        &self,
        supplied_node_handles: &BTreeMap<NodeKey, u64>,
    ) -> BTreeMap<NodeKey, u64> {
        let mut next_report_local = 1;
        let mut used = BTreeSet::new();
        self.report_local_node_order()
            .into_iter()
            .map(|node| {
                let handle = supplied_node_handles
                    .get(&node)
                    .copied()
                    .filter(|handle| *handle != 0)
                    .unwrap_or_else(|| {
                        while used.contains(&next_report_local) {
                            next_report_local += 1;
                        }
                        let handle = next_report_local;
                        next_report_local += 1;
                        handle
                    });
                used.insert(handle);
                (node, handle)
            })
            .collect()
    }

    fn report_local_node_order(&self) -> Vec<NodeKey> {
        let nodes_by_key: BTreeSet<NodeKey> = self.nodes.iter().map(|node| node.node).collect();
        let mut children_by_parent: BTreeMap<NodeKey, Vec<NodeKey>> = BTreeMap::new();
        let mut roots = Vec::new();
        for node in &self.nodes {
            if let Some(parent) = node.parent {
                children_by_parent
                    .entry(parent)
                    .or_default()
                    .push(node.node);
            } else {
                roots.push(node.node);
            }
        }
        roots.sort_unstable();
        for children in children_by_parent.values_mut() {
            children.sort_unstable();
        }

        let mut order = Vec::with_capacity(self.nodes.len());
        for root in roots {
            append_node_order(root, &children_by_parent, &mut order);
        }
        for node in nodes_by_key {
            if !order.contains(&node) {
                append_node_order(node, &children_by_parent, &mut order);
            }
        }
        order
    }

    fn node_for_camera(&self, camera: CameraKey) -> Option<NodeKey> {
        self.nodes
            .iter()
            .find(|node| node.camera == Some(camera))
            .map(|node| node.node)
    }
}

impl SceneInspectionReportV1 {
    pub fn node_by_handle(&self, handle: u64) -> Option<&SceneNodeInspectionV1> {
        self.nodes.iter().find(|node| node.handle == handle)
    }

    pub fn children_of(&self, handle: u64) -> Vec<&SceneNodeInspectionV1> {
        self.nodes
            .iter()
            .filter(|node| node.parent == Some(handle))
            .collect()
    }

    pub fn roots(&self) -> Vec<&SceneNodeInspectionV1> {
        self.nodes
            .iter()
            .filter(|node| node.parent.is_none())
            .collect()
    }

    pub fn find_by_tag(&self, tag: &str) -> Vec<&SceneNodeInspectionV1> {
        self.nodes
            .iter()
            .filter(|node| node.tags.iter().any(|candidate| candidate == tag))
            .collect()
    }
}

fn append_node_order(
    node: NodeKey,
    children_by_parent: &BTreeMap<NodeKey, Vec<NodeKey>>,
    order: &mut Vec<NodeKey>,
) {
    if order.contains(&node) {
        return;
    }
    order.push(node);
    if let Some(children) = children_by_parent.get(&node) {
        for child in children {
            append_node_order(*child, children_by_parent, order);
        }
    }
}
