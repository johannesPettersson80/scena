use std::collections::BTreeSet;
use std::sync::Arc;

use crate::geometry::{PrimitiveVertexAttributes, Vertex};

use super::PreparedPrimitive;
use crate::render::prepare::transforms::invert_matrix4;

#[derive(Debug, Clone, PartialEq)]
pub(in crate::render) struct PreparedDrawTransform {
    pub(super) world_from_model: [f32; 16],
    pub(super) normal_from_model: [f32; 16],
    pub(super) world_to_model: Option<[f32; 16]>,
    pub(super) model_from_normal: Option<[f32; 16]>,
}

impl PreparedDrawTransform {
    pub(in crate::render) fn shared(
        world_from_model: [f32; 16],
        normal_from_model: [f32; 16],
    ) -> Arc<Self> {
        Arc::new(Self {
            world_to_model: invert_matrix4(&world_from_model),
            model_from_normal: invert_matrix4(&normal_from_model),
            world_from_model,
            normal_from_model,
        })
    }

    pub(super) fn identity() -> Arc<Self> {
        let matrix = crate::render::prepare::transforms::identity_matrix4();
        Self::shared(matrix, matrix)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::render) struct PreparedModelVertex {
    pub(in crate::render) vertex: Vertex,
    pub(in crate::render) attributes: PrimitiveVertexAttributes,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::render) struct PreparedGeometryStorageMetrics {
    pub(in crate::render) triangle_count: u64,
    pub(in crate::render) model_vertex_buffer_count: u64,
    pub(in crate::render) model_vertex_bytes: u64,
    pub(in crate::render) unique_draw_transforms: u64,
    pub(in crate::render) draw_transform_bytes: u64,
    pub(in crate::render) triangle_reference_bytes: u64,
}

pub(in crate::render) fn share_model_space_vertex_buffer(
    primitives: &mut [PreparedPrimitive],
) -> PreparedGeometryStorageMetrics {
    if primitives.is_empty() {
        return PreparedGeometryStorageMetrics::default();
    }
    let mut model_vertices = Vec::with_capacity(primitives.len().saturating_mul(3));
    let mut transforms = BTreeSet::new();
    for primitive in primitives.iter() {
        transforms.insert(Arc::as_ptr(&primitive.draw_transform) as usize);
        let position_inverse = primitive.world_to_model();
        let normal_inverse = primitive.model_from_normal();
        model_vertices.extend(
            primitive
                .vertices()
                .iter()
                .zip(primitive.vertex_attributes())
                .map(|(vertex, attributes)| PreparedModelVertex {
                    vertex: Vertex {
                        position: position_inverse.map_or(vertex.position, |inverse| {
                            crate::render::prepare::transforms::unbake_position_to_model_space(
                                vertex.position,
                                &inverse,
                            )
                        }),
                        color: vertex.color,
                    },
                    attributes: PrimitiveVertexAttributes {
                        normal: normal_inverse.map_or(attributes.normal, |inverse| {
                            crate::render::prepare::transforms::unbake_normal_to_model_space(
                                attributes.normal,
                                &inverse,
                            )
                        }),
                        tex_coord0: attributes.tex_coord0,
                        tangent: normal_inverse.map_or(attributes.tangent, |inverse| {
                            crate::render::prepare::transforms::unbake_normal_to_model_space(
                                attributes.tangent,
                                &inverse,
                            )
                        }),
                        tangent_handedness: attributes.tangent_handedness,
                        shadow_visibility: attributes.shadow_visibility,
                        ambient_visibility: attributes.ambient_visibility,
                    },
                }),
        );
    }
    let model_vertices: Arc<[PreparedModelVertex]> = Arc::from(model_vertices);
    for (index, primitive) in primitives.iter_mut().enumerate() {
        primitive.model_vertex_offset = index.saturating_mul(3);
        primitive.model_vertices = Some(Arc::clone(&model_vertices));
    }
    let triangle_count = primitives.len() as u64;
    let model_vertex_bytes = (model_vertices.len() as u64)
        .saturating_mul(std::mem::size_of::<PreparedModelVertex>() as u64);
    PreparedGeometryStorageMetrics {
        triangle_count,
        model_vertex_buffer_count: 1,
        model_vertex_bytes,
        unique_draw_transforms: transforms.len() as u64,
        draw_transform_bytes: (transforms.len() as u64)
            .saturating_mul(std::mem::size_of::<PreparedDrawTransform>() as u64),
        triangle_reference_bytes: triangle_count.saturating_mul(
            (std::mem::size_of::<Arc<PreparedDrawTransform>>()
                + std::mem::size_of::<Arc<[PreparedModelVertex]>>()) as u64,
        ),
    }
}
