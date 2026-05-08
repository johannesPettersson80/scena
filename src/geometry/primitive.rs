use crate::material::Color;
use crate::scene::Vec3;

use super::{Primitive, PrimitiveVertexAttributes, Vertex};

impl Primitive {
    pub fn triangle(vertices: [Vertex; 3]) -> Self {
        Self {
            vertices,
            attributes: [PrimitiveVertexAttributes::default(); 3],
            render_material_slot: 0,
        }
    }

    pub(crate) fn triangle_with_attributes(
        vertices: [Vertex; 3],
        attributes: [PrimitiveVertexAttributes; 3],
    ) -> Self {
        Self {
            vertices,
            attributes,
            render_material_slot: 0,
        }
    }

    pub fn unlit_triangle() -> Self {
        Self::triangle([
            Vertex {
                position: Vec3::new(-0.6, -0.5, 0.0),
                color: Color::from_linear_rgb(1.0, 0.2, 0.1),
            },
            Vertex {
                position: Vec3::new(0.6, -0.5, 0.0),
                color: Color::from_linear_rgb(0.1, 0.8, 0.2),
            },
            Vertex {
                position: Vec3::new(0.0, 0.6, 0.0),
                color: Color::from_linear_rgb(0.1, 0.3, 1.0),
            },
        ])
    }

    pub fn vertices(&self) -> &[Vertex; 3] {
        &self.vertices
    }

    pub(crate) fn vertex_attributes(&self) -> &[PrimitiveVertexAttributes; 3] {
        &self.attributes
    }

    pub(crate) fn with_render_material_slot(mut self, slot: u32) -> Self {
        self.render_material_slot = slot;
        self
    }

    pub(crate) fn render_material_slot(&self) -> u32 {
        self.render_material_slot
    }
}
