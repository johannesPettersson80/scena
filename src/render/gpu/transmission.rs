use super::super::RasterTarget;
use super::pipeline::{MeshPipelineSet, create_unlit_pipeline_set};
use super::stats::GpuResourceStats;

#[derive(Debug)]
pub(super) struct TransmissionResources {
    #[allow(dead_code)]
    pub(super) texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    #[allow(dead_code)]
    pub(super) placeholder_texture: wgpu::Texture,
    pub(super) placeholder_view: wgpu::TextureView,
    pub(super) sampler: wgpu::Sampler,
    pub(super) pipelines: MeshPipelineSet,
}

pub(super) fn resource_stats(target: RasterTarget) -> GpuResourceStats {
    GpuResourceStats {
        textures: 2,
        render_targets: 1,
        pipelines: 2,
        approximate_gpu_memory_bytes: GpuResourceStats::target_bytes(target, 4, 1) + 4,
        ..GpuResourceStats::default()
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn create_transmission_resources(
    device: &wgpu::Device,
    triangle_shader: &wgpu::ShaderModule,
    target: RasterTarget,
    format: wgpu::TextureFormat,
    output_bind_group_layout: &wgpu::BindGroupLayout,
    material_bind_group_layout: &wgpu::BindGroupLayout,
    draw_bind_group_layout: &wgpu::BindGroupLayout,
    _depth_compare: Option<wgpu::CompareFunction>,
) -> TransmissionResources {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scena.round_e.transmission_scene_color"),
        size: wgpu::Extent3d {
            width: target.width,
            height: target.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("scena.round_e.transmission_scene_color_view"),
        ..Default::default()
    });
    let placeholder_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scena.round_e.transmission_placeholder"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let placeholder_view = placeholder_texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("scena.round_e.transmission_placeholder_view"),
        ..Default::default()
    });
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("scena.round_e.transmission_scene_color_sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        ..Default::default()
    });
    let pipelines = create_unlit_pipeline_set(
        device,
        triangle_shader,
        format,
        output_bind_group_layout,
        material_bind_group_layout,
        draw_bind_group_layout,
        None,
        1,
    );
    TransmissionResources {
        texture,
        view,
        placeholder_texture,
        placeholder_view,
        sampler,
        pipelines,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn transmission_scene_color_does_not_reuse_final_depth_prepass() {
        let source = include_str!("transmission.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("implementation precedes tests");
        assert!(
            source.contains("triangle_shader,\n        format,")
                && source.contains("draw_bind_group_layout,\n        None,")
                && !source.contains("transmission_scene_depth"),
            "transmission scene-color rendering must not reuse the final depth pre-pass; \
             the final depth buffer can reject the opaque target behind glass"
        );
    }
}
