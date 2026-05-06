//! Primitive meshes, generated helper geometry, technical lines, arrows, grids, and labels.

use crate::material::Color;
use crate::scene::Vec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    pub position: Vec3,
    pub color: Color,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Primitive {
    vertices: [Vertex; 3],
}

impl Primitive {
    pub fn triangle(vertices: [Vertex; 3]) -> Self {
        Self { vertices }
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
}
