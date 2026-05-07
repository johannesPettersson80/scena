use crate::assets::{GeometryHandle, MaterialHandle};
use crate::diagnostics::LookupError;

use super::{InstanceSetKey, NodeKey, NodeKind, Scene, Transform};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstanceId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceCullingPolicy {
    CpuBoundingBoxFallback,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Instance {
    id: InstanceId,
    transform: Transform,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InstanceSet {
    geometry: GeometryHandle,
    material: MaterialHandle,
    instances: Vec<Instance>,
    next_id: u64,
    culling_policy: InstanceCullingPolicy,
}

impl Scene {
    pub fn add_instance_set(
        &mut self,
        parent: NodeKey,
        geometry: GeometryHandle,
        material: MaterialHandle,
        transform: Transform,
    ) -> Result<InstanceSetKey, LookupError> {
        let instance_set = self
            .instance_sets
            .insert(InstanceSet::new(geometry, material));
        if let Err(error) = self.insert_node(parent, NodeKind::InstanceSet(instance_set), transform)
        {
            self.instance_sets.remove(instance_set);
            return Err(error);
        }
        Ok(instance_set)
    }

    pub fn instance_set(&self, instance_set: InstanceSetKey) -> Option<&InstanceSet> {
        self.instance_sets.get(instance_set)
    }

    pub fn reserve_instances(
        &mut self,
        instance_set: InstanceSetKey,
        additional: usize,
    ) -> Result<(), LookupError> {
        self.instance_set_mut(instance_set)?.reserve(additional);
        Ok(())
    }

    pub fn push_instance(
        &mut self,
        instance_set: InstanceSetKey,
        transform: Transform,
    ) -> Result<InstanceId, LookupError> {
        let id = self.instance_set_mut(instance_set)?.push(transform);
        self.structure_revision = self.structure_revision.saturating_add(1);
        Ok(id)
    }

    pub fn remove_instance(
        &mut self,
        instance_set: InstanceSetKey,
        instance: InstanceId,
    ) -> Result<Option<Instance>, LookupError> {
        let removed = self.instance_set_mut(instance_set)?.remove(instance);
        if removed.is_some() {
            self.structure_revision = self.structure_revision.saturating_add(1);
        }
        Ok(removed)
    }

    pub fn clear_instances(&mut self, instance_set: InstanceSetKey) -> Result<(), LookupError> {
        let changed = self.instance_set_mut(instance_set)?.clear();
        if changed {
            self.structure_revision = self.structure_revision.saturating_add(1);
        }
        Ok(())
    }

    fn instance_set_mut(
        &mut self,
        instance_set: InstanceSetKey,
    ) -> Result<&mut InstanceSet, LookupError> {
        self.instance_sets
            .get_mut(instance_set)
            .ok_or(LookupError::InstanceSetNotFound(instance_set))
    }
}

impl InstanceId {
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl Instance {
    pub const fn id(self) -> InstanceId {
        self.id
    }

    pub const fn transform(self) -> Transform {
        self.transform
    }
}

impl InstanceSet {
    const fn new(geometry: GeometryHandle, material: MaterialHandle) -> Self {
        Self {
            geometry,
            material,
            instances: Vec::new(),
            next_id: 1,
            culling_policy: InstanceCullingPolicy::CpuBoundingBoxFallback,
        }
    }

    pub const fn geometry(&self) -> GeometryHandle {
        self.geometry
    }

    pub const fn material(&self) -> MaterialHandle {
        self.material
    }

    pub const fn culling_policy(&self) -> InstanceCullingPolicy {
        self.culling_policy
    }

    pub fn len(&self) -> usize {
        self.instances.len()
    }

    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

    pub fn contains(&self, instance: InstanceId) -> bool {
        self.instances
            .iter()
            .any(|candidate| candidate.id == instance)
    }

    pub fn instances(&self) -> impl ExactSizeIterator<Item = &Instance> {
        self.instances.iter()
    }

    fn reserve(&mut self, additional: usize) {
        self.instances.reserve(additional);
    }

    fn push(&mut self, transform: Transform) -> InstanceId {
        let id = InstanceId(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        self.instances.push(Instance { id, transform });
        id
    }

    fn remove(&mut self, instance: InstanceId) -> Option<Instance> {
        let index = self
            .instances
            .iter()
            .position(|candidate| candidate.id == instance)?;
        Some(self.instances.remove(index))
    }

    fn clear(&mut self) -> bool {
        let changed = !self.instances.is_empty();
        self.instances.clear();
        changed
    }
}
