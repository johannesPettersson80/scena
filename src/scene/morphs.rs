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
        let weights = weights.into();
        // R04: geometry lives in `Assets`, so the authored entry point cannot
        // see the node's morph-target count. It validates what it can reach —
        // finiteness, and the width already established for this node by
        // import — while `GeometryDesc::morph_weight_width_matches` fails
        // closed against the true target count at consumption time.
        if weights.iter().any(|weight| !weight.is_finite()) {
            return Err(LookupError::InvalidMorphWeights {
                node,
                reason: "morph weights must all be finite",
            });
        }
        if let Some(established) = self.morph_weights.get(&node)
            && established.len() != weights.len()
        {
            return Err(LookupError::MorphWeightWidthMismatch {
                node,
                expected: established.len(),
                supplied: weights.len(),
            });
        }
        self.set_morph_weights_unchecked(node, weights);
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
