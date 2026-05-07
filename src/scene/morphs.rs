use crate::diagnostics::LookupError;

use super::{NodeKey, Scene};

impl Scene {
    pub fn morph_weights(&self, node: NodeKey) -> Option<&[f32]> {
        self.morph_weights.get(&node).map(Vec::as_slice)
    }

    pub fn set_morph_weights(
        &mut self,
        node: NodeKey,
        weights: impl Into<Vec<f32>>,
    ) -> Result<(), LookupError> {
        if !self.nodes.contains_key(node) {
            return Err(LookupError::NodeNotFound(node));
        }
        self.set_morph_weights_unchecked(node, weights.into());
        Ok(())
    }

    pub(crate) fn set_initial_morph_weights(&mut self, node: NodeKey, weights: &[f32]) {
        if !weights.is_empty() {
            self.morph_weights.insert(node, weights.to_vec());
        }
    }

    pub(crate) fn set_morph_weights_unchecked(&mut self, node: NodeKey, weights: Vec<f32>) -> bool {
        if self.morph_weights.get(&node) == Some(&weights) {
            return false;
        }
        self.morph_weights.insert(node, weights);
        self.structure_revision = self.structure_revision.saturating_add(1);
        true
    }
}
