use std::{collections::BTreeMap, f32::consts::PI};

use crate::assets::ENVIRONMENT_CUBEMAP_FACE_NORMALS;
use crate::scene::Vec3;

#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

use self::source_mips::{
    build_source_cubemap_mip_chain, sample_source_cubemap_lod, source_mip_resolution,
};
use super::super::pbr_brdf;

mod brdf;
mod source_mips;
#[cfg(test)]
use brdf::{
    build_brdf_lut, build_brdf_lut_with_sample_count, hammersley_2d, source_mip_level_for_sample,
};
use brdf::{
    build_brdf_lut_with_sample_count_profiled, integrate_ggx_specular, normalize_or_z,
    sample_count_for_roughness,
};

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

/// Deterministic work counters from one environment IBL bake.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EnvironmentBakeMetrics {
    /// Calls into the cubemap LOD sampler made by the GGX prefilter.
    pub source_texture_samples: u64,
    /// Hammersley samples evaluated while building the split-sum BRDF LUT.
    pub brdf_integration_samples: u64,
    /// RGBA texels emitted across all prefiltered cubemap faces and mips.
    pub prefilter_output_texels: u64,
    /// RG texels emitted into the split-sum BRDF LUT.
    pub brdf_lut_texels: u64,
    /// Bytes written to the cubemap and BRDF-LUT bake products.
    pub output_bytes_written: u64,
    /// Maximum bounded worker count selected by any bake stage.
    pub parallel_workers: u64,
    /// Independent face or row tasks made eligible for parallel execution.
    pub parallel_tasks: u64,
}

pub(in crate::render) fn bake_environment_ibl(
    source_face_pixels: &[Vec<f32>; 6],
    request: EnvironmentIblBakeRequest,
) -> BakedEnvironmentIbl {
    bake_environment_ibl_profiled(source_face_pixels, request).0
}

pub(in crate::render) fn bake_environment_ibl_profiled(
    source_face_pixels: &[Vec<f32>; 6],
    request: EnvironmentIblBakeRequest,
) -> (BakedEnvironmentIbl, EnvironmentBakeMetrics) {
    let task_count = request
        .mip_count
        .saturating_sub(1)
        .saturating_mul(6)
        .saturating_add(request.brdf_lut_size) as usize;
    bake_environment_ibl_profiled_with_workers(
        source_face_pixels,
        request,
        super::super::parallel::worker_count(task_count),
    )
}

fn bake_environment_ibl_profiled_with_workers(
    source_face_pixels: &[Vec<f32>; 6],
    request: EnvironmentIblBakeRequest,
    requested_workers: usize,
) -> (BakedEnvironmentIbl, EnvironmentBakeMetrics) {
    let mip_count = request.mip_count.max(1);
    let brdf_lut_size = request.brdf_lut_size.max(1);
    let mut metrics = EnvironmentBakeMetrics::default();
    let workers = requested_workers
        .max(1)
        .min(super::super::parallel::worker_count(
            mip_count
                .saturating_sub(1)
                .saturating_mul(6)
                .saturating_add(brdf_lut_size) as usize,
        ));
    metrics.parallel_workers = workers as u64;
    let mips = prefilter_specular_cubemap_mips_with_quality_profiled(
        source_face_pixels,
        request.source_resolution.max(1),
        mip_count,
        request.quality,
        workers,
        &mut metrics,
    );
    let brdf_lut = build_brdf_lut_with_sample_count_profiled(
        brdf_lut_size,
        request.brdf_sample_count,
        workers,
        &mut metrics,
    );
    metrics.output_bytes_written = metrics
        .prefilter_output_texels
        .saturating_mul(4)
        .saturating_mul(std::mem::size_of::<f32>() as u64)
        .saturating_add(
            metrics
                .brdf_lut_texels
                .saturating_mul(2)
                .saturating_mul(std::mem::size_of::<f32>() as u64),
        );
    (
        BakedEnvironmentIbl {
            mips,
            mip_count,
            brdf_lut,
            brdf_lut_size,
        },
        metrics,
    )
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

#[cfg(test)]
fn prefilter_specular_cubemap_mips_with_quality(
    source_face_pixels: &[Vec<f32>; 6],
    resolution: u32,
    mip_count: u32,
    quality: EnvironmentIblBakeQuality,
) -> Vec<[Vec<f32>; 6]> {
    prefilter_specular_cubemap_mips_with_quality_profiled(
        source_face_pixels,
        resolution,
        mip_count,
        quality,
        1,
        &mut EnvironmentBakeMetrics::default(),
    )
}

fn prefilter_specular_cubemap_mips_with_quality_profiled(
    source_face_pixels: &[Vec<f32>; 6],
    resolution: u32,
    mip_count: u32,
    quality: EnvironmentIblBakeQuality,
    workers: usize,
    metrics: &mut EnvironmentBakeMetrics,
) -> Vec<[Vec<f32>; 6]> {
    if mip_count == 0 {
        return Vec::new();
    }
    let mut mips = Vec::with_capacity(mip_count as usize);
    let source_mips = build_source_cubemap_mip_chain(source_face_pixels, resolution);
    for mip in 0..mip_count {
        let mip_resolution = (resolution >> mip).max(1);
        let mut mip_faces = if mip == 0 {
            source_face_pixels.clone()
        } else {
            let roughness = prefilter_roughness_for_mip(mip, mip_count);
            prefilter_face_pixels(
                &source_mips,
                mip_resolution,
                roughness,
                quality,
                workers,
                metrics,
            )
        };
        if mip > 0 {
            stitch_prefiltered_cubemap_edges(&mut mip_faces, mip_resolution);
        }
        metrics.prefilter_output_texels = metrics
            .prefilter_output_texels
            .saturating_add(u64::from(mip_resolution).pow(2).saturating_mul(6));
        mips.push(mip_faces);
    }
    mips
}

fn stitch_prefiltered_cubemap_edges(faces: &mut [Vec<f32>; 6], resolution: u32) {
    let size = resolution as usize;
    if size == 0 {
        return;
    }

    let mut edge_groups = BTreeMap::<(i32, i32, i32), Vec<(usize, usize)>>::new();
    for face in 0..6 {
        for step in 0..size {
            let tangent = (step as f32 + 0.5) / size as f32 * 2.0 - 1.0;
            for (x, y, u, v) in [
                (0, step, -1.0, tangent),
                (size - 1, step, 1.0, tangent),
                (step, 0, tangent, -1.0),
                (step, size - 1, tangent, 1.0),
            ] {
                let direction = cubemap_face_direction(face, u, v);
                let key = (
                    (direction.x * 1_000_000.0).round() as i32,
                    (direction.y * 1_000_000.0).round() as i32,
                    (direction.z * 1_000_000.0).round() as i32,
                );
                edge_groups
                    .entry(key)
                    .or_default()
                    .push((face, (y * size + x) * 4));
            }
        }
    }

    let source = faces.clone();
    let mut updates = BTreeMap::<(usize, usize), ([f64; 4], u32)>::new();
    for texels in edge_groups.values().filter(|texels| texels.len() >= 2) {
        let mut average = [0.0_f64; 4];
        for &(face, offset) in texels {
            for channel in 0..4 {
                average[channel] += f64::from(source[face][offset + channel]);
            }
        }
        let inverse_count = (texels.len() as f64).recip();
        average.iter_mut().for_each(|value| *value *= inverse_count);
        for &(face, offset) in texels {
            let update = updates.entry((face, offset)).or_insert(([0.0; 4], 0));
            for (sum, value) in update.0.iter_mut().zip(average) {
                *sum += value;
            }
            update.1 += 1;
        }
    }
    for ((face, offset), (sum, count)) in updates {
        let inverse_count = f64::from(count).recip();
        for (channel, value) in sum.into_iter().enumerate() {
            faces[face][offset + channel] = (value * inverse_count) as f32;
        }
    }
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
    workers: usize,
    metrics: &mut EnvironmentBakeMetrics,
) -> [Vec<f32>; 6] {
    #[cfg(target_arch = "wasm32")]
    let _ = workers;
    let sample_count = sample_count_for_roughness(roughness, quality);
    let mut faces: [Vec<f32>; 6] =
        std::array::from_fn(|_| vec![0.0_f32; (mip_resolution as usize).pow(2) * 4]);
    metrics.parallel_tasks = metrics.parallel_tasks.saturating_add(6);
    #[cfg(not(target_arch = "wasm32"))]
    if workers > 1 {
        let sample_counts = faces
            .par_iter_mut()
            .enumerate()
            .map(|(face_index, face_pixels)| {
                fill_prefilter_face(
                    source_mips,
                    mip_resolution,
                    roughness,
                    sample_count,
                    face_index,
                    face_pixels,
                )
            })
            .collect::<Vec<_>>();
        metrics.source_texture_samples = metrics
            .source_texture_samples
            .saturating_add(sample_counts.into_iter().sum());
        return faces;
    }
    for (face_index, face_pixels) in faces.iter_mut().enumerate() {
        metrics.source_texture_samples =
            metrics
                .source_texture_samples
                .saturating_add(fill_prefilter_face(
                    source_mips,
                    mip_resolution,
                    roughness,
                    sample_count,
                    face_index,
                    face_pixels,
                ));
    }
    faces
}

fn fill_prefilter_face(
    source_mips: &[[Vec<f32>; 6]],
    mip_resolution: u32,
    roughness: f32,
    sample_count: u32,
    face_index: usize,
    face_pixels: &mut [f32],
) -> u64 {
    let mut local_metrics = EnvironmentBakeMetrics::default();
    for y in 0..mip_resolution {
        for x in 0..mip_resolution {
            let u = (x as f32 + 0.5) / mip_resolution as f32 * 2.0 - 1.0;
            let v = (y as f32 + 0.5) / mip_resolution as f32 * 2.0 - 1.0;
            let normal = cubemap_face_direction(face_index, u, v);
            let prefiltered = integrate_ggx_specular(
                normal,
                roughness,
                sample_count,
                source_mips,
                &mut local_metrics,
            );
            let pixel_index = ((y * mip_resolution + x) * 4) as usize;
            face_pixels[pixel_index] = prefiltered.x;
            face_pixels[pixel_index + 1] = prefiltered.y;
            face_pixels[pixel_index + 2] = prefiltered.z;
            face_pixels[pixel_index + 3] = 1.0;
        }
    }
    local_metrics.source_texture_samples
}

/// Build the split-sum BRDF LUT — a 2D RG f32 texture indexed by
/// `(N·V, roughness)`. Returned slice is `size * size * 2` floats laid
/// out row-major. The shader computes specular as
/// `prefiltered_radiance * (F0 * lut.x + lut.y)`.
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

    #[test]
    fn pf09_parallel_environment_faces_and_rows_match_serial_bit_for_bit() {
        let source = overbright_softbox_cubemap();
        let request = EnvironmentIblBakeRequest {
            source_resolution: 64,
            mip_count: 5,
            quality: EnvironmentIblBakeQuality::InteractiveWebGl2,
            brdf_lut_size: 16,
            brdf_sample_count: 64,
        };

        let (serial, serial_metrics) =
            bake_environment_ibl_profiled_with_workers(&source, request, 1);
        let (parallel, parallel_metrics) =
            bake_environment_ibl_profiled_with_workers(&source, request, 4);

        assert_eq!(parallel, serial, "parallel face/row work must be bit-exact");
        assert_eq!(
            parallel_metrics.source_texture_samples,
            serial_metrics.source_texture_samples
        );
        assert_eq!(
            parallel_metrics.brdf_integration_samples,
            serial_metrics.brdf_integration_samples
        );
        assert_eq!(serial_metrics.parallel_workers, 1);
        assert!(
            parallel_metrics.parallel_workers > 1,
            "the focused native proof must exercise bounded parallel work"
        );
        assert!(parallel_metrics.parallel_tasks >= 6 + 16);
    }
}
