use crate::{Color, GeometryDesc, GeometryTopology, GeometryVertex, Vec3};

pub(super) fn contact_shadow_geometry(radius_x: f32, radius_z: f32, opacity: f32) -> GeometryDesc {
    const SEGMENTS: usize = 32;
    let mut vertices = Vec::with_capacity(SEGMENTS + 1);
    let mut colors = Vec::with_capacity(SEGMENTS + 1);
    vertices.push(GeometryVertex {
        position: Vec3::ZERO,
        normal: Vec3::Y,
    });
    colors.push(Color::from_linear_rgba(
        0.0,
        0.0,
        0.0,
        opacity.clamp(0.0, 0.45),
    ));
    for segment in 0..SEGMENTS {
        let angle = segment as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
        vertices.push(GeometryVertex {
            position: Vec3::new(angle.cos() * radius_x, 0.0, angle.sin() * radius_z),
            normal: Vec3::Y,
        });
        colors.push(Color::TRANSPARENT);
    }
    let mut indices = Vec::with_capacity(SEGMENTS * 3);
    for segment in 0..SEGMENTS {
        let current = segment as u32 + 1;
        let next = (segment + 1) as u32 % SEGMENTS as u32 + 1;
        indices.extend_from_slice(&[0, next, current]);
    }
    GeometryDesc::try_new_with_vertex_colors(GeometryTopology::Triangles, vertices, indices, colors)
        .expect("generated contact-shadow geometry is valid")
}

/// Build a floor whose area-light visibility can resolve a local cast shadow.
///
/// Area-light visibility is prepared per vertex and interpolated across each
/// triangle. A four-corner plane therefore samples visibility only at the
/// distant corners and cannot shadow the product footprint. The power-spaced
/// grid keeps most vertices near the subject while still covering the complete
/// camera-derived floor extent.
pub(super) fn photographic_floor_geometry(extent: f32) -> GeometryDesc {
    const SUBDIVISIONS: usize = 32;
    const CENTER_DENSITY_EXPONENT: f32 = 1.65;

    let extent = extent.abs().max(1.0e-4);
    let row = SUBDIVISIONS + 1;
    let mut vertices = Vec::with_capacity(row * row);
    let mut tex_coords = Vec::with_capacity(row * row);
    for z in 0..=SUBDIVISIONS {
        let z = floor_grid_coordinate(z, SUBDIVISIONS, extent, CENTER_DENSITY_EXPONENT);
        for x in 0..=SUBDIVISIONS {
            let x = floor_grid_coordinate(x, SUBDIVISIONS, extent, CENTER_DENSITY_EXPONENT);
            vertices.push(GeometryVertex {
                position: Vec3::new(x, 0.0, z),
                normal: Vec3::Y,
            });
            tex_coords.push([x / (extent * 2.0) + 0.5, z / (extent * 2.0) + 0.5]);
        }
    }

    let mut indices = Vec::with_capacity(SUBDIVISIONS * SUBDIVISIONS * 6);
    for z in 0..SUBDIVISIONS {
        for x in 0..SUBDIVISIONS {
            let top_left = (z * row + x) as u32;
            let top_right = top_left + 1;
            let bottom_left = top_left + row as u32;
            let bottom_right = bottom_left + 1;
            indices.extend_from_slice(&[
                top_left,
                bottom_right,
                top_right,
                top_left,
                bottom_left,
                bottom_right,
            ]);
        }
    }

    GeometryDesc::try_new_with_vertex_colors_and_tex_coords(
        GeometryTopology::Triangles,
        vertices.clone(),
        indices,
        vec![Color::WHITE; vertices.len()],
        tex_coords,
    )
    .expect("generated photographic floor geometry is valid")
}

fn floor_grid_coordinate(index: usize, subdivisions: usize, extent: f32, exponent: f32) -> f32 {
    let unit = index as f32 / subdivisions as f32 * 2.0 - 1.0;
    unit.signum() * unit.abs().powf(exponent) * extent
}
