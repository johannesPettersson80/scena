use crate::geometry::{Primitive, Vertex};
use crate::material::Color;
use crate::scene::{Particle, Scene, Transform, Vec3};

use super::super::camera::CameraProjection;
use super::transforms::transform_position;
use super::types::PreparedPrimitive;

pub(super) fn append_particle_primitives(
    scene: &Scene,
    camera_projection: Option<&CameraProjection>,
    origin_shift: Vec3,
    primitives: &mut Vec<PreparedPrimitive>,
) {
    let Some(camera_projection) = camera_projection else {
        return;
    };
    let (billboard_right, billboard_up) = camera_projection.billboard_axes();
    let particle_primitive_count = scene
        .particle_set_nodes()
        .map(|(_, particle_set, _)| particle_set.len().saturating_mul(2))
        .sum::<usize>();
    primitives.reserve(particle_primitive_count);
    for (node, particle_set, transform) in scene.particle_set_nodes() {
        for particle in particle_set.particles() {
            let Some((left_bottom, right_bottom, right_top, left_top)) = particle_corners(
                *particle,
                transform,
                camera_projection,
                billboard_right,
                billboard_up,
                origin_shift,
            ) else {
                continue;
            };
            let color = particle.color();
            primitives.push(
                PreparedPrimitive::new(
                    Primitive::triangle([
                        vertex(left_bottom, color),
                        vertex(right_bottom, color),
                        vertex(right_top, color),
                    ]),
                    Some(node),
                    Color::WHITE,
                )
                .without_semantic_attribution()
                .with_double_sided(true),
            );
            primitives.push(
                PreparedPrimitive::new(
                    Primitive::triangle([
                        vertex(left_bottom, color),
                        vertex(right_top, color),
                        vertex(left_top, color),
                    ]),
                    Some(node),
                    Color::WHITE,
                )
                .without_semantic_attribution()
                .with_double_sided(true),
            );
        }
    }
}

fn particle_corners(
    particle: Particle,
    transform: Transform,
    camera_projection: &CameraProjection,
    billboard_right: Vec3,
    billboard_up: Vec3,
    origin_shift: Vec3,
) -> Option<(Vec3, Vec3, Vec3, Vec3)> {
    let center = transform_position(particle.position(), transform, Vec3::ZERO);
    camera_projection.project(center)?;
    let world_units_per_pixel = camera_projection.world_units_per_pixel_at(center)?;
    let half_extent = world_units_per_pixel * particle.size_px() * 0.5;
    if !half_extent.is_finite() || half_extent <= 0.0 {
        return None;
    }
    let (sin, cos) = particle.rotation_radians().sin_cos();
    let x_axis = (billboard_right * cos + billboard_up * sin) * half_extent;
    let y_axis = (billboard_up * cos - billboard_right * sin) * half_extent;
    let center = center - origin_shift;
    Some((
        center - x_axis - y_axis,
        center + x_axis - y_axis,
        center + x_axis + y_axis,
        center - x_axis + y_axis,
    ))
}

fn vertex(position: Vec3, color: Color) -> Vertex {
    Vertex { position, color }
}
