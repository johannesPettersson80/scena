use crate::assets::Assets;
use crate::geometry::Aabb;

use super::transforms::compose_transform;
use super::view_math::{merge_optional_bounds, transform_aabb, union_aabb};
use super::{Node, NodeKey, NodeKind, Scene};

impl Scene {
    pub(super) fn scene_bounds_world(&self, include_helpers: bool) -> Option<Aabb> {
        self.mesh_bounds_nodes()
            .filter(|(node, _bounds)| {
                !matches!(
                    self.nodes.get(*node).map(|node_ref| &node_ref.kind),
                    Some(NodeKind::Label(_))
                )
            })
            .filter(|(node, _)| include_helpers || !self.has_tag(*node, "scena:inspection:helper"))
            .filter_map(|(node, bounds)| {
                let transform = self.world_transform(node)?;
                Some(transform_aabb(bounds, transform))
            })
            .reduce(union_aabb)
    }

    pub(super) fn visible_label_anchor_bounds_world(&self) -> Option<Aabb> {
        self.label_nodes()
            .map(|(_node, _label, _desc, transform)| {
                Aabb::new(transform.translation, transform.translation)
            })
            .reduce(union_aabb)
    }

    pub(super) fn visible_label_margin_px(&self, viewport_width: u32, viewport_height: u32) -> f32 {
        let max_label_half_extent = self
            .label_nodes()
            .map(|(_node, _label, desc, _transform)| {
                let metrics = desc.metrics();
                let padding = (desc.size() * 0.25).ceil().max(2.0);
                metrics.width_px.max(metrics.height_px) * 0.5 + padding + 8.0
            })
            .fold(0.0_f32, f32::max);
        // Keep overlay labels from consuming most of the frame. A per-side cap of 15%
        // leaves at least 70% of the shorter viewport dimension for the framed subject.
        let max_usable = viewport_width.min(viewport_height) as f32 * 0.15;
        max_label_half_extent.min(max_usable).max(0.0)
    }

    pub(super) fn node_subtree_bounds_world(&self, node: NodeKey) -> Option<Aabb> {
        let node_ref = self.nodes.get(node)?;
        let local_bounds = self.node_bounds.get(&node).and_then(|bounds| {
            let transform = self.world_transform(node)?;
            Some(transform_aabb(*bounds, transform))
        });
        node_ref
            .children
            .iter()
            .filter_map(|child| self.node_subtree_bounds_world(*child))
            .fold(local_bounds, |bounds, child_bounds| {
                Some(match bounds {
                    Some(bounds) => union_aabb(bounds, child_bounds),
                    None => child_bounds,
                })
            })
    }

    pub(super) fn asset_backed_scene_bounds_world<F>(
        &self,
        assets: &Assets<F>,
        include_helpers: bool,
    ) -> Option<Aabb> {
        let mut bounds = None;
        for (node, node_ref) in self.nodes.iter() {
            if !self.visible_for_active_camera(node) {
                continue;
            }
            if !include_helpers && self.has_tag(node, "scena:inspection:helper") {
                continue;
            }
            if let Some(node_bounds) = self.asset_backed_node_bounds_world(node, node_ref, assets) {
                bounds = Some(merge_optional_bounds(bounds, node_bounds));
            }
        }
        bounds
    }

    pub(super) fn visible_node_subtree_bounds_world(
        &self,
        node: NodeKey,
        include_helpers: bool,
    ) -> Option<Aabb> {
        if !self.visible_for_active_camera(node)
            || (!include_helpers && self.has_tag(node, "scena:inspection:helper"))
        {
            return None;
        }
        let node_ref = self.nodes.get(node)?;
        let local = self.node_bounds.get(&node).and_then(|bounds| {
            self.world_transform(node)
                .map(|transform| transform_aabb(*bounds, transform))
        });
        node_ref
            .children
            .iter()
            .filter_map(|child| self.visible_node_subtree_bounds_world(*child, include_helpers))
            .fold(local, |bounds, child| {
                Some(bounds.map_or(child, |bounds| union_aabb(bounds, child)))
            })
    }

    pub(super) fn visible_asset_backed_node_subtree_bounds_world<F>(
        &self,
        node: NodeKey,
        assets: &Assets<F>,
        include_helpers: bool,
    ) -> Option<Aabb> {
        if !self.visible_for_active_camera(node)
            || (!include_helpers && self.has_tag(node, "scena:inspection:helper"))
        {
            return None;
        }
        let node_ref = self.nodes.get(node)?;
        let stored = self.node_bounds.get(&node).and_then(|bounds| {
            self.world_transform(node)
                .map(|transform| transform_aabb(*bounds, transform))
        });
        let direct = self.asset_backed_node_bounds_world(node, node_ref, assets);
        let local = stored.into_iter().chain(direct).reduce(union_aabb);
        node_ref
            .children
            .iter()
            .filter_map(|child| {
                self.visible_asset_backed_node_subtree_bounds_world(*child, assets, include_helpers)
            })
            .fold(local, |bounds, child| {
                Some(bounds.map_or(child, |bounds| union_aabb(bounds, child)))
            })
    }

    pub(super) fn asset_backed_node_subtree_bounds_world<F>(
        &self,
        node: NodeKey,
        assets: &Assets<F>,
    ) -> Option<Aabb> {
        let node_ref = self.nodes.get(node)?;
        node_ref
            .children
            .iter()
            .filter_map(|child| self.asset_backed_node_subtree_bounds_world(*child, assets))
            .fold(
                self.asset_backed_node_bounds_world(node, node_ref, assets),
                |bounds, child_bounds| Some(merge_optional_bounds(bounds, child_bounds)),
            )
    }

    fn asset_backed_node_bounds_world<F>(
        &self,
        node: NodeKey,
        node_ref: &Node,
        assets: &Assets<F>,
    ) -> Option<Aabb> {
        match &node_ref.kind {
            NodeKind::Mesh(mesh) => {
                let geometry = assets.geometry(mesh.geometry())?;
                let transform = self.world_transform(node)?;
                Some(transform_aabb(geometry.bounds(), transform))
            }
            NodeKind::InstanceSet(instance_set) => {
                let instance_set = self.instance_sets.get(*instance_set)?;
                let geometry = assets.geometry(instance_set.geometry())?;
                let node_transform = self.world_transform(node)?;
                instance_set
                    .instances()
                    .map(|instance| {
                        transform_aabb(
                            geometry.bounds(),
                            compose_transform(node_transform, instance.transform()),
                        )
                    })
                    .reduce(union_aabb)
            }
            NodeKind::Empty
            | NodeKind::Renderable(_)
            | NodeKind::Model(_)
            | NodeKind::ParticleSet(_)
            | NodeKind::Label(_)
            | NodeKind::Camera(_)
            | NodeKind::Light(_) => None,
        }
    }
}
