use crate::geometry::Primitive;

use super::camera::CameraProjection;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct CulledPrimitives {
    pub(super) visible: Vec<Primitive>,
    pub(super) culled: u64,
}

pub(super) fn cull_cpu_frustum(
    primitives: Vec<Primitive>,
    camera: Option<&CameraProjection>,
) -> CulledPrimitives {
    let mut visible = Vec::with_capacity(primitives.len());
    let mut culled = 0_u64;
    for primitive in primitives {
        if camera.is_some_and(|camera| outside_camera_clip_box(&primitive, camera)) {
            culled = culled.saturating_add(1);
        } else {
            visible.push(primitive);
        }
    }
    CulledPrimitives { visible, culled }
}

fn outside_camera_clip_box(primitive: &Primitive, camera: &CameraProjection) -> bool {
    let vertices = primitive.vertices();
    let projected = vertices.map(|vertex| camera.project(vertex.position));
    if projected.iter().all(Option::is_none) {
        return true;
    }
    let [Some(a), Some(b), Some(c)] = projected else {
        return false;
    };
    all(&[a, b, c], |coordinate| coordinate.ndc_x < -1.0)
        || all(&[a, b, c], |coordinate| coordinate.ndc_x > 1.0)
        || all(&[a, b, c], |coordinate| coordinate.ndc_y < -1.0)
        || all(&[a, b, c], |coordinate| coordinate.ndc_y > 1.0)
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
    fn cpu_frustum_culling_without_camera_keeps_world_space_primitives() {
        let visible = Primitive::unlit_triangle();
        let culled = Primitive::triangle([
            vertex(2.0, -0.5, 0.0),
            vertex(3.0, -0.5, 0.0),
            vertex(2.5, 0.5, 0.0),
        ]);

        let result = cull_cpu_frustum(vec![visible.clone(), culled.clone()], None);

        assert_eq!(result.visible, vec![visible, culled]);
        assert_eq!(result.culled, 0);
    }

    fn vertex(x: f32, y: f32, z: f32) -> Vertex {
        Vertex {
            position: Vec3::new(x, y, z),
            color: Color::WHITE,
        }
    }
}
