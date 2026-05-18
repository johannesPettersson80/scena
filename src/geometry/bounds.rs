use crate::scene::Vec3;

use super::{Aabb, GeometryVertex};

impl Aabb {
    pub const fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn from_vertices(vertices: &[GeometryVertex]) -> Option<Self> {
        let first = vertices.first()?;
        let mut min = first.position;
        let mut max = first.position;
        for vertex in &vertices[1..] {
            min.x = min.x.min(vertex.position.x);
            min.y = min.y.min(vertex.position.y);
            min.z = min.z.min(vertex.position.z);
            max.x = max.x.max(vertex.position.x);
            max.y = max.y.max(vertex.position.y);
            max.z = max.z.max(vertex.position.z);
        }
        Some(Self { min, max })
    }

    pub fn contains(&self, point: Vec3) -> bool {
        point.x >= self.min.x
            && point.y >= self.min.y
            && point.z >= self.min.z
            && point.x <= self.max.x
            && point.y <= self.max.y
            && point.z <= self.max.z
    }

    pub fn center(self) -> Vec3 {
        Vec3::new(
            (self.min.x + self.max.x) * 0.5,
            (self.min.y + self.max.y) * 0.5,
            (self.min.z + self.max.z) * 0.5,
        )
    }

    pub fn half_extent(self) -> Vec3 {
        Vec3::new(
            (self.max.x - self.min.x).abs() * 0.5,
            (self.max.y - self.min.y).abs() * 0.5,
            (self.max.z - self.min.z).abs() * 0.5,
        )
    }

    pub fn bounding_sphere_radius(self) -> f32 {
        let half = self.half_extent();
        (half.x * half.x + half.y * half.y + half.z * half.z).sqrt()
    }

    pub fn union(self, other: Self) -> Self {
        Self::new(
            Vec3::new(
                self.min.x.min(other.min.x),
                self.min.y.min(other.min.y),
                self.min.z.min(other.min.z),
            ),
            Vec3::new(
                self.max.x.max(other.max.x),
                self.max.y.max(other.max.y),
                self.max.z.max(other.max.z),
            ),
        )
    }
}
