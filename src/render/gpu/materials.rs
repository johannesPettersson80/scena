use crate::render::prepare::{PreparedMaterialSlot, compute_material_batch_plan};

use super::material_batched::{MaterialBatchedResources, create_batched_material_resources};
use super::material_bindings::{
    MaterialTextureBindingMode, create_material_texture_layout_entries,
};
use super::material_mips::{downsample_rgba8_mip, downsample_rgba16f_mip};
use super::material_uniform::{
    MATERIAL_UNIFORM_ENTRY_STRIDE, MaterialUniformUpload, material_uniform_min_binding_size,
};
pub(super) use super::material_upload::{
    MaterialTextureUpload, address_mode, filter_mode, mipmap_filter_mode,
};

pub(super) fn material_anisotropy_clamp(
    mip_count: usize,
    sampler: crate::assets::TextureSamplerDesc,
) -> u16 {
    use crate::assets::TextureFilter;

    let linear_mag = matches!(sampler.mag_filter(), None | Some(TextureFilter::Linear));
    let linear_min = matches!(
        sampler.min_filter(),
        None | Some(
            TextureFilter::Linear
                | TextureFilter::LinearMipmapNearest
                | TextureFilter::LinearMipmapLinear
        )
    );
    let linear_mip = matches!(
        sampler.min_filter(),
        Some(TextureFilter::NearestMipmapLinear | TextureFilter::LinearMipmapLinear)
    );
    if mip_count > 1 && linear_mag && linear_min && linear_mip {
        8
    } else {
        1
    }
}

mod bind_group;
pub(super) use bind_group::create_material_bind_group;
mod resource_stats;
pub(super) use resource_stats::resource_stats;
mod texture_resource;
use texture_resource::create_texture_binding_resource;

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
    let mut entries = create_material_texture_layout_entries(texture_binding_mode);
    entries.push(wgpu::BindGroupLayoutEntry {
        binding: 2,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            // Plan line 778 commit 2: dynamic-offset uniform so the
            // batched path can swap material slots without rebinding.
            // Per-material fall-back uses offset 0.
            has_dynamic_offset: true,
            min_binding_size: Some(material_uniform_min_binding_size()),
        },
        count: None,
    });

    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("scena.material.bind_group_layout"),
        entries: &entries,
    })
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
    let clearcoat = create_texture_binding_resource(
        device,
        queue,
        "clearcoat",
        MaterialTextureUpload::from_clearcoat_texture(
            slot.and_then(|slot| slot.clearcoat.as_ref())
                .map(|texture| &texture.desc),
        ),
        texture_binding_mode,
    );
    let clearcoat_roughness = create_texture_binding_resource(
        device,
        queue,
        "clearcoat_roughness",
        MaterialTextureUpload::from_clearcoat_roughness_texture(
            slot.and_then(|slot| slot.clearcoat_roughness.as_ref())
                .map(|texture| &texture.desc),
        ),
        texture_binding_mode,
    );
    let clearcoat_normal = create_texture_binding_resource(
        device,
        queue,
        "clearcoat_normal",
        MaterialTextureUpload::from_clearcoat_normal_texture(
            slot.and_then(|slot| slot.clearcoat_normal.as_ref())
                .map(|texture| &texture.desc),
        ),
        texture_binding_mode,
    );
    let sheen_color = create_texture_binding_resource(
        device,
        queue,
        "sheen_color",
        MaterialTextureUpload::from_sheen_color_texture(
            slot.and_then(|slot| slot.sheen_color.as_ref())
                .map(|texture| &texture.desc),
        ),
        texture_binding_mode,
    );
    let sheen_roughness = create_texture_binding_resource(
        device,
        queue,
        "sheen_roughness",
        MaterialTextureUpload::from_sheen_roughness_texture(
            slot.and_then(|slot| slot.sheen_roughness.as_ref())
                .map(|texture| &texture.desc),
        ),
        texture_binding_mode,
    );
    let anisotropy = create_texture_binding_resource(
        device,
        queue,
        "anisotropy",
        MaterialTextureUpload::from_anisotropy_texture(
            slot.and_then(|slot| slot.anisotropy.as_ref())
                .map(|texture| &texture.desc),
        ),
        texture_binding_mode,
    );
    let iridescence = create_texture_binding_resource(
        device,
        queue,
        "iridescence",
        MaterialTextureUpload::from_iridescence_texture(
            slot.and_then(|slot| slot.iridescence.as_ref())
                .map(|texture| &texture.desc),
        ),
        texture_binding_mode,
    );
    let iridescence_thickness = create_texture_binding_resource(
        device,
        queue,
        "iridescence_thickness",
        MaterialTextureUpload::from_iridescence_thickness_texture(
            slot.and_then(|slot| slot.iridescence_thickness.as_ref())
                .map(|texture| &texture.desc),
        ),
        texture_binding_mode,
    );
    let texture_bindings = vec![
        base_color,
        normal,
        metallic_roughness,
        occlusion,
        emissive,
        clearcoat,
        clearcoat_roughness,
        clearcoat_normal,
        sheen_color,
        sheen_roughness,
        anisotropy,
        iridescence,
        iridescence_thickness,
    ];
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

pub(super) fn write_material_texture_layer_mips(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    upload: MaterialTextureUpload<'_>,
    mip_extents: &[(u32, u32)],
    layer_index: u32,
) {
    #[cfg(target_arch = "wasm32")]
    if let Some(image) = upload.browser_image {
        queue.copy_external_image_to_texture(
            &wgpu::CopyExternalImageSourceInfo {
                source: wgpu::ExternalImageSource::ImageBitmap(image.clone()),
                origin: wgpu::Origin2d::ZERO,
                flip_y: false,
            },
            wgpu::CopyExternalImageDestInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: layer_index,
                },
                aspect: wgpu::TextureAspect::All,
                color_space: wgpu::PredefinedColorSpace::Srgb,
                premultiplied_alpha: false,
            },
            wgpu::Extent3d {
                width: upload.width.max(1),
                height: upload.height.max(1),
                depth_or_array_layers: 1,
            },
        );
        return;
    }
    if let Some(rgba16f_bits) = upload.rgba16f_bits {
        let mut previous = rgba16f_bits.to_vec();
        for (mip_level, (width, height)) in mip_extents.iter().copied().enumerate() {
            let pixels = if mip_level == 0 {
                rgba16f_bits
            } else {
                previous = downsample_rgba16f_mip(
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
                bytemuck::cast_slice(pixels),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(width.saturating_mul(8)),
                    rows_per_image: None,
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
        }
        return;
    }
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
                upload.format == wgpu::TextureFormat::Rgba8UnormSrgb,
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

#[cfg(test)]
mod tests;
