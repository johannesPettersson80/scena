use crate::diagnostics::RenderError;
use crate::material::Color;
use crate::scene::Scene;
use crate::scene::{ClippingPlane, SectionBox};

use super::output::OutputTransform;
use super::prepare::PreparedPrimitive;
use super::{AntiAliasing, RasterTarget, Renderer, camera, cpu, cpu_strokes, output};

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
                self.target
                    .scaled(scale)
                    .ok_or_else(|| RenderError::InvalidSurfaceSize {
                        width: self.target.width.saturating_mul(scale),
                        height: self.target.height.saturating_mul(scale),
                    })?;
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
    target_frame: &mut [u8],
) {
    debug_assert_eq!(source_target.width, target.width.saturating_mul(scale));
    debug_assert_eq!(source_target.height, target.height.saturating_mul(scale));
    let sample_count = scale.saturating_mul(scale).max(1) as f32;
    for y in 0..target.height {
        for x in 0..target.width {
            let target_index = target.pixel_index(x, y);
            let mut linear = Color::TRANSPARENT;
            let mut encoded = [0.0_f32; 4];
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
                    let source_offset = source_index.saturating_mul(4);
                    for channel in 0..4 {
                        encoded[channel] += f32::from(source_frame[source_offset + channel]);
                    }
                }
            }
            linear.r /= sample_count;
            linear.g /= sample_count;
            linear.b /= sample_count;
            linear.a /= sample_count;
            target_linear[target_index] = linear;
            target_depth[target_index] = depth;
            let target_offset = target_index.saturating_mul(4);
            for channel in 0..4 {
                target_frame[target_offset + channel] =
                    (encoded[channel] / sample_count).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
}

pub(super) fn downsample_rgba8_box_filter(
    source_target: RasterTarget,
    scale: u32,
    source_frame: &[u8],
    target: RasterTarget,
    target_frame: &mut Vec<u8>,
) {
    debug_assert_eq!(source_target.width, target.width.saturating_mul(scale));
    debug_assert_eq!(source_target.height, target.height.saturating_mul(scale));
    target_frame.resize(target.byte_len(), 0);
    let sample_count = scale.saturating_mul(scale).max(1) as f32;
    for y in 0..target.height {
        for x in 0..target.width {
            let target_offset = target.pixel_index(x, y).saturating_mul(4);
            let mut encoded = [0.0_f32; 4];
            for sy in 0..scale {
                for sx in 0..scale {
                    let source_x = x.saturating_mul(scale).saturating_add(sx);
                    let source_y = y.saturating_mul(scale).saturating_add(sy);
                    let source_offset = source_target
                        .pixel_index(source_x, source_y)
                        .saturating_mul(4);
                    for channel in 0..4 {
                        encoded[channel] += f32::from(source_frame[source_offset + channel]);
                    }
                }
            }
            for channel in 0..4 {
                target_frame[target_offset + channel] =
                    (encoded[channel] / sample_count).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
}
