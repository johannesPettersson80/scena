use super::super::prepare::{PreparedEnvironmentCubemap, PreparedEnvironmentLighting};
use super::output::create_output_bind_group;
use super::shadow::{
    self, ShadowCasterResources, create_shadow_caster_resources, create_shadow_sampler,
};

/// Bundles the per-frame group-0 GPU resources that are shared by every
/// render pass: shadow caster (texture + pipeline + active flag), shadow
/// comparison sampler, environment cubemap (mip chain), environment
/// sampler, the BRDF LUT (split-sum specular composition), and the
/// output bind group that ties them together with the uniform buffer.
pub(super) struct OutputResources {
    pub(super) shadow_caster: ShadowCasterResources,
    pub(super) shadow_sampler: wgpu::Sampler,
    pub(super) environment_cubemap: wgpu::Texture,
    pub(super) environment_sampler: wgpu::Sampler,
    pub(super) brdf_lut_texture: wgpu::Texture,
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
        output_uniform,
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
    let brdf_lut_texture = create_brdf_lut_texture(device, queue, environment_lighting.cubemap());
    let brdf_lut_view = brdf_lut_texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("scena.output.brdf_lut_view"),
        ..Default::default()
    });
    let output_bind_group = create_output_bind_group(
        device,
        output_bind_group_layout,
        output_uniform,
        &shadow_caster.view,
        &shadow_sampler,
        &environment_cubemap_view,
        &environment_sampler,
        &brdf_lut_view,
    );
    OutputResources {
        shadow_caster,
        shadow_sampler,
        environment_cubemap,
        environment_sampler,
        brdf_lut_texture,
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
    let mip_count = prepared.map(|c| c.mip_count).unwrap_or(1).max(1);
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scena.m3.environment_cubemap"),
        size: wgpu::Extent3d {
            width: resolution,
            height: resolution,
            depth_or_array_layers: 6,
        },
        mip_level_count: mip_count,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        // Rgba16Float is filterable on every wgpu adapter we target;
        // Rgba32Float requires the `Float32Filterable` feature which the
        // V3D (Pi 5), most mobile GPUs, and many lavapipe builds lack.
        // HDR cubemap radiance values fit comfortably in fp16 (signed
        // exponent up to 15, mantissa 10 bits).
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    if let Some(prepared) = prepared {
        for (mip_index, faces) in prepared.mips.iter().enumerate() {
            let mip_resolution = (prepared.resolution >> mip_index).max(1);
            for (face_index, face_pixels) in faces.iter().enumerate() {
                let bytes = f32_slice_to_rgba16f_bytes(face_pixels);
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &texture,
                        mip_level: mip_index as u32,
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
                        // Rgba16Float is 8 bytes per pixel.
                        bytes_per_row: Some(mip_resolution * 8),
                        rows_per_image: Some(mip_resolution),
                    },
                    wgpu::Extent3d {
                        width: mip_resolution,
                        height: mip_resolution,
                        depth_or_array_layers: 1,
                    },
                );
            }
        }
    }
    texture
}

fn f32_slice_to_rgba16f_bytes(values: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * 2);
    for &value in values {
        let half = f32_to_f16_bits(value);
        bytes.extend_from_slice(&half.to_le_bytes());
    }
    bytes
}

/// Minimal IEEE-754 f32 → binary16 conversion (round to nearest, ties to
/// even). Handles subnormals, infinities, and NaN; preserves sign. Used by
/// the cubemap upload path so we don't pull in a half-precision crate.
fn f32_to_f16_bits(value: f32) -> u16 {
    let bits = value.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let exp32 = ((bits >> 23) & 0xff) as i32;
    let mant32 = bits & 0x007f_ffff;
    if exp32 == 0xff {
        // NaN / Inf
        let mant16 = if mant32 != 0 { 0x200 } else { 0 };
        return sign | 0x7c00 | mant16;
    }
    let exp = exp32 - 127 + 15;
    if exp >= 0x1f {
        return sign | 0x7c00; // overflow → Inf
    }
    if exp <= 0 {
        if exp < -10 {
            return sign; // underflow → ±0
        }
        let mant = mant32 | 0x0080_0000;
        let shift = 14 - exp;
        let rounded = (mant + (1 << (shift - 1))) >> shift;
        return sign | rounded as u16;
    }
    let mant = mant32 >> 13;
    let round = (mant32 & 0x1000) >> 12;
    sign | ((exp as u16) << 10) | (mant as u16 + round as u16)
}

/// Allocates the 2D BRDF LUT (RG32Float) and uploads the prepared LUT
/// pixels. Falls back to a 1×1 zero placeholder when no environment is
/// bound; the WGSL shader gates the LUT sample on the same coverage flag
/// as the diffuse cubemap so the placeholder is never read.
pub(super) fn create_brdf_lut_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    prepared: Option<&PreparedEnvironmentCubemap>,
) -> wgpu::Texture {
    let size = prepared.map(|c| c.brdf_lut_size).unwrap_or(1).max(1);
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scena.m3.brdf_lut"),
        size: wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rg32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    if let Some(prepared) = prepared {
        let bytes = f32_slice_to_bytes_le(&prepared.brdf_lut);
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(prepared.brdf_lut_size * 8),
                rows_per_image: Some(prepared.brdf_lut_size),
            },
            wgpu::Extent3d {
                width: prepared.brdf_lut_size,
                height: prepared.brdf_lut_size,
                depth_or_array_layers: 1,
            },
        );
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
