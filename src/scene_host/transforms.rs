use super::inputs::{
    transform_from_component_array, transform_from_components, validate_transform,
};
use super::instances::HostInstanceBinding;
use super::{SceneHostCore, SceneHostError};
use crate::{AssetFetcher, NodeKey, Transform};

#[derive(Debug, Clone)]
enum ResolvedTransformTarget {
    Node(NodeKey),
    InstanceRoot {
        handle: u64,
        binding: HostInstanceBinding,
    },
}

#[derive(Debug, Clone)]
struct ResolvedTransformUpdate {
    handle: u64,
    target: ResolvedTransformTarget,
    transform: Transform,
}

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn set_transform(&mut self, node: u64, transform: Transform) -> Result<(), SceneHostError> {
        let handle = node;
        let transform = validate_transform(transform)?;
        if self.is_instance_root_handle(handle) {
            let binding = self.preflight_instance_root_transform(handle)?;
            self.cancel_transform_transition(handle);
            return self.set_preflighted_instance_root_transform(handle, &binding, transform);
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
        let mut resolved = Vec::with_capacity(transforms.len());
        for (node, transform) in transforms {
            let transform = validate_transform(*transform)?;
            let target = if self.is_instance_root_handle(*node) {
                ResolvedTransformTarget::InstanceRoot {
                    handle: *node,
                    binding: self.preflight_instance_root_transform(*node)?,
                }
            } else {
                ResolvedTransformTarget::Node(self.resolve_node(*node)?)
            };
            resolved.push(ResolvedTransformUpdate {
                handle: *node,
                target,
                transform,
            });
        }

        for update in &resolved {
            self.cancel_transform_transition(update.handle);
        }
        let raw = resolved
            .iter()
            .filter_map(|update| match &update.target {
                ResolvedTransformTarget::Node(node) => Some((*node, update.transform)),
                ResolvedTransformTarget::InstanceRoot { .. } => None,
            })
            .collect::<Vec<_>>();
        if !raw.is_empty() {
            self.scene.set_transforms(&raw)?;
        }
        for update in resolved {
            if let ResolvedTransformTarget::InstanceRoot { handle, binding } = update.target {
                self.set_preflighted_instance_root_transform(handle, &binding, update.transform)?;
            }
        }
        Ok(())
    }

    pub fn set_transforms_components(
        &mut self,
        transforms: &[(u64, [f32; 10])],
    ) -> Result<(), SceneHostError> {
        let mut resolved = Vec::with_capacity(transforms.len());
        for (node, components) in transforms {
            let transform = transform_from_component_array(*components)?;
            resolved.push((*node, transform));
        }
        self.set_transforms(&resolved)
    }
}
