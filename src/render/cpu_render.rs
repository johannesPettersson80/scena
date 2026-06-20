use crate::diagnostics::RenderError;
use crate::material::Color;
use crate::scene::Scene;
use crate::scene::{ClippingPlane, SectionBox};

use super::output::OutputTransform;
use super::prepare::PreparedPrimitive;
use super::{
    AntiAliasing, RasterTarget, ReconstructionFilter, Renderer, camera, cpu, cpu_strokes, output,
};

impl Renderer {
    pub(super) fn draw_cpu(
        &mut self,
        scene: &Scene,
        camera: crate::scene::CameraKey,
        camera_projection: &camera::CameraProjection,
    ) -> Result<(), RenderError> {
        let (primitives, strokes, labels, clipping_planes, section_box) = {
            let prepared = self.prepared_state(scene)?;
            (
                prepared.primitives.clone(),
                prepared.strokes.clone(),
                prepared.labels.clone(),
                prepared.clipping_planes.clone(),
                prepared.section_box,
            )
        };
        let scale = self
            .anti_aliasing
            .cpu_supersample_scale()
            .max(self.supersample_factor);
        let full_frame_supersample = self.supersample_factor > 1;
        let mut overlays_drawn_before_resolve = false;
        if scale > 1 {
            let supersample_target =
                super::target::validate_supersample_target(self.target, scale)?;
            let supersample_projection =
                camera::CameraProjection::from_scene(scene, camera, supersample_target)?;
            let mut supersample_linear = vec![Color::BLACK; supersample_target.pixel_len()];
            let mut supersample_depth = vec![f32::INFINITY; supersample_target.pixel_len()];
            let mut supersample_frame = vec![0; supersample_target.byte_len()];
            let mut supersample_oit =
                vec![cpu::OitAccumPixel::default(); supersample_target.pixel_len()];
            self.stats.order_independent_transparency_passes =
                draw_cpu_geometry_pass(CpuGeometryPass {
                    target: supersample_target,
                    output: self.output,
                    background_color: self.background_color,
                    primitives: &primitives,
                    clipping_planes: &clipping_planes,
                    section_box,
                    camera_projection: &supersample_projection,
                    order_independent_transparency: self.order_independent_transparency,
                    linear_frame: &mut supersample_linear,
                    depth_frame: &mut supersample_depth,
                    frame: &mut supersample_frame,
                    oit_scratch: &mut supersample_oit,
                });
            if full_frame_supersample {
                let mut cpu_frame = cpu::CpuFrame::new(
                    supersample_target,
                    self.output,
                    &mut supersample_linear,
                    &mut supersample_depth,
                    &mut supersample_frame,
                );
                cpu_strokes::draw_overlay_layers_cpu(
                    &mut cpu_frame,
                    &strokes,
                    &labels,
                    &clipping_planes,
                    section_box,
                    &supersample_projection,
                );
                overlays_drawn_before_resolve = true;
            }
            let linear_frame = self
                .linear_frame
                .as_mut()
                .expect("CPU renderer owns a linear accumulator");
            let depth_frame = self
                .depth_frame
                .as_mut()
                .expect("CPU renderer owns a depth buffer");
            downsample_cpu_supersample(
                supersample_target,
                scale,
                &supersample_linear,
                &supersample_depth,
                &supersample_frame,
                self.target,
                linear_frame,
                depth_frame,
                &mut self.frame,
                self.reconstruction_filter,
            );
        } else {
            let linear_frame = self
                .linear_frame
                .as_mut()
                .expect("CPU renderer owns a linear accumulator");
            let depth_frame = self
                .depth_frame
                .as_mut()
                .expect("CPU renderer owns a depth buffer");
            self.stats.order_independent_transparency_passes =
                draw_cpu_geometry_pass(CpuGeometryPass {
                    target: self.target,
                    output: self.output,
                    background_color: self.background_color,
                    primitives: &primitives,
                    clipping_planes: &clipping_planes,
                    section_box,
                    camera_projection,
                    order_independent_transparency: self.order_independent_transparency,
                    linear_frame,
                    depth_frame,
                    frame: &mut self.frame,
                    oit_scratch: &mut self.oit_scratch,
                });
        }

        self.stats.ambient_occlusion_passes = match (
            self.screen_space_ambient_occlusion,
            self.depth_frame.as_ref(),
        ) {
            (Some(config), Some(depth_frame)) => {
                output::apply_screen_space_ambient_occlusion_rgba8(
                    self.target,
                    &mut self.frame,
                    depth_frame,
                    config,
                )
            }
            _ => 0,
        };
        self.stats.bloom_passes = self.bloom.map_or(0, |bloom| {
            output::apply_bloom_rgba8(self.target, &mut self.frame, &mut self.bloom_scratch, bloom)
        });
        self.stats.fxaa_passes = match self.anti_aliasing {
            AntiAliasing::None | AntiAliasing::Msaa4 | AntiAliasing::Msaa8 => 0,
            AntiAliasing::Fxaa => {
                output::apply_fxaa_rgba8(self.target, &mut self.frame, &mut self.fxaa_scratch)
            }
        };

        if !overlays_drawn_before_resolve {
            let linear_frame = self
                .linear_frame
                .as_mut()
                .expect("CPU renderer owns a linear accumulator");
            let depth_frame = self
                .depth_frame
                .as_mut()
                .expect("CPU renderer owns a depth buffer");
            let mut cpu_frame = cpu::CpuFrame::new(
                self.target,
                self.output,
                linear_frame,
                depth_frame,
                &mut self.frame,
            );
            cpu_strokes::draw_overlay_layers_cpu(
                &mut cpu_frame,
                &strokes,
                &labels,
                &clipping_planes,
                section_box,
                camera_projection,
            );
        }
        Ok(())
    }
}

struct CpuGeometryPass<'a> {
    target: RasterTarget,
    output: OutputTransform,
    background_color: Color,
    primitives: &'a [PreparedPrimitive],
    clipping_planes: &'a [ClippingPlane],
    section_box: Option<SectionBox>,
    camera_projection: &'a camera::CameraProjection,
    order_independent_transparency: Option<super::OrderIndependentTransparencyConfig>,
    linear_frame: &'a mut [Color],
    depth_frame: &'a mut [f32],
    frame: &'a mut [u8],
    oit_scratch: &'a mut [cpu::OitAccumPixel],
}

fn draw_cpu_geometry_pass(input: CpuGeometryPass<'_>) -> u64 {
    let mut cpu_frame = cpu::CpuFrame::new(
        input.target,
        input.output,
        input.linear_frame,
        input.depth_frame,
        input.frame,
    );
    cpu::clear_cpu(&mut cpu_frame, input.background_color);
    if let Some(config) = input.order_independent_transparency {
        cpu::clear_order_independent_transparency(input.oit_scratch);
        for primitive in input.primitives {
            if !primitive.gpu_triangle_path() {
                continue;
            }
            if cpu::primitive_needs_order_independent_transparency(primitive) {
                cpu::draw_order_independent_transparency_cpu(
                    &mut cpu_frame,
                    primitive,
                    input.clipping_planes,
                    input.section_box,
                    input.camera_projection,
                    input.oit_scratch,
                    config,
                );
            } else {
                cpu::draw_primitive_cpu(
                    &mut cpu_frame,
                    primitive,
                    input.clipping_planes,
                    input.section_box,
                    input.camera_projection,
                );
            }
        }
        cpu::resolve_order_independent_transparency_cpu(&mut cpu_frame, input.oit_scratch)
    } else {
        for primitive in input.primitives {
            if !primitive.gpu_triangle_path() {
                continue;
            }
            cpu::draw_primitive_cpu(
                &mut cpu_frame,
                primitive,
                input.clipping_planes,
                input.section_box,
                input.camera_projection,
            );
        }
        0
    }
}

#[allow(clippy::too_many_arguments)]
fn downsample_cpu_supersample(
    source_target: RasterTarget,
    scale: u32,
    source_linear: &[Color],
    source_depth: &[f32],
    source_frame: &[u8],
    target: RasterTarget,
    target_linear: &mut [Color],
    target_depth: &mut [f32],
    target_frame: &mut Vec<u8>,
    reconstruction_filter: ReconstructionFilter,
) {
    debug_assert_eq!(source_target.width, target.width.saturating_mul(scale));
    debug_assert_eq!(source_target.height, target.height.saturating_mul(scale));
    let sample_count = scale.saturating_mul(scale).max(1) as f32;
    for y in 0..target.height {
        for x in 0..target.width {
            let target_index = target.pixel_index(x, y);
            let mut linear = Color::TRANSPARENT;
            let mut depth = f32::INFINITY;
            for sy in 0..scale {
                for sx in 0..scale {
                    let source_x = x.saturating_mul(scale).saturating_add(sx);
                    let source_y = y.saturating_mul(scale).saturating_add(sy);
                    let source_index = source_target.pixel_index(source_x, source_y);
                    let source_color = source_linear[source_index];
                    linear.r += source_color.r;
                    linear.g += source_color.g;
                    linear.b += source_color.b;
                    linear.a += source_color.a;
                    depth = depth.min(source_depth[source_index]);
                }
            }
            linear.r /= sample_count;
            linear.g /= sample_count;
            linear.b /= sample_count;
            linear.a /= sample_count;
            target_linear[target_index] = linear;
            target_depth[target_index] = depth;
        }
    }
    downsample_rgba8_reconstruction_filter(
        source_target,
        scale,
        source_frame,
        target,
        target_frame,
        reconstruction_filter,
    );
}

#[cfg(test)]
pub(super) fn downsample_rgba8_box_filter(
    source_target: RasterTarget,
    scale: u32,
    source_frame: &[u8],
    target: RasterTarget,
    target_frame: &mut Vec<u8>,
) {
    downsample_rgba8_reconstruction_filter(
        source_target,
        scale,
        source_frame,
        target,
        target_frame,
        ReconstructionFilter::Box,
    );
}

pub(super) fn downsample_rgba8_reconstruction_filter(
    source_target: RasterTarget,
    scale: u32,
    source_frame: &[u8],
    target: RasterTarget,
    target_frame: &mut Vec<u8>,
    reconstruction_filter: ReconstructionFilter,
) {
    debug_assert_eq!(source_target.width, target.width.saturating_mul(scale));
    debug_assert_eq!(source_target.height, target.height.saturating_mul(scale));
    target_frame.resize(target.byte_len(), 0);
    for y in 0..target.height {
        for x in 0..target.width {
            let target_offset = target.pixel_index(x, y).saturating_mul(4);
            target_frame[target_offset..target_offset + 4].copy_from_slice(&sample_rgba8_kernel(
                source_target,
                scale,
                source_frame,
                x,
                y,
                reconstruction_filter,
            ));
        }
    }
}

fn sample_rgba8_kernel(
    source_target: RasterTarget,
    scale: u32,
    source_frame: &[u8],
    target_x: u32,
    target_y: u32,
    reconstruction_filter: ReconstructionFilter,
) -> [u8; 4] {
    match reconstruction_filter {
        ReconstructionFilter::Box => {
            sample_rgba8_box(source_target, scale, source_frame, target_x, target_y)
        }
        ReconstructionFilter::Tent | ReconstructionFilter::Gaussian => sample_rgba8_weighted(
            source_target,
            scale,
            source_frame,
            target_x,
            target_y,
            reconstruction_filter,
        ),
    }
}

fn sample_rgba8_box(
    source_target: RasterTarget,
    scale: u32,
    source_frame: &[u8],
    target_x: u32,
    target_y: u32,
) -> [u8; 4] {
    let mut linear = [0.0_f32; 3];
    let mut alpha = 0.0_f32;
    let mut weight_sum = 0.0_f32;
    for sy in 0..scale {
        for sx in 0..scale {
            let source_x = target_x.saturating_mul(scale).saturating_add(sx);
            let source_y = target_y.saturating_mul(scale).saturating_add(sy);
            accumulate_rgba8_sample(
                source_target,
                source_frame,
                source_x,
                source_y,
                1.0,
                &mut linear,
                &mut alpha,
                &mut weight_sum,
            );
        }
    }
    encode_linear_average(linear, alpha, weight_sum)
}

fn sample_rgba8_weighted(
    source_target: RasterTarget,
    scale: u32,
    source_frame: &[u8],
    target_x: u32,
    target_y: u32,
    reconstruction_filter: ReconstructionFilter,
) -> [u8; 4] {
    let scale_f = scale.max(1) as f32;
    let center_x = (target_x as f32 + 0.5) * scale_f;
    let center_y = (target_y as f32 + 0.5) * scale_f;
    let radius = match reconstruction_filter {
        ReconstructionFilter::Tent => scale_f,
        ReconstructionFilter::Gaussian => scale_f,
        ReconstructionFilter::Box => scale_f * 0.5,
    };
    let min_x = ((center_x - radius).floor() as i64).max(0) as u32;
    let max_x = ((center_x + radius).ceil() as u32).min(source_target.width.saturating_sub(1));
    let min_y = ((center_y - radius).floor() as i64).max(0) as u32;
    let max_y = ((center_y + radius).ceil() as u32).min(source_target.height.saturating_sub(1));
    let mut linear = [0.0_f32; 3];
    let mut alpha = 0.0_f32;
    let mut weight_sum = 0.0_f32;
    for sy in min_y..=max_y {
        let sample_y = sy as f32 + 0.5;
        let wy = reconstruction_weight((sample_y - center_y) / scale_f, reconstruction_filter);
        if wy <= 0.0 {
            continue;
        }
        for sx in min_x..=max_x {
            let sample_x = sx as f32 + 0.5;
            let wx = reconstruction_weight((sample_x - center_x) / scale_f, reconstruction_filter);
            let weight = wx * wy;
            if weight <= 0.0 {
                continue;
            }
            accumulate_rgba8_sample(
                source_target,
                source_frame,
                sx,
                sy,
                weight,
                &mut linear,
                &mut alpha,
                &mut weight_sum,
            );
        }
    }
    encode_linear_average(linear, alpha, weight_sum)
}

fn reconstruction_weight(
    distance_in_output_pixels: f32,
    reconstruction_filter: ReconstructionFilter,
) -> f32 {
    match reconstruction_filter {
        ReconstructionFilter::Box => {
            if distance_in_output_pixels.abs() <= 0.5 {
                1.0
            } else {
                0.0
            }
        }
        ReconstructionFilter::Tent => (1.0 - distance_in_output_pixels.abs()).max(0.0),
        ReconstructionFilter::Gaussian => {
            let sigma = 0.42_f32;
            (-0.5 * (distance_in_output_pixels / sigma).powi(2)).exp()
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn accumulate_rgba8_sample(
    source_target: RasterTarget,
    source_frame: &[u8],
    source_x: u32,
    source_y: u32,
    weight: f32,
    linear: &mut [f32; 3],
    alpha: &mut f32,
    weight_sum: &mut f32,
) {
    let source_offset = source_target
        .pixel_index(source_x, source_y)
        .saturating_mul(4);
    linear[0] += srgb8_to_linear(source_frame[source_offset]) * weight;
    linear[1] += srgb8_to_linear(source_frame[source_offset + 1]) * weight;
    linear[2] += srgb8_to_linear(source_frame[source_offset + 2]) * weight;
    *alpha += (f32::from(source_frame[source_offset + 3]) / 255.0) * weight;
    *weight_sum += weight;
}

fn encode_linear_average(linear: [f32; 3], alpha: f32, weight_sum: f32) -> [u8; 4] {
    if weight_sum <= f32::EPSILON {
        return [0, 0, 0, 0];
    }
    [
        linear_to_srgb8(linear[0] / weight_sum),
        linear_to_srgb8(linear[1] / weight_sum),
        linear_to_srgb8(linear[2] / weight_sum),
        ((alpha / weight_sum) * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

fn srgb8_to_linear(value: u8) -> f32 {
    let value = f32::from(value) / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb8(value: f32) -> u8 {
    let value = value.clamp(0.0, 1.0);
    let encoded = if value <= 0.003_130_8 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0).round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Backend;

    #[test]
    fn rgba8_supersample_downsample_averages_rgb_in_linear_light() {
        let source_target = RasterTarget {
            width: 2,
            height: 2,
            backend: Backend::HeadlessGpu,
        };
        let target = RasterTarget {
            width: 1,
            height: 1,
            backend: Backend::HeadlessGpu,
        };
        let source_frame = [
            0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255, 255, 255, 255, 255,
        ];
        let mut target_frame = Vec::new();

        downsample_rgba8_box_filter(source_target, 2, &source_frame, target, &mut target_frame);

        assert!(
            (185..=190).contains(&target_frame[0])
                && (185..=190).contains(&target_frame[1])
                && (185..=190).contains(&target_frame[2]),
            "linear-light average of 50% black + 50% white should encode near 188, got {:?}",
            &target_frame[..4]
        );
        assert_eq!(target_frame[3], 255);
    }
}
