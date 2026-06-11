use crate::render::prepare::{PreparedInstanceSet, PreparedPrimitive, PreparedStrokeSegment};

use super::instancing::{self, InstanceDrawBatch};
use super::strokes::StrokeDrawBatch;
use super::vertices::{self, DrawUniformValue, PrimitiveDrawBatch, encode_vertices};

pub(super) struct EncodedDrawResources {
    pub(super) draw_batches: Vec<PrimitiveDrawBatch>,
    pub(super) draw_uniforms: Vec<DrawUniformValue>,
    pub(super) instance_bytes: Vec<u8>,
    pub(super) instance_batches: Vec<InstanceDrawBatch>,
    pub(super) instance_count: usize,
    pub(super) identity_instance: u32,
    pub(super) stroke_batches: Vec<StrokeDrawBatch>,
}

pub(super) fn encode_retained_vertices(
    retained_primitives: &[PreparedPrimitive],
    retained_instances: &[PreparedInstanceSet],
) -> Vec<u8> {
    let retained_instance_primitives = retained_instances
        .iter()
        .flat_map(|set| set.primitives().iter().cloned());
    let all_retained_primitives = retained_primitives
        .iter()
        .cloned()
        .chain(retained_instance_primitives)
        .collect::<Vec<_>>();
    encode_vertices(&all_retained_primitives)
}

pub(super) fn retained_instance_buffer_capacity(
    retained_instances: &[PreparedInstanceSet],
) -> usize {
    retained_instances
        .iter()
        .map(|set| set.instances().len())
        .sum::<usize>()
        .saturating_add(1)
        .max(1)
}

pub(super) fn retained_draw_uniform_capacity(
    retained_primitives: &[PreparedPrimitive],
    retained_instances: &[PreparedInstanceSet],
    retained_stroke_count: usize,
    draw_uniform_count: usize,
) -> usize {
    retained_primitives
        .len()
        .saturating_add(
            retained_instances
                .iter()
                .map(|set| set.primitives().len())
                .sum::<usize>(),
        )
        .saturating_add(retained_stroke_count)
        .max(draw_uniform_count)
        .max(1)
}

pub(super) fn encode_draw_resources(
    draw_primitives: &[PreparedPrimitive],
    draw_instances: &[PreparedInstanceSet],
    draw_strokes: &[PreparedStrokeSegment],
) -> EncodedDrawResources {
    let (draw_batches, mut draw_uniforms) = vertices::encode_draw_batches(draw_primitives);
    let (instance_bytes, instance_batches, instance_count, identity_instance) =
        instancing::encode_instance_draw_state(draw_instances, &mut draw_uniforms);
    let stroke_batches = super::strokes::create_draw_batches(draw_strokes, &mut draw_uniforms);
    EncodedDrawResources {
        draw_batches,
        draw_uniforms,
        instance_bytes,
        instance_batches,
        instance_count,
        identity_instance,
        stroke_batches,
    }
}
