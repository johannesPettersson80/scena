use crate::diagnostics::RenderError;
use crate::material::Color;
use crate::scene::Scene;
use crate::scene::{ClippingPlane, SectionBox};

use super::output::OutputTransform;
use super::prepare::PreparedPrimitive;
use super::state::PreparedSceneState;
use super::{
    AntiAliasing, RasterTarget, Renderer, camera, cpu, cpu_resolve, cpu_strokes, cpu_transmission,
    output, screen_space_reflections,
};

mod parallel_policy;
mod row_bands;
#[cfg(not(target_arch = "wasm32"))]
use parallel_policy::cpu_geometry_worker_count;
use parallel_policy::should_parallelize_cpu_geometry_pass;
pub(super) use row_bands::CpuRowBandBins;
use row_bands::{CpuRowBandMetrics, resize_reusable_scratch, selected_primitives};

#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

const CPU_PARALLEL_MIN_PIXELS: usize = 512 * 512;
const CPU_PARALLEL_MIN_PRIMITIVES: usize = 64;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct CpuGeometryPassResult {
    oit_passes: u64,
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
        let full_frame_supersample = self.supersample_factor > 1;
        let mut overlays_drawn_before_resolve = false;
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
                rgba8_scratch: Some(&mut self.cpu_effect_rgba8_scratch),
                row_band_bins: Some(&mut self.cpu_row_band_bins),
                primitive_indices: None,
            });
            self.record_cpu_geometry_result(geometry_result);
            if full_frame_supersample {
                let mut cpu_frame = cpu::CpuFrame::new(
                    supersample_target,
                    self.output,
                    &mut self.cpu_supersample_linear_frame,
                    &mut self.cpu_supersample_depth_frame,
                    &mut self.cpu_supersample_frame,
                );
                cpu_strokes::draw_overlay_layers_cpu(
                    &mut cpu_frame,
                    strokes,
                    labels,
                    clipping_planes,
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
                rgba8_scratch: Some(&mut self.cpu_effect_rgba8_scratch),
                row_band_bins: Some(&mut self.cpu_row_band_bins),
                primitive_indices: None,
            });
            self.record_cpu_geometry_result(geometry_result);
        }

        self.stats.screen_space_reflection_passes =
            self.screen_space_reflections.map_or(0, |config| {
                screen_space_reflections::apply_rgba8(
                    self.target,
                    &mut self.frame,
                    &mut self.bloom_scratch,
                    config,
                )
            });
        self.stats.ambient_occlusion_passes = match (
            self.screen_space_ambient_occlusion,
            self.depth_frame.as_ref(),
        ) {
            (Some(config), Some(depth_frame)) => {
                output::apply_screen_space_ambient_occlusion_rgba8(
                    self.target,
                    &mut self.frame,
                    &mut self.bloom_scratch,
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
            (Some(config), Some(depth_frame)) => output::apply_depth_of_field_rgba8(
                self.target,
                &mut self.frame,
                &mut self.bloom_scratch,
                depth_frame,
                config,
            ),
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
                strokes,
                labels,
                clipping_planes,
                section_box,
                camera_projection,
            );
        }
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
    rgba8_scratch: Option<&'a mut Vec<u8>>,
    row_band_bins: Option<&'a mut CpuRowBandBins>,
    primitive_indices: Option<&'a [usize]>,
}

fn draw_cpu_geometry_pass(input: CpuGeometryPass<'_>) -> CpuGeometryPassResult {
    if should_parallelize_cpu_geometry_pass(&input) {
        return draw_cpu_geometry_pass_parallel(input);
    }
    draw_cpu_geometry_pass_serial(input)
}

fn draw_cpu_geometry_pass_serial(mut input: CpuGeometryPass<'_>) -> CpuGeometryPassResult {
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
    let oit_passes = {
        let mut cpu_frame = cpu::CpuFrame::new_rows(
            input.target,
            input.output,
            input.row_start,
            input.row_count,
            input.linear_frame,
            input.depth_frame,
            input.frame,
        );
        cpu::clear_cpu(&mut cpu_frame, input.background_color);
        let has_physical_transmission = input
            .primitives
            .iter()
            .any(cpu::primitive_needs_physical_transmission);
        let oit_passes = if let Some(config) = input.order_independent_transparency {
            cpu::clear_order_independent_transparency(input.oit_scratch);
            for primitive in selected_primitives(input.primitives, input.primitive_indices) {
                if !primitive.gpu_triangle_path() {
                    continue;
                }
                if cpu::primitive_needs_physical_transmission(primitive) {
                    continue;
                } else if cpu::primitive_needs_order_independent_transparency(primitive) {
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
                        material_reflections.as_deref_mut(),
                        input.screen_space_reflections,
                    );
                }
            }
            cpu::resolve_order_independent_transparency_cpu(&mut cpu_frame, input.oit_scratch)
        } else {
            for primitive in selected_primitives(input.primitives, input.primitive_indices) {
                if !primitive.gpu_triangle_path() {
                    continue;
                }
                if cpu::primitive_needs_physical_transmission(primitive) {
                    continue;
                }
                cpu::draw_primitive_cpu(
                    &mut cpu_frame,
                    primitive,
                    input.clipping_planes,
                    input.section_box,
                    input.camera_projection,
                    material_reflections.as_deref_mut(),
                    input.screen_space_reflections,
                );
            }
            0
        };
        if has_physical_transmission {
            let scene_color_frame = input
                .rgba8_scratch
                .as_mut()
                .expect("serial transmission pass receives prepared RGBA scratch");
            resize_reusable_scratch(scene_color_frame, cpu_frame.frame.len(), 0);
            scene_color_frame.copy_from_slice(cpu_frame.frame);
            for primitive in selected_primitives(input.primitives, input.primitive_indices) {
                if !primitive.gpu_triangle_path()
                    || !cpu::primitive_needs_physical_transmission(primitive)
                {
                    continue;
                }
                cpu_transmission::draw_physical_transmission_cpu(
                    &mut cpu_frame,
                    primitive,
                    scene_color_frame,
                    input.clipping_planes,
                    input.section_box,
                    input.camera_projection,
                );
            }
        }
        oit_passes
    };

    if let (Some(config), Some(material_reflections)) = (
        input.screen_space_reflections,
        material_reflections.as_deref(),
    ) {
        let scratch = input
            .rgba8_scratch
            .as_mut()
            .expect("serial SSR pass receives prepared RGBA scratch");
        resize_reusable_scratch(scratch, input.target.byte_len(), 0);
        screen_space_reflections::apply_material_rgba8(
            input.target,
            input.frame,
            scratch,
            material_reflections,
            config,
        );
    }

    CpuGeometryPassResult {
        oit_passes,
        row_bands: CpuRowBandMetrics {
            workers: 1,
            candidate_triangles: input.primitives.len() as u64,
            full_rescan_triangles: input.primitives.len() as u64,
            storage_growth_bytes: 0,
        },
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn draw_cpu_geometry_pass_parallel(mut input: CpuGeometryPass<'_>) -> CpuGeometryPassResult {
    let worker_count = cpu_geometry_worker_count(input.target);
    let width = input.target.width as usize;
    let rows_per_worker = (input.target.height as usize).div_ceil(worker_count).max(1);
    let chunk_pixels = rows_per_worker.saturating_mul(width);
    let chunk_bytes = chunk_pixels.saturating_mul(4);
    let target = input.target;
    let output = input.output;
    let background_color = input.background_color;
    let primitives = input.primitives;
    let clipping_planes = input.clipping_planes;
    let section_box = input.section_box;
    let camera_projection = input.camera_projection;
    let order_independent_transparency = input.order_independent_transparency;
    let linear_frame = input.linear_frame;
    let depth_frame = input.depth_frame;
    let frame = input.frame;
    let oit_scratch = input.oit_scratch;
    let row_band_metrics = input
        .row_band_bins
        .as_deref_mut()
        .expect("parallel CPU raster receives retained row-bin scratch")
        .rebuild(primitives, target, camera_projection, worker_count);
    let row_bands = input
        .row_band_bins
        .as_deref()
        .expect("row-bin scratch remains available after rebuild");

    let oit_passes = u64::from(
        linear_frame
            .par_chunks_mut(chunk_pixels)
            .zip(depth_frame.par_chunks_mut(chunk_pixels))
            .zip(frame.par_chunks_mut(chunk_bytes))
            .zip(oit_scratch.par_chunks_mut(chunk_pixels))
            .enumerate()
            .map(
                |(chunk_index, (((linear_frame, depth_frame), frame), oit_scratch))| {
                    let row_start = chunk_index.saturating_mul(rows_per_worker) as u32;
                    let row_count = (linear_frame.len() / width) as u32;
                    draw_cpu_geometry_pass_serial(CpuGeometryPass {
                        target,
                        output,
                        row_start,
                        row_count,
                        background_color,
                        primitives,
                        clipping_planes,
                        section_box,
                        camera_projection,
                        order_independent_transparency,
                        linear_frame,
                        depth_frame,
                        frame,
                        oit_scratch,
                        screen_space_reflections: None,
                        material_reflection_scratch: None,
                        rgba8_scratch: None,
                        row_band_bins: None,
                        primitive_indices: Some(&row_bands.bands[chunk_index]),
                    })
                },
            )
            .any(|result| result.oit_passes > 0),
    );
    CpuGeometryPassResult {
        oit_passes,
        row_bands: row_band_metrics,
    }
}

#[cfg(target_arch = "wasm32")]
fn draw_cpu_geometry_pass_parallel(input: CpuGeometryPass<'_>) -> CpuGeometryPassResult {
    draw_cpu_geometry_pass_serial(input)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::diagnostics::Backend;
    use crate::geometry::{Primitive, Vertex};
    use crate::material::Color;
    use crate::render::prepare::PreparedPrimitive;
    use crate::scene::Scene;

    #[test]
    fn cpu_parallel_row_bands_match_serial_opaque_output() {
        let target = RasterTarget {
            width: 640,
            height: 480,
            backend: Backend::Headless,
        };
        let mut scene = Scene::new();
        let camera = scene.add_default_camera().expect("camera inserts");
        let camera_projection =
            camera::CameraProjection::from_scene(&scene, camera, target).expect("projection");
        let primitives = (0..256)
            .map(|index| {
                let y = -0.95 + (index as f32 / 255.0) * 1.9;
                PreparedPrimitive::new(
                    Primitive::triangle([
                        Vertex {
                            position: crate::scene::Vec3::new(-0.01, y - 0.01, 0.0),
                            color: Color::WHITE,
                        },
                        Vertex {
                            position: crate::scene::Vec3::new(0.01, y - 0.01, 0.0),
                            color: Color::WHITE,
                        },
                        Vertex {
                            position: crate::scene::Vec3::new(0.0, y + 0.01, 0.0),
                            color: Color::WHITE,
                        },
                    ]),
                    None,
                    Color::WHITE,
                )
            })
            .collect::<Vec<_>>();

        let mut serial_linear = vec![Color::BLACK; target.pixel_len()];
        let mut serial_depth = vec![f32::INFINITY; target.pixel_len()];
        let mut serial_frame = vec![0; target.byte_len()];
        let mut serial_oit = vec![cpu::OitAccumPixel::default(); target.pixel_len()];

        let mut parallel_linear = vec![Color::BLACK; target.pixel_len()];
        let mut parallel_depth = vec![f32::INFINITY; target.pixel_len()];
        let mut parallel_frame = vec![0; target.byte_len()];
        let mut parallel_oit = vec![cpu::OitAccumPixel::default(); target.pixel_len()];
        let mut row_band_bins = CpuRowBandBins::default();

        let serial_oit_passes = draw_cpu_geometry_pass_serial(CpuGeometryPass {
            target,
            output: OutputTransform::default(),
            row_start: 0,
            row_count: target.height,
            background_color: Color::BLACK,
            primitives: &primitives,
            clipping_planes: &[],
            section_box: None,
            camera_projection: &camera_projection,
            order_independent_transparency: None,
            linear_frame: &mut serial_linear,
            depth_frame: &mut serial_depth,
            frame: &mut serial_frame,
            oit_scratch: &mut serial_oit,
            screen_space_reflections: None,
            material_reflection_scratch: None,
            rgba8_scratch: None,
            row_band_bins: None,
            primitive_indices: None,
        });

        let parallel_oit_passes = draw_cpu_geometry_pass_parallel(CpuGeometryPass {
            target,
            output: OutputTransform::default(),
            row_start: 0,
            row_count: target.height,
            background_color: Color::BLACK,
            primitives: &primitives,
            clipping_planes: &[],
            section_box: None,
            camera_projection: &camera_projection,
            order_independent_transparency: None,
            linear_frame: &mut parallel_linear,
            depth_frame: &mut parallel_depth,
            frame: &mut parallel_frame,
            oit_scratch: &mut parallel_oit,
            screen_space_reflections: None,
            material_reflection_scratch: None,
            rgba8_scratch: None,
            row_band_bins: Some(&mut row_band_bins),
            primitive_indices: None,
        });

        assert_eq!(serial_oit_passes.oit_passes, parallel_oit_passes.oit_passes);
        assert_eq!(serial_frame, parallel_frame);
        assert_eq!(serial_depth, parallel_depth);
        assert_eq!(serial_linear, parallel_linear);
    }

    #[test]
    fn pf10_reusable_effect_scratch_has_zero_warm_capacity_growth() {
        let mut rgba8 = Vec::new();
        let mut reflections = Vec::new();
        assert!(resize_reusable_scratch(&mut rgba8, 4_096, 0_u8) >= 4_096);
        assert!(
            resize_reusable_scratch(
                &mut reflections,
                1_024,
                screen_space_reflections::MaterialReflectionPixel::default(),
            ) > 0
        );
        let rgba8_capacity = rgba8.capacity();
        let reflection_capacity = reflections.capacity();

        assert_eq!(resize_reusable_scratch(&mut rgba8, 4_096, 0_u8), 0);
        assert_eq!(
            resize_reusable_scratch(
                &mut reflections,
                1_024,
                screen_space_reflections::MaterialReflectionPixel::default(),
            ),
            0
        );
        assert_eq!(rgba8.capacity(), rgba8_capacity);
        assert_eq!(reflections.capacity(), reflection_capacity);
    }

    #[test]
    fn pf09_row_band_bins_reduce_candidate_scans_and_preserve_order() {
        let target = RasterTarget {
            width: 640,
            height: 480,
            backend: Backend::Headless,
        };
        let mut scene = Scene::new();
        let camera = scene.add_default_camera().expect("camera inserts");
        let projection =
            camera::CameraProjection::from_scene(&scene, camera, target).expect("projection");
        let primitives = (0..256)
            .map(|index| {
                let y = -0.95 + (index as f32 / 255.0) * 1.9;
                let primitive = Primitive::triangle([
                    Vertex {
                        position: crate::scene::Vec3::new(-0.01, y - 0.01, 0.0),
                        color: Color::WHITE,
                    },
                    Vertex {
                        position: crate::scene::Vec3::new(0.01, y - 0.01, 0.0),
                        color: Color::WHITE,
                    },
                    Vertex {
                        position: crate::scene::Vec3::new(0.0, y + 0.01, 0.0),
                        color: Color::WHITE,
                    },
                ]);
                PreparedPrimitive::new(primitive, None, Color::WHITE)
            })
            .collect::<Vec<_>>();
        let mut bins = CpuRowBandBins::default();

        let metrics = bins.rebuild(&primitives, target, &projection, 8);

        assert_eq!(bins.band_count(), 8);
        assert!(
            metrics.candidate_triangles < metrics.full_rescan_triangles / 2,
            "screen-row bins must avoid rescanning all triangles in every band: {metrics:?}"
        );
        for band in bins.bands() {
            assert!(
                band.windows(2).all(|pair| pair[0] < pair[1]),
                "every band must retain source triangle ordering"
            );
        }
        let capacities = bins.capacities();
        let second = bins.rebuild(&primitives, target, &projection, 8);
        assert_eq!(
            bins.capacities(),
            capacities,
            "warm rebuild reuses bin storage"
        );
        assert_eq!(second.storage_growth_bytes, 0);
    }
}
