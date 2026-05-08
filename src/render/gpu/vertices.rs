use crate::geometry::{Primitive, PrimitiveVertexAttributes, Vertex};

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
}

pub(super) fn encode_vertices(primitives: &[Primitive]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(primitives.len() * 3 * VERTEX_BYTE_LEN);
    for primitive in primitives {
        for (vertex, attributes) in primitive
            .vertices()
            .iter()
            .zip(primitive.vertex_attributes().iter())
        {
            encode_vertex(&mut bytes, *vertex, *attributes);
        }
    }
    bytes
}

pub(super) fn encode_draw_batches(primitives: &[Primitive]) -> Vec<PrimitiveDrawBatch> {
    let mut batches: Vec<PrimitiveDrawBatch> = Vec::new();
    for (index, primitive) in primitives.iter().enumerate() {
        let start_vertex = (index as u32).saturating_mul(3);
        let material_slot = primitive.render_material_slot();
        if let Some(last) = batches.last_mut()
            && last.material_slot == material_slot
            && last.start_vertex.saturating_add(last.vertex_count) == start_vertex
        {
            last.vertex_count = last.vertex_count.saturating_add(3);
            continue;
        }
        batches.push(PrimitiveDrawBatch {
            start_vertex,
            vertex_count: 3,
            material_slot,
        });
    }
    batches
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

        let batches = encode_draw_batches(&[first, second, third]);

        assert_eq!(
            batches,
            vec![
                PrimitiveDrawBatch {
                    start_vertex: 0,
                    vertex_count: 6,
                    material_slot: 1,
                },
                PrimitiveDrawBatch {
                    start_vertex: 6,
                    vertex_count: 3,
                    material_slot: 2,
                },
            ],
            "GPU draw encoding must preserve prepared per-material slots instead of drawing \
             every primitive with one global material bind group"
        );
    }
}
