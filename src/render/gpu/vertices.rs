use crate::geometry::{Primitive, PrimitiveVertexAttributes, Vertex};
use crate::render::prepare::transforms::{
    invert_matrix4, unbake_normal_to_model_space, unbake_position_to_model_space,
};

pub(super) const VERTEX_BYTE_LEN: usize = 17 * std::mem::size_of::<f32>();
pub(super) const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 6] = [
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: 0,
        shader_location: 0,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 3 * std::mem::size_of::<f32>() as u64,
        shader_location: 1,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: 7 * std::mem::size_of::<f32>() as u64,
        shader_location: 2,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x2,
        offset: 10 * std::mem::size_of::<f32>() as u64,
        shader_location: 3,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 12 * std::mem::size_of::<f32>() as u64,
        shader_location: 4,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32,
        offset: 16 * std::mem::size_of::<f32>() as u64,
        shader_location: 5,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PrimitiveDrawBatch {
    pub(super) start_vertex: u32,
    pub(super) vertex_count: u32,
    pub(super) material_slot: u32,
    pub(super) draw_uniform_index: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct DrawUniformValue {
    pub(super) world_from_model: [f32; 16],
    pub(super) normal_from_model: [f32; 16],
}

/// Writes the prepared primitives as MODEL-SPACE vertex bytes for GPU upload.
/// CPU consumers (picking, culling, CPU rasterization, shadow occluders) read
/// the prepared primitives directly with world-baked vertices; the GPU
/// upload path recovers model-space by applying the inverse of the matrix
/// that produced the bake. The shader then applies the per-draw
/// `world_from_model` from the dynamic-offset draw uniform, yielding the
/// same world-space position as the CPU path. Phase 1A.2 closure for
/// scena-wgpu-architect F2.
pub(super) fn encode_vertices(primitives: &[Primitive]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(primitives.len() * 3 * VERTEX_BYTE_LEN);
    for primitive in primitives {
        let world_from_model = primitive.world_from_model();
        let normal_from_model = primitive.normal_from_model();
        let position_inverse = invert_matrix4(&world_from_model);
        let normal_inverse = invert_matrix4(&normal_from_model);
        for (vertex, attributes) in primitive
            .vertices()
            .iter()
            .zip(primitive.vertex_attributes().iter())
        {
            // Recover model-space position via the inverse-transform-of-bake.
            // `position_inverse` is None only if `world_from_model` is
            // singular (zero scale on an axis), in which case we fall back
            // to the world-baked vertex which the GPU will then double-
            // transform against the singular forward matrix — pixels would
            // be degenerate either way, so we avoid panicking and let the
            // upstream culling stage decide.
            let model_vertex = match position_inverse {
                Some(inv) => Vertex {
                    position: unbake_position_to_model_space(vertex.position, &inv),
                    color: vertex.color,
                },
                None => *vertex,
            };
            let model_attributes = match normal_inverse {
                Some(inv) => PrimitiveVertexAttributes {
                    normal: unbake_normal_to_model_space(attributes.normal, &inv),
                    tex_coord0: attributes.tex_coord0,
                    tangent: unbake_normal_to_model_space(attributes.tangent, &inv),
                    tangent_handedness: attributes.tangent_handedness,
                    shadow_visibility: attributes.shadow_visibility,
                },
                None => *attributes,
            };
            encode_vertex(&mut bytes, model_vertex, model_attributes);
        }
    }
    bytes
}

pub(super) fn encode_draw_batches(
    primitives: &[Primitive],
) -> (Vec<PrimitiveDrawBatch>, Vec<DrawUniformValue>) {
    let mut batches: Vec<PrimitiveDrawBatch> = Vec::new();
    let mut draw_uniforms: Vec<DrawUniformValue> = Vec::new();
    for (index, primitive) in primitives.iter().enumerate() {
        let start_vertex = (index as u32).saturating_mul(3);
        let material_slot = primitive.render_material_slot();
        // F8 fallback: when world_from_model is singular (zero scale on an
        // axis), encode_vertices keeps the world-baked vertex unchanged. To
        // avoid the GPU shader re-multiplying that already-world-space vertex
        // against the singular forward matrix, upload identity in the draw
        // uniform for that primitive. Result: shader applies identity ×
        // world_baked = world_baked = correct (matches pre-1A.2 behavior for
        // degenerate primitives).
        let raw_world_from_model = primitive.world_from_model();
        let raw_normal_from_model = primitive.normal_from_model();
        let world_from_model = if invert_matrix4(&raw_world_from_model).is_some() {
            raw_world_from_model
        } else {
            identity_matrix4()
        };
        let normal_from_model = if invert_matrix4(&raw_normal_from_model).is_some() {
            raw_normal_from_model
        } else {
            identity_matrix4()
        };
        let draw_uniform_index = match draw_uniforms
            .iter()
            .position(|value| value.world_from_model == world_from_model)
        {
            Some(existing) => existing as u32,
            None => {
                draw_uniforms.push(DrawUniformValue {
                    world_from_model,
                    normal_from_model,
                });
                (draw_uniforms.len() - 1) as u32
            }
        };
        if let Some(last) = batches.last_mut()
            && last.material_slot == material_slot
            && last.draw_uniform_index == draw_uniform_index
            && last.start_vertex.saturating_add(last.vertex_count) == start_vertex
        {
            last.vertex_count = last.vertex_count.saturating_add(3);
            continue;
        }
        batches.push(PrimitiveDrawBatch {
            start_vertex,
            vertex_count: 3,
            material_slot,
            draw_uniform_index,
        });
    }
    if draw_uniforms.is_empty() {
        draw_uniforms.push(DrawUniformValue {
            world_from_model: identity_matrix4(),
            normal_from_model: identity_matrix4(),
        });
    }
    (batches, draw_uniforms)
}

const fn identity_matrix4() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

fn encode_vertex(bytes: &mut Vec<u8>, vertex: Vertex, attributes: PrimitiveVertexAttributes) {
    for value in [
        vertex.position.x,
        vertex.position.y,
        vertex.position.z,
        vertex.color.r,
        vertex.color.g,
        vertex.color.b,
        vertex.color.a,
        attributes.normal.x,
        attributes.normal.y,
        attributes.normal.z,
        attributes.tex_coord0[0],
        attributes.tex_coord0[1],
        attributes.tangent.x,
        attributes.tangent.y,
        attributes.tangent.z,
        attributes.tangent_handedness,
        attributes.shadow_visibility.clamp(0.0, 1.0),
    ] {
        bytes.extend_from_slice(&value.to_ne_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{PrimitiveVertexAttributes, Vertex};
    use crate::material::Color;
    use crate::scene::Vec3;

    #[test]
    fn gpu_vertex_stream_carries_normals_and_texcoord0() {
        assert_eq!(VERTEX_BYTE_LEN, 17 * std::mem::size_of::<f32>());
        assert!(
            VERTEX_ATTRIBUTES
                .iter()
                .any(|attribute| attribute.shader_location == 2
                    && attribute.format == wgpu::VertexFormat::Float32x3),
            "normal attribute must be passed to GPU shaders"
        );
        assert!(
            VERTEX_ATTRIBUTES
                .iter()
                .any(|attribute| attribute.shader_location == 3
                    && attribute.format == wgpu::VertexFormat::Float32x2),
            "TEXCOORD_0 must be passed to GPU shaders"
        );
        assert!(
            VERTEX_ATTRIBUTES
                .iter()
                .any(|attribute| attribute.shader_location == 4
                    && attribute.format == wgpu::VertexFormat::Float32x4),
            "tangent attribute must include handedness for tangent-space normal maps"
        );
        assert!(
            VERTEX_ATTRIBUTES
                .iter()
                .any(|attribute| attribute.shader_location == 5
                    && attribute.format == wgpu::VertexFormat::Float32),
            "prepared directional shadow visibility must be passed to GPU shaders"
        );

        let primitive = Primitive::triangle_with_attributes(
            [
                Vertex {
                    position: Vec3::new(1.0, 2.0, 3.0),
                    color: Color::from_linear_rgba(0.1, 0.2, 0.3, 0.4),
                },
                Vertex {
                    position: Vec3::new(4.0, 5.0, 6.0),
                    color: Color::from_linear_rgba(0.5, 0.6, 0.7, 0.8),
                },
                Vertex {
                    position: Vec3::new(7.0, 8.0, 9.0),
                    color: Color::from_linear_rgba(0.9, 1.0, 0.1, 0.2),
                },
            ],
            [
                PrimitiveVertexAttributes {
                    normal: Vec3::new(0.0, 1.0, 0.0),
                    tex_coord0: [0.25, 0.75],
                    tangent: Vec3::new(1.0, 0.0, 0.0),
                    tangent_handedness: -1.0,
                    shadow_visibility: 0.25,
                },
                PrimitiveVertexAttributes::default(),
                PrimitiveVertexAttributes::default(),
            ],
        );

        let bytes = encode_vertices(&[primitive]);
        let first_vertex = bytes[..VERTEX_BYTE_LEN]
            .chunks_exact(4)
            .map(|chunk| f32::from_ne_bytes(chunk.try_into().expect("f32 bytes")))
            .collect::<Vec<_>>();
        assert_eq!(
            first_vertex,
            vec![
                1.0, 2.0, 3.0, 0.1, 0.2, 0.3, 0.4, 0.0, 1.0, 0.0, 0.25, 0.75, 1.0, 0.0, 0.0, -1.0,
                0.25
            ]
        );
    }

    #[test]
    fn gpu_draw_batches_preserve_prepared_material_slots() {
        let first = Primitive::unlit_triangle().with_render_material_slot(1);
        let second = Primitive::unlit_triangle().with_render_material_slot(1);
        let third = Primitive::unlit_triangle().with_render_material_slot(2);

        let (batches, draw_uniforms) = encode_draw_batches(&[first, second, third]);

        assert_eq!(
            batches,
            vec![
                PrimitiveDrawBatch {
                    start_vertex: 0,
                    vertex_count: 6,
                    material_slot: 1,
                    draw_uniform_index: 0,
                },
                PrimitiveDrawBatch {
                    start_vertex: 6,
                    vertex_count: 3,
                    material_slot: 2,
                    draw_uniform_index: 0,
                },
            ],
            "GPU draw encoding must preserve prepared per-material slots instead of drawing \
             every primitive with one global material bind group"
        );
        assert_eq!(
            draw_uniforms.len(),
            1,
            "primitives sharing identity world_from_model collapse to a single draw-uniform slot",
        );
    }

    #[test]
    fn gpu_draw_batches_split_when_world_from_model_differs() {
        let first = Primitive::unlit_triangle().with_render_material_slot(1);
        let translated = Primitive::unlit_triangle()
            .with_render_material_slot(1)
            .with_world_from_model(
                [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 5.0, 0.0, 0.0, 1.0,
                ],
                [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
            );

        let (batches, draw_uniforms) = encode_draw_batches(&[first, translated]);

        assert_eq!(
            batches.len(),
            2,
            "primitives with distinct world_from_model must split into separate draw batches"
        );
        assert_eq!(
            batches[0].draw_uniform_index, 0,
            "the first batch maps to the first draw-uniform slot"
        );
        assert_eq!(
            batches[1].draw_uniform_index, 1,
            "the second batch indexes the new draw-uniform slot for the translated primitive"
        );
        assert_eq!(
            draw_uniforms.len(),
            2,
            "each unique world_from_model must produce its own draw-uniform slot"
        );
        assert_eq!(
            draw_uniforms[1].world_from_model[12], 5.0,
            "the second draw-uniform slot must record the translated world transform exactly, \
             not the per-vertex baked positions"
        );
    }
}
