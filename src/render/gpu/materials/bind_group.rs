use super::super::material_bindings::MATERIAL_TEXTURE_BINDING_INDICES;
use super::super::material_uniform::MATERIAL_UNIFORM_BYTE_LEN;
use super::MaterialTextureBindingResources;

pub(in crate::render::gpu) fn create_material_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    texture_bindings: &[MaterialTextureBindingResources],
    uniform: &wgpu::Buffer,
) -> wgpu::BindGroup {
    let mut entries = Vec::with_capacity(MATERIAL_TEXTURE_BINDING_INDICES.len() * 2 + 1);
    for (bindings, resources) in MATERIAL_TEXTURE_BINDING_INDICES
        .into_iter()
        .zip(texture_bindings)
    {
        entries.push(wgpu::BindGroupEntry {
            binding: bindings.sampler,
            resource: wgpu::BindingResource::Sampler(resources.sampler()),
        });
        entries.push(wgpu::BindGroupEntry {
            binding: bindings.texture,
            resource: wgpu::BindingResource::TextureView(resources.view()),
        });
    }
    entries.push(wgpu::BindGroupEntry {
        binding: 2,
        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
            buffer: uniform,
            offset: 0,
            // The dynamic-offset path slices a single MATERIAL_UNIFORM_BYTE_LEN
            // window out of the larger buffer; per-material fall-back uses
            // the same window with offset 0.
            size: std::num::NonZeroU64::new(MATERIAL_UNIFORM_BYTE_LEN),
        }),
    });

    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("scena.material.fallback_bind_group"),
        layout,
        entries: &entries,
    })
}
