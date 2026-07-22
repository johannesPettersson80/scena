use super::instancing::INSTANCE_BYTE_LEN;
use super::pipeline::GPU_COLOR_FORMAT;
use super::resource_encoding::retained_instance_buffer_capacity;
use super::{GpuDeviceState, GpuOutputPlan};
use crate::PrepareError;
use crate::render::RasterTarget;
use crate::render::prepare::{
    PreparedInstanceSet, PreparedLabelAtlas, PreparedPrimitive, PreparedStrokeSegment,
};
use crate::render::semantic_aov::GpuSemanticAttribution;

pub(super) struct GeometryBuffers {
    pub(super) vertex_buffer: wgpu::Buffer,
    pub(super) vertex_buffer_size: u64,
    pub(super) instance_buffer: wgpu::Buffer,
    pub(super) instance_buffer_size: u64,
    pub(super) instance_buffer_capacity: usize,
}

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
    let maximum =
        super::msaa::max_supported_sample_count(&gpu.device, &gpu.adapter, &sample_formats);
    if sample_count > maximum {
        return Err(PrepareError::UnsupportedSampleCount {
            backend: target.backend,
            requested: sample_count,
            maximum,
        });
    }
    Ok((sample_count, scene_format))
}

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
