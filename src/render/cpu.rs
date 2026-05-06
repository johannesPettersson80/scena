use crate::geometry::{Primitive, Vertex};
use crate::material::Color;
use crate::scene::{ClippingPlane, Vec3};

use super::RasterTarget;
use super::output::OutputTransform;

pub(super) fn clear_cpu(
    target: RasterTarget,
    output: OutputTransform,
    linear_frame: &mut [Color],
    frame: &mut [u8],
    color: Color,
) {
    let rgba = output.encode_rgba8(color);
    for (linear, pixel) in linear_frame.iter_mut().zip(frame.chunks_exact_mut(4)) {
        *linear = color;
        pixel.copy_from_slice(&rgba);
    }
    debug_assert_eq!(linear_frame.len(), target.pixel_len());
}

pub(super) fn draw_primitive_cpu(
    target: RasterTarget,
    output: OutputTransform,
    linear_frame: &mut [Color],
    frame: &mut [u8],
    primitive: &Primitive,
    clipping_planes: &[ClippingPlane],
) {
    let [a, b, c] = primitive.vertices();
    let a = ScreenVertex::from_vertex(*a, target);
    let b = ScreenVertex::from_vertex(*b, target);
    let c = ScreenVertex::from_vertex(*c, target);

    let min_x = a.x.min(b.x).min(c.x).floor().max(0.0) as u32;
    let max_x = a.x.max(b.x).max(c.x).ceil().min(target.width as f32 - 1.0) as u32;
    let min_y = a.y.min(b.y).min(c.y).floor().max(0.0) as u32;
    let max_y = a.y.max(b.y).max(c.y).ceil().min(target.height as f32 - 1.0) as u32;

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
                write_pixel(target, output, linear_frame, frame, x, y, color);
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ScreenVertex {
    x: f32,
    y: f32,
    position: Vec3,
    color: Color,
}

impl ScreenVertex {
    fn from_vertex(vertex: Vertex, target: RasterTarget) -> Self {
        let width = target.width.saturating_sub(1) as f32;
        let height = target.height.saturating_sub(1) as f32;
        Self {
            x: (vertex.position.x * 0.5 + 0.5) * width,
            y: (1.0 - (vertex.position.y * 0.5 + 0.5)) * height,
            position: vertex.position,
            color: vertex.color,
        }
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

fn is_clipped(position: Vec3, clipping_planes: &[ClippingPlane]) -> bool {
    clipping_planes
        .iter()
        .any(|plane| !plane.contains(position))
}

fn write_pixel(
    target: RasterTarget,
    output: OutputTransform,
    linear_frame: &mut [Color],
    frame: &mut [u8],
    x: u32,
    y: u32,
    color: Color,
) {
    let pixel_index = target.pixel_index(x, y);
    let blended = blend_source_over(color, linear_frame[pixel_index]);
    linear_frame[pixel_index] = blended;

    let byte_index = pixel_index * 4;
    frame[byte_index..byte_index + 4].copy_from_slice(&output.encode_rgba8(blended));
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
