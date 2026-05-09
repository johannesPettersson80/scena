//! Plan line 778 / RFC 866 commit 2: shared `texture_2d_array<f32>`
//! allocation for the batched material path. When `MaterialBatchPlan` reports
//! `batchable && layer_count >= 2`, every material role is collapsed into one
//! N-layer texture array and a single bind group, so the render pass can swap
//! materials with a 256-byte dynamic uniform offset instead of N bind-group
//! switches. Per-material 1-layer fall-back stays in `materials.rs`.

use crate::render::prepare::PreparedMaterialSlot;

use super::material_mips::mip_level_extents;
use super::material_uniform::{MATERIAL_UNIFORM_ENTRY_STRIDE, MaterialUniformUpload};
use super::materials::{
    MaterialTextureBindingResources, MaterialTextureUpload, address_mode,
    create_material_bind_group, filter_mode, mipmap_filter_mode, write_material_texture_layer_mips,
};

#[derive(Debug)]
pub(super) struct MaterialBatchedResources {
    /// One shared bind group reused for every draw; per-draw dynamic offset
    /// selects the per-material uniform slot, and `material_layer_index` in
    /// the uniform selects the texture-array layer.
    pub(super) bind_group: wgpu::BindGroup,
    /// Layer count populated into the array textures. Equals
    /// `material_slot_count + 1` to reserve layer 0 for the synthetic
    /// fallback slot referenced when a draw points at a missing material.
    pub(super) layer_count: u32,
    #[allow(dead_code)]
    pub(super) texture_bindings: Vec<MaterialTextureBindingResources>,
    #[allow(dead_code)]
    pub(super) uniform: wgpu::Buffer,
    pub(super) texture_byte_len: u64,
}

pub(super) fn create_batched_material_resources(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    material_slots: &[PreparedMaterialSlot],
) -> MaterialBatchedResources {
    let layer_count = material_slots.len() as u32 + 1;
    let base_color = create_batched_role_resource(
        device,
        queue,
        "base_color",
        layer_count,
        material_slots,
        |slot| {
            MaterialTextureUpload::from_base_color_texture(
                slot.base_color.as_ref().map(|texture| &texture.desc),
            )
        },
        || MaterialTextureUpload::from_base_color_texture(None),
    );
    let normal = create_batched_role_resource(
        device,
        queue,
        "normal",
        layer_count,
        material_slots,
        |slot| {
            MaterialTextureUpload::from_normal_texture(
                slot.normal.as_ref().map(|texture| &texture.desc),
            )
        },
        || MaterialTextureUpload::from_normal_texture(None),
    );
    let metallic_roughness = create_batched_role_resource(
        device,
        queue,
        "metallic_roughness",
        layer_count,
        material_slots,
        |slot| {
            MaterialTextureUpload::from_metallic_roughness_texture(
                slot.metallic_roughness
                    .as_ref()
                    .map(|texture| &texture.desc),
            )
        },
        || MaterialTextureUpload::from_metallic_roughness_texture(None),
    );
    let occlusion = create_batched_role_resource(
        device,
        queue,
        "occlusion",
        layer_count,
        material_slots,
        |slot| {
            MaterialTextureUpload::from_occlusion_texture(
                slot.occlusion.as_ref().map(|texture| &texture.desc),
            )
        },
        || MaterialTextureUpload::from_occlusion_texture(None),
    );
    let emissive = create_batched_role_resource(
        device,
        queue,
        "emissive",
        layer_count,
        material_slots,
        |slot| {
            MaterialTextureUpload::from_emissive_texture(
                slot.emissive.as_ref().map(|texture| &texture.desc),
            )
        },
        || MaterialTextureUpload::from_emissive_texture(None),
    );
    let texture_bindings = vec![base_color, normal, metallic_roughness, occlusion, emissive];
    let texture_byte_len = texture_bindings
        .iter()
        .map(MaterialTextureBindingResources::byte_len)
        .sum();

    let uniform_size = MATERIAL_UNIFORM_ENTRY_STRIDE.saturating_mul(u64::from(layer_count));
    let uniform = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scena.material.batched_uniform"),
        size: uniform_size,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    // Layer 0 = synthetic fallback slot; layers 1..=N follow the slot order
    // so draws encode the layer index as `slot.draw.material_slot`.
    let fallback_uniform = MaterialUniformUpload::from_material(None, None).with_layer_index(0);
    queue.write_buffer(&uniform, 0, &fallback_uniform.encode());
    for (index, slot) in material_slots.iter().enumerate() {
        let layer_index = (index + 1) as u32;
        let upload = MaterialUniformUpload::from_material(
            Some(&slot.material),
            slot.base_color
                .as_ref()
                .and_then(|texture| texture.transform),
        )
        .with_layer_index(layer_index);
        let offset = MATERIAL_UNIFORM_ENTRY_STRIDE.saturating_mul(u64::from(layer_index));
        queue.write_buffer(&uniform, offset, &upload.encode());
    }
    let bind_group = create_material_bind_group(device, layout, &texture_bindings, &uniform);

    MaterialBatchedResources {
        bind_group,
        layer_count,
        texture_bindings,
        uniform,
        texture_byte_len,
    }
}

fn create_batched_role_resource<RoleFn, FallbackFn>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &'static str,
    layer_count: u32,
    material_slots: &[PreparedMaterialSlot],
    mut role_for: RoleFn,
    fallback_for: FallbackFn,
) -> MaterialTextureBindingResources
where
    RoleFn: FnMut(&PreparedMaterialSlot) -> MaterialTextureUpload<'_>,
    FallbackFn: Fn() -> MaterialTextureUpload<'static>,
{
    let fallback = fallback_for();
    // Pick the upload-shape (dimensions + sampler + format) from the first
    // slot that contributes a populated role; fall back to the synthetic
    // shape when no slot contributes. Plan invariant: every contributing
    // slot agrees on shape, so the choice is deterministic.
    let template = material_slots
        .iter()
        .map(&mut role_for)
        .find(|upload| upload.uses_decoded_texture)
        .unwrap_or(fallback);
    let mip_extents = mip_level_extents(
        template.width,
        template.height,
        template.sampler.min_filter(),
    );
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(match label {
            "base_color" => "scena.material.batched_base_color",
            "normal" => "scena.material.batched_normal",
            "metallic_roughness" => "scena.material.batched_metallic_roughness",
            "occlusion" => "scena.material.batched_occlusion",
            "emissive" => "scena.material.batched_emissive",
            _ => "scena.material.batched_texture",
        }),
        size: wgpu::Extent3d {
            width: template.width,
            height: template.height,
            depth_or_array_layers: layer_count,
        },
        mip_level_count: mip_extents.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: template.format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    write_material_texture_layer_mips(queue, &texture, fallback, &mip_extents, 0);
    for (index, slot) in material_slots.iter().enumerate() {
        let upload = role_for(slot);
        write_material_texture_layer_mips(
            queue,
            &texture,
            upload,
            &mip_extents,
            (index + 1) as u32,
        );
    }
    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..wgpu::TextureViewDescriptor::default()
    });
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("scena.material.batched_sampler"),
        address_mode_u: address_mode(template.sampler.wrap_s()),
        address_mode_v: address_mode(template.sampler.wrap_t()),
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: filter_mode(template.sampler.mag_filter()),
        min_filter: filter_mode(template.sampler.min_filter()),
        mipmap_filter: mipmap_filter_mode(template.sampler.min_filter()),
        ..wgpu::SamplerDescriptor::default()
    });
    MaterialTextureBindingResources::from_parts(
        texture,
        view,
        sampler,
        template.byte_len_for_layers(layer_count),
    )
}
