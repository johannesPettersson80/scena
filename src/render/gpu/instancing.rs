use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

use crate::material::Color;
use crate::render::prepare::{PreparedInstanceRecord, PreparedInstanceSet};
use crate::render::semantic_aov::GpuSemanticAttribution;

use super::vertices::{DrawUniformInterner, draw_uniform_value_for, palette_rgba_f32};

pub(super) const INSTANCE_BYTE_LEN: usize = 40 * std::mem::size_of::<f32>();
pub(super) const INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 10] = [
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
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 36 * std::mem::size_of::<f32>() as u64,
        shader_location: 15,
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
    pub(super) semantic_eligible: bool,
}

#[derive(Debug, Default)]
struct InstanceRangeIndex {
    buckets: HashMap<u64, Vec<InstanceRangeEntry>>,
}

#[derive(Debug)]
struct InstanceRangeEntry {
    records: Vec<PreparedInstanceRecord>,
    semantic_ids: Vec<u32>,
    start: u32,
}

#[derive(Debug, Default)]
struct InstanceRangeIndexMetrics {
    record_comparisons: u64,
}

pub(super) fn encode_instance_draw_state_with_semantics(
    sets: &[PreparedInstanceSet],
    draw_uniforms: &mut DrawUniformInterner,
    attribution: Option<&GpuSemanticAttribution>,
) -> (Vec<u8>, Vec<InstanceDrawBatch>, usize, u32) {
    let total_instances = sets.iter().map(|set| set.instances().len()).sum::<usize>();
    let mut bytes = Vec::with_capacity((total_instances + 1) * INSTANCE_BYTE_LEN);
    let mut encoded_ranges = InstanceRangeIndex::default();
    let mut index_metrics = InstanceRangeIndexMetrics::default();
    let mut batches: Vec<InstanceDrawBatch> = Vec::new();

    for set in sets {
        if set.instances().is_empty() || set.primitives().is_empty() {
            continue;
        }
        let semantic_ids = set
            .instances()
            .iter()
            .map(|record| {
                attribution
                    .map(|attribution| {
                        attribution.palette_index(
                            set.source_node(),
                            record.source_instance(),
                            set.primitives()
                                .iter()
                                .find(|primitive| primitive.semantic_opaque())
                                .and_then(|primitive| primitive.semantic_material()),
                        )
                    })
                    .unwrap_or(0)
            })
            .collect::<Vec<_>>();
        let start_instance = instance_record_range_start_profiled(
            &mut bytes,
            &mut encoded_ranges,
            set.instances(),
            &semantic_ids,
            &mut index_metrics,
        );
        let instance_count = set.instances().len() as u32;
        for primitive in set.primitives() {
            let draw_uniform_index = draw_uniforms.intern(draw_uniform_value_for(primitive));
            let start_vertex = primitive.original_vertex_offset();
            let material_slot = primitive.render_material_slot();
            let depth_prepass_eligible = primitive.depth_prepass_eligible();
            let double_sided = primitive.double_sided();
            let semantic_eligible = primitive.semantic_opaque();
            if let Some(last) = batches.last_mut()
                && last.material_slot == material_slot
                && last.draw_uniform_index == draw_uniform_index
                && last.depth_prepass_eligible == depth_prepass_eligible
                && last.double_sided == double_sided
                && last.semantic_eligible == semantic_eligible
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
                semantic_eligible,
            });
        }
    }

    let identity_instance = (bytes.len() / INSTANCE_BYTE_LEN) as u32;
    encode_identity_instance(&mut bytes);
    (bytes, batches, total_instances, identity_instance)
}

fn instance_record_range_start_profiled(
    bytes: &mut Vec<u8>,
    encoded_ranges: &mut InstanceRangeIndex,
    records: &[PreparedInstanceRecord],
    semantic_ids: &[u32],
    metrics: &mut InstanceRangeIndexMetrics,
) -> u32 {
    let hash = instance_records_hash(records, semantic_ids);
    if let Some(bucket) = encoded_ranges.buckets.get(&hash) {
        for entry in bucket {
            metrics.record_comparisons = metrics.record_comparisons.saturating_add(1);
            if instance_records_bitwise_eq(&entry.records, records)
                && entry.semantic_ids == semantic_ids
            {
                return entry.start;
            }
        }
    }
    let start = (bytes.len() / INSTANCE_BYTE_LEN) as u32;
    for (record, semantic_id) in records.iter().zip(semantic_ids) {
        encode_instance(bytes, *record, *semantic_id);
    }
    encoded_ranges
        .buckets
        .entry(hash)
        .or_default()
        .push(InstanceRangeEntry {
            records: records.to_vec(),
            semantic_ids: semantic_ids.to_vec(),
            start,
        });
    start
}

fn instance_records_hash(records: &[PreparedInstanceRecord], semantic_ids: &[u32]) -> u64 {
    let mut hasher = DefaultHasher::new();
    records.len().hash(&mut hasher);
    for record in records {
        hash_f32s(record.world_from_model(), &mut hasher);
        hash_f32s(record.normal_from_model(), &mut hasher);
        for component in [
            record.tint().r,
            record.tint().g,
            record.tint().b,
            record.tint().a,
        ] {
            component.to_bits().hash(&mut hasher);
        }
    }
    semantic_ids.hash(&mut hasher);
    hasher.finish()
}

fn hash_f32s(values: [f32; 16], hasher: &mut DefaultHasher) {
    for value in values {
        value.to_bits().hash(hasher);
    }
}

fn instance_records_bitwise_eq(
    left: &[PreparedInstanceRecord],
    right: &[PreparedInstanceRecord],
) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            matrices_bitwise_eq(left.world_from_model(), right.world_from_model())
                && matrices_bitwise_eq(left.normal_from_model(), right.normal_from_model())
                && [left.tint().r, left.tint().g, left.tint().b, left.tint().a]
                    .into_iter()
                    .zip([
                        right.tint().r,
                        right.tint().g,
                        right.tint().b,
                        right.tint().a,
                    ])
                    .all(|(left, right)| left.to_bits() == right.to_bits())
        })
}

fn matrices_bitwise_eq(left: [f32; 16], right: [f32; 16]) -> bool {
    left.into_iter()
        .zip(right)
        .all(|(left, right)| left.to_bits() == right.to_bits())
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
        .chain([0.0; 4])
    {
        bytes.extend_from_slice(&value.to_ne_bytes());
    }
}

fn encode_instance(bytes: &mut Vec<u8>, record: PreparedInstanceRecord, semantic_id: u32) {
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
        .chain(palette_rgba_f32(semantic_id))
    {
        bytes.extend_from_slice(&value.to_ne_bytes());
    }
}

const fn identity_matrix4() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pf10_instance_range_dedup_uses_a_stable_near_linear_index() {
        const RANGE_COUNT: usize = 2_048;
        let mut bytes = Vec::new();
        let mut index = InstanceRangeIndex::default();
        let mut metrics = InstanceRangeIndexMetrics::default();
        let ranges = (0..RANGE_COUNT)
            .map(|range| {
                let mut world = identity_matrix4();
                world[12] = range as f32;
                vec![PreparedInstanceRecord::auto_batched(
                    world,
                    identity_matrix4(),
                    Color::WHITE,
                )]
            })
            .collect::<Vec<_>>();

        for records in &ranges {
            instance_record_range_start_profiled(
                &mut bytes,
                &mut index,
                records,
                &[0],
                &mut metrics,
            );
        }
        let original_len = bytes.len();
        for records in &ranges {
            instance_record_range_start_profiled(
                &mut bytes,
                &mut index,
                records,
                &[0],
                &mut metrics,
            );
        }

        assert_eq!(
            bytes.len(),
            original_len,
            "repeated ranges must reuse encoded bytes"
        );
        assert!(
            metrics.record_comparisons <= (RANGE_COUNT * 2) as u64,
            "range indexing must be near-linear, got {} comparisons",
            metrics.record_comparisons
        );
    }
}
