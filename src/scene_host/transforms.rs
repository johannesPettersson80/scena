use super::inputs::{
    transform_from_component_array, transform_from_components, validate_transform,
};
use super::{SceneHostCore, SceneHostError};
use crate::{AssetFetcher, Transform};

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn set_transform(&mut self, node: u64, transform: Transform) -> Result<(), SceneHostError> {
        let transform = validate_transform(transform)?;
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
        for (node, transform) in transforms {
            raw.push((self.resolve_node(*node)?, validate_transform(*transform)?));
        }
        self.scene.set_transforms(&raw)?;
        Ok(())
    }

    pub fn set_transforms_components(
        &mut self,
        transforms: &[(u64, [f32; 10])],
    ) -> Result<(), SceneHostError> {
        let mut raw = Vec::with_capacity(transforms.len());
        for (node, components) in transforms {
            raw.push((
                self.resolve_node(*node)?,
                transform_from_component_array(*components)?,
            ));
        }
        self.scene.set_transforms(&raw)?;
        Ok(())
    }
}
