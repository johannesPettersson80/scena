//! Verbs for a node the scene recipe builder is authoring.
//!
//! A recipe node is created at its parent's origin, unrotated and unscaled,
//! and a placement verb then gives it its local transform. These verbs solve
//! that transform in the exact frame of the parent.

use super::super::super::{Transform, Vec3, WorldTransform, fit_local_transform};
use super::{
    Placed, Placement, PlacementFrame, finite, non_invertible_frame, turned_local_in_parent,
};

impl PlacementFrame {
    /// Places a node at the world point `eye` looking at `target`, with the
    /// world scale as close to one as a uniform local scale allows.
    pub(crate) fn look_from(self, eye: Vec3, target: Vec3, up: Vec3) -> Placement {
        let desired = Transform::at(eye).looking_at(target, up);
        let Some(parent) = self.world else {
            return Ok(desired);
        };
        if let Some(parent) = parent.exact_trs()
            && let Some(local) = turned_local_in_parent(Some(parent), desired)
        {
            return local;
        }
        let volume = parent.matrix().matrix3.determinant().abs();
        if !volume.is_finite() || volume <= f32::MIN_POSITIVE {
            return Err(non_invertible_frame());
        }
        let placed = Placed {
            parent: Some(parent),
            local: Transform::IDENTITY,
            world: parent,
        };
        placed.aimed_from(eye, target, up, Vec3::splat(volume.cbrt().recip()))
    }

    /// The local transform that brings a node as close to the world
    /// placement `desired` as translation, rotation, and scale allow: the
    /// origin, the X axis direction, and every axis length exactly, and the
    /// Y axis in the plane `desired` spans with X and Y.
    pub(crate) fn fitted_to(self, desired: WorldTransform) -> Placement {
        let parent = self.world.unwrap_or(WorldTransform::IDENTITY);
        if let Some(desired) = desired.exact_trs() {
            let exact_parent = match self.world {
                None => Some(None),
                Some(parent) => parent.exact_trs().map(Some),
            };
            if let Some(local) =
                exact_parent.and_then(|parent| turned_local_in_parent(parent, desired))
            {
                return local;
            }
        }
        fit_local_transform(parent, desired)
            .ok_or_else(non_invertible_frame)
            .and_then(finite)
    }

    /// Moves a node that sits unrotated and unscaled at the frame origin by
    /// `world_delta`.
    pub(crate) fn moved_from_origin(self, world_delta: Vec3) -> Placement {
        self.placed(Transform::IDENTITY).moved_by(world_delta)
    }
}
