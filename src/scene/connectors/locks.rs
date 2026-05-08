use super::super::{NodeKey, Scene};
use super::ConnectionError;

impl Scene {
    pub fn lock_node_for_connections(&mut self, node: NodeKey) -> Result<(), ConnectionError> {
        if !self.nodes.contains_key(node) {
            return Err(ConnectionError::NodeNotFound(node));
        }
        self.connection_locked_nodes.insert(node);
        self.structure_revision = self.structure_revision.saturating_add(1);
        Ok(())
    }

    pub fn unlock_node_for_connections(&mut self, node: NodeKey) -> Result<(), ConnectionError> {
        if !self.nodes.contains_key(node) {
            return Err(ConnectionError::NodeNotFound(node));
        }
        if self.connection_locked_nodes.remove(&node) {
            self.structure_revision = self.structure_revision.saturating_add(1);
        }
        Ok(())
    }

    pub fn node_connections_locked(&self, node: NodeKey) -> Result<bool, ConnectionError> {
        if !self.nodes.contains_key(node) {
            return Err(ConnectionError::NodeNotFound(node));
        }
        Ok(self.connection_locked_nodes.contains(&node))
    }
}
