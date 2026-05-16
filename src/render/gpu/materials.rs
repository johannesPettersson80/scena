use crate::render::prepare::{PreparedMaterialSlot, compute_material_batch_plan};

use super::material_batched::{MaterialBatchedResources, create_batched_material_resources};
use super::material_mips::{downsample_rgba8_mip, mip_level_extents};
use super::material_uniform::{
    MATERIAL_UNIFORM_BYTE_LEN, MATERIAL_UNIFORM_ENTRY_STRIDE, MaterialUniformUpload,
};
pub(super) use super::material_upload::{
    MaterialTextureUpload, address_mode, filter_mode, mipmap_filter_mode,
};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(super) enum MaterialTextureBindingMode {
    Texture2d,
    Texture2dArray,
}

impl MaterialTextureBindingMode {
    fn view_dimension(self) -> wgpu::TextureViewDimension {
        match self {
            Self::Texture2d => wgpu::TextureViewDimension::D2,
            Self::Texture2dArray => wgpu::TextureViewDimension::D2Array,
        }
    }

    fn supports_batching(self) -> bool {
        matches!(self, Self::Texture2dArray)
    }
}

/// Plan line 778 commit 2: material GPU resources can take one of two shapes.
///
/// * `PerMaterial` keeps the legacy fall-back path: one
///   `MaterialTextureResources` per slot, each owning its own bind group with
///   one texture per role and a 96-byte uniform buffer addressed with dynamic
///   offset 0. WebGPU/native bind those textures as 1-layer
///   `texture_2d_array<f32>` views; WebGL2 uses ordinary `texture_2d<f32>`
///   views because wgpu 29's GL backend samples material array textures as
///   black in Chromium WebGL2.
/// * `Batched` collapses N materials into a single bind group whose textures
///   are N-layer arrays and whose uniform buffer holds N entries of size
///   `MATERIAL_UNIFORM_ENTRY_STRIDE`. Each draw selects its layer with a
///   dynamic uniform offset.
///
/// Both paths share the same WGSL pipeline because the bind group layout has
/// `has_dynamic_offset: true` on the uniform binding regardless.
#[derive(Debug)]
pub(super) enum MaterialResources {
    PerMaterial(Vec<MaterialTextureResources>),
    Batched(MaterialBatchedResources),
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

impl MaterialTextureBindingResources {
    pub(super) fn from_parts(
        texture: wgpu::Texture,
        view: wgpu::TextureView,
        sampler: wgpu::Sampler,
        byte_len: u64,
    ) -> Self {
        Self {
            texture,
            view,
            sampler,
            byte_len,
        }
    }

    pub(super) fn byte_len(&self) -> u64 {
        self.byte_len
    }

    pub(super) fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub(super) fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }
}

pub(super) fn create_material_bind_group_layout(
    device: &wgpu::Device,
    texture_binding_mode: MaterialTextureBindingMode,
) -> wgpu::BindGroupLayout {
    let mut entries = vec![
        texture_sampler_layout_entry(BASE_COLOR_BINDINGS.sampler),
        texture_layout_entry(BASE_COLOR_BINDINGS.texture, texture_binding_mode),
        wgpu::BindGroupLayoutEntry {
            binding: 2,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                // Plan line 778 commit 2: dynamic-offset uniform so the
                // batched path can swap material slots without rebinding.
                // Per-material fall-back uses offset 0.
                has_dynamic_offset: true,
                min_binding_size: std::num::NonZeroU64::new(MATERIAL_UNIFORM_BYTE_LEN),
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
        entries.push(texture_layout_entry(bindings.texture, texture_binding_mode));
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

fn texture_layout_entry(
    binding: u32,
    texture_binding_mode: MaterialTextureBindingMode,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: texture_binding_mode.view_dimension(),
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
    texture_binding_mode: MaterialTextureBindingMode,
) -> MaterialResources {
    if texture_binding_mode.supports_batching() {
        let plan = compute_material_batch_plan(material_slots);
        if plan.batchable && plan.layer_count >= 2 {
            return MaterialResources::Batched(create_batched_material_resources(
                device,
                queue,
                layout,
                material_slots,
            ));
        }
    }
    let mut resources = Vec::with_capacity(material_slots.len() + 1);
    resources.push(create_material_resource(
        device,
        queue,
        layout,
        None,
        texture_binding_mode,
    ));
    resources.extend(material_slots.iter().map(|slot| {
        create_material_resource(device, queue, layout, Some(slot), texture_binding_mode)
    }));
    MaterialResources::PerMaterial(resources)
}

pub(super) fn material_texture_byte_len(resources: &MaterialResources) -> u64 {
    match resources {
        MaterialResources::PerMaterial(slots) => {
            slots.iter().map(|slot| slot.texture_byte_len).sum()
        }
        MaterialResources::Batched(batched) => batched.texture_byte_len,
    }
}

pub(super) fn material_texture_count(resources: &MaterialResources) -> u64 {
    match resources {
        MaterialResources::PerMaterial(slots) => slots.len() as u64,
        // Batched: every layer is one logical material occupying a slice of
        // the shared array texture; report the layer count so external stats
        // continue to track per-material totals.
        MaterialResources::Batched(batched) => u64::from(batched.layer_count),
    }
}

/// Plan line 778 commit 2: count of distinct material bind groups consumed by
/// `encode_unlit_pass`. Always 1 on the batched path (one shared bind group
/// services every draw with dynamic-offset uniforms) and `slots.len()` on the
/// per-material path. The renderer surfaces this through
/// `RendererStats::material_bind_groups` so a "collapses to single bind"
/// test can assert the path collapse without dragging in
/// command-encoder introspection.
pub(super) fn material_bind_group_count(resources: &MaterialResources) -> u32 {
    match resources {
        MaterialResources::PerMaterial(slots) => slots.len() as u32,
        MaterialResources::Batched(_) => 1,
    }
}

fn create_material_resource(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    slot: Option<&PreparedMaterialSlot>,
    texture_binding_mode: MaterialTextureBindingMode,
) -> MaterialTextureResources {
    let material_uniform = MaterialUniformUpload::from_material(
        slot.map(|slot| &slot.material),
        slot.and_then(|slot| slot.base_color.as_ref())
            .and_then(|texture| texture.transform),
    )
    .with_layer_index(0);
    let base_color = create_texture_binding_resource(
        device,
        queue,
        "base_color",
        MaterialTextureUpload::from_base_color_texture(
            slot.and_then(|slot| slot.base_color.as_ref())
                .map(|texture| &texture.desc),
        ),
        texture_binding_mode,
    );
    let normal = create_texture_binding_resource(
        device,
        queue,
        "normal",
        MaterialTextureUpload::from_normal_texture(
            slot.and_then(|slot| slot.normal.as_ref())
                .map(|texture| &texture.desc),
        ),
        texture_binding_mode,
    );
    let metallic_roughness = create_texture_binding_resource(
        device,
        queue,
        "metallic_roughness",
        MaterialTextureUpload::from_metallic_roughness_texture(
            slot.and_then(|slot| slot.metallic_roughness.as_ref())
                .map(|texture| &texture.desc),
        ),
        texture_binding_mode,
    );
    let occlusion = create_texture_binding_resource(
        device,
        queue,
        "occlusion",
        MaterialTextureUpload::from_occlusion_texture(
            slot.and_then(|slot| slot.occlusion.as_ref())
                .map(|texture| &texture.desc),
        ),
        texture_binding_mode,
    );
    let emissive = create_texture_binding_resource(
        device,
        queue,
        "emissive",
        MaterialTextureUpload::from_emissive_texture(
            slot.and_then(|slot| slot.emissive.as_ref())
                .map(|texture| &texture.desc),
        ),
        texture_binding_mode,
    );
    let texture_bindings = vec![base_color, normal, metallic_roughness, occlusion, emissive];
    let texture_byte_len = texture_bindings
        .iter()
        .map(|binding| binding.byte_len)
        .sum();
    let uniform = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scena.material.uniform"),
        size: MATERIAL_UNIFORM_ENTRY_STRIDE,
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
    texture_binding_mode: MaterialTextureBindingMode,
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
        ..wgpu::SamplerDescriptor::default()
    });
    MaterialTextureBindingResources::from_parts(texture, view, sampler, upload.byte_len())
}

pub(super) fn write_material_texture_layer_mips(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    upload: MaterialTextureUpload<'_>,
    mip_extents: &[(u32, u32)],
    layer_index: u32,
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
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: layer_index,
                },
                aspect: wgpu::TextureAspect::All,
            },
            pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width.saturating_mul(4)),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }
}

pub(super) fn create_material_bind_group(
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

#[cfg(test)]
mod tests {
    use crate::assets::{AssetPath, TextureDesc, TextureSamplerDesc, TextureSourceFormat};
    use crate::material::TextureColorSpace;

    #[test]
    fn material_resources_define_shader_visible_texture_bindings() {
        let source = include_str!("materials.rs");
        let batched_source = include_str!("material_batched.rs");
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
                && source.contains("scena.material.fallback_bind_group")
                && source.contains("Self::Texture2d => wgpu::TextureViewDimension::D2")
                && source.contains("TextureViewDimension::D2Array")
                && batched_source.contains("scena.material.batched_uniform"),
            "backend material scaffolding must allocate a sampler, texture view, and bind group \
             plus the batched array path that closes plan line 778"
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
        let source = include_str!("materials.rs");
        let upload_source = include_str!("material_upload.rs");
        assert!(
            source.contains("address_mode(upload.sampler.wrap_s())")
                && source.contains("address_mode(upload.sampler.wrap_t())")
                && source.contains("filter_mode(upload.sampler.mag_filter())")
                && source.contains("filter_mode(upload.sampler.min_filter())")
                && source.contains("mipmap_filter_mode(upload.sampler.min_filter())")
                && upload_source
                    .contains("TextureWrap::MirroredRepeat => wgpu::AddressMode::MirrorRepeat")
                && upload_source.contains("TextureWrap::Repeat => wgpu::AddressMode::Repeat")
                && upload_source.contains("TextureFilter::Nearest")
                && upload_source.contains("TextureFilter::LinearMipmapLinear"),
            "wgpu material upload must honor glTF sampler wrap/filter metadata instead of \
             hardcoding linear clamp-to-edge"
        );
    }
}
