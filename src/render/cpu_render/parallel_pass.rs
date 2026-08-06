use super::*;

#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn draw_cpu_geometry_pass_parallel(
    input: CpuGeometryPass<'_>,
    projected_primitives: &[cpu_geometry::CpuProjectedPrimitive],
    row_bands: &CpuRowBandBins,
    primitive_flags: CpuPrimitiveFlags,
) -> CpuGeometryPassResult {
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
    let linear_frame_len = linear_frame.len();
    let screen_space_reflections = input.screen_space_reflections;
    let mut linear_scratch = input.linear_scratch;

    // Reflections are recorded during rasterization, which is row-separable, and
    // resolved afterwards, which is not. Hand each worker its own band of the
    // frame-wide buffer and resolve once below, so enabling SSR no longer forces
    // the whole geometry pass onto one thread.
    let mut reflection_scratch = input.material_reflection_scratch;
    let reflection_bands = screen_space_reflections.map(|_| {
        let scratch = reflection_scratch
            .as_mut()
            .expect("parallel SSR pass receives prepared reflection scratch");
        resize_reusable_scratch(
            scratch,
            target.pixel_len(),
            screen_space_reflections::MaterialReflectionPixel::default(),
        );
        scratch.fill(screen_space_reflections::MaterialReflectionPixel::default());
        scratch
            .chunks_mut(chunk_pixels)
            .map(Some)
            .collect::<Vec<_>>()
    });

    // The band closure is shared, but the iterator chain is not: zipping a
    // per-band reflection slice needs a `Vec` of borrows, and building one on a
    // frame that has no reflections costs a heap allocation on the steady-state
    // render path, which `m9_parallel_cpu_render_has_low_steady_state_allocations`
    // budgets to the byte. The no-SSR chain therefore never builds it.
    let band = |chunk_index: usize,
                linear_frame: &mut [Color],
                depth_frame: &mut [f32],
                frame: &mut [u8],
                oit_scratch: &mut [cpu::OitAccumPixel],
                material_reflection_rows: Option<
        &mut [screen_space_reflections::MaterialReflectionPixel],
    >| {
        let row_start = chunk_index.saturating_mul(rows_per_worker) as u32;
        let row_count = (linear_frame.len() / width) as u32;
        draw_cpu_geometry_pass_serial(
            CpuGeometryPass {
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
                screen_space_reflections,
                material_reflection_scratch: None,
                material_reflection_rows,
                linear_scratch: None,
                row_band_bins: None,
                primitive_indices: Some(&row_bands.bands[chunk_index]),
            },
            projected_primitives,
            primitive_flags,
        )
    };
    let combine = |mut aggregate: CpuGeometryPassResult, result: CpuGeometryPassResult| {
        aggregate.oit_passes = aggregate.oit_passes.max(result.oit_passes);
        aggregate.output_pixels_encoded = aggregate
            .output_pixels_encoded
            .saturating_add(result.output_pixels_encoded);
        aggregate
    };

    let bands = linear_frame
        .par_chunks_mut(chunk_pixels)
        .zip(depth_frame.par_chunks_mut(chunk_pixels))
        .zip(frame.par_chunks_mut(chunk_bytes))
        .zip(oit_scratch.par_chunks_mut(chunk_pixels));
    let aggregate = match reflection_bands {
        Some(reflection_bands) => {
            debug_assert_eq!(
                reflection_bands.len(),
                linear_frame_len.div_ceil(chunk_pixels.max(1)),
                "every rasterized band must own exactly one reflection band"
            );
            bands
                .zip(reflection_bands.into_par_iter())
                .enumerate()
                .map(
                    |(
                        chunk_index,
                        (
                            (((linear_frame, depth_frame), frame), oit_scratch),
                            material_reflection_rows,
                        ),
                    )| {
                        band(
                            chunk_index,
                            linear_frame,
                            depth_frame,
                            frame,
                            oit_scratch,
                            material_reflection_rows,
                        )
                    },
                )
                .reduce(CpuGeometryPassResult::default, combine)
        }
        None => bands
            .enumerate()
            .map(
                |(chunk_index, (((linear_frame, depth_frame), frame), oit_scratch))| {
                    band(
                        chunk_index,
                        linear_frame,
                        depth_frame,
                        frame,
                        oit_scratch,
                        None,
                    )
                },
            )
            .reduce(CpuGeometryPassResult::default, combine),
    };
    if let (Some(config), Some(scratch)) = (screen_space_reflections, reflection_scratch) {
        let linear_scratch = linear_scratch
            .as_mut()
            .expect("parallel SSR pass receives prepared linear scratch");
        resize_reusable_scratch(linear_scratch, target.pixel_len(), Color::BLACK);
        screen_space_reflections::apply_material_linear(
            target,
            linear_frame,
            linear_scratch,
            scratch,
            config,
        );
    }

    CpuGeometryPassResult {
        oit_passes: aggregate.oit_passes,
        output_pixels_encoded: aggregate.output_pixels_encoded,
        primitive_flag_scan_items: 0,
        row_bands: CpuRowBandMetrics::default(),
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn draw_cpu_geometry_pass_parallel(
    _input: CpuGeometryPass<'_>,
    _projected_primitives: &[cpu_geometry::CpuProjectedPrimitive],
    _row_bands: &CpuRowBandBins,
    _primitive_flags: CpuPrimitiveFlags,
) -> CpuGeometryPassResult {
    unreachable!("WASM never selects the parallel CPU geometry path")
}
