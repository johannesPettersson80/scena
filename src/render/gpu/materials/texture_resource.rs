use super::super::material_bindings::MaterialTextureBindingMode;
use super::super::material_mips::mip_level_extents;
use super::{
    MaterialTextureBindingResources, MaterialTextureUpload, address_mode, filter_mode,
    material_anisotropy_clamp, mipmap_filter_mode, write_material_texture_layer_mips,
};

pub(super) fn create_texture_binding_resource(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &'static str,
    upload: MaterialTextureUpload<'_>,
    texture_binding_mode: MaterialTextureBindingMode,
) -> MaterialTextureBindingResources {
    let mip_extents = {
        #[cfg(target_arch = "wasm32")]
        if upload.browser_image.is_some() {
            vec![(upload.width.max(1), upload.height.max(1))]
        } else {
            mip_level_extents(upload.width, upload.height, upload.mip_extent_filter())
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            mip_level_extents(upload.width, upload.height, upload.mip_extent_filter())
        }
    };
    #[cfg(target_arch = "wasm32")]
    let texture_usage = if upload.browser_image.is_some() {
        wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::RENDER_ATTACHMENT
    } else {
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST
    };
    #[cfg(not(target_arch = "wasm32"))]
    let texture_usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(texture_label(label, upload.uses_decoded_texture)),
        size: wgpu::Extent3d {
            width: upload.width,
            height: upload.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: mip_extents.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: upload.format,
        usage: texture_usage,
        view_formats: &[],
    });
    write_material_texture_layer_mips(queue, &texture, upload, &mip_extents, 0);
    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(texture_binding_mode.view_dimension()),
        ..wgpu::TextureViewDescriptor::default()
    });
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some(if upload.uses_decoded_texture {
            "scena.material.sampler"
        } else {
            "scena.material.fallback_sampler"
        }),
        address_mode_u: address_mode(upload.sampler.wrap_s()),
        address_mode_v: address_mode(upload.sampler.wrap_t()),
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: filter_mode(upload.sampler.mag_filter()),
        min_filter: filter_mode(upload.sampler.min_filter()),
        mipmap_filter: mipmap_filter_mode(upload.sampler.min_filter()),
        anisotropy_clamp: material_anisotropy_clamp(mip_extents.len(), upload.sampler),
        ..wgpu::SamplerDescriptor::default()
    });
    MaterialTextureBindingResources::from_parts(texture, view, sampler, upload.byte_len())
}

fn texture_label(label: &str, decoded: bool) -> &'static str {
    match (decoded, label) {
        (true, "base_color") => "scena.material.base_color",
        (true, "normal") => "scena.material.normal",
        (true, "metallic_roughness") => "scena.material.metallic_roughness",
        (true, "occlusion") => "scena.material.occlusion",
        (true, "emissive") => "scena.material.emissive",
        (true, "clearcoat") => "scena.material.clearcoat",
        (true, "clearcoat_roughness") => "scena.material.clearcoat_roughness",
        (true, "clearcoat_normal") => "scena.material.clearcoat_normal",
        (true, "sheen_color") => "scena.material.sheen_color",
        (true, "sheen_roughness") => "scena.material.sheen_roughness",
        (true, "anisotropy") => "scena.material.anisotropy",
        (true, "iridescence") => "scena.material.iridescence",
        (true, "iridescence_thickness") => "scena.material.iridescence_thickness",
        (true, _) => "scena.material.texture",
        (false, "base_color") => "scena.material.fallback_base_color",
        (false, "normal") => "scena.material.fallback_normal",
        (false, "metallic_roughness") => "scena.material.fallback_metallic_roughness",
        (false, "occlusion") => "scena.material.fallback_occlusion",
        (false, "emissive") => "scena.material.fallback_emissive",
        (false, "clearcoat") => "scena.material.fallback_clearcoat",
        (false, "clearcoat_roughness") => "scena.material.fallback_clearcoat_roughness",
        (false, "clearcoat_normal") => "scena.material.fallback_clearcoat_normal",
        (false, "sheen_color") => "scena.material.fallback_sheen_color",
        (false, "sheen_roughness") => "scena.material.fallback_sheen_roughness",
        (false, "anisotropy") => "scena.material.fallback_anisotropy",
        (false, "iridescence") => "scena.material.fallback_iridescence",
        (false, "iridescence_thickness") => "scena.material.fallback_iridescence_thickness",
        (false, _) => "scena.material.fallback_texture",
    }
}
