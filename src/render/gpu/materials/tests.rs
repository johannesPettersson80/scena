use crate::assets::{AssetPath, TextureDesc, TextureSamplerDesc, TextureSourceFormat};
use crate::material::TextureColorSpace;

#[test]
fn material_resources_define_shader_visible_texture_bindings() {
    let source = include_str!("../materials.rs");
    let bind_group_source = include_str!("bind_group.rs");
    let texture_resource_source = include_str!("texture_resource.rs");
    let bindings_source = include_str!("../material_bindings.rs");
    let batched_source = include_str!("../material_batched.rs");
    assert!(
        bindings_source.contains("SamplerBindingType::Filtering")
            && bindings_source.contains("TextureSampleType::Float { filterable: true }")
            && bindings_source.contains("NORMAL_BINDINGS")
            && bindings_source.contains("METALLIC_ROUGHNESS_BINDINGS")
            && bindings_source.contains("OCCLUSION_BINDINGS")
            && bindings_source.contains("EMISSIVE_BINDINGS")
            && bindings_source.contains("CLEARCOAT_BINDINGS")
            && bindings_source.contains("CLEARCOAT_ROUGHNESS_BINDINGS")
            && bindings_source.contains("CLEARCOAT_NORMAL_BINDINGS")
            && bindings_source.contains("SHEEN_COLOR_BINDINGS")
            && bindings_source.contains("SHEEN_ROUGHNESS_BINDINGS")
            && bindings_source.contains("ANISOTROPY_BINDINGS")
            && bindings_source.contains("IRIDESCENCE_BINDINGS")
            && bindings_source.contains("IRIDESCENCE_THICKNESS_BINDINGS")
            && bindings_source.contains("MATERIAL_TEXTURE_BINDING_INDICES")
            && bindings_source.contains("Self::Texture2d => wgpu::TextureViewDimension::D2")
            && bindings_source.contains("TextureViewDimension::D2Array")
            && source.contains("MaterialTextureUpload")
            && source.contains("MaterialUniformUpload")
            && source.contains("binding: 2")
            && source.contains("scena.material.uniform")
            && texture_resource_source.contains("scena.material.base_color")
            && texture_resource_source.contains("scena.material.normal")
            && texture_resource_source.contains("scena.material.metallic_roughness")
            && texture_resource_source.contains("scena.material.occlusion")
            && texture_resource_source.contains("scena.material.emissive")
            && texture_resource_source.contains("scena.material.clearcoat")
            && texture_resource_source.contains("scena.material.clearcoat_roughness")
            && texture_resource_source.contains("scena.material.clearcoat_normal")
            && texture_resource_source.contains("scena.material.sheen_color")
            && texture_resource_source.contains("scena.material.sheen_roughness")
            && texture_resource_source.contains("scena.material.anisotropy")
            && texture_resource_source.contains("scena.material.iridescence")
            && texture_resource_source.contains("scena.material.iridescence_thickness")
            && texture_resource_source.contains("scena.material.fallback_base_color")
            && bind_group_source.contains("scena.material.fallback_bind_group")
            && batched_source.contains("scena.material.batched_uniform")
            && batched_source.contains("scena.material.batched_clearcoat")
            && batched_source.contains("scena.material.batched_clearcoat_roughness")
            && batched_source.contains("scena.material.batched_clearcoat_normal")
            && batched_source.contains("scena.material.batched_sheen_color")
            && batched_source.contains("scena.material.batched_sheen_roughness")
            && batched_source.contains("scena.material.batched_anisotropy")
            && batched_source.contains("scena.material.batched_iridescence")
            && batched_source.contains("scena.material.batched_iridescence_thickness")
            && batched_source.contains("template.browser_image.is_some()")
            && batched_source.contains("wgpu::TextureUsages::RENDER_ATTACHMENT"),
        "backend material scaffolding must allocate a sampler, texture view, and bind group \
         plus the batched array path and browser ImageBitmap upload usage that closes plan line 778"
    );
}

#[test]
fn decoded_base_color_texture_becomes_backend_upload() {
    let texture = TextureDesc::new_with_bytes(
        AssetPath::from(
            "data:image/png;base64,\
             iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==",
        ),
        TextureColorSpace::Srgb,
        TextureSamplerDesc::default(),
        TextureSourceFormat::Png,
        None,
    )
    .expect("inline PNG texture decodes");

    let upload = super::MaterialTextureUpload::from_base_color_texture(Some(&texture));

    assert!(upload.uses_decoded_texture);
    assert_eq!(upload.width, 1);
    assert_eq!(upload.height, 1);
    assert_eq!(upload.rgba8, &[255, 0, 0, 255]);
    assert_eq!(upload.format, wgpu::TextureFormat::Rgba8UnormSrgb);
}

#[test]
fn wgpu_material_upload_uses_texture_sampler_metadata() {
    let texture_resource_source = include_str!("texture_resource.rs");
    let upload_source = include_str!("../material_upload.rs");
    assert!(
        texture_resource_source.contains("address_mode(upload.sampler.wrap_s())")
            && texture_resource_source.contains("address_mode(upload.sampler.wrap_t())")
            && texture_resource_source.contains("filter_mode(upload.sampler.mag_filter())")
            && texture_resource_source.contains("filter_mode(upload.sampler.min_filter())")
            && texture_resource_source.contains("mipmap_filter_mode(upload.sampler.min_filter())")
            && upload_source
                .contains("TextureWrap::MirroredRepeat => wgpu::AddressMode::MirrorRepeat")
            && upload_source.contains("TextureWrap::Repeat => wgpu::AddressMode::Repeat")
            && upload_source.contains("TextureFilter::Nearest")
            && upload_source.contains("TextureFilter::LinearMipmapLinear"),
        "wgpu material upload must honor glTF sampler wrap/filter metadata instead of \
         hardcoding linear clamp-to-edge"
    );
}
