use crate::material::Color;

use super::RasterTarget;
pub(super) use super::cpu_clip::CpuTriangleClipInputs;
use super::cpu_geometry::{self, CpuProjectedPrimitive, CpuScreenTriangle, CpuScreenVertex};
pub(super) use super::cpu_overlay::write_label_overlay_pixel;
use super::output::{OrderIndependentTransparencyConfig, OutputTransform};
use super::prepare::PreparedPrimitive;
use super::screen_space_reflections::{MaterialReflectionPixel, ScreenSpaceReflectionConfig};

#[derive(Debug, Clone, Copy)]
pub(super) struct OitAccumPixel {
    color: [f32; 3],
    alpha_weight: f32,
    revealage: f32,
}

impl Default for OitAccumPixel {
    fn default() -> Self {
        Self {
            color: [0.0; 3],
            alpha_weight: 0.0,
            revealage: 1.0,
        }
    }
}

pub(super) struct CpuFrame<'frame> {
    pub(super) target: RasterTarget,
    pub(super) output: OutputTransform,
    pub(super) row_start: u32,
    row_count: u32,
    pub(super) linear_frame: &'frame mut [Color],
    pub(super) depth_frame: &'frame mut [f32],
    pub(super) frame: &'frame mut [u8],
}

impl<'frame> CpuFrame<'frame> {
    pub(super) const fn new(
        target: RasterTarget,
        output: OutputTransform,
        linear_frame: &'frame mut [Color],
        depth_frame: &'frame mut [f32],
        frame: &'frame mut [u8],
    ) -> Self {
        Self::new_rows(
            target,
            output,
            0,
            target.height,
            linear_frame,
            depth_frame,
            frame,
        )
    }

    pub(super) const fn new_rows(
        target: RasterTarget,
        output: OutputTransform,
        row_start: u32,
        row_count: u32,
        linear_frame: &'frame mut [Color],
        depth_frame: &'frame mut [f32],
        frame: &'frame mut [u8],
    ) -> Self {
        Self {
            target,
            output,
            row_start,
            row_count,
            linear_frame,
            depth_frame,
            frame,
        }
    }

    pub(super) fn row_end(&self) -> u32 {
        self.row_start
            .saturating_add(self.row_count)
            .min(self.target.height)
    }

    fn local_pixel_len(&self) -> usize {
        (self.row_end().saturating_sub(self.row_start) as usize) * (self.target.width as usize)
    }

    pub(super) fn local_pixel_index(&self, x: u32, y: u32) -> Option<usize> {
        if x >= self.target.width || y < self.row_start || y >= self.row_end() {
            return None;
        }
        let local_y = y.saturating_sub(self.row_start);
        Some((local_y as usize) * (self.target.width as usize) + (x as usize))
    }
}

pub(super) fn clear_cpu(cpu_frame: &mut CpuFrame<'_>, color: Color) {
    let rgba = cpu_frame.output.encode_clear_rgba8(color);
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
    debug_assert_eq!(cpu_frame.depth_frame.len(), cpu_frame.local_pixel_len());
    debug_assert_eq!(cpu_frame.linear_frame.len(), cpu_frame.local_pixel_len());
}

pub(super) fn encode_cpu_frame(cpu_frame: &mut CpuFrame<'_>) -> u64 {
    let mut encoded = 0_u64;
    for ((linear, depth), pixel) in cpu_frame
        .linear_frame
        .iter()
        .zip(cpu_frame.depth_frame.iter())
        .zip(cpu_frame.frame.chunks_exact_mut(4))
    {
        if depth.is_finite() {
            pixel.copy_from_slice(&cpu_frame.output.encode_rgba8(*linear));
            encoded = encoded.saturating_add(1);
        }
    }
    encoded
}

pub(super) fn clear_order_independent_transparency(accum: &mut [OitAccumPixel]) {
    for pixel in accum {
        *pixel = OitAccumPixel::default();
    }
}

pub(super) fn primitive_needs_order_independent_transparency(
    primitive: &PreparedPrimitive,
) -> bool {
    primitive
        .vertices()
        .iter()
        .any(|vertex| clamp_alpha_or(vertex.color.a, 1.0) < 1.0 - f32::EPSILON)
}

pub(super) fn primitive_needs_physical_transmission(primitive: &PreparedPrimitive) -> bool {
    primitive.material_transmission().is_some()
}

pub(super) fn draw_primitive_cpu(
    cpu_frame: &mut CpuFrame<'_>,
    primitive: &PreparedPrimitive,
    projected: &CpuProjectedPrimitive,
    context: CpuTriangleClipInputs<'_>,
    mut material_reflections: Option<&mut [MaterialReflectionPixel]>,
    reflection_config: Option<ScreenSpaceReflectionConfig>,
) {
    for triangle in projected.triangles() {
        draw_projected_primitive_cpu(
            cpu_frame,
            primitive,
            *triangle,
            context,
            material_reflections.as_deref_mut(),
            reflection_config,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_projected_primitive_cpu(
    cpu_frame: &mut CpuFrame<'_>,
    primitive: &PreparedPrimitive,
    triangle: CpuScreenTriangle,
    context: CpuTriangleClipInputs<'_>,
    mut material_reflections: Option<&mut [MaterialReflectionPixel]>,
    reflection_config: Option<ScreenSpaceReflectionConfig>,
) {
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
        return;
    }

    let area = cpu_geometry::edge(a, b, c.x, c.y);
    if area.abs() <= f32::EPSILON {
        return;
    }
    if !primitive.double_sided() && area < 0.0 {
        return;
    }
    let inverse_area = area.recip();
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let [w0, w1, w2] = affine_barycentric_weights(
                [
                    cpu_geometry::edge(b, c, px, py),
                    cpu_geometry::edge(c, a, px, py),
                    cpu_geometry::edge(a, b, px, py),
                ],
                inverse_area,
            );
            if !cpu_geometry::barycentric_sample_is_inside([w0, w1, w2]) {
                continue;
            }
            let weights = cpu_geometry::perspective_weights(camera, vertices, [w0, w1, w2]);
            let position =
                cpu_geometry::weighted_vec3([a.position, b.position, c.position], weights);
            if cpu_geometry::point_is_clipped(position, clipping_planes, section_box) {
                continue;
            }
            let color = multiply_color(mix_color(vertices, weights), primitive.tint());
            let depth = mix_depth(vertices, [w0, w1, w2]);
            if write_pixel(cpu_frame, x, y, color, depth)
                && let (Some(buffer), Some(config), Some(reflection)) = (
                    material_reflections.as_deref_mut(),
                    reflection_config,
                    primitive.material_reflection(),
                )
            {
                super::cpu_reflections::record_material_reflection_pixel(
                    super::cpu_reflections::MaterialReflectionRecord {
                        target: cpu_frame.target,
                        camera,
                        material_reflections: buffer,
                        x,
                        y,
                        position,
                        normal: cpu_geometry::weighted_vec3(
                            [
                                a.attributes.normal,
                                b.attributes.normal,
                                c.attributes.normal,
                            ],
                            weights,
                        ),
                        reflection,
                        config,
                    },
                );
            }
        }
    }
}

pub(super) fn draw_order_independent_transparency_cpu(
    cpu_frame: &mut CpuFrame<'_>,
    primitive: &PreparedPrimitive,
    projected: &CpuProjectedPrimitive,
    context: CpuTriangleClipInputs<'_>,
    accum: &mut [OitAccumPixel],
    config: OrderIndependentTransparencyConfig,
) {
    for triangle in projected.triangles() {
        draw_projected_order_independent_transparency_cpu(
            cpu_frame, primitive, *triangle, context, accum, config,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_projected_order_independent_transparency_cpu(
    cpu_frame: &mut CpuFrame<'_>,
    primitive: &PreparedPrimitive,
    triangle: CpuScreenTriangle,
    context: CpuTriangleClipInputs<'_>,
    accum: &mut [OitAccumPixel],
    config: OrderIndependentTransparencyConfig,
) {
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
        return;
    }

    let area = cpu_geometry::edge(a, b, c.x, c.y);
    if area.abs() <= f32::EPSILON {
        return;
    }
    if !primitive.double_sided() && area < 0.0 {
        return;
    }
    let inverse_area = area.recip();

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let [w0, w1, w2] = affine_barycentric_weights(
                [
                    cpu_geometry::edge(b, c, px, py),
                    cpu_geometry::edge(c, a, px, py),
                    cpu_geometry::edge(a, b, px, py),
                ],
                inverse_area,
            );
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
            accumulate_order_independent_transparency(
                &mut accum[pixel_index],
                multiply_color(mix_color(vertices, weights), primitive.tint()),
                config,
            );
        }
    }
}

pub(super) fn resolve_order_independent_transparency_cpu(
    cpu_frame: &mut CpuFrame<'_>,
    accum: &[OitAccumPixel],
) -> u64 {
    let mut touched = false;
    for (pixel_index, pixel) in accum.iter().enumerate() {
        if pixel.alpha_weight <= f32::EPSILON {
            continue;
        }
        touched = true;
        let alpha = (1.0 - pixel.revealage).clamp(0.0, 1.0);
        let transparent = Color::from_linear_rgba(
            pixel.color[0] / pixel.alpha_weight,
            pixel.color[1] / pixel.alpha_weight,
            pixel.color[2] / pixel.alpha_weight,
            alpha,
        );
        let blended = blend_source_over(transparent, cpu_frame.linear_frame[pixel_index]);
        cpu_frame.linear_frame[pixel_index] = blended;
        let byte_index = pixel_index * 4;
        cpu_frame.frame[byte_index..byte_index + 4]
            .copy_from_slice(&cpu_frame.output.encode_rgba8(blended));
    }
    u64::from(touched)
}

fn mix_color(vertices: [CpuScreenVertex; 3], weights: [f32; 3]) -> Color {
    mix_color_affine(
        vertices[0].color,
        vertices[1].color,
        vertices[2].color,
        weights[0],
        weights[1],
        weights[2],
    )
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

fn mix_depth(vertices: [CpuScreenVertex; 3], affine: [f32; 3]) -> f32 {
    vertices[0].projected.depth * affine[0]
        + vertices[1].projected.depth * affine[1]
        + vertices[2].projected.depth * affine[2]
}

#[inline]
fn affine_barycentric_weights(edge_values: [f32; 3], inverse_area: f32) -> [f32; 3] {
    edge_values.map(|edge| edge * inverse_area)
}

pub(super) fn write_pixel(
    cpu_frame: &mut CpuFrame<'_>,
    x: u32,
    y: u32,
    color: Color,
    depth: f32,
) -> bool {
    if !depth.is_finite() {
        return false;
    }
    let Some(pixel_index) = cpu_frame.local_pixel_index(x, y) else {
        return false;
    };
    if depth > cpu_frame.depth_frame[pixel_index] + f32::EPSILON {
        return false;
    }
    let blended = blend_source_over(color, cpu_frame.linear_frame[pixel_index]);
    cpu_frame.linear_frame[pixel_index] = blended;
    let source_alpha = clamp_alpha_or(color.a, 1.0);
    if source_alpha >= 1.0 - f32::EPSILON {
        cpu_frame.depth_frame[pixel_index] = depth;
    } else {
        let byte_index = pixel_index * 4;
        cpu_frame.frame[byte_index..byte_index + 4]
            .copy_from_slice(&cpu_frame.output.encode_rgba8(blended));
    }

    true
}

fn accumulate_order_independent_transparency(
    pixel: &mut OitAccumPixel,
    color: Color,
    config: OrderIndependentTransparencyConfig,
) {
    let alpha = boosted_alpha(clamp_alpha_or(color.a, 1.0), config.coverage_boost());
    if alpha <= 0.0 {
        return;
    }
    pixel.color[0] += color.r * alpha;
    pixel.color[1] += color.g * alpha;
    pixel.color[2] += color.b * alpha;
    pixel.alpha_weight += alpha;
    pixel.revealage *= 1.0 - alpha;
}

fn boosted_alpha(alpha: f32, coverage_boost: f32) -> f32 {
    let boost = coverage_boost.clamp(0.25, 4.0);
    1.0 - (1.0 - alpha.clamp(0.0, 1.0)).powf(boost)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affine_barycentric_weights_use_one_precomputed_inverse_area() {
        let area = 8.0_f32;
        let edge_values = [2.0, 3.0, 3.0];
        let weights = affine_barycentric_weights(edge_values, area.recip());

        assert_eq!(weights, [0.25, 0.375, 0.375]);
        assert_eq!(weights.iter().sum::<f32>(), 1.0);
    }

    #[test]
    fn opaque_and_oit_hot_loops_pin_one_reciprocal_and_zero_per_pixel_divisions() {
        let source = include_str!("cpu.rs");
        let opaque = source
            .split_once("fn draw_projected_primitive_cpu(")
            .expect("opaque raster function exists")
            .1
            .split_once("pub(super) fn draw_order_independent_transparency_cpu(")
            .expect("opaque raster function boundary exists")
            .0;
        let oit = source
            .split_once("fn draw_projected_order_independent_transparency_cpu(")
            .expect("OIT raster function exists")
            .1
            .split_once("pub(super) fn resolve_order_independent_transparency_cpu(")
            .expect("OIT raster function boundary exists")
            .0;

        for (name, hot_loop) in [("opaque", opaque), ("oit", oit)] {
            assert_eq!(
                hot_loop.matches("let inverse_area = area.recip();").count(),
                1,
                "{name} rasterization must compute one reciprocal per triangle",
            );
            assert!(
                !hot_loop.contains(" / area"),
                "{name} covered-pixel work must not divide each barycentric edge by area",
            );
            assert_eq!(
                hot_loop.matches("affine_barycentric_weights(").count(),
                1,
                "{name} rasterization must share the multiply-only barycentric helper for opaque, overdraw, and clipped samples",
            );
        }
    }
}
