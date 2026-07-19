use std::cell::RefCell;
use std::marker::PhantomData;

use super::Scene;

/// Scene-owned rollback boundary for multi-registry mutations.
///
/// This is deliberately private: cloning a `Scene` publicly would duplicate
/// typed keys while retaining the same identity. Transactions clone only the
/// retained CPU state, restore it on drop unless committed, and collapse every
/// changed revision lane to one observable boundary at commit.
pub(super) struct SceneTransaction<'scene> {
    scene: &'scene mut Scene,
    snapshot: Option<Scene>,
}

impl<'scene> SceneTransaction<'scene> {
    pub(super) fn new(scene: &'scene mut Scene) -> Self {
        let snapshot = scene.transaction_snapshot();
        Self {
            scene,
            snapshot: Some(snapshot),
        }
    }

    pub(super) fn scene(&mut self) -> &mut Scene {
        self.scene
    }

    pub(super) fn commit(mut self) {
        let snapshot = self
            .snapshot
            .as_ref()
            .expect("an uncommitted transaction owns a snapshot");
        self.scene.structure_revision =
            committed_revision(snapshot.structure_revision, self.scene.structure_revision);
        self.scene.transform_revision =
            committed_revision(snapshot.transform_revision, self.scene.transform_revision);
        self.scene.appearance_revision =
            committed_revision(snapshot.appearance_revision, self.scene.appearance_revision);
        self.scene.visibility_revision =
            committed_revision(snapshot.visibility_revision, self.scene.visibility_revision);
        self.snapshot = None;
    }
}

impl Drop for SceneTransaction<'_> {
    fn drop(&mut self) {
        if let Some(snapshot) = self.snapshot.take() {
            *self.scene = snapshot;
        }
    }
}

impl Scene {
    fn transaction_snapshot(&self) -> Self {
        Self {
            identity: self.identity.clone(),
            nodes: self.nodes.clone(),
            cameras: self.cameras.clone(),
            lights: self.lights.clone(),
            instance_sets: self.instance_sets.clone(),
            particle_sets: self.particle_sets.clone(),
            animation_mixers: self.animation_mixers.clone(),
            labels: self.labels.clone(),
            anchors: self.anchors.clone(),
            retired_anchors: self.retired_anchors.clone(),
            annotations: self.annotations.clone(),
            callouts: self.callouts.clone(),
            measurements: self.measurements.clone(),
            overlay_owners: self.overlay_owners.clone(),
            connectors: self.connectors.clone(),
            retired_connectors: self.retired_connectors.clone(),
            connection_locked_nodes: self.connection_locked_nodes.clone(),
            node_bounds: self.node_bounds.clone(),
            section_box: self.section_box.clone(),
            mesh_lods: self.mesh_lods.clone(),
            morph_weights: self.morph_weights.clone(),
            skin_bindings: self.skin_bindings.clone(),
            clipping_planes: self.clipping_planes.clone(),
            active_clipping_planes: self.active_clipping_planes.clone(),
            origin_shift: self.origin_shift,
            root: self.root,
            active_camera: self.active_camera,
            camera_layer_masks: self.camera_layer_masks.clone(),
            interaction: self.interaction.clone(),
            inspection_toolkit: self.inspection_toolkit.clone(),
            structure_revision: self.structure_revision,
            transform_revision: self.transform_revision,
            appearance_revision: self.appearance_revision,
            visibility_revision: self.visibility_revision,
            resolved_cache: RefCell::new(super::resolved_cache::ResolvedSceneCache::default()),
            not_sync: PhantomData,
        }
    }
}

fn committed_revision(before: u64, after: u64) -> u64 {
    if before == after {
        before
    } else {
        before.saturating_add(1)
    }
}
