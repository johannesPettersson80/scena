use crate::geometry::Vertex;
use crate::material::Color;
use crate::scene::{ClippingPlane, SectionBox, Vec3};

use super::RasterTarget;
use super::camera::CameraProjection;
use super::cpu::CpuFrame;
use super::physical_transmission::{PhysicalTransmissionInputs, physical_transmission_color};
use super::prepare::PreparedPrimitive;

pub(super) fn draw_physical_transmission_cpu(
    cpu_frame: &mut CpuFrame<'_>,
    primitive: &PreparedPrimitive,
    scene_color_frame: &[u8],
    clipping_planes: &[ClippingPlane],
    section_box: Option<SectionBox>,
    camera: &CameraProjection,
) {
    let Some(transmission) = primitive.material_transmission() else {
        return;
    };
    let [a, b, c] = primitive.vertices();
    let [attributes_a, attributes_b, attributes_c] = primitive.vertex_attributes();
    let Some(a) = ScreenVertex::from_vertex(*a, cpu_frame.target, camera) else {
        return;
    };
    let Some(b) = ScreenVertex::from_vertex(*b, cpu_frame.target, camera) else {
        return;
    };
    let Some(c) = ScreenVertex::from_vertex(*c, cpu_frame.target, camera) else {
        return;
    };

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
        return;
    }

    let area = edge(a, b, c.x, c.y);
    if area.abs() <= f32::EPSILON {
        return;
    }
    if !primitive.double_sided() && area < 0.0 {
        return;
    }

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let w0 = edge(b, c, px, py) / area;
            let w1 = edge(c, a, px, py) / area;
            let w2 = edge(a, b, px, py) / area;
            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                continue;
            }
            let position = mix_position(a.position, b.position, c.position, w0, w1, w2);
            if is_clipped(position, clipping_planes, section_box) {
                continue;
            }
            let depth = mix_depth(a.depth, b.depth, c.depth, w0, w1, w2);
            if !depth.is_finite() {
                continue;
            }
            let Some(pixel_index) = cpu_frame.local_pixel_index(x, y) else {
                continue;
            };
            if depth > cpu_frame.depth_frame[pixel_index] + f32::EPSILON {
                continue;
            }

            let surface_color = multiply_color(mix_color(a, b, c, w0, w1, w2), primitive.tint());
            let surface_post = cpu_frame.output.post_color(surface_color);
            let normal =
                attributes_a.normal * w0 + attributes_b.normal * w1 + attributes_c.normal * w2;
            let view = camera.camera_position() - position;
            let target = cpu_frame.target;
            let transmitted = physical_transmission_color(
                transmission,
                PhysicalTransmissionInputs {
                    frag_coord: [px, py],
                    viewport: [target.width as f32, target.height as f32],
                    normal,
                    view,
                    tint: Vec3::new(surface_color.r, surface_color.g, surface_color.b),
                    surface_rgb: Vec3::new(surface_post.r, surface_post.g, surface_post.b),
                },
                |uv| sample_post_scene_color(scene_color_frame, target, uv),
            );
            let final_color =
                Color::from_linear_rgba(transmitted.x, transmitted.y, transmitted.z, 1.0);
            cpu_frame.linear_frame[pixel_index] = final_color;
            cpu_frame.depth_frame[pixel_index] = depth;
            let byte_index = pixel_index * 4;
            cpu_frame.frame[byte_index..byte_index + 4]
                .copy_from_slice(&cpu_frame.output.encode_post_rgba8(final_color));
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ScreenVertex {
    x: f32,
    y: f32,
    depth: f32,
    inv_depth: f32,
    position: Vec3,
    color: Color,
}

impl ScreenVertex {
    fn from_vertex(
        vertex: Vertex,
        target: RasterTarget,
        camera: &CameraProjection,
    ) -> Option<Self> {
        let projected = camera.project(vertex.position)?;
        let width = target.width.saturating_sub(1) as f32;
        let height = target.height.saturating_sub(1) as f32;
        Some(Self {
            x: (projected.ndc_x * 0.5 + 0.5) * width,
            y: (1.0 - (projected.ndc_y * 0.5 + 0.5)) * height,
            depth: projected.depth,
            inv_depth: projected.view_depth.recip(),
            position: vertex.position,
            color: vertex.color,
        })
    }
}

fn edge(a: ScreenVertex, b: ScreenVertex, x: f32, y: f32) -> f32 {
    (x - a.x) * (b.y - a.y) - (y - a.y) * (b.x - a.x)
}

fn mix_color(
    a: ScreenVertex,
    b: ScreenVertex,
    c: ScreenVertex,
    w0: f32,
    w1: f32,
    w2: f32,
) -> Color {
    let iw0 = w0 * a.inv_depth;
    let iw1 = w1 * b.inv_depth;
    let iw2 = w2 * c.inv_depth;
    let inv_sum = iw0 + iw1 + iw2;
    if inv_sum.abs() <= f32::EPSILON || !inv_sum.is_finite() {
        return mix_color_affine(a.color, b.color, c.color, w0, w1, w2);
    }
    let w0 = iw0 / inv_sum;
    let w1 = iw1 / inv_sum;
    let w2 = iw2 / inv_sum;
    mix_color_affine(a.color, b.color, c.color, w0, w1, w2)
}

fn mix_color_affine(a: Color, b: Color, c: Color, w0: f32, w1: f32, w2: f32) -> Color {
    Color::from_linear_rgba(
        a.r * w0 + b.r * w1 + c.r * w2,
        a.g * w0 + b.g * w1 + c.g * w2,
        a.b * w0 + b.b * w1 + c.b * w2,
        a.a * w0 + b.a * w1 + c.a * w2,
    )
}

fn multiply_color(color: Color, tint: Color) -> Color {
    Color::from_linear_rgba(
        color.r * tint.r,
        color.g * tint.g,
        color.b * tint.b,
        color.a * tint.a,
    )
}

fn sample_post_scene_color(frame: &[u8], target: RasterTarget, uv: [f32; 2]) -> Vec3 {
    let x = uv[0].clamp(0.0, 1.0) * target.width.saturating_sub(1) as f32;
    let y = uv[1].clamp(0.0, 1.0) * target.height.saturating_sub(1) as f32;
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

fn sample_post_scene_texel(frame: &[u8], target: RasterTarget, x: u32, y: u32) -> Vec3 {
    let byte_index = target.pixel_index(x, y).saturating_mul(4);
    let Some(rgba) = frame.get(byte_index..byte_index + 4) else {
        return Vec3::ZERO;
    };
    Vec3::new(
        srgb_u8_to_linear(rgba[0]),
        srgb_u8_to_linear(rgba[1]),
        srgb_u8_to_linear(rgba[2]),
    )
}

fn srgb_u8_to_linear(value: u8) -> f32 {
    let value = f32::from(value) / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn mix_position(a: Vec3, b: Vec3, c: Vec3, w0: f32, w1: f32, w2: f32) -> Vec3 {
    Vec3::new(
        a.x * w0 + b.x * w1 + c.x * w2,
        a.y * w0 + b.y * w1 + c.y * w2,
        a.z * w0 + b.z * w1 + c.z * w2,
    )
}

fn mix_depth(a: f32, b: f32, c: f32, w0: f32, w1: f32, w2: f32) -> f32 {
    a * w0 + b * w1 + c * w2
}

fn is_clipped(
    position: Vec3,
    clipping_planes: &[ClippingPlane],
    section_box: Option<SectionBox>,
) -> bool {
    clipping_planes
        .iter()
        .any(|plane| !plane.contains(position))
        || section_box.is_some_and(|section| section.clips(position))
}
