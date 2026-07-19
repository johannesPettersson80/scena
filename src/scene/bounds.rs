use super::{NodeKey, NodeKind, Scene};
use crate::{Aabb, LookupError};

impl Scene {
    /// Assigns caller-authored local bounds to an empty scene node.
    ///
    /// Geometry, imports, particles, labels, and other renderable owners keep
    /// their own bounds; this API refuses to replace those authoritative values.
    pub fn set_authored_node_bounds(
        &mut self,
        node: NodeKey,
        bounds: Aabb,
    ) -> Result<(), LookupError> {
        let Some(node_ref) = self.nodes.get(node) else {
            return Err(LookupError::NodeNotFound(node));
        };
        if !matches!(node_ref.kind, NodeKind::Empty) || self.node_bounds.contains_key(&node) {
            return Err(LookupError::InvalidBounds {
                reason: "authored bounds cannot override geometry- or asset-owned bounds",
            });
        }
        if !bounds.min.is_finite()
            || !bounds.max.is_finite()
            || bounds.min.x > bounds.max.x
            || bounds.min.y > bounds.max.y
            || bounds.min.z > bounds.max.z
        {
            return Err(LookupError::InvalidBounds {
                reason: "authored bounds must be finite with min <= max",
            });
        }
        self.node_bounds.insert(node, bounds);
        self.structure_revision = self.structure_revision.saturating_add(1);
        Ok(())
    }

    /// Returns the local bounds explicitly owned by the scene node, if any.
    pub fn node_local_bounds(&self, node: NodeKey) -> Result<Option<Aabb>, LookupError> {
        if !self.nodes.contains_key(node) {
            return Err(LookupError::NodeNotFound(node));
        }
        Ok(self.node_bounds.get(&node).copied())
    }
}
