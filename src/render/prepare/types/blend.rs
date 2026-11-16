//! Blend-order bookkeeping carried by a prepared triangle.

use super::PreparedPrimitive;

impl PreparedPrimitive {
    /// Collection-order number of an alpha-blended triangle, or `None` when
    /// the triangle is drawn before the blended tail.
    pub(in crate::render) const fn blend_sequence(&self) -> Option<u32> {
        self.blend_sequence
    }

    pub(in crate::render) fn set_blend_sequence(&mut self, sequence: u32) {
        self.blend_sequence = Some(sequence);
    }

    /// Points the triangle at another slot of the retained GPU vertex buffer.
    pub(in crate::render) fn set_vertex_offset(&mut self, vertex_offset: u32) {
        self.original_vertex_offset = vertex_offset;
    }
}
