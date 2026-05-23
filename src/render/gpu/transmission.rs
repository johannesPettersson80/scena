use super::super::RasterTarget;
use super::material_bindings::MaterialTextureBindingMode;
use super::pipeline::create_unlit_pipeline;

#[derive(Debug)]
pub(super) struct TransmissionResources {
    #[allow(dead_code)]
    pub(super) texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    #[allow(dead_code)]
    pub(super) placeholder_texture: wgpu::Texture,
    pub(super) placeholder_view: wgpu::TextureView,
    pub(super) sampler: wgpu::Sampler,
    pub(super) pipeline: wgpu::RenderPipeline,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn create_transmission_resources(
    device: &wgpu::Device,
    target: RasterTarget,
    format: wgpu::TextureFormat,
    output_bind_group_layout: &wgpu::BindGroupLayout,
    material_bind_group_layout: &wgpu::BindGroupLayout,
    draw_bind_group_layout: &wgpu::BindGroupLayout,
    texture_binding_mode: MaterialTextureBindingMode,
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
    let pipeline = create_unlit_pipeline(
        device,
        format,
        output_bind_group_layout,
        material_bind_group_layout,
        draw_bind_group_layout,
        texture_binding_mode,
        None,
    );
    TransmissionResources {
        texture,
        view,
        placeholder_texture,
        placeholder_view,
        sampler,
        pipeline,
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
            source.contains("texture_binding_mode,\n        None,")
                && !source.contains("transmission_scene_depth"),
            "transmission scene-color rendering must not reuse the final depth pre-pass; \
             the final depth buffer can reject the opaque target behind glass"
        );
    }
}
