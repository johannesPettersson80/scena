#[cfg(not(target_arch = "wasm32"))]
use super::instancing::INSTANCE_BYTE_LEN;
#[cfg(not(target_arch = "wasm32"))]
use super::pipeline::GPU_COLOR_FORMAT;
#[cfg(not(target_arch = "wasm32"))]
use super::resource_encoding::retained_instance_buffer_capacity;
#[cfg(not(target_arch = "wasm32"))]
use super::{GpuDeviceState, GpuOutputPlan};
use crate::PrepareError;
use crate::diagnostics::Backend;
#[cfg(not(target_arch = "wasm32"))]
use crate::render::RasterTarget;
#[cfg(not(target_arch = "wasm32"))]
use crate::render::prepare::{
    PreparedInstanceSet, PreparedLabelAtlas, PreparedPrimitive, PreparedStrokeSegment,
};
#[cfg(not(target_arch = "wasm32"))]
use crate::render::semantic_aov::GpuSemanticAttribution;

#[cfg(not(target_arch = "wasm32"))]
pub(super) struct GeometryBuffers {
    pub(super) vertex_buffer: wgpu::Buffer,
    pub(super) vertex_buffer_size: u64,
    pub(super) instance_buffer: wgpu::Buffer,
    pub(super) instance_buffer_size: u64,
    pub(super) instance_buffer_capacity: usize,
}

pub(super) fn validate_geometry_buffer_size(
    backend: Backend,
    requested: u64,
    maximum: u64,
) -> Result<(), PrepareError> {
    if requested <= maximum {
        return Ok(());
    }
    Err(PrepareError::GpuResourceUpload {
        backend,
        reason: format!(
            "prepared geometry requires {requested} vertex-buffer bytes, exceeding device max_buffer_size {maximum}"
        ),
    })
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(super) const fn browser_depth_prepass_required(
    requested: bool,
    depth_color_requested: bool,
    rasterized_triangle_instances: u64,
    has_depth_tested_overlays: bool,
) -> bool {
    requested
        && (depth_color_requested || rasterized_triangle_instances > 1 || has_depth_tested_overlays)
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn create_geometry_buffers(
    device: &wgpu::Device,
    vertex_bytes: &[u8],
    instance_bytes: &[u8],
    retained_instances: &[PreparedInstanceSet],
) -> GeometryBuffers {
    let vertex_buffer_size = vertex_bytes.len().max(4) as u64;
    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scena.m0.scene_vertices"),
        size: vertex_buffer_size,
        usage: wgpu::BufferUsages::VERTEX,
        mapped_at_creation: true,
    });
    if !vertex_bytes.is_empty() {
        vertex_buffer
            .slice(..)
            .get_mapped_range_mut()
            .copy_from_slice(vertex_bytes);
    }
    vertex_buffer.unmap();

    let instance_buffer_capacity = retained_instance_buffer_capacity(retained_instances);
    let instance_buffer_size = (instance_buffer_capacity * INSTANCE_BYTE_LEN).max(4) as u64;
    let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scena.m4.scene_instances"),
        size: instance_buffer_size,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: true,
    });
    let mut initial_instance_bytes = instance_bytes.to_vec();
    initial_instance_bytes.resize(instance_buffer_size as usize, 0);
    instance_buffer
        .slice(..)
        .get_mapped_range_mut()
        .copy_from_slice(&initial_instance_bytes);
    instance_buffer.unmap();

    GeometryBuffers {
        vertex_buffer,
        vertex_buffer_size,
        instance_buffer,
        instance_buffer_size,
        instance_buffer_capacity,
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn validate_sample_count(
    gpu: &GpuDeviceState,
    target: RasterTarget,
    output_plan: GpuOutputPlan,
) -> Result<(u32, wgpu::TextureFormat), PrepareError> {
    let sample_count = output_plan.sample_count();
    let scene_format = if output_plan.post_enabled() {
        super::post::scene_color_format()
    } else {
        GPU_COLOR_FORMAT
    };
    let mut sample_formats = vec![scene_format, wgpu::TextureFormat::Depth32Float];
    if !output_plan.post_enabled()
        && let Some(surface_format) = gpu.surface.as_ref().map(|surface| surface.config.format)
        && !sample_formats.contains(&surface_format)
    {
        sample_formats.push(surface_format);
    }
    let maximum = gpu.max_supported_sample_count_cached(&sample_formats);
    if sample_count > maximum {
        return Err(PrepareError::UnsupportedSampleCount {
            backend: target.backend,
            requested: sample_count,
            maximum,
        });
    }
    Ok((sample_count, scene_format))
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn build_semantic_attribution(
    target: RasterTarget,
    enabled: bool,
    draw_primitives: &[PreparedPrimitive],
    draw_instances: &[PreparedInstanceSet],
    draw_strokes: &[PreparedStrokeSegment],
    draw_labels: &PreparedLabelAtlas,
) -> Result<Option<GpuSemanticAttribution>, PrepareError> {
    enabled
        .then(|| {
            crate::render::semantic_aov::build_gpu_semantic_attribution(
                draw_primitives,
                draw_instances,
                draw_strokes.len(),
                draw_labels.quads().len(),
            )
        })
        .transpose()
        .map_err(|entries| PrepareError::GpuResourceUpload {
            backend: target.backend,
            reason: format!(
                "semantic AOV requires {entries} palette entries, exceeding the 24-bit limit"
            ),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Backend;

    #[test]
    fn oversized_geometry_buffer_is_rejected_before_wgpu_allocation() {
        let error = validate_geometry_buffer_size(Backend::HeadlessGpu, 912_567_276, 268_435_456)
            .expect_err("an oversized vertex buffer must fail before Device::create_buffer");
        let PrepareError::GpuResourceUpload { backend, reason } = error else {
            panic!("unexpected error: {error:?}");
        };
        assert_eq!(backend, Backend::HeadlessGpu);
        assert!(reason.contains("912567276"));
        assert!(reason.contains("268435456"));
        assert!(reason.contains("prepared geometry"));
    }

    #[test]
    fn browser_single_triangle_skips_an_unobservable_depth_prepass() {
        let support = include_str!("prepare_resources_support.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("support implementation precedes tests");
        let browser_prepare = include_str!("prepare_resources_wasm.rs");
        assert!(
            support.contains("browser_depth_prepass_required")
                && browser_prepare.contains("browser_depth_prepass_required"),
            "browser preparation must decide depth allocation from observable scene demand",
        );
        assert!(!browser_depth_prepass_required(true, false, 1, false));
        assert!(browser_depth_prepass_required(true, false, 2, false));
        assert!(browser_depth_prepass_required(true, true, 1, false));
        assert!(browser_depth_prepass_required(true, false, 1, true));
        assert!(!browser_depth_prepass_required(false, true, 2, true));
    }
}
