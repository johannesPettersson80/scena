use crate::geometry::{Primitive, Vertex};

pub(super) const VERTEX_BYTE_LEN: usize = 7 * std::mem::size_of::<f32>();
pub(super) const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 2] = [
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
];

pub(super) fn encode_vertices(primitives: &[Primitive]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(primitives.len() * 3 * VERTEX_BYTE_LEN);
    for primitive in primitives {
        for vertex in primitive.vertices() {
            encode_vertex(&mut bytes, *vertex);
        }
    }
    bytes
}

fn encode_vertex(bytes: &mut Vec<u8>, vertex: Vertex) {
    for value in [
        vertex.position.x,
        vertex.position.y,
        vertex.position.z,
        vertex.color.r,
        vertex.color.g,
        vertex.color.b,
        vertex.color.a,
    ] {
        bytes.extend_from_slice(&value.to_ne_bytes());
    }
}
