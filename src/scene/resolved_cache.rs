use slotmap::SecondaryMap;

use super::transforms::compose_transform;
use super::{CameraKey, NodeKey, Scene, Transform};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResolvedSceneCacheStats {
    pub rebuilds: u64,
    /// World/visibility queries never allocate an ancestor chain. Rebuilds
    /// use one reusable top-down traversal stack instead.
    pub ancestor_vec_allocations: u64,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ResolvedNodeState {
    pub(super) world_transform: Transform,
    pub(super) hierarchy_visible: bool,
    pub(super) camera_visible: bool,
}

#[derive(Debug)]
pub(super) struct ResolvedSceneCache {
    structure_revision: u64,
    transform_revision: u64,
    visibility_revision: u64,
    active_camera: Option<CameraKey>,
    camera_layer_mask: u64,
    initialized: bool,
    nodes: SecondaryMap<NodeKey, ResolvedNodeState>,
    traversal_stack: Vec<(NodeKey, Transform, bool)>,
    stats: ResolvedSceneCacheStats,
}

impl Default for ResolvedSceneCache {
    fn default() -> Self {
        Self {
            structure_revision: 0,
            transform_revision: 0,
            visibility_revision: 0,
            active_camera: None,
            camera_layer_mask: u64::MAX,
            initialized: false,
            nodes: SecondaryMap::new(),
            traversal_stack: Vec::new(),
            stats: ResolvedSceneCacheStats::default(),
        }
    }
}

impl ResolvedSceneCache {
    fn matches(&self, scene: &Scene, camera_layer_mask: u64) -> bool {
        self.initialized
            && self.structure_revision == scene.structure_revision
            && self.transform_revision == scene.transform_revision
            && self.visibility_revision == scene.visibility_revision
            && self.active_camera == scene.active_camera
            && self.camera_layer_mask == camera_layer_mask
    }
}

impl Scene {
    pub(super) fn resolved_node_state(&self, node: NodeKey) -> Option<ResolvedNodeState> {
        self.ensure_resolved_scene_cache();
        self.resolved_cache.borrow().nodes.get(node).copied()
    }

    fn ensure_resolved_scene_cache(&self) {
        let camera_layer_mask = self
            .active_camera
            .and_then(|camera| self.camera_layer_masks.get(&camera).copied())
            .unwrap_or(u64::MAX);
        if self
            .resolved_cache
            .borrow()
            .matches(self, camera_layer_mask)
        {
            return;
        }

        let mut cache = self.resolved_cache.borrow_mut();
        cache.nodes.clear();
        cache.traversal_stack.clear();
        let root = self.root;
        cache
            .traversal_stack
            .push((root, Transform::IDENTITY, true));
        while let Some((node_key, parent_world, parent_visible)) = cache.traversal_stack.pop() {
            let Some(node) = self.nodes.get(node_key) else {
                continue;
            };
            let world_transform = compose_transform(parent_world, node.transform);
            let hierarchy_visible = parent_visible && node.visible;
            let camera_visible = hierarchy_visible && camera_layer_mask & node.layer_mask != 0;
            cache.nodes.insert(
                node_key,
                ResolvedNodeState {
                    world_transform,
                    hierarchy_visible,
                    camera_visible,
                },
            );
            cache.traversal_stack.extend(
                node.children
                    .iter()
                    .rev()
                    .map(|child| (*child, world_transform, hierarchy_visible)),
            );
        }
        cache.structure_revision = self.structure_revision;
        cache.transform_revision = self.transform_revision;
        cache.visibility_revision = self.visibility_revision;
        cache.active_camera = self.active_camera;
        cache.camera_layer_mask = camera_layer_mask;
        cache.initialized = true;
        cache.stats.rebuilds = cache.stats.rebuilds.saturating_add(1);
    }

    /// Returns the hierarchy-resolved visibility cached for a node.
    #[doc(hidden)]
    pub fn resolved_visibility(&self, node: NodeKey) -> Option<bool> {
        self.nodes
            .contains_key(node)
            .then(|| self.visible_in_hierarchy(node))
    }

    /// Returns deterministic cache work counters without forcing a rebuild.
    #[doc(hidden)]
    pub fn resolved_scene_cache_stats(&self) -> ResolvedSceneCacheStats {
        self.resolved_cache.borrow().stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{PerspectiveCamera, Vec3};

    #[test]
    fn pf04_pf05_contract_cache_tracks_reparent_remove_replace_and_camera_layers() {
        let mut scene = Scene::new();
        let left = scene
            .add_empty(scene.root(), Transform::at(Vec3::new(1.0, 0.0, 0.0)))
            .expect("left parent inserts");
        let right = scene
            .add_empty(scene.root(), Transform::at(Vec3::new(10.0, 0.0, 0.0)))
            .expect("right parent inserts");
        let child = scene
            .add_empty(left, Transform::at(Vec3::new(2.0, 0.0, 0.0)))
            .expect("child inserts");
        assert_eq!(scene.world_transform(child).unwrap().translation.x, 3.0);

        scene.nodes[left]
            .children
            .retain(|candidate| *candidate != child);
        scene.nodes[right].children.push(child);
        scene.nodes[child].parent = Some(right);
        scene.structure_revision = scene.structure_revision.saturating_add(1);
        scene.transform_revision = scene.transform_revision.saturating_add(1);
        assert_eq!(scene.world_transform(child).unwrap().translation.x, 12.0);

        scene.remove_node(child).expect("child removes");
        assert!(scene.world_transform(child).is_none());
        let replacement = scene
            .add_empty(right, Transform::at(Vec3::new(3.0, 0.0, 0.0)))
            .expect("replacement inserts");
        assert_eq!(
            scene.world_transform(replacement).unwrap().translation.x,
            13.0
        );

        scene.set_layer_mask(replacement, 0b10).unwrap();
        let camera_a = scene
            .add_perspective_camera(
                scene.root(),
                PerspectiveCamera::default(),
                Transform::IDENTITY,
            )
            .unwrap();
        let camera_b = scene
            .add_perspective_camera(
                scene.root(),
                PerspectiveCamera::default(),
                Transform::IDENTITY,
            )
            .unwrap();
        scene.set_camera_layer_mask(camera_a, 0b01).unwrap();
        scene.set_camera_layer_mask(camera_b, 0b10).unwrap();
        scene.set_active_camera(camera_a).unwrap();
        assert!(!scene.visible_for_active_camera(replacement));
        scene.set_active_camera(camera_b).unwrap();
        assert!(scene.visible_for_active_camera(replacement));
    }
}
