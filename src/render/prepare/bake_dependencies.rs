//! What a baked template depends on beyond what the retained path re-derives.
//!
//! A full prepare bakes the scene into triangles. The retained prepare path
//! reuses those triangles after a change and re-derives per-draw matrices,
//! tints, visibility, culling and blend order. Anything else a bake folded
//! into the vertex data stays as it was baked, so the bake records here which
//! of its inputs that data came from. A retained prepare is only valid while
//! those inputs are unchanged.

use std::cell::Cell;

/// Inputs of a bake that live on in its vertex data.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::render) struct BakeDependencies {
    /// Vertex colours, tessellation density or billboard orientation were
    /// computed from the camera.
    pub(in crate::render) view: bool,
    /// Vertex data was computed from where things are in the world: lighting,
    /// shadow or occlusion baked per vertex, texture scale taken from a node's
    /// world scale, or triangles authored directly in world space.
    pub(in crate::render) placement: bool,
}

/// Collects [`BakeDependencies`] while a scene is baked.
#[derive(Debug, Default)]
pub(in crate::render) struct BakeDependencyRecorder {
    view: Cell<bool>,
    placement: Cell<bool>,
}

impl BakeDependencyRecorder {
    pub(in crate::render) fn depends_on_view(&self) {
        self.view.set(true);
    }

    pub(in crate::render) fn depends_on_placement(&self) {
        self.placement.set(true);
    }

    pub(in crate::render) fn finish(&self) -> BakeDependencies {
        BakeDependencies {
            view: self.view.get(),
            placement: self.placement.get(),
        }
    }
}
