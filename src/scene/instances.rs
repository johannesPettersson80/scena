use crate::assets::{GeometryHandle, MaterialHandle};
use crate::diagnostics::LookupError;
use crate::material::Color;

use super::{InstanceSetKey, NodeKey, NodeKind, Scene, Transform};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstanceId(u64);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Instance {
    id: InstanceId,
    transform: Transform,
    visible: bool,
    tint: Option<Color>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InstanceSet {
    geometry: GeometryHandle,
    material: MaterialHandle,
    instances: Vec<Instance>,
    next_id: u64,
}

impl Scene {
    pub fn add_instance_set(
        &mut self,
        parent: NodeKey,
        geometry: GeometryHandle,
        material: MaterialHandle,
        transform: Transform,
    ) -> Result<InstanceSetKey, LookupError> {
        self.add_instance_set_node(parent, geometry, material, transform)
            .map(|(_, instance_set)| instance_set)
    }

    pub fn add_instance_set_node(
        &mut self,
        parent: NodeKey,
        geometry: GeometryHandle,
        material: MaterialHandle,
        transform: Transform,
    ) -> Result<(NodeKey, InstanceSetKey), LookupError> {
        let instance_set = self
            .instance_sets
            .insert(InstanceSet::new(geometry, material));
        match self.insert_node(parent, NodeKind::InstanceSet(instance_set), transform) {
            Ok(node) => Ok((node, instance_set)),
            Err(error) => {
                self.instance_sets.remove(instance_set);
                Err(error)
            }
        }
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

    pub fn set_instance_transform(
        &mut self,
        instance_set: InstanceSetKey,
        instance: InstanceId,
        transform: Transform,
    ) -> Result<(), LookupError> {
        if self
            .instance_set_mut(instance_set)?
            .set_transform(instance_set, instance, transform)?
        {
            self.transform_revision = self.transform_revision.saturating_add(1);
        }
        Ok(())
    }

    pub fn set_instance_visible(
        &mut self,
        instance_set: InstanceSetKey,
        instance: InstanceId,
        visible: bool,
    ) -> Result<(), LookupError> {
        if self
            .instance_set_mut(instance_set)?
            .set_visible(instance_set, instance, visible)?
        {
            self.visibility_revision = self.visibility_revision.saturating_add(1);
        }
        Ok(())
    }

    pub fn set_instance_tint(
        &mut self,
        instance_set: InstanceSetKey,
        instance: InstanceId,
        tint: Option<Color>,
    ) -> Result<(), LookupError> {
        validate_instance_tint(instance_set, instance, tint)?;
        if self
            .instance_set_mut(instance_set)?
            .set_tint(instance_set, instance, tint)?
        {
            self.appearance_revision = self.appearance_revision.saturating_add(1);
        }
        Ok(())
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

fn validate_instance_tint(
    instance_set: InstanceSetKey,
    instance: InstanceId,
    tint: Option<Color>,
) -> Result<(), LookupError> {
    let Some(tint) = tint else {
        return Ok(());
    };
    if tint.r.is_finite()
        && tint.g.is_finite()
        && tint.b.is_finite()
        && tint.a.is_finite()
        && tint.a == 1.0
    {
        Ok(())
    } else {
        Err(LookupError::InvalidInstanceTint {
            instance_set,
            instance,
            reason: "instanced scene roots only accept opaque per-instance tint until a transparent instancing path exists",
        })
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

    pub const fn visible(self) -> bool {
        self.visible
    }

    pub const fn tint(self) -> Option<Color> {
        self.tint
    }
}

impl InstanceSet {
    const fn new(geometry: GeometryHandle, material: MaterialHandle) -> Self {
        Self {
            geometry,
            material,
            instances: Vec::new(),
            next_id: 1,
        }
    }

    pub const fn geometry(&self) -> GeometryHandle {
        self.geometry
    }

    pub const fn material(&self) -> MaterialHandle {
        self.material
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
        self.instances.push(Instance {
            id,
            transform,
            visible: true,
            tint: None,
        });
        id
    }

    fn set_transform(
        &mut self,
        instance_set: InstanceSetKey,
        instance: InstanceId,
        transform: Transform,
    ) -> Result<bool, LookupError> {
        let instance = self.instance_mut(instance_set, instance)?;
        let changed = instance.transform != transform;
        if changed {
            instance.transform = transform;
        }
        Ok(changed)
    }

    fn set_visible(
        &mut self,
        instance_set: InstanceSetKey,
        instance: InstanceId,
        visible: bool,
    ) -> Result<bool, LookupError> {
        let instance = self.instance_mut(instance_set, instance)?;
        let changed = instance.visible != visible;
        if changed {
            instance.visible = visible;
        }
        Ok(changed)
    }

    fn set_tint(
        &mut self,
        instance_set: InstanceSetKey,
        instance: InstanceId,
        tint: Option<Color>,
    ) -> Result<bool, LookupError> {
        let instance = self.instance_mut(instance_set, instance)?;
        let changed = instance.tint != tint;
        if changed {
            instance.tint = tint;
        }
        Ok(changed)
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

    fn instance_mut(
        &mut self,
        instance_set: InstanceSetKey,
        instance: InstanceId,
    ) -> Result<&mut Instance, LookupError> {
        self.instances
            .iter_mut()
            .find(|candidate| candidate.id == instance)
            .ok_or(LookupError::InstanceNotFound {
                instance_set,
                instance,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Assets, GeometryDesc, MaterialDesc};

    #[test]
    fn scene_set_instance_tint_rejects_non_opaque_tint_before_backend_divergence() {
        let assets = Assets::new();
        let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.1, 0.1, 0.1));
        let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
        let mut scene = Scene::new();
        let instance_set = scene
            .add_instance_set(scene.root(), geometry, material, Transform::IDENTITY)
            .expect("instance set inserts");
        let instance = scene
            .push_instance(instance_set, Transform::IDENTITY)
            .expect("instance inserts");

        let error = scene
            .set_instance_tint(
                instance_set,
                instance,
                Some(Color::from_linear_rgba(1.0, 0.0, 0.0, 0.5)),
            )
            .expect_err("non-opaque instance tint must fail closed");

        assert!(
            error.to_string().contains("opaque"),
            "error should explain the opaque-tint invariant: {error}"
        );
        assert_eq!(
            scene
                .instance_set(instance_set)
                .expect("instance set exists")
                .instances()
                .find(|candidate| candidate.id() == instance)
                .expect("instance exists")
                .tint(),
            None
        );
    }
}
