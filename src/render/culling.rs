use crate::geometry::Primitive;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct CulledPrimitives {
    pub(super) visible: Vec<Primitive>,
    pub(super) culled: u64,
}

pub(super) fn cull_cpu_frustum(primitives: Vec<Primitive>) -> CulledPrimitives {
    let mut visible = Vec::with_capacity(primitives.len());
    let mut culled = 0_u64;
    for primitive in primitives {
        if outside_clip_box(&primitive) {
            culled = culled.saturating_add(1);
        } else {
            visible.push(primitive);
        }
    }
    CulledPrimitives { visible, culled }
}

fn outside_clip_box(primitive: &Primitive) -> bool {
    let vertices = primitive.vertices();
    all(vertices, |coordinate| coordinate.position.x < -1.0)
        || all(vertices, |coordinate| coordinate.position.x > 1.0)
        || all(vertices, |coordinate| coordinate.position.y < -1.0)
        || all(vertices, |coordinate| coordinate.position.y > 1.0)
        || all(vertices, |coordinate| coordinate.position.z < -1.0)
        || all(vertices, |coordinate| coordinate.position.z > 1.0)
}

fn all<T>(items: &[T; 3], predicate: impl Fn(&T) -> bool) -> bool {
    predicate(&items[0]) && predicate(&items[1]) && predicate(&items[2])
}

#[cfg(test)]
mod tests {
    use crate::geometry::{Primitive, Vertex};
    use crate::material::Color;
    use crate::scene::Vec3;

    use super::cull_cpu_frustum;

    #[test]
    fn cpu_frustum_culling_counts_only_fully_outside_primitives() {
        let visible = Primitive::unlit_triangle();
        let culled = Primitive::triangle([
            vertex(2.0, -0.5, 0.0),
            vertex(3.0, -0.5, 0.0),
            vertex(2.5, 0.5, 0.0),
        ]);

        let result = cull_cpu_frustum(vec![visible.clone(), culled]);

        assert_eq!(result.visible, vec![visible]);
        assert_eq!(result.culled, 1);
    }

    fn vertex(x: f32, y: f32, z: f32) -> Vertex {
        Vertex {
            position: Vec3::new(x, y, z),
            color: Color::WHITE,
        }
    }
}
