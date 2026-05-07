use crate::diagnostics::LookupError;

use super::{NodeKey, Scene, Transform};

impl Scene {
    pub fn set_transform(
        &mut self,
        node: NodeKey,
        transform: Transform,
    ) -> Result<(), LookupError> {
        let node = self
            .nodes
            .get_mut(node)
            .ok_or(LookupError::NodeNotFound(node))?;
        if node.transform != transform {
            node.transform = transform;
            self.structure_revision = self.structure_revision.saturating_add(1);
        }
        Ok(())
    }
}
