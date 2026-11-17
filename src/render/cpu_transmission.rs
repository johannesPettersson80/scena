use crate::material::Color;
use crate::scene::Vec3;

use super::RasterTarget;
use super::cpu::{CpuFrame, CpuTriangleClipInputs};
use super::cpu_color::{mix_color, multiply_color};
use super::cpu_geometry::{self, CpuProjectedPrimitive, CpuScreenTriangle, CpuScreenVertex};
use super::physical_transmission::{
    PhysicalTransmissionInputs, PreparedPhysicalTransmission, physical_transmission_color,
};
use super::prepare::PreparedPrimitive;

pub(super) fn draw_physical_transmission_cpu(
    cpu_frame: &mut CpuFrame<'_>,
    primitive: &PreparedPrimitive,
    projected: &CpuProjectedPrimitive,
    scene_color_frame: &[Color],
    context: CpuTriangleClipInputs<'_>,
) -> u64 {
    let Some(transmission) = primitive.material_transmission() else {
        return 0;
    };
    let mut pixels_encoded = 0_u64;
    for triangle in projected.triangles() {
        pixels_encoded = pixels_encoded.saturating_add(draw_projected_physical_transmission_cpu(
            cpu_frame,
            primitive,
            *triangle,
            transmission,
            scene_color_frame,
            context,
        ));
    }
    pixels_encoded
}

#[allow(clippy::too_many_arguments)]
fn draw_projected_physical_transmission_cpu(
    cpu_frame: &mut CpuFrame<'_>,
    primitive: &PreparedPrimitive,
    triangle: CpuScreenTriangle,
    transmission: PreparedPhysicalTransmission,
    scene_color_frame: &[Color],
    context: CpuTriangleClipInputs<'_>,
) -> u64 {
    let CpuTriangleClipInputs {
        clipping_planes,
        section_box,
        camera,
    } = context;
    let vertices = triangle.vertices();
    let [a, b, c] = vertices;

    let min_x = a.x.min(b.x).min(c.x).floor().max(0.0) as u32;
    let max_x =
        a.x.max(b.x)
            .max(c.x)
            .ceil()
            .min(cpu_frame.target.width as f32 - 1.0) as u32;
    let min_y =
        a.y.min(b.y)
            .min(c.y)
            .floor()
            .max(cpu_frame.row_start as f32) as u32;
    let max_y =
        a.y.max(b.y)
            .max(c.y)
            .ceil()
            .min(cpu_frame.row_end().saturating_sub(1) as f32) as u32;
    if min_y > max_y {
        return 0;
    }

    let area = cpu_geometry::edge(a, b, c.x, c.y);
    if area.abs() <= f32::EPSILON {
        return 0;
    }
    if !primitive.double_sided() && area < 0.0 {
        return 0;
    }
    let inverse_area = area.recip();
    let mut pixels_encoded = 0_u64;
    // World units per model unit along each model axis: the material
    // thickness is authored in model units.
    let world_from_model = primitive.world_from_model();
    let column_length = |column: usize| {
        Vec3::new(
            world_from_model[column * 4],
            world_from_model[column * 4 + 1],
            world_from_model[column * 4 + 2],
        )
        .length()
    };
    let model_scale = Vec3::new(column_length(0), column_length(1), column_length(2));

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let w0 = cpu_geometry::edge(b, c, px, py) * inverse_area;
            let w1 = cpu_geometry::edge(c, a, px, py) * inverse_area;
            let w2 = cpu_geometry::edge(a, b, px, py) * inverse_area;
            if !cpu_geometry::barycentric_sample_is_inside([w0, w1, w2]) {
                continue;
            }
            let weights = cpu_geometry::perspective_weights(camera, vertices, [w0, w1, w2]);
            let position =
                cpu_geometry::weighted_vec3([a.position, b.position, c.position], weights);
            if cpu_geometry::point_is_clipped(position, clipping_planes, section_box) {
                continue;
            }
            let depth = mix_depth(vertices, [w0, w1, w2]);
            if !depth.is_finite() {
                continue;
            }
            let Some(pixel_index) = cpu_frame.local_pixel_index(x, y) else {
                continue;
            };
            if depth > cpu_frame.depth_frame[pixel_index] + f32::EPSILON {
                continue;
            }

            let tint = primitive.tint();
            let surface_color = multiply_color(mix_color(vertices, weights), tint);
            let normal = cpu_geometry::weighted_vec3(
                [
                    a.attributes.normal,
                    b.attributes.normal,
                    c.attributes.normal,
                ],
                weights,
            );
            let view = camera.camera_position() - position;
            let target = cpu_frame.target;
            let base_color = transmission.base_color();
            let transmitted = physical_transmission_color(
                transmission,
                PhysicalTransmissionInputs {
                    frag_coord: [px, py],
                    viewport: [target.width as f32, target.height as f32],
                    position,
                    normal,
                    view,
                    model_scale,
                    tint: Vec3::new(
                        base_color.x * tint.r,
                        base_color.y * tint.g,
                        base_color.z * tint.b,
                    ),
                    surface_rgb: Vec3::new(surface_color.r, surface_color.g, surface_color.b),
                },
                |world| scene_color_uv(camera, target, world),
                |uv| sample_post_scene_color(scene_color_frame, target, uv),
            );
            let final_color =
                Color::from_linear_rgba(transmitted.x, transmitted.y, transmitted.z, 1.0);
            cpu_frame.linear_frame[pixel_index] = final_color;
            cpu_frame.depth_frame[pixel_index] = depth;
            pixels_encoded = pixels_encoded.saturating_add(1);
        }
    }
    pixels_encoded
}

/// Scene-colour texture coordinates of a world position, or `None` outside
/// the view volume. The CPU rasterizer spreads clip space over
/// `0..=width - 1` pixels and samples pixel `x` at `x + 0.5`, so the same
/// mapping is used here: a surface point then reads the pixel it covers.
fn scene_color_uv(
    camera: &super::camera::CameraProjection,
    target: RasterTarget,
    world: Vec3,
) -> Option<[f32; 2]> {
    let projected = camera.project(world)?;
    let (width, height) = (target.width as f32, target.height as f32);
    Some([
        (projected.ndc_x * 0.5 + 0.5) * (width - 1.0) / width,
        (0.5 - projected.ndc_y * 0.5) * (height - 1.0) / height,
    ])
}

/// Bilinear read of the scene colour with clamp-to-edge addressing. Texel
/// centres sit at half-integer pixel coordinates, as on the GPU, so the
/// coordinates of a fragment read exactly the pixel behind it.
fn sample_post_scene_color(frame: &[Color], target: RasterTarget, uv: [f32; 2]) -> Vec3 {
    let x = (uv[0] * target.width as f32 - 0.5).clamp(0.0, target.width.saturating_sub(1) as f32);
    let y = (uv[1] * target.height as f32 - 0.5).clamp(0.0, target.height.saturating_sub(1) as f32);
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = x0.saturating_add(1).min(target.width.saturating_sub(1));
    let y1 = y0.saturating_add(1).min(target.height.saturating_sub(1));
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;
    let c00 = sample_post_scene_texel(frame, target, x0, y0);
    let c10 = sample_post_scene_texel(frame, target, x1, y0);
    let c01 = sample_post_scene_texel(frame, target, x0, y1);
    let c11 = sample_post_scene_texel(frame, target, x1, y1);
    let top = c00 * (1.0 - tx) + c10 * tx;
    let bottom = c01 * (1.0 - tx) + c11 * tx;
    top * (1.0 - ty) + bottom * ty
}

fn sample_post_scene_texel(frame: &[Color], target: RasterTarget, x: u32, y: u32) -> Vec3 {
    let Some(color) = frame.get(target.pixel_index(x, y)) else {
        return Vec3::ZERO;
    };
    Vec3::new(color.r, color.g, color.b)
}

fn mix_depth(vertices: [CpuScreenVertex; 3], affine: [f32; 3]) -> f32 {
    vertices[0].projected.depth * affine[0]
        + vertices[1].projected.depth * affine[1]
        + vertices[2].projected.depth * affine[2]
}
