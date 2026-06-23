use std::f32::consts::PI;

use crate::assets::ENVIRONMENT_CUBEMAP_FACE_NORMALS;
use crate::scene::Vec3;

use self::source_mips::{
    build_source_cubemap_mip_chain, sample_source_cubemap_lod, source_mip_resolution,
};

mod source_mips;

/// Rust-owned image-based-lighting baker for scena's runtime and WASM paths.
///
/// This module intentionally follows the Khronos glTF IBL Sampler filtered
/// importance sampling shape: GGX prefilter samples choose a source cubemap LOD
/// from the sample PDF, and the BRDF LUT uses the same deterministic Hammersley
/// sequence. External tools are reference oracles only; this implementation is
/// the self-contained bake path used by native and browser prepare.
///
/// One sample direction-weight pair from the Hammersley sequence routed
/// through GGX importance sampling. Used by both the specular cubemap
/// prefilter and the BRDF LUT integrator.
struct GgxSample {
    direction: Vec3,
    n_dot_l: f32,
    n_dot_h: f32,
    v_dot_h: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::render) enum EnvironmentIblBakeQuality {
    Reference,
    InteractiveWebGl2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::render) struct EnvironmentIblBakeRequest {
    pub(in crate::render) source_resolution: u32,
    pub(in crate::render) mip_count: u32,
    pub(in crate::render) quality: EnvironmentIblBakeQuality,
    pub(in crate::render) brdf_lut_size: u32,
    pub(in crate::render) brdf_sample_count: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::render) struct BakedEnvironmentIbl {
    pub(in crate::render) mips: Vec<[Vec<f32>; 6]>,
    pub(in crate::render) mip_count: u32,
    pub(in crate::render) brdf_lut: Vec<f32>,
    pub(in crate::render) brdf_lut_size: u32,
}

pub(in crate::render) fn bake_environment_ibl(
    source_face_pixels: &[Vec<f32>; 6],
    request: EnvironmentIblBakeRequest,
) -> BakedEnvironmentIbl {
    let mip_count = request.mip_count.max(1);
    let brdf_lut_size = request.brdf_lut_size.max(1);
    BakedEnvironmentIbl {
        mips: prefilter_specular_cubemap_mips_with_quality(
            source_face_pixels,
            request.source_resolution.max(1),
            mip_count,
            request.quality,
        ),
        mip_count,
        brdf_lut: build_brdf_lut_with_sample_count(brdf_lut_size, request.brdf_sample_count),
        brdf_lut_size,
    }
}

/// Builds the GGX-prefiltered specular cubemap mip chain (one face buffer
/// per face per mip, RGBA32F). Mip 0 is the source radiance verbatim;
/// each subsequent mip is the source radiance convolved with a GGX BRDF
/// kernel at `prefilter_roughness_for_mip(mip, mip_count)`. The split-sum
/// approximation (Karis 2013) assumes view = normal at every fragment so
/// the prefilter is independent of camera position and a 2D BRDF LUT
/// can carry the view-dependent fresnel + geometry terms.
#[cfg(test)]
fn prefilter_specular_cubemap_mips(
    source_face_pixels: &[Vec<f32>; 6],
    resolution: u32,
    mip_count: u32,
) -> Vec<[Vec<f32>; 6]> {
    prefilter_specular_cubemap_mips_with_quality(
        source_face_pixels,
        resolution,
        mip_count,
        EnvironmentIblBakeQuality::Reference,
    )
}

fn prefilter_specular_cubemap_mips_with_quality(
    source_face_pixels: &[Vec<f32>; 6],
    resolution: u32,
    mip_count: u32,
    quality: EnvironmentIblBakeQuality,
) -> Vec<[Vec<f32>; 6]> {
    if mip_count == 0 {
        return Vec::new();
    }
    let mut mips = Vec::with_capacity(mip_count as usize);
    let source_mips = build_source_cubemap_mip_chain(source_face_pixels, resolution);
    for mip in 0..mip_count {
        let mip_resolution = (resolution >> mip).max(1);
        let mip_faces = if mip == 0 {
            source_face_pixels.clone()
        } else {
            let roughness = prefilter_roughness_for_mip(mip, mip_count);
            prefilter_face_pixels(&source_mips, mip_resolution, roughness, quality)
        };
        mips.push(mip_faces);
    }
    mips
}

/// Roughness represented by a prefilter mip index.
///
/// The mip chain has very few levels, so spacing them linearly makes near-mirror
/// materials (`roughness ~= 0.05`) blend in too much of the first heavily blurred
/// mip on GPU samplers. Squared spacing concentrates the available mip levels at
/// the low-roughness end: for five mips the represented roughness values are
/// `0.0, 0.0625, 0.25, 0.5625, 1.0`.
pub(in crate::render) fn prefilter_roughness_for_mip(mip: u32, mip_count: u32) -> f32 {
    let max_mip = mip_count.saturating_sub(1);
    if max_mip == 0 {
        return 0.0;
    }
    let normalized = mip.min(max_mip) as f32 / max_mip as f32;
    normalized * normalized
}

/// Fractional mip level for a material roughness.
///
/// This is the inverse of `prefilter_roughness_for_mip` and must stay in sync
/// with the WGSL `environment_prefilter_mip` helper in both output shaders.
pub(in crate::render) fn prefilter_lod_for_roughness(roughness: f32, mip_count: u32) -> f32 {
    let max_mip = mip_count.saturating_sub(1);
    if max_mip == 0 {
        return 0.0;
    }
    roughness.clamp(0.0, 1.0).sqrt() * max_mip as f32
}

pub(in crate::render) fn sample_prefiltered_cubemap_lod(
    mips: &[[Vec<f32>; 6]],
    direction: Vec3,
    lod: f32,
) -> Vec3 {
    sample_source_cubemap_lod(mips, direction, lod)
}

/// Builds the GGX prefilter for a single mip level of the specular
/// cubemap. Returns six face buffers of size `mip_resolution^2 * 4`.
fn prefilter_face_pixels(
    source_mips: &[[Vec<f32>; 6]],
    mip_resolution: u32,
    roughness: f32,
    quality: EnvironmentIblBakeQuality,
) -> [Vec<f32>; 6] {
    let sample_count = sample_count_for_roughness(roughness, quality);
    let mut faces: [Vec<f32>; 6] =
        std::array::from_fn(|_| vec![0.0_f32; (mip_resolution as usize).pow(2) * 4]);
    for (face_index, face_pixels) in faces.iter_mut().enumerate() {
        for y in 0..mip_resolution {
            for x in 0..mip_resolution {
                let u = (x as f32 + 0.5) / mip_resolution as f32 * 2.0 - 1.0;
                let v = (y as f32 + 0.5) / mip_resolution as f32 * 2.0 - 1.0;
                let normal = cubemap_face_direction(face_index, u, v);
                let prefiltered =
                    integrate_ggx_specular(normal, roughness, sample_count, source_mips);
                let pixel_index = ((y * mip_resolution + x) * 4) as usize;
                face_pixels[pixel_index] = prefiltered.x;
                face_pixels[pixel_index + 1] = prefiltered.y;
                face_pixels[pixel_index + 2] = prefiltered.z;
                face_pixels[pixel_index + 3] = 1.0;
            }
        }
    }
    faces
}

/// Build the split-sum BRDF LUT — a 2D RG f32 texture indexed by
/// `(N·V, roughness)`. Returned slice is `size * size * 2` floats laid
/// out row-major. The shader computes specular as
/// `prefiltered_radiance * (F0 * lut.x + lut.y)`.
#[cfg(test)]
fn build_brdf_lut(size: u32) -> Vec<f32> {
    build_brdf_lut_with_sample_count(size, 1024)
}

fn build_brdf_lut_with_sample_count(size: u32, sample_count: u32) -> Vec<f32> {
    let resolved_size = size.max(1);
    let mut pixels = vec![0.0_f32; (resolved_size as usize).pow(2) * 2];
    for y in 0..resolved_size {
        let roughness = (y as f32 + 0.5) / resolved_size as f32;
        for x in 0..resolved_size {
            let n_dot_v = (x as f32 + 0.5) / resolved_size as f32;
            let (scale, bias) = integrate_brdf_lut_cell(n_dot_v, roughness, sample_count);
            let pixel_index = ((y * resolved_size + x) * 2) as usize;
            pixels[pixel_index] = scale;
            pixels[pixel_index + 1] = bias;
        }
    }
    pixels
}

fn integrate_ggx_specular(
    normal: Vec3,
    roughness: f32,
    sample_count: u32,
    source_mips: &[[Vec<f32>; 6]],
) -> Vec3 {
    if sample_count == 0 {
        return sample_source_cubemap_lod(source_mips, normal, 0.0);
    }
    let view = normal;
    let source_resolution = source_mip_resolution(source_mips, 0);
    let mut accumulated = Vec3::ZERO;
    let mut total_weight = 0.0_f32;
    for sample_index in 0..sample_count {
        let sample = importance_sample_ggx(sample_index, sample_count, normal, roughness, view);
        if sample.n_dot_l <= 0.0 {
            continue;
        }
        let pdf = ggx_sample_pdf(sample.n_dot_h, sample.v_dot_h, roughness);
        let source_mip =
            source_mip_level_for_sample(roughness, sample_count, source_resolution, pdf);
        let radiance = sample_source_cubemap_lod(source_mips, sample.direction, source_mip);
        accumulated.x += radiance.x * sample.n_dot_l;
        accumulated.y += radiance.y * sample.n_dot_l;
        accumulated.z += radiance.z * sample.n_dot_l;
        total_weight += sample.n_dot_l;
    }
    if total_weight <= f32::EPSILON {
        return Vec3::ZERO;
    }
    let inverse = total_weight.recip();
    Vec3::new(
        accumulated.x * inverse,
        accumulated.y * inverse,
        accumulated.z * inverse,
    )
}

fn integrate_brdf_lut_cell(n_dot_v: f32, roughness: f32, sample_count: u32) -> (f32, f32) {
    if sample_count == 0 {
        return (0.0, 0.0);
    }
    let view = Vec3::new(
        (1.0 - n_dot_v * n_dot_v).max(0.0).sqrt(),
        0.0,
        n_dot_v.clamp(0.0, 1.0),
    );
    let normal = Vec3::new(0.0, 0.0, 1.0);
    let mut scale = 0.0_f32;
    let mut bias = 0.0_f32;
    for sample_index in 0..sample_count {
        let xi = hammersley_2d(sample_index, sample_count);
        let half = importance_sample_ggx_local(xi, normal, roughness);
        let v_dot_h = (view.x * half.x + view.y * half.y + view.z * half.z).max(0.0);
        let light = reflect_vec3(view, half);
        let n_dot_l = light.z.clamp(0.0, 1.0);
        if n_dot_l <= 0.0 {
            continue;
        }
        let n_dot_h = half.z.clamp(0.0, 1.0);
        if n_dot_h <= 0.0 {
            continue;
        }
        let geometry = geometry_smith_ggx(n_dot_v, n_dot_l, roughness);
        let visibility = geometry * v_dot_h / (n_dot_h * n_dot_v.max(1e-4));
        let fresnel = (1.0 - v_dot_h).clamp(0.0, 1.0).powi(5);
        scale += (1.0 - fresnel) * visibility;
        bias += fresnel * visibility;
    }
    (scale / sample_count as f32, bias / sample_count as f32)
}

fn importance_sample_ggx(
    sample_index: u32,
    sample_count: u32,
    normal: Vec3,
    roughness: f32,
    view: Vec3,
) -> GgxSample {
    let xi = hammersley_2d(sample_index, sample_count);
    let half_local = importance_sample_ggx_local(xi, Vec3::new(0.0, 0.0, 1.0), roughness);
    let half_world = transform_local_to_world(half_local, normal);
    let direction = reflect_vec3(view, half_world);
    let n_dot_l =
        (normal.x * direction.x + normal.y * direction.y + normal.z * direction.z).clamp(0.0, 1.0);
    let n_dot_h = dot(normal, half_world).clamp(0.0, 1.0);
    let v_dot_h = dot(view, half_world).clamp(0.0, 1.0);
    GgxSample {
        direction: normalize_or_z(direction),
        n_dot_l,
        n_dot_h,
        v_dot_h,
    }
}

fn importance_sample_ggx_local(xi: (f32, f32), normal_local: Vec3, roughness: f32) -> Vec3 {
    let alpha = roughness * roughness;
    let phi = 2.0 * PI * xi.0;
    let cos_theta_squared = ((1.0 - xi.1) / (1.0 + (alpha * alpha - 1.0) * xi.1)).max(0.0);
    let cos_theta = cos_theta_squared.sqrt();
    let sin_theta = (1.0 - cos_theta_squared).max(0.0).sqrt();
    let half_local = Vec3::new(sin_theta * phi.cos(), sin_theta * phi.sin(), cos_theta);
    let dot = normal_local.x * half_local.x
        + normal_local.y * half_local.y
        + normal_local.z * half_local.z;
    if dot >= 0.0 {
        half_local
    } else {
        Vec3::new(-half_local.x, -half_local.y, -half_local.z)
    }
}

fn transform_local_to_world(local: Vec3, normal: Vec3) -> Vec3 {
    let up = if normal.z.abs() < 0.999 {
        Vec3::new(0.0, 0.0, 1.0)
    } else {
        Vec3::new(1.0, 0.0, 0.0)
    };
    let tangent = normalize_or_z(cross(up, normal));
    let bitangent = cross(normal, tangent);
    Vec3::new(
        local.x * tangent.x + local.y * bitangent.x + local.z * normal.x,
        local.x * tangent.y + local.y * bitangent.y + local.z * normal.y,
        local.x * tangent.z + local.y * bitangent.z + local.z * normal.z,
    )
}

fn hammersley_2d(index: u32, count: u32) -> (f32, f32) {
    let count_inv = (count.max(1) as f32).recip();
    (
        index as f32 * count_inv,
        radical_inverse_van_der_corput(index),
    )
}

fn radical_inverse_van_der_corput(mut bits: u32) -> f32 {
    bits = bits.rotate_right(16);
    bits = ((bits & 0x55555555) << 1) | ((bits & 0xAAAAAAAA) >> 1);
    bits = ((bits & 0x33333333) << 2) | ((bits & 0xCCCCCCCC) >> 2);
    bits = ((bits & 0x0F0F0F0F) << 4) | ((bits & 0xF0F0F0F0) >> 4);
    bits = ((bits & 0x00FF00FF) << 8) | ((bits & 0xFF00FF00) >> 8);
    bits as f32 * 2.328_306_4e-10
}

fn geometry_smith_ggx(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    let alpha = roughness * roughness;
    let k = alpha * 0.5;
    let smith_v = n_dot_v / (n_dot_v * (1.0 - k) + k).max(1e-4);
    let smith_l = n_dot_l / (n_dot_l * (1.0 - k) + k).max(1e-4);
    smith_v * smith_l
}

fn ggx_normal_distribution(n_dot_h: f32, roughness: f32) -> f32 {
    let alpha = roughness * roughness;
    let alpha_squared = (alpha * alpha).max(1e-6);
    let n_dot_h_squared = n_dot_h.clamp(0.0, 1.0) * n_dot_h.clamp(0.0, 1.0);
    let denominator = (n_dot_h_squared * (alpha_squared - 1.0) + 1.0).max(1e-6);
    alpha_squared / (PI * denominator * denominator)
}

fn ggx_sample_pdf(n_dot_h: f32, v_dot_h: f32, roughness: f32) -> f32 {
    let distribution = ggx_normal_distribution(n_dot_h, roughness);
    (distribution * n_dot_h.clamp(0.0, 1.0) / (4.0 * v_dot_h.max(1e-4))).max(1e-6)
}

fn source_mip_level_for_sample(
    roughness: f32,
    sample_count: u32,
    source_resolution: u32,
    pdf: f32,
) -> f32 {
    if roughness <= 1e-4 || source_resolution <= 1 || sample_count == 0 {
        return 0.0;
    }
    let resolution = source_resolution as f32;
    // Khronos glTF IBL Sampler `computeLod`: filtered importance sampling
    // chooses the source mip from the sample PDF and source cubemap texel
    // count, independent of the output mip resolution.
    let weighted_texel_count = 6.0 * resolution * resolution;
    (0.5 * (weighted_texel_count / (sample_count as f32 * pdf.max(1e-6))).log2()).max(0.0)
}

fn reflect_vec3(view: Vec3, normal: Vec3) -> Vec3 {
    let dot = dot(view, normal);
    Vec3::new(
        2.0 * dot * normal.x - view.x,
        2.0 * dot * normal.y - view.y,
        2.0 * dot * normal.z - view.z,
    )
}

fn dot(a: Vec3, b: Vec3) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

fn cross(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

fn normalize_or_z(value: Vec3) -> Vec3 {
    let length = (value.x * value.x + value.y * value.y + value.z * value.z).sqrt();
    if length <= f32::EPSILON || !length.is_finite() {
        Vec3::new(0.0, 0.0, 1.0)
    } else {
        let inv = length.recip();
        Vec3::new(value.x * inv, value.y * inv, value.z * inv)
    }
}

/// Number of importance samples per pixel for a given roughness. Mip 0
/// (roughness 0) needs no convolution and we route it through this
/// table only for completeness; smoother surfaces converge at fewer
/// samples while rougher surfaces benefit from many more.
fn sample_count_for_roughness(roughness: f32, quality: EnvironmentIblBakeQuality) -> u32 {
    let stepped = (roughness.clamp(0.0, 1.0) * 8.0).round() as u32;
    match quality {
        EnvironmentIblBakeQuality::Reference => match stepped {
            0 => 32,
            1 | 2 => 96,
            3 | 4 => 192,
            5 | 6 => 384,
            _ => 768,
        },
        EnvironmentIblBakeQuality::InteractiveWebGl2 => match stepped {
            0 => 16,
            1 | 2 => 96,
            3 | 4 => 128,
            _ => 192,
        },
    }
}

fn cubemap_face_direction(face_index: usize, u: f32, v: f32) -> Vec3 {
    let normal = ENVIRONMENT_CUBEMAP_FACE_NORMALS[face_index.min(5)];
    let raw = match face_index {
        0 => Vec3::new(1.0, -v, -u),
        1 => Vec3::new(-1.0, -v, u),
        2 => Vec3::new(u, 1.0, v),
        3 => Vec3::new(u, -1.0, -v),
        4 => Vec3::new(u, -v, 1.0),
        _ => Vec3::new(-u, -v, -1.0),
    };
    let _ = normal;
    normalize_or_z(raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uniform_cubemap(value: f32) -> [Vec<f32>; 6] {
        std::array::from_fn(|_| {
            let mut face = Vec::with_capacity(64 * 64 * 4);
            for _ in 0..(64 * 64) {
                face.extend_from_slice(&[value, value, value, 1.0]);
            }
            face
        })
    }

    fn overbright_softbox_cubemap() -> [Vec<f32>; 6] {
        let resolution = 64;
        let mut faces = uniform_cubemap(0.02);
        let face = &mut faces[4];
        let index = ((32 * resolution + 32) * 4) as usize;
        face[index] = 80.0;
        face[index + 1] = 80.0;
        face[index + 2] = 80.0;
        faces
    }

    fn bright_outlier_fraction(face_pixels: &[f32], threshold: f32) -> f32 {
        let pixels = face_pixels.len() / 4;
        if pixels == 0 {
            return 0.0;
        }
        let outliers = face_pixels
            .chunks_exact(4)
            .filter(|pixel| pixel[0].max(pixel[1]).max(pixel[2]) >= threshold)
            .count();
        outliers as f32 / pixels as f32
    }

    fn max_rgb(face_pixels: &[f32]) -> f32 {
        face_pixels
            .chunks_exact(4)
            .map(|pixel| pixel[0].max(pixel[1]).max(pixel[2]))
            .fold(0.0_f32, f32::max)
    }

    #[test]
    fn ggx_prefilter_suppresses_tiny_hdr_firefly_outliers() {
        let source = overbright_softbox_cubemap();
        let mips = prefilter_specular_cubemap_mips_with_quality(
            &source,
            64,
            5,
            EnvironmentIblBakeQuality::InteractiveWebGl2,
        );
        let near_mirror_face = &mips[1][4];
        let max_near_mirror = max_rgb(near_mirror_face);
        let near_mirror_bright_fraction = bright_outlier_fraction(near_mirror_face, 4.0);
        let rough_face = &mips[2][4];
        let max_rough_firefly = max_rgb(rough_face);
        let rough_bright_fraction = bright_outlier_fraction(rough_face, 4.0);
        assert!(
            max_near_mirror <= 20.0 && near_mirror_bright_fraction <= 0.005,
            "near-mirror prefilter mip must retain bright reflection detail without isolated firefly coverage; max={max_near_mirror:.3}, bright_fraction={near_mirror_bright_fraction:.5}"
        );
        assert!(
            max_rough_firefly <= 8.0 && rough_bright_fraction <= 0.005,
            "rough prefilter mip must blur tiny HDR softboxes instead of baking firefly texels; max={max_rough_firefly:.3}, bright_fraction={rough_bright_fraction:.5}"
        );
    }

    #[test]
    fn prefilter_returns_one_buffer_per_requested_mip() {
        let source: [Vec<f32>; 6] = std::array::from_fn(|_| vec![0.5; 4 * 4 * 4]);
        let mips = prefilter_specular_cubemap_mips(&source, 4, 3);
        assert_eq!(mips.len(), 3, "one buffer per mip including mip 0");
        for (mip, faces) in mips.iter().enumerate() {
            let expected_resolution = 4 >> mip;
            assert_eq!(
                faces[0].len(),
                (expected_resolution as usize).pow(2) * 4,
                "mip {mip} face buffer must size to its mip resolution"
            );
        }
    }

    #[test]
    fn bake_environment_ibl_owns_specular_mips_and_brdf_lut_product() {
        let source: [Vec<f32>; 6] = std::array::from_fn(|_| vec![0.25; 8 * 8 * 4]);
        let baked = bake_environment_ibl(
            &source,
            EnvironmentIblBakeRequest {
                source_resolution: 8,
                mip_count: 4,
                quality: EnvironmentIblBakeQuality::InteractiveWebGl2,
                brdf_lut_size: 8,
                brdf_sample_count: 64,
            },
        );
        assert_eq!(baked.mip_count, 4);
        assert_eq!(baked.mips.len(), 4);
        assert_eq!(baked.brdf_lut_size, 8);
        assert_eq!(baked.brdf_lut.len(), 8 * 8 * 2);
        assert_eq!(
            baked.mips[0][0].len(),
            8 * 8 * 4,
            "baker owns the mip-0 source-radiance payload"
        );
    }

    #[test]
    fn prefilter_roughness_lod_mapping_is_shared_and_low_roughness_concentrated() {
        let mip_count = 5;
        assert!((prefilter_roughness_for_mip(0, mip_count) - 0.0).abs() < 1.0e-6);
        assert!((prefilter_roughness_for_mip(1, mip_count) - 0.0625).abs() < 1.0e-6);
        assert!((prefilter_roughness_for_mip(2, mip_count) - 0.25).abs() < 1.0e-6);
        assert!((prefilter_roughness_for_mip(3, mip_count) - 0.5625).abs() < 1.0e-6);
        assert!((prefilter_roughness_for_mip(4, mip_count) - 1.0).abs() < 1.0e-6);

        for mip in 0..mip_count {
            let roughness = prefilter_roughness_for_mip(mip, mip_count);
            let lod = prefilter_lod_for_roughness(roughness, mip_count);
            assert!(
                (lod - mip as f32).abs() < 1.0e-5,
                "roughness-to-LOD must invert prefilter mip roughness for mip {mip}: roughness={roughness}, lod={lod}"
            );
        }
        assert!(
            prefilter_lod_for_roughness(0.05, mip_count) < 1.0,
            "chrome-like roughness must sample only the sharp source and the first low-roughness mip"
        );
    }

    #[test]
    fn source_mip_lod_matches_khronos_filtered_importance_sampling_formula() {
        let source_resolution = 64;
        let sample_count = 128;
        let pdf = 0.25;
        let expected = 0.5 * ((6.0 * 64.0_f32 * 64.0) / (sample_count as f32 * pdf)).log2();
        let actual = source_mip_level_for_sample(0.5, sample_count, source_resolution, pdf);
        assert!(
            (actual - expected).abs() < 1.0e-5,
            "filtered importance sampling source LOD must match Khronos glTF IBL Sampler computeLod; expected {expected}, got {actual}"
        );
    }

    #[test]
    fn prefilter_of_uniform_cubemap_remains_uniform_per_face() {
        let source = uniform_cubemap(0.42);
        let mips = prefilter_specular_cubemap_mips(&source, 64, 4);
        // Mip 0 is the verbatim source; later mips integrate the GGX
        // kernel over a uniform input — the integral of any kernel over
        // a constant source returns the same constant.
        for (mip, faces) in mips.iter().enumerate() {
            for (face_index, face_pixels) in faces.iter().enumerate() {
                for (pixel_offset, value) in face_pixels.iter().enumerate() {
                    let channel = pixel_offset % 4;
                    let expected = if channel == 3 { 1.0 } else { 0.42 };
                    let tolerance = if channel == 3 || mip == 0 { 1e-4 } else { 0.05 };
                    assert!(
                        (value - expected).abs() < tolerance,
                        "mip {mip} face {face_index} pixel {pixel_offset} channel {channel} = \
                         {value} drifted from uniform input by more than {tolerance}"
                    );
                }
            }
        }
    }

    #[test]
    fn brdf_lut_endpoints_match_split_sum_reference() {
        let lut = build_brdf_lut(64);
        // At (NoV ≈ 1, roughness ≈ 0) the GGX kernel collapses to a delta
        // at the reflection direction so scale ≈ 1, bias ≈ 0.
        let bottom_right_index = 63 * 2;
        let scale_low_roughness = lut[bottom_right_index];
        let bias_low_roughness = lut[bottom_right_index + 1];
        assert!(
            scale_low_roughness > 0.7 && scale_low_roughness < 1.05,
            "low-roughness, high-NoV scale = {scale_low_roughness} must approach 1"
        );
        assert!(
            bias_low_roughness < 0.1,
            "low-roughness bias = {bias_low_roughness} must approach 0"
        );
        // At (NoV ≈ 0, any roughness) the integral of fresnel-weighted
        // GGX visibility tends to small positive values rather than 0
        // because the BRDF still picks up grazing-angle contributions.
        let grazing_index = 32 * 64 * 2;
        let scale_grazing = lut[grazing_index];
        let bias_grazing = lut[grazing_index + 1];
        assert!(
            scale_grazing.is_finite() && bias_grazing.is_finite(),
            "BRDF LUT must produce finite values everywhere"
        );
    }

    #[test]
    fn interactive_prefilter_profile_caps_browser_runtime_work() {
        assert_eq!(
            sample_count_for_roughness(1.0, EnvironmentIblBakeQuality::Reference),
            768,
            "reference quality keeps the existing rough-environment sample count"
        );
        assert_eq!(
            sample_count_for_roughness(0.28, EnvironmentIblBakeQuality::InteractiveWebGl2),
            96,
            "WebGL2 smooth-metal prefiltering must sample enough directions for chrome/brushed-metal presets"
        );
        assert_eq!(
            sample_count_for_roughness(1.0, EnvironmentIblBakeQuality::InteractiveWebGl2),
            192,
            "WebGL2 rough-environment prefiltering stays below reference quality while no longer using the old 16-sample cap"
        );
        assert!(
            sample_count_for_roughness(1.0, EnvironmentIblBakeQuality::InteractiveWebGl2)
                < sample_count_for_roughness(1.0, EnvironmentIblBakeQuality::Reference),
            "interactive WebGL2 profile remains bounded below the reference offline sample count"
        );
        assert_eq!(
            build_brdf_lut_with_sample_count(4, 64).len(),
            4 * 4 * 2,
            "interactive BRDF LUT generation keeps the same texture layout"
        );
    }

    #[test]
    fn hammersley_radical_inverse_is_deterministic() {
        let count = 8;
        let mut seen = std::collections::HashSet::new();
        for index in 0..count {
            let (a, b) = hammersley_2d(index, count);
            assert!(
                a.is_finite() && b.is_finite(),
                "Hammersley pair {index} must be finite"
            );
            assert!(
                seen.insert((a.to_bits(), b.to_bits())),
                "Hammersley sequence must produce unique 2D samples within {count}"
            );
        }
    }
}
