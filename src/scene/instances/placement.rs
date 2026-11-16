//! Exact placement of instances whose transform shears.
//!
//! An [`Instance`] is placed by a `Transform`, which cannot hold shear. An
//! instanced scene import flattens an asset hierarchy into instances, and a
//! rotated node under a non-uniformly scaled one flattens to exactly that. The
//! set therefore keeps the exact local matrix of every such instance next to
//! its `Transform`, and everything that draws, bounds, or picks an instance
//! asks the set for the instance's placement instead of reading the
//! `Transform` alone.

use super::super::WorldTransform;
use super::{Instance, InstanceSet};

impl InstanceSet {
    /// The exact placement of `instance` in the space of the set's node.
    pub(crate) fn placement(&self, instance: &Instance) -> WorldTransform {
        if self.sheared.is_empty() {
            return WorldTransform::from(instance.transform);
        }
        match self.sheared.get(&instance.id) {
            Some(matrix) => WorldTransform::from_matrix(*matrix),
            None => WorldTransform::from(instance.transform),
        }
    }

    /// Whether any visible instance cannot share the set's instanced draw.
    ///
    /// An instance that mirrors the geometry relative to its set would be
    /// drawn with the set's winding and culled from the wrong side. A sheared
    /// instance has a placement its `Transform` cannot express, and an
    /// instance record rebuilt from that `Transform` would drop the shear.
    /// Either sends the whole set to the per-instance bake, where each
    /// instance is placed by its exact matrix and its winding is restored.
    pub(crate) fn needs_per_instance_bake(&self) -> bool {
        self.has_sheared_instance()
            || self
                .instances
                .iter()
                .any(|instance| instance.visible && self.placement(instance).is_mirrored())
    }

    /// Whether a visible instance carries shear that its `Transform` cannot
    /// express.
    pub(super) fn has_sheared_instance(&self) -> bool {
        !self.sheared.is_empty()
            && self
                .instances
                .iter()
                .any(|instance| instance.visible && self.sheared.contains_key(&instance.id))
    }
}
