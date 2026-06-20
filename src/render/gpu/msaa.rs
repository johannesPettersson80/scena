use crate::diagnostics::RenderError;

use super::super::RasterTarget;
use super::pipeline::{GPU_COLOR_FORMAT, UnlitPipelines};
use super::{GpuPreparedResources, MsaaColorResources};

pub(super) fn offscreen_pipelines_for_sample_count(
    resources: &GpuPreparedResources,
    sample_count: u32,
) -> UnlitPipelines<'_> {
    match sample_count {
        4 => resources.offscreen_msaa4_pipelines.refs(),
        8 => resources
            .offscreen_msaa8_pipelines
            .as_ref()
            .expect("offscreen MSAA8 pipelines must be prepared before encoding")
            .refs(),
        _ => resources.offscreen_pipelines.refs(),
    }
}

pub(super) fn ensure_offscreen_msaa8_pipelines(
    adapter: &wgpu::Adapter,
    device: &wgpu::Device,
    resources: &mut GpuPreparedResources,
    target: RasterTarget,
) -> Result<(), RenderError> {
    if resources.offscreen_msaa8_pipelines.is_some() {
        return Ok(());
    }
    if !texture_format_supports_sample_count(device, adapter, GPU_COLOR_FORMAT, 8) {
        return Err(RenderError::UnsupportedSampleCount {
            backend: target.backend,
            requested: 8,
            maximum: max_supported_sample_count(device, adapter, &[GPU_COLOR_FORMAT]),
        });
    }
    resources.offscreen_msaa8_pipelines = Some(super::pipeline::create_unlit_pipeline_set(
        device,
        GPU_COLOR_FORMAT,
        &resources.output_bind_group_layout,
        &resources.material_bind_group_layout,
        &resources.draw_bind_group_layout,
        resources.texture_binding_mode,
        resources.depth_compare,
        8,
    ));
    Ok(())
}

pub(super) fn max_supported_sample_count(
    device: &wgpu::Device,
    adapter: &wgpu::Adapter,
    formats: &[wgpu::TextureFormat],
) -> u32 {
    [8, 4, 1]
        .into_iter()
        .find(|sample_count| {
            formats.iter().all(|format| {
                texture_format_supports_sample_count(device, adapter, *format, *sample_count)
            })
        })
        .unwrap_or(1)
}

pub(super) fn texture_format_supports_sample_count(
    device: &wgpu::Device,
    adapter: &wgpu::Adapter,
    format: wgpu::TextureFormat,
    sample_count: u32,
) -> bool {
    if sample_count > 4
        && !device
            .features()
            .contains(wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES)
    {
        return false;
    }
    adapter
        .get_texture_format_features(format)
        .flags
        .sample_count_supported(sample_count)
}

pub(super) fn ensure_msaa_color_resources(
    device: &wgpu::Device,
    resources: &mut GpuPreparedResources,
    target: RasterTarget,
    format: wgpu::TextureFormat,
    sample_count: u32,
) {
    if resources.msaa_color.as_ref().is_some_and(|msaa| {
        msaa.target == target && msaa.format == format && msaa.sample_count == sample_count
    }) {
        return;
    }
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scena.headless_gpu.msaa_color"),
        size: wgpu::Extent3d {
            width: target.width,
            height: target.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    resources.msaa_color = Some(MsaaColorResources {
        target,
        format,
        sample_count,
        texture,
        view,
    });
}
