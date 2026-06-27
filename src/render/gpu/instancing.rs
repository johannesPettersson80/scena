use crate::material::Color;
use crate::render::prepare::{PreparedInstanceRecord, PreparedInstanceSet};

use super::vertices::{DrawUniformValue, draw_uniform_index_for, draw_uniform_value_for};

pub(super) const INSTANCE_BYTE_LEN: usize = 36 * std::mem::size_of::<f32>();
pub(super) const INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 9] = [
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 0,
        shader_location: 6,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 4 * std::mem::size_of::<f32>() as u64,
        shader_location: 7,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 8 * std::mem::size_of::<f32>() as u64,
        shader_location: 8,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 12 * std::mem::size_of::<f32>() as u64,
        shader_location: 9,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 16 * std::mem::size_of::<f32>() as u64,
        shader_location: 10,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 20 * std::mem::size_of::<f32>() as u64,
        shader_location: 11,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 24 * std::mem::size_of::<f32>() as u64,
        shader_location: 12,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 28 * std::mem::size_of::<f32>() as u64,
        shader_location: 13,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 32 * std::mem::size_of::<f32>() as u64,
        shader_location: 14,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct InstanceDrawBatch {
    pub(super) start_vertex: u32,
    pub(super) vertex_count: u32,
    pub(super) start_instance: u32,
    pub(super) instance_count: u32,
    pub(super) material_slot: u32,
    pub(super) draw_uniform_index: u32,
    pub(super) depth_prepass_eligible: bool,
    pub(super) double_sided: bool,
}

pub(super) fn encode_instance_draw_state(
    sets: &[PreparedInstanceSet],
    draw_uniforms: &mut Vec<DrawUniformValue>,
) -> (Vec<u8>, Vec<InstanceDrawBatch>, usize, u32) {
    let total_instances = sets.iter().map(|set| set.instances().len()).sum::<usize>();
    let mut bytes = Vec::with_capacity((total_instances + 1) * INSTANCE_BYTE_LEN);
    let mut encoded_ranges: Vec<(Vec<PreparedInstanceRecord>, u32)> = Vec::new();
    let mut batches: Vec<InstanceDrawBatch> = Vec::new();

    for set in sets {
        if set.instances().is_empty() || set.primitives().is_empty() {
            continue;
        }
        let start_instance =
            instance_record_range_start(&mut bytes, &mut encoded_ranges, set.instances());
        let instance_count = set.instances().len() as u32;
        for primitive in set.primitives() {
            let draw_uniform_index =
                draw_uniform_index_for(draw_uniforms, draw_uniform_value_for(primitive));
            let start_vertex = primitive.original_vertex_offset();
            let material_slot = primitive.render_material_slot();
            let depth_prepass_eligible = primitive.depth_prepass_eligible();
            let double_sided = primitive.double_sided();
            if let Some(last) = batches.last_mut()
                && last.material_slot == material_slot
                && last.draw_uniform_index == draw_uniform_index
                && last.depth_prepass_eligible == depth_prepass_eligible
                && last.double_sided == double_sided
                && last.start_instance == start_instance
                && last.instance_count == instance_count
                && last.start_vertex.saturating_add(last.vertex_count) == start_vertex
            {
                last.vertex_count = last.vertex_count.saturating_add(3);
                continue;
            }
            batches.push(InstanceDrawBatch {
                start_vertex,
                vertex_count: 3,
                start_instance,
                instance_count,
                material_slot,
                draw_uniform_index,
                depth_prepass_eligible,
                double_sided,
            });
        }
    }

    let identity_instance = (bytes.len() / INSTANCE_BYTE_LEN) as u32;
    encode_identity_instance(&mut bytes);
    (bytes, batches, total_instances, identity_instance)
}

fn instance_record_range_start(
    bytes: &mut Vec<u8>,
    encoded_ranges: &mut Vec<(Vec<PreparedInstanceRecord>, u32)>,
    records: &[PreparedInstanceRecord],
) -> u32 {
    if let Some((_records, start)) = encoded_ranges
        .iter()
        .find(|(encoded, _start)| encoded.as_slice() == records)
    {
        return *start;
    }
    let start = (bytes.len() / INSTANCE_BYTE_LEN) as u32;
    for record in records {
        encode_instance(bytes, *record);
    }
    encoded_ranges.push((records.to_vec(), start));
    start
}

fn encode_identity_instance(bytes: &mut Vec<u8>) {
    for value in identity_matrix4()
        .into_iter()
        .chain(identity_matrix4())
        .chain([
            Color::WHITE.r,
            Color::WHITE.g,
            Color::WHITE.b,
            Color::WHITE.a,
        ])
    {
        bytes.extend_from_slice(&value.to_ne_bytes());
    }
}

fn encode_instance(bytes: &mut Vec<u8>, record: PreparedInstanceRecord) {
    for value in record
        .world_from_model()
        .into_iter()
        .chain(record.normal_from_model())
        .chain([
            record.tint().r,
            record.tint().g,
            record.tint().b,
            record.tint().a,
        ])
    {
        bytes.extend_from_slice(&value.to_ne_bytes());
    }
}

const fn identity_matrix4() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}
