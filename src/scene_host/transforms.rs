use super::inputs::{
    transform_from_component_array, transform_from_components, validate_transform,
};
use super::{SceneHostCore, SceneHostError};
use crate::{AssetFetcher, Transform};

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn set_transform(&mut self, node: u64, transform: Transform) -> Result<(), SceneHostError> {
        let handle = node;
        let transform = validate_transform(transform)?;
        if self.is_instance_root_handle(handle) {
            self.instance_handles.get(
                handle,
                super::SceneHostErrorCode::NodeHandleNotFound,
                super::SceneHostErrorCode::StaleNodeHandle,
            )?;
            self.cancel_transform_transition(handle);
            return self.set_instance_root_transform(handle, transform);
        }
        let node = self.resolve_node(handle)?;
        self.cancel_transform_transition(handle);
        self.scene.set_transform(node, transform)?;
        Ok(())
    }

    pub fn set_transform_components(
        &mut self,
        node: u64,
        translation: [f32; 3],
        rotation: [f32; 4],
        scale: [f32; 3],
    ) -> Result<(), SceneHostError> {
        let transform = transform_from_components(translation, rotation, scale)?;
        self.set_transform(node, transform)
    }

    pub fn set_transforms(
        &mut self,
        transforms: &[(u64, Transform)],
    ) -> Result<(), SceneHostError> {
        let mut raw = Vec::with_capacity(transforms.len());
        let mut instance_roots = Vec::new();
        let mut handles = Vec::with_capacity(transforms.len());
        for (node, transform) in transforms {
            let transform = validate_transform(*transform)?;
            handles.push(*node);
            if self.is_instance_root_handle(*node) {
                instance_roots.push((*node, transform));
            } else {
                raw.push((self.resolve_node(*node)?, transform));
            }
        }
        for handle in handles {
            self.cancel_transform_transition(handle);
        }
        if !raw.is_empty() {
            self.scene.set_transforms(&raw)?;
        }
        for (node, transform) in instance_roots {
            self.set_instance_root_transform(node, transform)?;
        }
        Ok(())
    }

    pub fn set_transforms_components(
        &mut self,
        transforms: &[(u64, [f32; 10])],
    ) -> Result<(), SceneHostError> {
        let mut raw = Vec::with_capacity(transforms.len());
        let mut instance_roots = Vec::new();
        let mut handles = Vec::with_capacity(transforms.len());
        for (node, components) in transforms {
            let transform = transform_from_component_array(*components)?;
            handles.push(*node);
            if self.is_instance_root_handle(*node) {
                instance_roots.push((*node, transform));
            } else {
                raw.push((self.resolve_node(*node)?, transform));
            }
        }
        for handle in handles {
            self.cancel_transform_transition(handle);
        }
        if !raw.is_empty() {
            self.scene.set_transforms(&raw)?;
        }
        for (node, transform) in instance_roots {
            self.set_instance_root_transform(node, transform)?;
        }
        Ok(())
    }
}
