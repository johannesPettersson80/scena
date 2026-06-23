use crate::diagnostics::RenderError;
use crate::material::Color;
use crate::scene::Scene;
use crate::scene::{ClippingPlane, SectionBox};

use super::output::OutputTransform;
use super::prepare::PreparedPrimitive;
use super::{
    AntiAliasing, RasterTarget, Renderer, camera, cpu, cpu_resolve, cpu_strokes, output,
    screen_space_reflections,
};

#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

const CPU_PARALLEL_MIN_PIXELS: usize = 512 * 512;
const CPU_PARALLEL_MIN_PRIMITIVES: usize = 64;
#[cfg(not(target_arch = "wasm32"))]
const CPU_PARALLEL_MAX_WORKERS: usize = 8;

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
            self.stats.order_independent_transparency_passes =
                draw_cpu_geometry_pass(CpuGeometryPass {
                    target: supersample_target,
                    output: self.output,
                    row_start: 0,
                    row_count: supersample_target.height,
                    background_color: self.background_color,
                    primitives: &primitives,
                    clipping_planes: &clipping_planes,
                    section_box,
                    camera_projection: &supersample_projection,
                    order_independent_transparency: self.order_independent_transparency,
                    linear_frame: &mut self.cpu_supersample_linear_frame,
                    depth_frame: &mut self.cpu_supersample_depth_frame,
                    frame: &mut self.cpu_supersample_frame,
                    oit_scratch: &mut self.cpu_supersample_oit_scratch,
                    screen_space_reflections: self.screen_space_reflections,
                });
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
            self.stats.order_independent_transparency_passes =
                draw_cpu_geometry_pass(CpuGeometryPass {
                    target: self.target,
                    output: self.output,
                    row_start: 0,
                    row_count: self.target.height,
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
                    screen_space_reflections: self.screen_space_reflections,
                });
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
}

fn draw_cpu_geometry_pass(input: CpuGeometryPass<'_>) -> u64 {
    if should_parallelize_cpu_geometry_pass(&input) {
        return draw_cpu_geometry_pass_parallel(input);
    }
    draw_cpu_geometry_pass_serial(input)
}

fn draw_cpu_geometry_pass_serial(input: CpuGeometryPass<'_>) -> u64 {
    debug_assert!(
        input.row_start == 0 || input.screen_space_reflections.is_none(),
        "row-scoped CPU geometry passes do not own the full material-reflection scratch buffer"
    );
    let mut material_reflections = input.screen_space_reflections.map(|_| {
        vec![screen_space_reflections::MaterialReflectionPixel::default(); input.target.pixel_len()]
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
                        material_reflections.as_deref_mut(),
                        input.screen_space_reflections,
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
                    material_reflections.as_deref_mut(),
                    input.screen_space_reflections,
                );
            }
            0
        }
    };

    if let (Some(config), Some(material_reflections)) = (
        input.screen_space_reflections,
        material_reflections.as_deref(),
    ) {
        let mut scratch = vec![0; input.target.byte_len()];
        screen_space_reflections::apply_material_rgba8(
            input.target,
            input.frame,
            &mut scratch,
            material_reflections,
            config,
        );
    }

    oit_passes
}

fn should_parallelize_cpu_geometry_pass(input: &CpuGeometryPass<'_>) -> bool {
    input.screen_space_reflections.is_none()
        && input.primitives.len() >= CPU_PARALLEL_MIN_PRIMITIVES
        && input.target.pixel_len() >= CPU_PARALLEL_MIN_PIXELS
        && cpu_geometry_worker_count(input.target) > 1
}

#[cfg(not(target_arch = "wasm32"))]
fn cpu_geometry_worker_count(target: RasterTarget) -> usize {
    std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .min(CPU_PARALLEL_MAX_WORKERS)
        .min(target.height as usize)
        .max(1)
}

#[cfg(target_arch = "wasm32")]
fn cpu_geometry_worker_count(_target: RasterTarget) -> usize {
    1
}

#[cfg(not(target_arch = "wasm32"))]
fn draw_cpu_geometry_pass_parallel(input: CpuGeometryPass<'_>) -> u64 {
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

    u64::from(
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
                    })
                },
            )
            .any(|passes| passes > 0),
    )
}

#[cfg(target_arch = "wasm32")]
fn draw_cpu_geometry_pass_parallel(input: CpuGeometryPass<'_>) -> u64 {
    draw_cpu_geometry_pass_serial(input)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::diagnostics::Backend;
    use crate::geometry::Primitive;
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
        let primitives = vec![PreparedPrimitive::new(
            Primitive::unlit_triangle(),
            None,
            Color::WHITE,
        )];

        let mut serial_linear = vec![Color::BLACK; target.pixel_len()];
        let mut serial_depth = vec![f32::INFINITY; target.pixel_len()];
        let mut serial_frame = vec![0; target.byte_len()];
        let mut serial_oit = vec![cpu::OitAccumPixel::default(); target.pixel_len()];

        let mut parallel_linear = vec![Color::BLACK; target.pixel_len()];
        let mut parallel_depth = vec![f32::INFINITY; target.pixel_len()];
        let mut parallel_frame = vec![0; target.byte_len()];
        let mut parallel_oit = vec![cpu::OitAccumPixel::default(); target.pixel_len()];

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
        });

        assert_eq!(serial_oit_passes, parallel_oit_passes);
        assert_eq!(serial_frame, parallel_frame);
        assert_eq!(serial_depth, parallel_depth);
        assert_eq!(serial_linear, parallel_linear);
    }
}
