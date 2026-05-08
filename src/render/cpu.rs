use crate::geometry::{Primitive, Vertex};
use crate::material::Color;
use crate::scene::{ClippingPlane, Vec3};

use super::RasterTarget;
use super::camera::CameraProjection;
use super::output::OutputTransform;

pub(super) struct CpuFrame<'frame> {
    target: RasterTarget,
    output: OutputTransform,
    linear_frame: &'frame mut [Color],
    depth_frame: &'frame mut [f32],
    frame: &'frame mut [u8],
}

impl<'frame> CpuFrame<'frame> {
    pub(super) const fn new(
        target: RasterTarget,
        output: OutputTransform,
        linear_frame: &'frame mut [Color],
        depth_frame: &'frame mut [f32],
        frame: &'frame mut [u8],
    ) -> Self {
        Self {
            target,
            output,
            linear_frame,
            depth_frame,
            frame,
        }
    }
}

pub(super) fn clear_cpu(cpu_frame: &mut CpuFrame<'_>, color: Color) {
    let rgba = cpu_frame.output.encode_rgba8(color);
    for ((linear, depth), pixel) in cpu_frame
        .linear_frame
        .iter_mut()
        .zip(cpu_frame.depth_frame.iter_mut())
        .zip(cpu_frame.frame.chunks_exact_mut(4))
    {
        *linear = color;
        *depth = f32::INFINITY;
        pixel.copy_from_slice(&rgba);
    }
    debug_assert_eq!(cpu_frame.linear_frame.len(), cpu_frame.target.pixel_len());
    debug_assert_eq!(cpu_frame.depth_frame.len(), cpu_frame.target.pixel_len());
}

pub(super) fn draw_primitive_cpu(
    cpu_frame: &mut CpuFrame<'_>,
    primitive: &Primitive,
    clipping_planes: &[ClippingPlane],
    camera: &CameraProjection,
) {
    let [a, b, c] = primitive.vertices();
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
    let min_y = a.y.min(b.y).min(c.y).floor().max(0.0) as u32;
    let max_y =
        a.y.max(b.y)
            .max(c.y)
            .ceil()
            .min(cpu_frame.target.height as f32 - 1.0) as u32;

    let area = edge(a, b, c.x, c.y);
    if area.abs() <= f32::EPSILON {
        return;
    }

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let w0 = edge(b, c, px, py) / area;
            let w1 = edge(c, a, px, py) / area;
            let w2 = edge(a, b, px, py) / area;
            if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                let position = mix_position(a.position, b.position, c.position, w0, w1, w2);
                if is_clipped(position, clipping_planes) {
                    continue;
                }
                let color = mix_color(a.color, b.color, c.color, w0, w1, w2);
                let depth = mix_depth(a.depth, b.depth, c.depth, w0, w1, w2);
                write_pixel(cpu_frame, x, y, color, depth);
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ScreenVertex {
    x: f32,
    y: f32,
    depth: f32,
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
            position: vertex.position,
            color: vertex.color,
        })
    }
}

fn edge(a: ScreenVertex, b: ScreenVertex, x: f32, y: f32) -> f32 {
    (x - a.x) * (b.y - a.y) - (y - a.y) * (b.x - a.x)
}

fn mix_color(a: Color, b: Color, c: Color, w0: f32, w1: f32, w2: f32) -> Color {
    Color::from_linear_rgba(
        a.r * w0 + b.r * w1 + c.r * w2,
        a.g * w0 + b.g * w1 + c.g * w2,
        a.b * w0 + b.b * w1 + c.b * w2,
        a.a * w0 + b.a * w1 + c.a * w2,
    )
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

fn is_clipped(position: Vec3, clipping_planes: &[ClippingPlane]) -> bool {
    clipping_planes
        .iter()
        .any(|plane| !plane.contains(position))
}

fn write_pixel(cpu_frame: &mut CpuFrame<'_>, x: u32, y: u32, color: Color, depth: f32) {
    if !depth.is_finite() {
        return;
    }
    let pixel_index = cpu_frame.target.pixel_index(x, y);
    if depth > cpu_frame.depth_frame[pixel_index] + f32::EPSILON {
        return;
    }
    let blended = blend_source_over(color, cpu_frame.linear_frame[pixel_index]);
    cpu_frame.linear_frame[pixel_index] = blended;
    if clamp_alpha_or(color.a, 1.0) >= 1.0 - f32::EPSILON {
        cpu_frame.depth_frame[pixel_index] = depth;
    }

    let byte_index = pixel_index * 4;
    cpu_frame.frame[byte_index..byte_index + 4]
        .copy_from_slice(&cpu_frame.output.encode_rgba8(blended));
}

fn blend_source_over(source: Color, destination: Color) -> Color {
    let source_alpha = clamp_alpha_or(source.a, 1.0);
    let destination_alpha = clamp_alpha_or(destination.a, 1.0);
    if source_alpha == 1.0 {
        return Color::from_linear_rgba(source.r, source.g, source.b, 1.0);
    }
    if source_alpha <= 0.0 {
        return destination;
    }

    let inverse_source_alpha = 1.0 - source_alpha;
    let output_alpha = source_alpha + destination_alpha * inverse_source_alpha;
    let premultiplied_r =
        source.r * source_alpha + destination.r * destination_alpha * inverse_source_alpha;
    let premultiplied_g =
        source.g * source_alpha + destination.g * destination_alpha * inverse_source_alpha;
    let premultiplied_b =
        source.b * source_alpha + destination.b * destination_alpha * inverse_source_alpha;

    if output_alpha <= f32::EPSILON {
        Color::from_linear_rgba(0.0, 0.0, 0.0, 0.0)
    } else {
        Color::from_linear_rgba(
            premultiplied_r / output_alpha,
            premultiplied_g / output_alpha,
            premultiplied_b / output_alpha,
            output_alpha,
        )
    }
}

fn clamp_alpha_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        fallback
    }
}
