use crate::assets::{TextureDesc, TextureFilter, TextureSamplerDesc, TextureWrap};
use crate::material::TextureColorSpace;
use crate::render::prepare::PreparedMaterialSlot;

use super::material_mips::{downsample_rgba8_mip, mip_level_extents};
use super::material_uniform::{MATERIAL_UNIFORM_BYTE_LEN, MaterialUniformUpload};

const FALLBACK_WHITE_RGBA8: &[u8; 4] = &[255, 255, 255, 255];
const FALLBACK_NORMAL_RGBA8: &[u8; 4] = &[128, 128, 255, 255];
const FALLBACK_METALLIC_ROUGHNESS_RGBA8: &[u8; 4] = &[255, 255, 0, 255];

const BASE_COLOR_BINDINGS: TextureBindingIndices = TextureBindingIndices {
    sampler: 0,
    texture: 1,
};
const NORMAL_BINDINGS: TextureBindingIndices = TextureBindingIndices {
    sampler: 3,
    texture: 4,
};
const METALLIC_ROUGHNESS_BINDINGS: TextureBindingIndices = TextureBindingIndices {
    sampler: 5,
    texture: 6,
};
const OCCLUSION_BINDINGS: TextureBindingIndices = TextureBindingIndices {
    sampler: 7,
    texture: 8,
};
const EMISSIVE_BINDINGS: TextureBindingIndices = TextureBindingIndices {
    sampler: 9,
    texture: 10,
};

#[derive(Debug, Clone, Copy)]
struct TextureBindingIndices {
    sampler: u32,
    texture: u32,
}

#[derive(Debug)]
pub(super) struct MaterialTextureResources {
    // These objects must stay alive for the bind group; the render pass reads the bind group.
    #[allow(dead_code)]
    pub(super) texture_bindings: Vec<MaterialTextureBindingResources>,
    #[allow(dead_code)]
    pub(super) uniform: wgpu::Buffer,
    pub(super) bind_group: wgpu::BindGroup,
    pub(super) texture_byte_len: u64,
}

#[derive(Debug)]
pub(super) struct MaterialTextureBindingResources {
    #[allow(dead_code)]
    texture: wgpu::Texture,
    #[allow(dead_code)]
    view: wgpu::TextureView,
    #[allow(dead_code)]
    sampler: wgpu::Sampler,
    byte_len: u64,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct MaterialTextureUpload<'a> {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) rgba8: &'a [u8],
    pub(super) format: wgpu::TextureFormat,
    pub(super) sampler: TextureSamplerDesc,
    pub(super) uses_decoded_texture: bool,
}

impl<'a> MaterialTextureUpload<'a> {
    pub(super) fn from_base_color_texture(texture: Option<&'a TextureDesc>) -> Self {
        Self::from_texture(
            texture,
            FALLBACK_WHITE_RGBA8,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        )
    }

    pub(super) fn from_normal_texture(texture: Option<&'a TextureDesc>) -> Self {
        Self::from_linear_texture(texture, FALLBACK_NORMAL_RGBA8)
    }

    pub(super) fn from_metallic_roughness_texture(texture: Option<&'a TextureDesc>) -> Self {
        Self::from_linear_texture(texture, FALLBACK_METALLIC_ROUGHNESS_RGBA8)
    }

    pub(super) fn from_occlusion_texture(texture: Option<&'a TextureDesc>) -> Self {
        Self::from_linear_texture(texture, FALLBACK_WHITE_RGBA8)
    }

    pub(super) fn from_emissive_texture(texture: Option<&'a TextureDesc>) -> Self {
        Self::from_base_color_texture(texture)
    }

    pub(super) fn from_linear_texture(
        texture: Option<&'a TextureDesc>,
        fallback_rgba8: &'a [u8; 4],
    ) -> Self {
        Self::from_texture(texture, fallback_rgba8, wgpu::TextureFormat::Rgba8Unorm)
    }

    fn from_texture(
        texture: Option<&'a TextureDesc>,
        fallback_rgba8: &'a [u8; 4],
        fallback_format: wgpu::TextureFormat,
    ) -> Self {
        if let Some(texture) = texture
            && let Some((width, height, rgba8)) = texture.decoded_rgba8()
            && width > 0
            && height > 0
            && !rgba8.is_empty()
        {
            let format = match texture.color_space() {
                TextureColorSpace::Srgb => wgpu::TextureFormat::Rgba8UnormSrgb,
                TextureColorSpace::Linear => wgpu::TextureFormat::Rgba8Unorm,
            };
            return Self {
                width,
                height,
                rgba8,
                format,
                sampler: texture.sampler(),
                uses_decoded_texture: true,
            };
        }

        Self {
            width: 1,
            height: 1,
            rgba8: fallback_rgba8,
            format: fallback_format,
            sampler: TextureSamplerDesc::default(),
            uses_decoded_texture: false,
        }
    }

    pub(super) fn byte_len(self) -> u64 {
        mip_level_extents(self.width, self.height, self.sampler.min_filter())
            .into_iter()
            .map(|(width, height)| {
                u64::from(width)
                    .saturating_mul(u64::from(height))
                    .saturating_mul(4)
            })
            .sum()
    }
}

pub(super) fn create_material_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let mut entries = vec![
        texture_sampler_layout_entry(BASE_COLOR_BINDINGS.sampler),
        texture_layout_entry(BASE_COLOR_BINDINGS.texture),
        wgpu::BindGroupLayoutEntry {
            binding: 2,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
    ];
    for bindings in [
        NORMAL_BINDINGS,
        METALLIC_ROUGHNESS_BINDINGS,
        OCCLUSION_BINDINGS,
        EMISSIVE_BINDINGS,
    ] {
        entries.push(texture_sampler_layout_entry(bindings.sampler));
        entries.push(texture_layout_entry(bindings.texture));
    }

    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("scena.material.bind_group_layout"),
        entries: &entries,
    })
}

fn texture_sampler_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

fn texture_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

pub(super) fn create_material_resources(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    material_slots: &[PreparedMaterialSlot],
) -> Vec<MaterialTextureResources> {
    let mut resources = Vec::with_capacity(material_slots.len() + 1);
    resources.push(create_material_resource(device, queue, layout, None));
    resources.extend(
        material_slots
            .iter()
            .map(|slot| create_material_resource(device, queue, layout, Some(slot))),
    );
    resources
}

pub(super) fn material_texture_byte_len(resources: &[MaterialTextureResources]) -> u64 {
    resources
        .iter()
        .map(|resources| resources.texture_byte_len)
        .sum()
}

fn create_material_resource(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    slot: Option<&PreparedMaterialSlot>,
) -> MaterialTextureResources {
    let material_uniform = MaterialUniformUpload::from_material(
        slot.map(|slot| &slot.material),
        slot.and_then(|slot| slot.base_color.as_ref())
            .and_then(|texture| texture.transform),
    );
    let base_color = create_texture_binding_resource(
        device,
        queue,
        "base_color",
        MaterialTextureUpload::from_base_color_texture(
            slot.and_then(|slot| slot.base_color.as_ref())
                .map(|texture| &texture.desc),
        ),
    );
    let normal = create_texture_binding_resource(
        device,
        queue,
        "normal",
        MaterialTextureUpload::from_normal_texture(
            slot.and_then(|slot| slot.normal.as_ref())
                .map(|texture| &texture.desc),
        ),
    );
    let metallic_roughness = create_texture_binding_resource(
        device,
        queue,
        "metallic_roughness",
        MaterialTextureUpload::from_metallic_roughness_texture(
            slot.and_then(|slot| slot.metallic_roughness.as_ref())
                .map(|texture| &texture.desc),
        ),
    );
    let occlusion = create_texture_binding_resource(
        device,
        queue,
        "occlusion",
        MaterialTextureUpload::from_occlusion_texture(
            slot.and_then(|slot| slot.occlusion.as_ref())
                .map(|texture| &texture.desc),
        ),
    );
    let emissive = create_texture_binding_resource(
        device,
        queue,
        "emissive",
        MaterialTextureUpload::from_emissive_texture(
            slot.and_then(|slot| slot.emissive.as_ref())
                .map(|texture| &texture.desc),
        ),
    );
    let texture_bindings = vec![base_color, normal, metallic_roughness, occlusion, emissive];
    let texture_byte_len = texture_bindings
        .iter()
        .map(|binding| binding.byte_len)
        .sum();
    let uniform = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scena.material.uniform"),
        size: MATERIAL_UNIFORM_BYTE_LEN,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&uniform, 0, &material_uniform.encode());
    let bind_group = create_material_bind_group(device, layout, &texture_bindings, &uniform);

    MaterialTextureResources {
        texture_bindings,
        uniform,
        bind_group,
        texture_byte_len,
    }
}

fn create_texture_binding_resource(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &'static str,
    upload: MaterialTextureUpload<'_>,
) -> MaterialTextureBindingResources {
    let mip_extents = mip_level_extents(upload.width, upload.height, upload.sampler.min_filter());
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(if upload.uses_decoded_texture {
            match label {
                "base_color" => "scena.material.base_color",
                "normal" => "scena.material.normal",
                "metallic_roughness" => "scena.material.metallic_roughness",
                "occlusion" => "scena.material.occlusion",
                "emissive" => "scena.material.emissive",
                _ => "scena.material.texture",
            }
        } else {
            match label {
                "base_color" => "scena.material.fallback_base_color",
                "normal" => "scena.material.fallback_normal",
                "metallic_roughness" => "scena.material.fallback_metallic_roughness",
                "occlusion" => "scena.material.fallback_occlusion",
                "emissive" => "scena.material.fallback_emissive",
                _ => "scena.material.fallback_texture",
            }
        }),
        size: wgpu::Extent3d {
            width: upload.width,
            height: upload.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: mip_extents.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: upload.format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    write_material_texture_mips(queue, &texture, upload, &mip_extents);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
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
        ..wgpu::SamplerDescriptor::default()
    });
    MaterialTextureBindingResources {
        texture,
        view,
        sampler,
        byte_len: upload.byte_len(),
    }
}

fn write_material_texture_mips(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    upload: MaterialTextureUpload<'_>,
    mip_extents: &[(u32, u32)],
) {
    let mut previous = upload.rgba8.to_vec();
    for (mip_level, (width, height)) in mip_extents.iter().copied().enumerate() {
        let pixels = if mip_level == 0 {
            upload.rgba8
        } else {
            previous = downsample_rgba8_mip(
                &previous,
                mip_extents[mip_level - 1].0,
                mip_extents[mip_level - 1].1,
                width,
                height,
            );
            previous.as_slice()
        };
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: mip_level as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width.saturating_mul(4)),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }
}

fn create_material_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    texture_bindings: &[MaterialTextureBindingResources],
    uniform: &wgpu::Buffer,
) -> wgpu::BindGroup {
    let binding_indices = [
        BASE_COLOR_BINDINGS,
        NORMAL_BINDINGS,
        METALLIC_ROUGHNESS_BINDINGS,
        OCCLUSION_BINDINGS,
        EMISSIVE_BINDINGS,
    ];
    let mut entries = Vec::with_capacity(11);
    for (bindings, resources) in binding_indices.into_iter().zip(texture_bindings) {
        entries.push(wgpu::BindGroupEntry {
            binding: bindings.sampler,
            resource: wgpu::BindingResource::Sampler(&resources.sampler),
        });
        entries.push(wgpu::BindGroupEntry {
            binding: bindings.texture,
            resource: wgpu::BindingResource::TextureView(&resources.view),
        });
    }
    entries.push(wgpu::BindGroupEntry {
        binding: 2,
        resource: uniform.as_entire_binding(),
    });

    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("scena.material.fallback_bind_group"),
        layout,
        entries: &entries,
    })
}

fn address_mode(wrap: TextureWrap) -> wgpu::AddressMode {
    match wrap {
        TextureWrap::ClampToEdge => wgpu::AddressMode::ClampToEdge,
        TextureWrap::MirroredRepeat => wgpu::AddressMode::MirrorRepeat,
        TextureWrap::Repeat => wgpu::AddressMode::Repeat,
    }
}

fn filter_mode(filter: Option<TextureFilter>) -> wgpu::FilterMode {
    match filter {
        Some(
            TextureFilter::Nearest
            | TextureFilter::NearestMipmapNearest
            | TextureFilter::NearestMipmapLinear,
        ) => wgpu::FilterMode::Nearest,
        Some(
            TextureFilter::Linear
            | TextureFilter::LinearMipmapNearest
            | TextureFilter::LinearMipmapLinear,
        )
        | None => wgpu::FilterMode::Linear,
    }
}

fn mipmap_filter_mode(filter: Option<TextureFilter>) -> wgpu::MipmapFilterMode {
    match filter {
        Some(TextureFilter::NearestMipmapNearest | TextureFilter::LinearMipmapNearest) => {
            wgpu::MipmapFilterMode::Nearest
        }
        Some(TextureFilter::NearestMipmapLinear | TextureFilter::LinearMipmapLinear) => {
            wgpu::MipmapFilterMode::Linear
        }
        Some(TextureFilter::Nearest | TextureFilter::Linear) | None => {
            wgpu::MipmapFilterMode::Nearest
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::assets::{AssetPath, TextureDesc, TextureSamplerDesc, TextureSourceFormat};
    use crate::material::TextureColorSpace;

    #[test]
    fn material_resources_define_shader_visible_texture_bindings() {
        let source = include_str!("materials.rs");
        assert!(
            source.contains("SamplerBindingType::Filtering")
                && source.contains("TextureSampleType::Float { filterable: true }")
                && source.contains("MaterialTextureUpload")
                && source.contains("MaterialUniformUpload")
                && source.contains("binding: 2")
                && source.contains("NORMAL_BINDINGS")
                && source.contains("METALLIC_ROUGHNESS_BINDINGS")
                && source.contains("OCCLUSION_BINDINGS")
                && source.contains("EMISSIVE_BINDINGS")
                && source.contains("scena.material.uniform")
                && source.contains("scena.material.base_color")
                && source.contains("scena.material.normal")
                && source.contains("scena.material.metallic_roughness")
                && source.contains("scena.material.occlusion")
                && source.contains("scena.material.emissive")
                && source.contains("scena.material.fallback_base_color")
                && source.contains("scena.material.fallback_bind_group"),
            "backend material scaffolding must allocate a sampler, texture view, and bind group"
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
    fn webgl2_material_upload_uses_texture_sampler_metadata() {
        let source = include_str!("webgl2_materials.rs");
        let cache_source = include_str!("webgl2.rs");
        let texture_set_source = include_str!("webgl2_texture_set.rs");
        assert!(
            source.contains("upload.sampler.wrap_s()")
                && source.contains("upload.sampler.wrap_t()")
                && source.contains("webgl2_wrap_mode")
                && source.contains("webgl2_filter_mode")
                && source.contains("TEXTURE_WRAP_S")
                && source.contains("TEXTURE_MIN_FILTER")
                && cache_source.contains("upload_webgl2_material_texture_set")
                && texture_set_source.contains("WebGl2MaterialTextureSet")
                && texture_set_source.contains("base_color: WebGlTexture")
                && texture_set_source.contains("normal: WebGlTexture")
                && texture_set_source.contains("metallic_roughness: WebGlTexture")
                && texture_set_source.contains("occlusion: WebGlTexture")
                && texture_set_source.contains("emissive: WebGlTexture"),
            "WebGL2 material upload must honor texture sampler wrap/filter metadata instead of \
             hardcoding linear clamp-to-edge"
        );
    }

    #[test]
    fn webgl2_material_shader_declares_fragment_texture_transform_uniforms() {
        let source = include_str!("webgl2_program.rs");
        let fragment_shader = source
            .split("pub(super) const FRAGMENT_SHADER")
            .nth(1)
            .expect("WebGL2 fragment shader source is present");

        assert!(
            fragment_shader.contains("uniform vec4 base_color_uv_offset_scale;")
                && fragment_shader.contains("uniform vec4 base_color_uv_rotation;")
                && fragment_shader.contains("texture(base_color_texture, transformed_uv)"),
            "WebGL2 fragment shader must declare and apply the same base-color texture \
             transform uniforms that render code sets per material"
        );
    }
}
