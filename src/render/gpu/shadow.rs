pub(super) fn create_shadow_texture(
    device: &wgpu::Device,
    resolution: Option<u32>,
) -> Option<wgpu::Texture> {
    resolution.map(|size| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("scena.m2.directional_shadow_map"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
    })
}
