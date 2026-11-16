//! Vertex-buffer slots of alpha-blended triangles on the retained GPU path.
//!
//! The GPU draws contiguous vertex ranges. A full prepare therefore lays the
//! blended triangles out in the vertex buffer in the order it draws them, and
//! the whole blended tail stays a handful of draw calls. When the retained
//! path re-sorts that tail for a new camera, the triangles keep their data but
//! have to trade places in the buffer; otherwise every triangle that left its
//! neighbours would become a draw call of its own. This module tracks which
//! triangle sits in which slot, hands a new draw order its slots, and says
//! which part of the region has to be written again.

use crate::render::prepare::PreparedPrimitive;

const VERTICES_PER_TRIANGLE: u32 = 3;

/// Which blended triangle each slot of the blended vertex region holds.
///
/// Triangles are named by their collection-order number
/// ([`PreparedPrimitive::blend_sequence`]), which is stable across re-sorts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(in crate::render) struct BlendVertexSlots {
    first_vertex: u32,
    uploaded: Vec<u32>,
}

/// A draw order placed into the blended vertex region.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::render) struct BlendSlotAssignment {
    order: Vec<u32>,
    unchanged: usize,
}

impl BlendVertexSlots {
    /// The region as a full prepare uploads it: the blended triangles of the
    /// retained template, in template order.
    pub(in crate::render) fn from_template(retained: &[PreparedPrimitive]) -> Self {
        let mut blended = retained
            .iter()
            .filter(|primitive| primitive.gpu_triangle_path())
            .filter_map(|primitive| Some((primitive.blend_sequence()?, primitive)))
            .peekable();
        let first_vertex = blended
            .peek()
            .map_or(0, |(_, primitive)| primitive.original_vertex_offset());
        Self {
            first_vertex,
            uploaded: blended.map(|(sequence, _)| sequence).collect(),
        }
    }

    /// Gives the blended triangles of `draw_list` consecutive slots in draw
    /// order, so they batch the way a full prepare of that order would.
    pub(in crate::render) fn assign(
        &self,
        draw_list: &mut [PreparedPrimitive],
    ) -> BlendSlotAssignment {
        let mut order = Vec::new();
        for primitive in draw_list
            .iter_mut()
            .filter(|primitive| primitive.gpu_triangle_path())
        {
            let Some(sequence) = primitive.blend_sequence() else {
                continue;
            };
            let slot = order.len() as u32;
            primitive.set_vertex_offset(
                self.first_vertex
                    .saturating_add(slot.saturating_mul(VERTICES_PER_TRIANGLE)),
            );
            order.push(sequence);
        }
        let unchanged = self
            .uploaded
            .iter()
            .zip(&order)
            .take_while(|(uploaded, wanted)| uploaded == wanted)
            .count();
        BlendSlotAssignment { order, unchanged }
    }

    /// Records that the stale part of `assignment` reached the vertex buffer.
    /// Slots past the assignment keep the triangles they already held.
    pub(in crate::render) fn record_upload(&mut self, assignment: &BlendSlotAssignment) {
        let order = &assignment.order;
        if self.uploaded.len() < order.len() {
            self.uploaded.resize(order.len(), u32::MAX);
        }
        self.uploaded[..order.len()].copy_from_slice(order);
    }
}

impl BlendSlotAssignment {
    /// The triangles whose slot holds some other triangle and must be written
    /// to the vertex buffer. Blended triangles close a draw list, and slots
    /// only go stale from the first difference on, so this is a tail of
    /// `draw_list`.
    pub(in crate::render) fn stale<'a>(
        &self,
        draw_list: &'a [PreparedPrimitive],
    ) -> &'a [PreparedPrimitive] {
        let stale = self.order.len() - self.unchanged;
        &draw_list[draw_list.len().saturating_sub(stale)..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Primitive;
    use crate::material::Color;

    /// A template of two opaque triangles followed by `blended` blended ones.
    fn template(blended: u32) -> Vec<PreparedPrimitive> {
        (0..2 + blended)
            .map(|index| {
                let mut primitive =
                    PreparedPrimitive::new(Primitive::unlit_triangle(), None, Color::WHITE)
                        .with_original_vertex_offset(index * VERTICES_PER_TRIANGLE);
                if index >= 2 {
                    primitive.set_blend_sequence(index - 2);
                }
                primitive
            })
            .collect()
    }

    /// The draw list a re-sort would produce: both opaque triangles, then the
    /// blended triangles with these collection-order numbers.
    fn draw_list(template: &[PreparedPrimitive], blended_order: &[u32]) -> Vec<PreparedPrimitive> {
        let mut list = template[..2].to_vec();
        list.extend(
            blended_order
                .iter()
                .map(|sequence| template[2 + *sequence as usize].clone()),
        );
        list
    }

    fn slots_of(list: &[PreparedPrimitive]) -> Vec<u32> {
        list.iter()
            .filter(|primitive| primitive.blend_sequence().is_some())
            .map(PreparedPrimitive::original_vertex_offset)
            .collect()
    }

    #[test]
    fn a_re_sorted_tail_gets_consecutive_slots_in_draw_order() {
        let template = template(4);
        let slots = BlendVertexSlots::from_template(&template);
        let mut list = draw_list(&template, &[3, 2, 1, 0]);

        let assignment = slots.assign(&mut list);

        assert_eq!(
            slots_of(&list),
            [6, 9, 12, 15],
            "blended triangles must occupy consecutive slots in the order they are drawn"
        );
        assert_eq!(
            list[..2]
                .iter()
                .map(PreparedPrimitive::original_vertex_offset)
                .collect::<Vec<_>>(),
            [0, 3],
            "opaque triangles keep their slots"
        );
        assert_eq!(assignment.stale(&list).len(), 4);
    }

    #[test]
    fn only_slots_from_the_first_difference_on_are_written_again() {
        let template = template(4);
        let mut slots = BlendVertexSlots::from_template(&template);

        let mut unchanged = draw_list(&template, &[0, 1, 2, 3]);
        assert!(slots.assign(&mut unchanged).stale(&unchanged).is_empty());

        let mut swapped = draw_list(&template, &[0, 1, 3, 2]);
        let assignment = slots.assign(&mut swapped);
        assert_eq!(
            assignment
                .stale(&swapped)
                .iter()
                .map(|primitive| primitive.blend_sequence())
                .collect::<Vec<_>>(),
            [Some(3), Some(2)]
        );

        slots.record_upload(&assignment);
        let mut again = draw_list(&template, &[0, 1, 3, 2]);
        assert!(
            slots.assign(&mut again).stale(&again).is_empty(),
            "an order that is already uploaded needs no write"
        );
    }

    #[test]
    fn slots_past_a_culled_draw_list_keep_what_they_held() {
        let template = template(4);
        let mut slots = BlendVertexSlots::from_template(&template);

        // Two triangles are culled; the two that remain already lead the
        // region, so nothing is written and the rest stays known.
        let mut culled = draw_list(&template, &[0, 1]);
        let assignment = slots.assign(&mut culled);
        assert!(assignment.stale(&culled).is_empty());
        slots.record_upload(&assignment);

        let mut all = draw_list(&template, &[0, 1, 2, 3]);
        assert!(
            slots.assign(&mut all).stale(&all).is_empty(),
            "slots the shorter draw list did not use still hold their triangles"
        );
    }

    #[test]
    fn a_template_without_blended_triangles_assigns_nothing() {
        let template = template(0);
        let slots = BlendVertexSlots::from_template(&template);
        let mut list = template.clone();

        let assignment = slots.assign(&mut list);

        assert!(assignment.stale(&list).is_empty());
        assert_eq!(list, template);
    }
}
