use super::{SceneHostCore, SceneHostError};
use crate::{AnnotationAnchor, AssetFetcher, Vec3};

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn set_node_annotation(
        &mut self,
        id: &str,
        node: u64,
        local_offset: [f32; 3],
    ) -> Result<(), SceneHostError> {
        let node = self.resolve_node(node)?;
        self.scene.set_annotation_anchor(AnnotationAnchor::node(
            id,
            node,
            Vec3::new(local_offset[0], local_offset[1], local_offset[2]),
        ))?;
        Ok(())
    }

    pub fn set_world_annotation(
        &mut self,
        id: &str,
        position: [f32; 3],
    ) -> Result<(), SceneHostError> {
        self.scene.set_annotation_anchor(AnnotationAnchor::world(
            id,
            Vec3::new(position[0], position[1], position[2]),
        ))?;
        Ok(())
    }

    pub fn clear_annotation(&mut self, id: &str) -> bool {
        self.scene.clear_annotation_anchor(id)
    }
}
