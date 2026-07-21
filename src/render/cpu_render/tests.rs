use super::test_support::occupy_all_but_one_rayon_worker;
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
    let mut serial_projection_cache = CpuRowBandBins::default();
    serial_projection_cache.rebuild(&primitives, target, &camera_projection, 1);
    let mut row_band_bins = CpuRowBandBins::default();
    row_band_bins.rebuild(
        &primitives,
        target,
        &camera_projection,
        cpu_geometry_worker_count(target),
    );

    let serial_oit_passes = draw_cpu_geometry_pass_serial(
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
            screen_space_reflections: None,
            material_reflection_scratch: None,
            rgba8_scratch: None,
            row_band_bins: None,
            primitive_indices: None,
        },
        &serial_projection_cache.projected_primitives,
        CpuPrimitiveFlags::scan(&primitives),
    );

    let parallel_oit_passes = draw_cpu_geometry_pass_parallel(
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
            screen_space_reflections: None,
            material_reflection_scratch: None,
            rgba8_scratch: None,
            row_band_bins: None,
            primitive_indices: None,
        },
        &row_band_bins.projected_primitives,
        &row_band_bins,
        CpuPrimitiveFlags::scan(&primitives),
    );

    assert_eq!(serial_oit_passes.oit_passes, parallel_oit_passes.oit_passes);
    assert_eq!(serial_frame, parallel_frame);
    assert_eq!(serial_depth, parallel_depth);
    assert_eq!(serial_linear, parallel_linear);
}

#[test]
fn cpu_parallel_oit_completes_every_row_band_when_rayon_is_contended() {
    let target = RasterTarget {
        width: 128,
        height: 128,
        backend: Backend::Headless,
    };
    assert!(
        cpu_geometry_worker_count(target) > 1,
        "focused regression requires more than one CPU row band"
    );
    let mut scene = Scene::new();
    let camera = scene.add_default_camera().expect("camera inserts");
    let camera_projection =
        camera::CameraProjection::from_scene(&scene, camera, target).expect("projection");
    let translucent = Color::from_linear_rgba(1.0, 0.15, 0.05, 0.45);
    let mut primitives = (0..64)
        .map(|_| {
            PreparedPrimitive::new(
                Primitive::triangle([
                    Vertex {
                        position: crate::scene::Vec3::new(-2.0, -2.0, 0.0),
                        color: translucent,
                    },
                    Vertex {
                        position: crate::scene::Vec3::new(2.0, -2.0, 0.0),
                        color: translucent,
                    },
                    Vertex {
                        position: crate::scene::Vec3::new(0.0, 2.0, 0.0),
                        color: translucent,
                    },
                ]),
                None,
                Color::WHITE,
            )
        })
        .collect::<Vec<_>>();
    primitives.push(PreparedPrimitive::new(
        Primitive::triangle([
            Vertex {
                position: crate::scene::Vec3::new(-0.8, -0.4, -0.05),
                color: Color::CYAN,
            },
            Vertex {
                position: crate::scene::Vec3::new(0.8, -0.4, -0.05),
                color: Color::CYAN,
            },
            Vertex {
                position: crate::scene::Vec3::new(0.0, 0.8, -0.05),
                color: Color::CYAN,
            },
        ]),
        None,
        Color::WHITE,
    ));
    let oit = Some(super::super::OrderIndependentTransparencyConfig::weighted_blended());

    let mut serial_linear = vec![Color::MAGENTA; target.pixel_len()];
    let mut serial_depth = vec![-123.0; target.pixel_len()];
    let mut serial_frame = vec![0xA5; target.byte_len()];
    let mut serial_oit = vec![cpu::OitAccumPixel::default(); target.pixel_len()];
    let mut serial_projection_cache = CpuRowBandBins::default();
    serial_projection_cache.rebuild(&primitives, target, &camera_projection, 1);
    let serial_result = draw_cpu_geometry_pass_serial(
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
            order_independent_transparency: oit,
            linear_frame: &mut serial_linear,
            depth_frame: &mut serial_depth,
            frame: &mut serial_frame,
            oit_scratch: &mut serial_oit,
            screen_space_reflections: None,
            material_reflection_scratch: None,
            rgba8_scratch: None,
            row_band_bins: None,
            primitive_indices: None,
        },
        &serial_projection_cache.projected_primitives,
        CpuPrimitiveFlags::scan(&primitives),
    );
    assert_eq!(serial_result.oit_passes, 1, "fixture must exercise OIT");

    let mut parallel_linear = vec![Color::MAGENTA; target.pixel_len()];
    let mut parallel_depth = vec![-123.0; target.pixel_len()];
    let mut parallel_frame = vec![0xA5; target.byte_len()];
    let mut parallel_oit = vec![cpu::OitAccumPixel::default(); target.pixel_len()];
    let mut row_band_bins = CpuRowBandBins::default();
    row_band_bins.rebuild(
        &primitives,
        target,
        &camera_projection,
        cpu_geometry_worker_count(target),
    );
    let blockers = occupy_all_but_one_rayon_worker();
    let parallel_result = draw_cpu_geometry_pass_parallel(
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
            order_independent_transparency: oit,
            linear_frame: &mut parallel_linear,
            depth_frame: &mut parallel_depth,
            frame: &mut parallel_frame,
            oit_scratch: &mut parallel_oit,
            screen_space_reflections: None,
            material_reflection_scratch: None,
            rgba8_scratch: None,
            row_band_bins: None,
            primitive_indices: None,
        },
        &row_band_bins.projected_primitives,
        &row_band_bins,
        CpuPrimitiveFlags::scan(&primitives),
    );
    drop(blockers);

    assert_eq!(parallel_result.oit_passes, serial_result.oit_passes);
    assert_eq!(parallel_frame, serial_frame, "every row band must be drawn");
    assert_eq!(
        parallel_depth, serial_depth,
        "every row band must be cleared"
    );
    assert_eq!(
        parallel_linear, serial_linear,
        "every row band must resolve OIT"
    );
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

    assert_eq!(bins.band_count(), metrics.workers as usize);
    assert_eq!(
        metrics.full_rescan_triangles,
        primitives.len() as u64 * metrics.workers
    );
    match metrics.workers {
        1 => assert_eq!(
            metrics.candidate_triangles, metrics.full_rescan_triangles,
            "one worker has no cross-band rescans to eliminate"
        ),
        2 => assert!(
            metrics.candidate_triangles < metrics.full_rescan_triangles,
            "two row bands must reduce candidate scans: {metrics:?}"
        ),
        _ => assert!(
            metrics.candidate_triangles < metrics.full_rescan_triangles / 2,
            "three or more row bands must avoid at least half of full rescans: {metrics:?}"
        ),
    }
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
