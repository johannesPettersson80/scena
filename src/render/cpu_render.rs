use crate::diagnostics::RenderError;
use crate::scene::Scene;

use super::{AntiAliasing, Renderer, camera, cpu, cpu_strokes, output};

impl Renderer {
    pub(super) fn draw_cpu(
        &mut self,
        scene: &Scene,
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
        cpu::clear_cpu(&mut cpu_frame, self.background_color);
        self.stats.order_independent_transparency_passes =
            if let Some(config) = self.order_independent_transparency {
                cpu::clear_order_independent_transparency(&mut self.oit_scratch);
                for primitive in &primitives {
                    if !primitive.gpu_triangle_path() {
                        continue;
                    }
                    if cpu::primitive_needs_order_independent_transparency(primitive) {
                        cpu::draw_order_independent_transparency_cpu(
                            &mut cpu_frame,
                            primitive,
                            &clipping_planes,
                            section_box,
                            camera_projection,
                            &mut self.oit_scratch,
                            config,
                        );
                    } else {
                        cpu::draw_primitive_cpu(
                            &mut cpu_frame,
                            primitive,
                            &clipping_planes,
                            section_box,
                            camera_projection,
                        );
                    }
                }
                cpu::resolve_order_independent_transparency_cpu(&mut cpu_frame, &self.oit_scratch)
            } else {
                for primitive in &primitives {
                    if !primitive.gpu_triangle_path() {
                        continue;
                    }
                    cpu::draw_primitive_cpu(
                        &mut cpu_frame,
                        primitive,
                        &clipping_planes,
                        section_box,
                        camera_projection,
                    );
                }
                0
            };

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
            AntiAliasing::None => 0,
            AntiAliasing::Fxaa => {
                output::apply_fxaa_rgba8(self.target, &mut self.frame, &mut self.fxaa_scratch)
            }
        };

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
        Ok(())
    }
}
