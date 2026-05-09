use super::super::prepare::{PreparedEnvironmentCubemap, PreparedEnvironmentLighting};
use super::output::create_output_bind_group;
use super::shadow::{
    self, ShadowCasterResources, create_shadow_caster_resources, create_shadow_sampler,
};

/// Bundles the per-frame group-0 GPU resources that are shared by every
/// render pass: shadow caster (texture + pipeline + active flag), shadow
/// comparison sampler, environment cubemap, environment sampler, and the
/// output bind group that ties them together with the uniform buffer.
pub(super) struct OutputResources {
    pub(super) shadow_caster: ShadowCasterResources,
    pub(super) shadow_sampler: wgpu::Sampler,
    pub(super) environment_cubemap: wgpu::Texture,
    pub(super) environment_sampler: wgpu::Sampler,
    pub(super) output_bind_group: wgpu::BindGroup,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_output_resources(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    output_bind_group_layout: &wgpu::BindGroupLayout,
    draw_bind_group_layout: &wgpu::BindGroupLayout,
    output_uniform: &wgpu::Buffer,
    directional_shadow_map_resolution: Option<u32>,
    environment_lighting: &PreparedEnvironmentLighting,
) -> OutputResources {
    let _ = shadow::SHADOW_CASTER_SHADER;
    let shadow_caster = create_shadow_caster_resources(
        device,
        directional_shadow_map_resolution,
        output_bind_group_layout,
        draw_bind_group_layout,
    );
    let shadow_sampler = create_shadow_sampler(device);
    let environment_cubemap =
        create_environment_cubemap_texture(device, queue, environment_lighting.cubemap());
    let environment_cubemap_view = environment_cubemap.create_view(&wgpu::TextureViewDescriptor {
        label: Some("scena.output.environment_cubemap_view"),
        dimension: Some(wgpu::TextureViewDimension::Cube),
        ..Default::default()
    });
    let environment_sampler = create_environment_sampler(device);
    let output_bind_group = create_output_bind_group(
        device,
        output_bind_group_layout,
        output_uniform,
        &shadow_caster.view,
        &shadow_sampler,
        &environment_cubemap_view,
        &environment_sampler,
    );
    OutputResources {
        shadow_caster,
        shadow_sampler,
        environment_cubemap,
        environment_sampler,
        output_bind_group,
    }
}

/// Allocates the environment cubemap with one of two shapes:
///
/// * If `prepared` carries decoded pixels, the cubemap matches that face
///   resolution and is uploaded with `queue.write_texture` per layer; the
///   fragment shader's `textureSampleLevel(environment_cubemap, ...)` reads
///   the real environment radiance.
/// * Otherwise a 1×1 RGBA32F placeholder is returned. The placeholder still
///   produces a valid `texture_cube<f32>` view so the output bind group is
///   well-formed; the fragment shader skips the sample whenever
///   `environment_diffuse_intensity.w` is zero.
pub(super) fn create_environment_cubemap_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    prepared: Option<&PreparedEnvironmentCubemap>,
) -> wgpu::Texture {
    let resolution = prepared.map(|c| c.resolution).unwrap_or(1).max(1);
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scena.m3.environment_cubemap"),
        size: wgpu::Extent3d {
            width: resolution,
            height: resolution,
            depth_or_array_layers: 6,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    if let Some(prepared) = prepared {
        for (face_index, face_pixels) in prepared.face_pixels.iter().enumerate() {
            let bytes = f32_slice_to_bytes_le(face_pixels);
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: face_index as u32,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &bytes,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(prepared.resolution * 16),
                    rows_per_image: Some(prepared.resolution),
                },
                wgpu::Extent3d {
                    width: prepared.resolution,
                    height: prepared.resolution,
                    depth_or_array_layers: 1,
                },
            );
        }
    }
    texture
}

/// Linear-filtered sampler for the environment cubemap. ClampToEdge addressing
/// matches WebGPU's recommendation for cubemaps; mipmap_filter is `Linear` so
/// the same sampler trilinearly interpolates the GGX prefilter mip chain that
/// will attach in Phase 1C step 2.
pub(super) fn create_environment_sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("scena.output.environment_sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
        ..Default::default()
    })
}

fn f32_slice_to_bytes_le(values: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * 4);
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}
