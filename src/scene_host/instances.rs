use std::collections::BTreeSet;

use super::{SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::assets::{GeometryHandle, MaterialHandle, SceneAsset, SceneAssetMesh};
use crate::scene::WorldTransform;
use crate::{
    AssetFetcher, Color, InstanceId, InstanceSetKey, NodeKey, SceneHostInstanceEntryInspectionV1,
    SceneHostInstanceSetInspectionV1, Transform,
};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct HostInstanceBinding {
    pub(super) entries: Vec<HostInstanceEntry>,
    pub(super) root_transform: Transform,
    pub(super) visible: bool,
    pub(super) tint: Option<Color>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct HostInstanceEntry {
    pub(super) set_node: NodeKey,
    pub(super) set: InstanceSetKey,
    pub(super) instance: InstanceId,
    /// The drawable's placement in its asset as translation, rotation, and
    /// scale: the decomposition of `local_placement` when the asset shears it.
    pub(super) local_transform: Transform,
    /// The drawable's exact placement in its asset.
    pub(super) local_placement: WorldTransform,
}

/// One mesh of a flattened asset. Its placement is the exact matrix product
/// of the asset hierarchy above it, so a rotated node under a non-uniformly
/// scaled one keeps its shear.
#[derive(Debug, Clone, PartialEq)]
struct InstancedDrawable {
    geometry: GeometryHandle,
    material: MaterialHandle,
    local_placement: WorldTransform,
}

#[derive(Debug, Clone)]
struct InstanceSetBuild {
    set_node: NodeKey,
    set: InstanceSetKey,
    geometry: GeometryHandle,
    material: MaterialHandle,
    local_placements: Vec<WorldTransform>,
}

impl<F: AssetFetcher> SceneHostCore<F> {
    pub(super) fn instantiate_scene_asset_instanced_under(
        &mut self,
        parent: NodeKey,
        scene_asset: &SceneAsset,
        count: usize,
    ) -> Result<Vec<u64>, SceneHostError> {
        if count == 0 {
            return Err(invalid_input(
                "instanced scene import count must be greater than zero",
            ));
        }

        let drawables = collect_drawables(scene_asset);
        if drawables.is_empty() {
            return Err(invalid_input(
                "instanced scene import requires at least one mesh primitive",
            ));
        }

        let mut builds: Vec<InstanceSetBuild> = Vec::new();
        for drawable in drawables {
            if let Some(build) = builds.iter_mut().find(|build| {
                build.geometry == drawable.geometry && build.material == drawable.material
            }) {
                build.local_placements.push(drawable.local_placement);
                continue;
            }
            let (set_node, set) = self.scene.add_instance_set_node(
                parent,
                drawable.geometry,
                drawable.material,
                Transform::IDENTITY,
            )?;
            self.register_node(set_node);
            builds.push(InstanceSetBuild {
                set_node,
                set,
                geometry: drawable.geometry,
                material: drawable.material,
                local_placements: vec![drawable.local_placement],
            });
        }

        for build in &builds {
            self.scene.reserve_instances(
                build.set,
                count.saturating_mul(build.local_placements.len()),
            )?;
        }

        let mut handles = Vec::with_capacity(count);
        for _ in 0..count {
            let root_transform = Transform::IDENTITY;
            let mut entries = Vec::new();
            for build in &builds {
                for local_placement in &build.local_placements {
                    let instance = self.scene.push_instance_placement(
                        build.set,
                        instance_placement(root_transform, *local_placement),
                    )?;
                    entries.push(HostInstanceEntry {
                        set_node: build.set_node,
                        set: build.set,
                        instance,
                        local_transform: local_placement.trs(),
                        local_placement: *local_placement,
                    });
                }
            }
            let binding = HostInstanceBinding {
                entries,
                root_transform,
                visible: true,
                tint: None,
            };
            let handle = self.instance_handles.insert(binding.clone());
            for entry in binding.entries {
                self.instance_handle_map
                    .insert((entry.set_node, entry.instance), handle);
            }
            handles.push(handle);
        }

        Ok(handles)
    }

    pub(super) fn set_instance_root_transform(
        &mut self,
        handle: u64,
        transform: Transform,
    ) -> Result<(), SceneHostError> {
        let binding = self.preflight_instance_root_transform(handle)?;
        self.set_preflighted_instance_root_transform(handle, &binding, transform)
    }

    pub(super) fn set_preflighted_instance_root_transform(
        &mut self,
        handle: u64,
        binding: &HostInstanceBinding,
        transform: Transform,
    ) -> Result<(), SceneHostError> {
        if binding.root_transform == transform {
            return Ok(());
        }
        for entry in &binding.entries {
            self.scene.set_instance_placement(
                entry.set,
                entry.instance,
                instance_placement(transform, entry.local_placement),
            )?;
        }
        self.instance_binding_mut(handle)?.root_transform = transform;
        Ok(())
    }

    pub(super) fn set_instance_root_visible(
        &mut self,
        handle: u64,
        visible: bool,
    ) -> Result<(), SceneHostError> {
        let binding = self.instance_binding(handle)?.clone();
        if binding.visible == visible {
            return Ok(());
        }
        for entry in &binding.entries {
            self.scene
                .set_instance_visible(entry.set, entry.instance, visible)?;
        }
        self.instance_binding_mut(handle)?.visible = visible;
        Ok(())
    }

    pub(super) fn set_instance_root_tint(
        &mut self,
        handle: u64,
        tint: Option<Color>,
    ) -> Result<(), SceneHostError> {
        if tint.is_some_and(|tint| tint.a < 1.0) {
            return Err(invalid_input(
                "instanced scene roots only accept opaque per-instance tint in this release",
            ));
        }
        let binding = self.instance_binding(handle)?.clone();
        if binding.tint == tint {
            return Ok(());
        }
        for entry in &binding.entries {
            self.scene
                .set_instance_tint(entry.set, entry.instance, tint)?;
        }
        self.instance_binding_mut(handle)?.tint = tint;
        Ok(())
    }

    pub(super) fn remove_instance_root(&mut self, handle: u64) -> Result<(), SceneHostError> {
        let binding = self.instance_handles.remove(
            handle,
            SceneHostErrorCode::NodeHandleNotFound,
            SceneHostErrorCode::StaleNodeHandle,
        )?;
        for entry in binding.entries {
            self.scene.remove_instance(entry.set, entry.instance)?;
            self.instance_handle_map
                .remove(&(entry.set_node, entry.instance));
        }
        Ok(())
    }

    pub(super) fn invalidate_instance_bindings_for_nodes(&mut self, removed_nodes: &[NodeKey]) {
        if removed_nodes.is_empty() {
            return;
        }
        let removed_nodes = removed_nodes.iter().copied().collect::<BTreeSet<_>>();
        let handles = self
            .instance_handle_map
            .iter()
            .filter_map(|((node, _instance), handle)| {
                removed_nodes.contains(node).then_some(*handle)
            })
            .collect::<BTreeSet<_>>();
        for handle in handles {
            let Ok(binding) = self.instance_handles.remove(
                handle,
                SceneHostErrorCode::NodeHandleNotFound,
                SceneHostErrorCode::StaleNodeHandle,
            ) else {
                continue;
            };
            for entry in binding.entries {
                self.instance_handle_map
                    .remove(&(entry.set_node, entry.instance));
            }
        }
    }

    pub(super) fn instance_bindings_report(&self) -> Vec<SceneHostInstanceSetInspectionV1> {
        let handles = self
            .instance_handle_map
            .values()
            .copied()
            .collect::<BTreeSet<_>>();
        handles
            .into_iter()
            .filter_map(|handle| {
                let binding = self.instance_binding(handle).ok()?;
                let entries = binding
                    .entries
                    .iter()
                    .map(|entry| SceneHostInstanceEntryInspectionV1 {
                        set_node: self.node_handle_map.get(&entry.set_node).copied(),
                        instance_id: entry.instance.as_u64(),
                        local_transform: entry.local_transform,
                    })
                    .collect::<Vec<_>>();
                Some(SceneHostInstanceSetInspectionV1 {
                    root_handle: handle,
                    visible: binding.visible,
                    tint: binding.tint,
                    root_transform: binding.root_transform,
                    entries,
                })
            })
            .collect()
    }

    pub(super) fn preflight_instance_root_transform(
        &self,
        handle: u64,
    ) -> Result<HostInstanceBinding, SceneHostError> {
        let binding = self.instance_binding(handle)?.clone();
        for entry in &binding.entries {
            if self.scene.node(entry.set_node).is_none() {
                return Err(SceneHostError::new(
                    SceneHostErrorCode::NodeHandleNotFound,
                    format!(
                        "instance root {handle} references missing set node {:?}",
                        entry.set_node
                    ),
                ));
            }
            let Some(instance_set) = self.scene.instance_set(entry.set) else {
                return Err(SceneHostError::new(
                    SceneHostErrorCode::NodeHandleNotFound,
                    format!(
                        "instance root {handle} references missing instance set {:?}",
                        entry.set
                    ),
                ));
            };
            if !instance_set.contains(entry.instance) {
                return Err(SceneHostError::new(
                    SceneHostErrorCode::NodeHandleNotFound,
                    format!(
                        "instance root {handle} references missing instance {:?}",
                        entry.instance
                    ),
                ));
            }
        }
        Ok(binding)
    }

    fn instance_binding(&self, handle: u64) -> Result<&HostInstanceBinding, SceneHostError> {
        self.instance_handles.get(
            handle,
            SceneHostErrorCode::NodeHandleNotFound,
            SceneHostErrorCode::StaleNodeHandle,
        )
    }

    fn instance_binding_mut(
        &mut self,
        handle: u64,
    ) -> Result<&mut HostInstanceBinding, SceneHostError> {
        self.instance_handles.get_mut(
            handle,
            SceneHostErrorCode::NodeHandleNotFound,
            SceneHostErrorCode::StaleNodeHandle,
        )
    }
}

fn collect_drawables(scene_asset: &SceneAsset) -> Vec<InstancedDrawable> {
    let mut child_indices = BTreeSet::new();
    for node in scene_asset.nodes() {
        child_indices.extend(node.children().iter().copied());
    }

    let mut drawables = Vec::new();
    for root in (0..scene_asset.nodes().len()).filter(|index| !child_indices.contains(index)) {
        collect_node_drawables(scene_asset, root, WorldTransform::IDENTITY, &mut drawables);
    }
    drawables
}

/// Where an instance root at `root_transform` puts a drawable of its asset:
/// the root transform times the drawable's exact placement. The product is
/// sheared when the root scales non-uniformly and the drawable is rotated, or
/// when the asset already shears the drawable.
fn instance_placement(
    root_transform: Transform,
    local_placement: WorldTransform,
) -> WorldTransform {
    WorldTransform::from(root_transform).then(local_placement)
}

fn collect_node_drawables(
    scene_asset: &SceneAsset,
    node_index: usize,
    parent_from_asset: WorldTransform,
    drawables: &mut Vec<InstancedDrawable>,
) {
    let Some(node) = scene_asset.nodes().get(node_index) else {
        return;
    };
    let node_from_asset = parent_from_asset.child(node.transform());
    for mesh in node.meshes() {
        collect_mesh_drawables(mesh, node_from_asset, node.instance_transforms(), drawables);
    }
    for child in node.children() {
        collect_node_drawables(scene_asset, *child, node_from_asset, drawables);
    }
}

fn collect_mesh_drawables(
    mesh: &SceneAssetMesh,
    node_from_asset: WorldTransform,
    instance_transforms: &[Transform],
    drawables: &mut Vec<InstancedDrawable>,
) {
    if instance_transforms.is_empty() {
        drawables.push(InstancedDrawable {
            geometry: mesh.geometry(),
            material: mesh.material(),
            local_placement: node_from_asset,
        });
        return;
    }
    for instance_transform in instance_transforms {
        drawables.push(InstancedDrawable {
            geometry: mesh.geometry(),
            material: mesh.material(),
            local_placement: node_from_asset.child(*instance_transform),
        });
    }
}

fn invalid_input(message: impl Into<String>) -> SceneHostError {
    SceneHostError::new(SceneHostErrorCode::InvalidInput, message.into())
}
