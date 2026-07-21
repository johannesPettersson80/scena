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
    let aggregate = linear_frame
        .par_chunks_mut(chunk_pixels)
        .zip(depth_frame.par_chunks_mut(chunk_pixels))
        .zip(frame.par_chunks_mut(chunk_bytes))
        .zip(oit_scratch.par_chunks_mut(chunk_pixels))
        .enumerate()
        .map(
            |(chunk_index, (((linear_frame, depth_frame), frame), oit_scratch))| {
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
                        screen_space_reflections: None,
                        material_reflection_scratch: None,
                        rgba8_scratch: None,
                        row_band_bins: None,
                        primitive_indices: Some(&row_bands.bands[chunk_index]),
                    },
                    projected_primitives,
                    primitive_flags,
                )
            },
        )
        .reduce(CpuGeometryPassResult::default, |mut aggregate, result| {
            aggregate.oit_passes = aggregate.oit_passes.max(result.oit_passes);
            aggregate.output_pixels_encoded = aggregate
                .output_pixels_encoded
                .saturating_add(result.output_pixels_encoded);
            aggregate
        });
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
