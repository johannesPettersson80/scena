//! Parallel-path tests for screen-space reflections.
//!
//! Split out of `tests.rs` to keep each module inside the significant-line
//! budget; these share the same fixtures and assert the same invariant, that
//! the parallel path is byte-identical to the serial one.

use super::super::*;
use crate::ScreenSpaceReflectionConfig;
use crate::diagnostics::Backend;
use crate::geometry::{Primitive, Vertex};
use crate::material::Color;
use crate::render::prepare::{PreparedMaterialReflection, PreparedPrimitive};
use crate::scene::Scene;

/// Screen-space reflections used to force the CPU geometry pass onto one thread.
/// Recording is row-separable and the resolve runs once on the joined frame, so
/// the parallel path must produce the *same bytes*, not merely a similar image -
/// anything less is a silent quality regression traded for speed.
#[test]
fn cpu_parallel_row_bands_match_serial_output_with_screen_space_reflections() {
    let target = RasterTarget {
        width: 512,
        height: 512,
        backend: Backend::Headless,
    };
    let mut scene = Scene::new();
    let camera = scene.add_default_camera().expect("camera inserts");
    let camera_projection =
        camera::CameraProjection::from_scene(&scene, camera, target).expect("projection");
    let reflections = ScreenSpaceReflectionConfig::studio_floor();

    // Wide, overlapping, reflective quads spread down the frame so every row
    // band records reflection pixels and bands share primitives at their seams.
    let primitives = (0..96)
        .map(|index| {
            let y = -0.95 + (index as f32 / 95.0) * 1.9;
            let shade = 0.25 + (index % 7) as f32 / 12.0;
            PreparedPrimitive::new(
                Primitive::triangle([
                    Vertex {
                        position: crate::scene::Vec3::new(-0.6, y - 0.06, 0.0),
                        color: Color::from_linear_rgb(shade, shade * 0.8, shade * 0.6),
                    },
                    Vertex {
                        position: crate::scene::Vec3::new(0.6, y - 0.06, 0.0),
                        color: Color::from_linear_rgb(shade * 0.7, shade, shade * 0.9),
                    },
                    Vertex {
                        position: crate::scene::Vec3::new(0.0, y + 0.06, 0.0),
                        color: Color::from_linear_rgb(shade * 0.9, shade * 0.6, shade),
                    },
                ]),
                None,
                Color::WHITE,
            )
            .with_material_reflection(PreparedMaterialReflection::new(
                0.9,
                0.1 + (index % 5) as f32 / 20.0,
            ))
        })
        .collect::<Vec<_>>();
    assert!(
        primitives.len() >= 64,
        "the parallel policy requires at least 64 primitives to engage"
    );

    let mut serial_linear = vec![Color::BLACK; target.pixel_len()];
    let mut serial_depth = vec![f32::INFINITY; target.pixel_len()];
    let mut serial_frame = vec![0; target.byte_len()];
    let mut serial_oit = vec![cpu::OitAccumPixel::default(); target.pixel_len()];
    let mut serial_reflection_scratch = Vec::new();
    let mut serial_linear_scratch = Vec::new();

    let mut parallel_linear = vec![Color::BLACK; target.pixel_len()];
    let mut parallel_depth = vec![f32::INFINITY; target.pixel_len()];
    let mut parallel_frame = vec![0; target.byte_len()];
    let mut parallel_oit = vec![cpu::OitAccumPixel::default(); target.pixel_len()];
    let mut parallel_reflection_scratch = Vec::new();
    let mut parallel_linear_scratch = Vec::new();

    let mut serial_projection_cache = CpuRowBandBins::default();
    serial_projection_cache.rebuild(&primitives, target, &camera_projection, 1);
    let mut row_band_bins = CpuRowBandBins::default();
    let worker_count = cpu_geometry_worker_count(target);
    row_band_bins.rebuild(&primitives, target, &camera_projection, worker_count);

    draw_cpu_geometry_pass_serial(
        CpuGeometryPass {
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
            screen_space_reflections: Some(reflections),
            material_reflection_scratch: Some(&mut serial_reflection_scratch),
            material_reflection_rows: None,
            linear_scratch: Some(&mut serial_linear_scratch),
            row_band_bins: None,
            primitive_indices: None,
        },
        &serial_projection_cache.projected_primitives,
        CpuPrimitiveFlags::scan(&primitives),
    );

    draw_cpu_geometry_pass_parallel(
        CpuGeometryPass {
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
            screen_space_reflections: Some(reflections),
            material_reflection_scratch: Some(&mut parallel_reflection_scratch),
            material_reflection_rows: None,
            linear_scratch: Some(&mut parallel_linear_scratch),
            row_band_bins: None,
            primitive_indices: None,
        },
        &row_band_bins.projected_primitives,
        &row_band_bins,
        CpuPrimitiveFlags::scan(&primitives),
    );

    assert_eq!(
        serial_reflection_scratch, parallel_reflection_scratch,
        "every row band must record the same reflection pixels as the serial pass"
    );
    assert!(
        serial_reflection_scratch
            .iter()
            .any(|pixel| *pixel != screen_space_reflections::MaterialReflectionPixel::default()),
        "the fixture must actually record reflections, or this proves nothing"
    );
    assert_eq!(serial_depth, parallel_depth);
    assert_eq!(
        serial_linear, parallel_linear,
        "the resolved reflection frame must be identical, not merely similar"
    );
    assert_eq!(serial_frame, parallel_frame);
}

/// The policy half of the same change: SSR must no longer veto parallelism.
#[test]
fn screen_space_reflections_no_longer_serialize_the_cpu_geometry_pass() {
    let target = RasterTarget {
        width: 512,
        height: 512,
        backend: Backend::Headless,
    };
    if cpu_geometry_worker_count(target) <= 1 {
        return;
    }
    let mut scene = Scene::new();
    let camera = scene.add_default_camera().expect("camera inserts");
    let camera_projection =
        camera::CameraProjection::from_scene(&scene, camera, target).expect("projection");
    let primitives = (0..64)
        .map(|index| {
            let y = -0.9 + (index as f32 / 63.0) * 1.8;
            PreparedPrimitive::new(
                Primitive::triangle([
                    Vertex {
                        position: crate::scene::Vec3::new(-0.5, y - 0.02, 0.0),
                        color: Color::WHITE,
                    },
                    Vertex {
                        position: crate::scene::Vec3::new(0.5, y - 0.02, 0.0),
                        color: Color::WHITE,
                    },
                    Vertex {
                        position: crate::scene::Vec3::new(0.0, y + 0.02, 0.0),
                        color: Color::WHITE,
                    },
                ]),
                None,
                Color::WHITE,
            )
        })
        .collect::<Vec<_>>();
    let mut linear = vec![Color::BLACK; target.pixel_len()];
    let mut depth = vec![f32::INFINITY; target.pixel_len()];
    let mut frame = vec![0; target.byte_len()];
    let mut oit = vec![cpu::OitAccumPixel::default(); target.pixel_len()];
    let mut reflection_scratch = Vec::new();
    let mut linear_scratch = Vec::new();
    let mut bins = CpuRowBandBins::default();

    let input = CpuGeometryPass {
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
        linear_frame: &mut linear,
        depth_frame: &mut depth,
        frame: &mut frame,
        oit_scratch: &mut oit,
        screen_space_reflections: Some(ScreenSpaceReflectionConfig::studio_floor()),
        material_reflection_scratch: Some(&mut reflection_scratch),
        material_reflection_rows: None,
        linear_scratch: Some(&mut linear_scratch),
        row_band_bins: Some(&mut bins),
        primitive_indices: None,
    };
    let flags = CpuPrimitiveFlags::scan(&primitives);
    assert!(
        should_parallelize_cpu_geometry_pass(&input, flags),
        "a large reflective scene must still use every core"
    );
    let result = draw_cpu_geometry_pass(input);
    assert!(
        result.row_bands.workers > 1,
        "the SSR pass must report real parallel workers, got {}",
        result.row_bands.workers
    );
}
