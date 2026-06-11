use super::inputs::{
    transform_from_component_array, transform_from_components, validate_transform,
};
use super::{SceneHostCore, SceneHostError};
use crate::{AssetFetcher, Transform};

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn set_transform(&mut self, node: u64, transform: Transform) -> Result<(), SceneHostError> {
        let transform = validate_transform(transform)?;
        if self.is_instance_root_handle(node) {
            return self.set_instance_root_transform(node, transform);
        }
        let node = self.resolve_node(node)?;
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
        for (node, transform) in transforms {
            let transform = validate_transform(*transform)?;
            if self.is_instance_root_handle(*node) {
                instance_roots.push((*node, transform));
            } else {
                raw.push((self.resolve_node(*node)?, transform));
            }
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
        for (node, components) in transforms {
            let transform = transform_from_component_array(*components)?;
            if self.is_instance_root_handle(*node) {
                instance_roots.push((*node, transform));
            } else {
                raw.push((self.resolve_node(*node)?, transform));
            }
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
