use crate::diagnostics::RenderError;
use crate::material::Color;
use crate::scene::Scene;
use crate::scene::{ClippingPlane, SectionBox};

use super::output::OutputTransform;
use super::prepare::PreparedPrimitive;
use super::state::PreparedSceneState;
use super::{
    AntiAliasing, RasterTarget, Renderer, camera, cpu, cpu_geometry, cpu_resolve, cpu_strokes,
    cpu_transmission, output, screen_space_reflections,
};

mod parallel_pass;
mod parallel_policy;
mod row_bands;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
use parallel_pass::draw_cpu_geometry_pass_parallel;
#[cfg(not(target_arch = "wasm32"))]
use parallel_policy::cpu_geometry_worker_count;
use parallel_policy::{CpuPrimitiveFlags, should_parallelize_cpu_geometry_pass};
pub(super) use row_bands::CpuRowBandBins;
use row_bands::{CpuRowBandMetrics, resize_reusable_scratch, selected_primitives};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct CpuGeometryPassResult {
    oit_passes: u64,
    output_pixels_encoded: u64,
    primitive_flag_scan_items: u64,
    row_bands: CpuRowBandMetrics,
}

impl Renderer {
    pub(super) fn draw_cpu(
        &mut self,
        scene: &Scene,
        camera: crate::scene::CameraKey,
        camera_projection: &camera::CameraProjection,
    ) -> Result<(), RenderError> {
        self.prepared_state(scene)?;
        // Move the prepared owner out temporarily so mutable frame buffers and
        // immutable prepared lists can be borrowed independently. Moving the
        // Vec/atlas owners is allocation-free and preserves their backing
        // storage; the state is restored on every Result path below.
        let prepared = self
            .prepared
            .take()
            .expect("prepared_state verified a prepared owner");
        let result = self.draw_cpu_from_prepared(scene, camera, camera_projection, &prepared);
        self.prepared = Some(prepared);
        result
    }

    fn draw_cpu_from_prepared(
        &mut self,
        scene: &Scene,
        camera: crate::scene::CameraKey,
        camera_projection: &camera::CameraProjection,
        prepared: &PreparedSceneState,
    ) -> Result<(), RenderError> {
        let primitives = &prepared.primitives;
        let strokes = &prepared.strokes;
        let labels = &prepared.labels;
        let clipping_planes = &prepared.clipping_planes;
        let section_box = prepared.section_box;
        let scale = self
            .anti_aliasing
            .cpu_supersample_scale()
            .max(self.supersample_factor);
        if scale > 1 {
            let supersample_target =
                super::target::validate_supersample_target(self.target, scale)?;
            let supersample_projection =
                camera::CameraProjection::from_scene(scene, camera, supersample_target)?;
            self.cpu_supersample_linear_frame
                .resize(supersample_target.pixel_len(), Color::BLACK);
            self.cpu_supersample_depth_frame
                .resize(supersample_target.pixel_len(), f32::INFINITY);
            self.cpu_supersample_frame
                .resize(supersample_target.byte_len(), 0);
            self.cpu_supersample_oit_scratch.resize(
                supersample_target.pixel_len(),
                cpu::OitAccumPixel::default(),
            );
            let geometry_result = draw_cpu_geometry_pass(CpuGeometryPass {
                target: supersample_target,
                output: self.output,
                row_start: 0,
                row_count: supersample_target.height,
                background_color: self.background_color,
                primitives,
                clipping_planes,
                section_box,
                camera_projection: &supersample_projection,
                order_independent_transparency: self.order_independent_transparency,
                linear_frame: &mut self.cpu_supersample_linear_frame,
                depth_frame: &mut self.cpu_supersample_depth_frame,
                frame: &mut self.cpu_supersample_frame,
                oit_scratch: &mut self.cpu_supersample_oit_scratch,
                screen_space_reflections: self.screen_space_reflections,
                material_reflection_scratch: Some(&mut self.cpu_material_reflection_scratch),
                linear_scratch: Some(&mut self.cpu_effect_linear_scratch),
                row_band_bins: Some(&mut self.cpu_row_band_bins),
                primitive_indices: None,
            });
            self.record_cpu_geometry_result(geometry_result);
            let linear_frame = self
                .linear_frame
                .as_mut()
                .expect("CPU renderer owns a linear accumulator");
            let depth_frame = self
                .depth_frame
                .as_mut()
                .expect("CPU renderer owns a depth buffer");
            cpu_resolve::downsample_cpu_supersample(
                supersample_target,
                scale,
                &self.cpu_supersample_linear_frame,
                &self.cpu_supersample_depth_frame,
                &self.cpu_supersample_frame,
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
            let geometry_result = draw_cpu_geometry_pass(CpuGeometryPass {
                target: self.target,
                output: self.output,
                row_start: 0,
                row_count: self.target.height,
                background_color: self.background_color,
                primitives,
                clipping_planes,
                section_box,
                camera_projection,
                order_independent_transparency: self.order_independent_transparency,
                linear_frame,
                depth_frame,
                frame: &mut self.frame,
                oit_scratch: &mut self.oit_scratch,
                screen_space_reflections: self.screen_space_reflections,
                material_reflection_scratch: Some(&mut self.cpu_material_reflection_scratch),
                linear_scratch: Some(&mut self.cpu_effect_linear_scratch),
                row_band_bins: Some(&mut self.cpu_row_band_bins),
                primitive_indices: None,
            });
            self.record_cpu_geometry_result(geometry_result);
        }

        let linear_frame = self
            .linear_frame
            .as_mut()
            .expect("CPU renderer owns a linear accumulator");
        self.cpu_meter_linear_frame.clear();
        self.cpu_meter_linear_frame.extend_from_slice(linear_frame);
        resize_reusable_scratch(
            &mut self.cpu_effect_linear_scratch,
            self.target.pixel_len(),
            Color::BLACK,
        );
        resize_reusable_scratch(
            &mut self.cpu_effect_linear_scratch_2,
            self.target.pixel_len(),
            Color::BLACK,
        );
        self.stats.screen_space_reflection_passes =
            self.screen_space_reflections.map_or(0, |config| {
                screen_space_reflections::apply_linear(
                    self.target,
                    linear_frame,
                    &mut self.cpu_effect_linear_scratch,
                    config,
                )
            });
        self.stats.ambient_occlusion_passes = match (
            self.screen_space_ambient_occlusion,
            self.depth_frame.as_ref(),
        ) {
            (Some(config), Some(depth_frame)) => {
                output::apply_screen_space_ambient_occlusion_linear(
                    self.target,
                    linear_frame,
                    &mut self.cpu_effect_linear_scratch,
                    depth_frame,
                    config,
                )
            }
            _ => 0,
        };
        self.stats.depth_of_field_passes = match (
            super::depth_of_field_post_config(self.depth_of_field, camera_projection),
            self.depth_frame.as_ref(),
        ) {
            (Some(config), Some(depth_frame)) => output::apply_depth_of_field_linear(
                self.target,
                linear_frame,
                &mut self.cpu_effect_linear_scratch,
                depth_frame,
                config,
            ),
            _ => 0,
        };
        self.stats.bloom_passes = self.bloom.map_or(0, |bloom| {
            output::apply_bloom_linear(
                self.target,
                linear_frame,
                &mut self.cpu_effect_linear_scratch,
                &mut self.cpu_effect_linear_scratch_2,
                bloom,
            )
        });
        let post_enabled = self.bloom.is_some()
            || self.screen_space_ambient_occlusion.is_some()
            || self.screen_space_reflections.is_some()
            || self.depth_of_field.is_some()
            || self.anti_aliasing.uses_post_fxaa();
        {
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
            self.last_render_work_metrics.cpu_output_pixels_encoded =
                cpu::encode_cpu_frame(&mut cpu_frame, post_enabled);
        }
        self.stats.fxaa_passes = match self.anti_aliasing {
            AntiAliasing::None | AntiAliasing::Msaa4 | AntiAliasing::Msaa8 => 0,
            AntiAliasing::Fxaa => {
                output::apply_fxaa_rgba8(self.target, &mut self.frame, &mut self.fxaa_scratch)
            }
        };
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
            strokes,
            labels,
            clipping_planes,
            section_box,
            camera_projection,
        );
        Ok(())
    }

    fn record_cpu_geometry_result(&mut self, result: CpuGeometryPassResult) {
        self.stats.order_independent_transparency_passes = result.oit_passes;
        self.last_render_work_metrics.cpu_parallel_workers = result.row_bands.workers;
        self.last_render_work_metrics.cpu_raster_candidate_triangles =
            result.row_bands.candidate_triangles;
        self.last_render_work_metrics
            .cpu_raster_full_rescan_triangles = result.row_bands.full_rescan_triangles;
        self.last_render_work_metrics
            .cpu_raster_bin_storage_growth_bytes = result.row_bands.storage_growth_bytes;
        self.last_render_work_metrics.cpu_output_pixels_encoded = result.output_pixels_encoded;
        self.last_render_work_metrics.cpu_primitive_flag_scan_items =
            result.primitive_flag_scan_items;
    }
}

struct CpuGeometryPass<'a> {
    target: RasterTarget,
    output: OutputTransform,
    row_start: u32,
    row_count: u32,
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
    screen_space_reflections: Option<super::ScreenSpaceReflectionConfig>,
    material_reflection_scratch:
        Option<&'a mut Vec<screen_space_reflections::MaterialReflectionPixel>>,
    linear_scratch: Option<&'a mut Vec<Color>>,
    row_band_bins: Option<&'a mut CpuRowBandBins>,
    primitive_indices: Option<&'a [usize]>,
}

fn draw_cpu_geometry_pass(input: CpuGeometryPass<'_>) -> CpuGeometryPassResult {
    let primitive_flag_scan_items = input.primitives.len() as u64;
    let primitive_flags = CpuPrimitiveFlags::scan(input.primitives);
    let parallel = should_parallelize_cpu_geometry_pass(&input, primitive_flags);
    #[cfg(not(target_arch = "wasm32"))]
    let worker_count = if parallel {
        cpu_geometry_worker_count(input.target)
    } else {
        1
    };
    #[cfg(target_arch = "wasm32")]
    let worker_count = 1;
    let mut input = input;
    let bins = input
        .row_band_bins
        .take()
        .expect("CPU geometry pass receives retained projection/bin scratch");
    let row_bands = bins.rebuild(
        input.primitives,
        input.target,
        input.camera_projection,
        worker_count,
    );
    let projected = &bins.projected_primitives;
    let mut result = if parallel {
        draw_cpu_geometry_pass_parallel(input, projected, &*bins, primitive_flags)
    } else {
        draw_cpu_geometry_pass_serial(input, projected, primitive_flags)
    };
    result.row_bands = row_bands;
    result.primitive_flag_scan_items = primitive_flag_scan_items;
    result
}

fn draw_cpu_geometry_pass_serial(
    mut input: CpuGeometryPass<'_>,
    projected_primitives: &[cpu_geometry::CpuProjectedPrimitive],
    primitive_flags: CpuPrimitiveFlags,
) -> CpuGeometryPassResult {
    debug_assert!(
        input.row_start == 0 || input.screen_space_reflections.is_none(),
        "row-scoped CPU geometry passes do not own the full material-reflection scratch buffer"
    );
    let mut material_reflections = input.screen_space_reflections.map(|_| {
        let scratch = input
            .material_reflection_scratch
            .as_mut()
            .expect("serial SSR pass receives prepared reflection scratch");
        resize_reusable_scratch(
            scratch,
            input.target.pixel_len(),
            screen_space_reflections::MaterialReflectionPixel::default(),
        );
        scratch.fill(screen_space_reflections::MaterialReflectionPixel::default());
        &mut scratch[..]
    });
    let (oit_passes, output_pixels_encoded) = {
        let mut cpu_frame = cpu::CpuFrame::new_rows(
            input.target,
            input.output,
            input.row_start,
            input.row_count,
            input.linear_frame,
            input.depth_frame,
            input.frame,
        );
        let raster_context = cpu::CpuTriangleClipInputs {
            clipping_planes: input.clipping_planes,
            section_box: input.section_box,
            camera: input.camera_projection,
        };
        cpu::clear_cpu(&mut cpu_frame, input.background_color);
        let oit_passes = if let Some(config) = input.order_independent_transparency {
            cpu::clear_order_independent_transparency(input.oit_scratch);
            for (primitive, projected) in selected_primitives(
                input.primitives,
                projected_primitives,
                input.primitive_indices,
            ) {
                if !primitive.gpu_triangle_path() {
                    continue;
                }
                if cpu::primitive_needs_physical_transmission(primitive) {
                    continue;
                } else if cpu::primitive_needs_order_independent_transparency(primitive) {
                    cpu::draw_order_independent_transparency_cpu(
                        &mut cpu_frame,
                        primitive,
                        projected,
                        raster_context.for_primitive(primitive),
                        input.oit_scratch,
                        config,
                    );
                } else {
                    cpu::draw_primitive_cpu(
                        &mut cpu_frame,
                        primitive,
                        projected,
                        raster_context.for_primitive(primitive),
                        material_reflections.as_deref_mut(),
                        input.screen_space_reflections,
                    );
                }
            }
            cpu::resolve_order_independent_transparency_cpu(&mut cpu_frame, input.oit_scratch)
        } else {
            for (primitive, projected) in selected_primitives(
                input.primitives,
                projected_primitives,
                input.primitive_indices,
            ) {
                if !primitive.gpu_triangle_path() {
                    continue;
                }
                if cpu::primitive_needs_physical_transmission(primitive) {
                    continue;
                }
                cpu::draw_primitive_cpu(
                    &mut cpu_frame,
                    primitive,
                    projected,
                    raster_context.for_primitive(primitive),
                    material_reflections.as_deref_mut(),
                    input.screen_space_reflections,
                );
            }
            0
        };
        if primitive_flags.has_physical_transmission {
            let scene_color_frame = input
                .linear_scratch
                .as_mut()
                .expect("serial transmission pass receives prepared linear scratch");
            resize_reusable_scratch(
                scene_color_frame,
                cpu_frame.linear_frame.len(),
                Color::BLACK,
            );
            scene_color_frame.copy_from_slice(cpu_frame.linear_frame);
            for (primitive, projected) in selected_primitives(
                input.primitives,
                projected_primitives,
                input.primitive_indices,
            ) {
                if !primitive.gpu_triangle_path()
                    || !cpu::primitive_needs_physical_transmission(primitive)
                {
                    continue;
                }
                cpu_transmission::draw_physical_transmission_cpu(
                    &mut cpu_frame,
                    primitive,
                    projected,
                    scene_color_frame,
                    raster_context.for_primitive(primitive),
                );
            }
            (oit_passes, 0)
        } else {
            (oit_passes, 0)
        }
    };

    if let (Some(config), Some(material_reflections)) = (
        input.screen_space_reflections,
        material_reflections.as_deref(),
    ) {
        let scratch = input
            .linear_scratch
            .as_mut()
            .expect("serial SSR pass receives prepared linear scratch");
        resize_reusable_scratch(scratch, input.target.pixel_len(), Color::BLACK);
        screen_space_reflections::apply_material_linear(
            input.target,
            input.linear_frame,
            scratch,
            material_reflections,
            config,
        );
    }

    CpuGeometryPassResult {
        oit_passes,
        output_pixels_encoded,
        primitive_flag_scan_items: 0,
        row_bands: CpuRowBandMetrics {
            workers: 1,
            candidate_triangles: input.primitives.len() as u64,
            full_rescan_triangles: input.primitives.len() as u64,
            storage_growth_bytes: 0,
        },
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
