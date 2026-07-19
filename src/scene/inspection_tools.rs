use std::collections::BTreeMap;
use std::collections::BTreeSet;

use crate::assets::Assets;
use crate::diagnostics::LookupError;
use crate::geometry::{Aabb, GeometryDesc};
use crate::material::{Color, MaterialDesc};

use super::{CameraKey, NodeKey, Scene, Transform};

pub const INSPECTION_HELPER_TAG: &str = "scena:inspection:helper";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SceneVisibilitySnapshot {
    entries: Vec<SceneVisibilitySnapshotEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneVisibilitySnapshotEntry {
    pub node: NodeKey,
    pub visible: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SceneTintSnapshot {
    entries: Vec<SceneTintSnapshotEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneTintSnapshotEntry {
    pub node: NodeKey,
    pub tint: Option<Color>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectionHelperKind {
    BoundingBox,
    WorldAxesTriad,
    LocalAxesTriad,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InspectionHelperReport {
    pub kind: InspectionHelperKind,
    pub node: NodeKey,
    pub target: Option<NodeKey>,
    pub bounds: Option<Aabb>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct InspectionToolkitReport {
    pub isolated_nodes: Vec<NodeKey>,
    pub hidden_by_isolate_count: usize,
    pub ghosted_nodes: Vec<NodeKey>,
    pub helper_nodes: Vec<InspectionHelperReport>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct InspectionToolkitState {
    isolated_nodes: Vec<NodeKey>,
    hidden_by_isolate: BTreeSet<NodeKey>,
    helpers: BTreeMap<NodeKey, InspectionHelperRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct InspectionHelperRecord {
    kind: InspectionHelperKind,
    target: Option<NodeKey>,
    bounds: Option<Aabb>,
}

struct InspectionHelperSpec {
    geometry: GeometryDesc,
    transform: Transform,
    parent: NodeKey,
    record: InspectionHelperRecord,
    color: Color,
}

impl SceneVisibilitySnapshot {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> &[SceneVisibilitySnapshotEntry] {
        &self.entries
    }
}

impl SceneTintSnapshot {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> &[SceneTintSnapshotEntry] {
        &self.entries
    }
}

impl Scene {
    pub fn hide(&mut self, node: NodeKey) -> Result<(), LookupError> {
        self.set_visible(node, false)
    }

    pub fn show(&mut self, node: NodeKey) -> Result<(), LookupError> {
        self.set_visible(node, true)
    }

    pub fn toggle_visibility(&mut self, node: NodeKey) -> Result<bool, LookupError> {
        let visible = self.visible(node).ok_or(LookupError::NodeNotFound(node))?;
        let next = !visible;
        self.set_visible(node, next)?;
        Ok(next)
    }

    pub fn show_only(
        &mut self,
        nodes: impl IntoIterator<Item = NodeKey>,
    ) -> Result<SceneVisibilitySnapshot, LookupError> {
        let selected_nodes = nodes.into_iter().collect::<Vec<_>>();
        let mut keep = BTreeSet::from([self.root()]);
        for node in selected_nodes.iter().copied() {
            self.collect_isolate_keep_set(node, &mut keep)?;
        }
        let snapshot = self.apply_visibility_keep_set(&keep)?;
        self.inspection_toolkit.isolated_nodes = selected_nodes;
        self.inspection_toolkit.hidden_by_isolate = snapshot
            .entries
            .iter()
            .filter_map(|entry| entry.visible.then_some(entry.node))
            .collect();
        Ok(snapshot)
    }

    pub fn isolate(
        &mut self,
        nodes: impl IntoIterator<Item = NodeKey>,
    ) -> Result<SceneVisibilitySnapshot, LookupError> {
        self.show_only(nodes)
    }

    pub fn restore_visibility(
        &mut self,
        snapshot: &SceneVisibilitySnapshot,
    ) -> Result<(), LookupError> {
        for entry in &snapshot.entries {
            self.set_visible(entry.node, entry.visible)?;
        }
        self.inspection_toolkit.isolated_nodes.clear();
        self.inspection_toolkit.hidden_by_isolate.clear();
        Ok(())
    }

    pub fn ghost(&mut self, node: NodeKey, alpha: f32) -> Result<SceneTintSnapshot, LookupError> {
        if !alpha.is_finite() || !(0.0..=1.0).contains(&alpha) {
            return Err(LookupError::InvalidFramingOption {
                field: "alpha",
                reason: "ghost alpha must be finite and between 0 and 1",
            });
        }
        self.ghost_with_tint(node, Color::from_linear_rgba(1.0, 1.0, 1.0, alpha))
    }

    pub fn ghost_with_tint(
        &mut self,
        node: NodeKey,
        tint: Color,
    ) -> Result<SceneTintSnapshot, LookupError> {
        if !color_is_finite(tint) {
            return Err(LookupError::InvalidFramingOption {
                field: "tint",
                reason: "ghost tint must contain finite color channels",
            });
        }
        let mut nodes = Vec::new();
        self.collect_subtree(node, &mut nodes)?;
        let mut entries = Vec::new();
        for node in nodes {
            let previous = self.node_tint(node)?;
            if previous != Some(tint) {
                entries.push(SceneTintSnapshotEntry {
                    node,
                    tint: previous,
                });
                self.set_node_tint(node, Some(tint))?;
            }
        }
        Ok(SceneTintSnapshot { entries })
    }

    pub fn restore_tints(&mut self, snapshot: &SceneTintSnapshot) -> Result<(), LookupError> {
        for entry in &snapshot.entries {
            self.set_node_tint(entry.node, entry.tint)?;
        }
        Ok(())
    }

    pub fn fit_selection_with_assets<F>(
        &mut self,
        camera: CameraKey,
        nodes: impl IntoIterator<Item = NodeKey>,
        assets: &Assets<F>,
    ) -> Result<Aabb, LookupError> {
        let bounds = self.selected_bounds_with_assets(nodes, assets)?;
        self.frame(camera, bounds)?;
        Ok(bounds)
    }

    pub fn add_bounding_box_overlay<F>(
        &mut self,
        assets: &Assets<F>,
        node: NodeKey,
    ) -> Result<InspectionHelperReport, LookupError> {
        let bounds = self
            .node_world_bounds(node, assets)?
            .ok_or(LookupError::ImportHasNoBounds)?;
        let helper = self.add_inspection_helper(
            assets,
            InspectionHelperSpec {
                geometry: GeometryDesc::bounding_box(bounds),
                transform: Transform::IDENTITY,
                parent: self.root(),
                record: InspectionHelperRecord {
                    kind: InspectionHelperKind::BoundingBox,
                    target: Some(node),
                    bounds: Some(bounds),
                },
                color: Color::YELLOW,
            },
        )?;
        Ok(helper)
    }

    pub fn add_world_axes_triad<F>(
        &mut self,
        assets: &Assets<F>,
        length: f32,
    ) -> Result<InspectionHelperReport, LookupError> {
        validate_helper_length(length)?;
        self.add_inspection_helper(
            assets,
            InspectionHelperSpec {
                geometry: GeometryDesc::axes(length),
                transform: Transform::IDENTITY,
                parent: self.root(),
                record: InspectionHelperRecord {
                    kind: InspectionHelperKind::WorldAxesTriad,
                    target: None,
                    bounds: None,
                },
                color: Color::WHITE,
            },
        )
    }

    pub fn add_local_axes_triad<F>(
        &mut self,
        assets: &Assets<F>,
        node: NodeKey,
        length: f32,
    ) -> Result<InspectionHelperReport, LookupError> {
        if !self.nodes.contains_key(node) {
            return Err(LookupError::NodeNotFound(node));
        }
        validate_helper_length(length)?;
        self.add_inspection_helper(
            assets,
            InspectionHelperSpec {
                geometry: GeometryDesc::axes(length),
                transform: Transform::IDENTITY,
                parent: node,
                record: InspectionHelperRecord {
                    kind: InspectionHelperKind::LocalAxesTriad,
                    target: Some(node),
                    bounds: None,
                },
                color: Color::CYAN,
            },
        )
    }

    pub fn inspection_toolkit_report(&self) -> InspectionToolkitReport {
        let isolated_nodes = self
            .inspection_toolkit
            .isolated_nodes
            .iter()
            .copied()
            .filter(|node| self.nodes.contains_key(*node))
            .collect::<Vec<_>>();
        let hidden_by_isolate_count = self
            .inspection_toolkit
            .hidden_by_isolate
            .iter()
            .filter(|node| {
                self.nodes
                    .get(**node)
                    .is_some_and(|node_ref| !node_ref.visible)
            })
            .count();
        let ghosted_nodes = self
            .nodes
            .iter()
            .filter_map(|(node, node_ref)| {
                node_ref
                    .tint
                    .is_some_and(|tint| tint.a.is_finite() && tint.a < 1.0)
                    .then_some(node)
            })
            .collect::<Vec<_>>();
        let helper_nodes = self
            .inspection_toolkit
            .helpers
            .iter()
            .filter_map(|(node, record)| {
                self.nodes
                    .contains_key(*node)
                    .then_some(InspectionHelperReport {
                        kind: record.kind,
                        node: *node,
                        target: record.target,
                        bounds: record.bounds,
                    })
            })
            .collect();
        InspectionToolkitReport {
            isolated_nodes,
            hidden_by_isolate_count,
            ghosted_nodes,
            helper_nodes,
        }
    }

    fn selected_bounds_with_assets<F>(
        &self,
        nodes: impl IntoIterator<Item = NodeKey>,
        assets: &Assets<F>,
    ) -> Result<Aabb, LookupError> {
        let mut bounds: Option<Aabb> = None;
        let mut any_node = false;
        for node in nodes {
            any_node = true;
            if let Some(node_bounds) = self.node_world_bounds(node, assets)? {
                bounds = Some(match bounds {
                    Some(bounds) => bounds.union(node_bounds),
                    None => node_bounds,
                });
            }
        }
        if any_node {
            bounds.ok_or(LookupError::ImportHasNoBounds)
        } else {
            Err(LookupError::ImportHasNoBounds)
        }
    }

    fn add_inspection_helper<F>(
        &mut self,
        assets: &Assets<F>,
        spec: InspectionHelperSpec,
    ) -> Result<InspectionHelperReport, LookupError> {
        let geometry = assets.create_geometry(spec.geometry);
        let material = assets.create_material(MaterialDesc::line(spec.color, 1.0));
        let node = self
            .mesh(geometry, material)
            .parent(spec.parent)
            .transform(spec.transform)
            .add()?;
        self.set_helper_on_top(node, true)?;
        self.add_tag(node, INSPECTION_HELPER_TAG)?;
        self.inspection_toolkit.helpers.insert(node, spec.record);
        Ok(InspectionHelperReport {
            kind: spec.record.kind,
            node,
            target: spec.record.target,
            bounds: spec.record.bounds,
        })
    }

    fn collect_isolate_keep_set(
        &self,
        node: NodeKey,
        keep: &mut BTreeSet<NodeKey>,
    ) -> Result<(), LookupError> {
        if !self.nodes.contains_key(node) {
            return Err(LookupError::NodeNotFound(node));
        }
        let mut current = Some(node);
        while let Some(node) = current {
            keep.insert(node);
            current = self.nodes.get(node).and_then(|node| node.parent);
        }
        self.collect_subtree_into_set(node, keep)
    }

    fn collect_subtree_into_set(
        &self,
        node: NodeKey,
        keep: &mut BTreeSet<NodeKey>,
    ) -> Result<(), LookupError> {
        let node_ref = self
            .nodes
            .get(node)
            .ok_or(LookupError::NodeNotFound(node))?;
        keep.insert(node);
        for child in &node_ref.children {
            self.collect_subtree_into_set(*child, keep)?;
        }
        Ok(())
    }

    fn collect_subtree(&self, node: NodeKey, nodes: &mut Vec<NodeKey>) -> Result<(), LookupError> {
        let node_ref = self
            .nodes
            .get(node)
            .ok_or(LookupError::NodeNotFound(node))?;
        nodes.push(node);
        for child in &node_ref.children {
            self.collect_subtree(*child, nodes)?;
        }
        Ok(())
    }

    fn apply_visibility_keep_set(
        &mut self,
        keep: &BTreeSet<NodeKey>,
    ) -> Result<SceneVisibilitySnapshot, LookupError> {
        let updates = self
            .nodes
            .iter()
            .map(|(node, node_ref)| {
                let desired = keep.contains(&node);
                (node, node_ref.visible, desired)
            })
            .filter(|(_, current, desired)| current != desired)
            .collect::<Vec<_>>();
        let mut entries = Vec::with_capacity(updates.len());
        for (node, current, desired) in updates {
            entries.push(SceneVisibilitySnapshotEntry {
                node,
                visible: current,
            });
            self.set_visible(node, desired)?;
        }
        Ok(SceneVisibilitySnapshot { entries })
    }
}

fn color_is_finite(color: Color) -> bool {
    color.r.is_finite() && color.g.is_finite() && color.b.is_finite() && color.a.is_finite()
}

fn validate_helper_length(length: f32) -> Result<(), LookupError> {
    if length.is_finite() && length > 0.0 {
        Ok(())
    } else {
        Err(LookupError::InvalidFramingOption {
            field: "length",
            reason: "inspection helper length must be finite and positive",
        })
    }
}
