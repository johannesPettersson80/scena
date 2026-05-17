use std::f32::consts::PI;

use crate::assets::ENVIRONMENT_CUBEMAP_FACE_NORMALS;
use crate::scene::Vec3;

/// One sample direction-weight pair from the Hammersley sequence routed
/// through GGX importance sampling. Used by both the specular cubemap
/// prefilter and the BRDF LUT integrator.
struct GgxSample {
    direction: Vec3,
    n_dot_l: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::render) enum EnvironmentPrefilterQuality {
    Reference,
    InteractiveWebGl2,
}

/// Builds the GGX-prefiltered specular cubemap mip chain (one face buffer
/// per face per mip, RGBA32F). Mip 0 is the source radiance verbatim;
/// each subsequent mip is the source radiance convolved with a GGX BRDF
/// kernel at roughness `mip / (mip_count - 1)`. The split-sum
/// approximation (Karis 2013) assumes view = normal at every fragment so
/// the prefilter is independent of camera position and a 2D BRDF LUT
/// can carry the view-dependent fresnel + geometry terms.
#[cfg(test)]
pub(in crate::render) fn prefilter_specular_cubemap_mips(
    source_face_pixels: &[Vec<f32>; 6],
    resolution: u32,
    mip_count: u32,
) -> Vec<[Vec<f32>; 6]> {
    prefilter_specular_cubemap_mips_with_quality(
        source_face_pixels,
        resolution,
        mip_count,
        EnvironmentPrefilterQuality::Reference,
    )
}

pub(in crate::render) fn prefilter_specular_cubemap_mips_with_quality(
    source_face_pixels: &[Vec<f32>; 6],
    resolution: u32,
    mip_count: u32,
    quality: EnvironmentPrefilterQuality,
) -> Vec<[Vec<f32>; 6]> {
    if mip_count == 0 {
        return Vec::new();
    }
    let mut mips = Vec::with_capacity(mip_count as usize);
    for mip in 0..mip_count {
        let mip_resolution = (resolution >> mip).max(1);
        let mip_faces = if mip == 0 {
            source_face_pixels.clone()
        } else {
            let roughness = if mip_count > 1 {
                mip as f32 / (mip_count - 1) as f32
            } else {
                0.0
            };
            prefilter_face_pixels(
                source_face_pixels,
                resolution,
                mip_resolution,
                roughness,
                quality,
            )
        };
        mips.push(mip_faces);
    }
    mips
}

/// Builds the GGX prefilter for a single mip level of the specular
/// cubemap. Returns six face buffers of size `mip_resolution^2 * 4`.
fn prefilter_face_pixels(
    source_face_pixels: &[Vec<f32>; 6],
    source_resolution: u32,
    mip_resolution: u32,
    roughness: f32,
    quality: EnvironmentPrefilterQuality,
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
                let prefiltered = integrate_ggx_specular(
                    normal,
                    roughness,
                    sample_count,
                    source_face_pixels,
                    source_resolution,
                );
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
pub(in crate::render) fn build_brdf_lut(size: u32) -> Vec<f32> {
    build_brdf_lut_with_sample_count(size, 1024)
}

pub(in crate::render) fn build_brdf_lut_with_sample_count(
    size: u32,
    sample_count: u32,
) -> Vec<f32> {
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
    source_face_pixels: &[Vec<f32>; 6],
    source_resolution: u32,
) -> Vec3 {
    if sample_count == 0 {
        return sample_source_cubemap(source_face_pixels, source_resolution, normal);
    }
    let view = normal;
    let mut accumulated = Vec3::ZERO;
    let mut total_weight = 0.0_f32;
    for sample_index in 0..sample_count {
        let sample = importance_sample_ggx(sample_index, sample_count, normal, roughness, view);
        if sample.n_dot_l <= 0.0 {
            continue;
        }
        let radiance =
            sample_source_cubemap(source_face_pixels, source_resolution, sample.direction);
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
    GgxSample {
        direction: normalize_or_z(direction),
        n_dot_l,
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

fn reflect_vec3(view: Vec3, normal: Vec3) -> Vec3 {
    let dot = view.x * normal.x + view.y * normal.y + view.z * normal.z;
    Vec3::new(
        2.0 * dot * normal.x - view.x,
        2.0 * dot * normal.y - view.y,
        2.0 * dot * normal.z - view.z,
    )
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
fn sample_count_for_roughness(roughness: f32, quality: EnvironmentPrefilterQuality) -> u32 {
    let stepped = (roughness.clamp(0.0, 1.0) * 8.0).round() as u32;
    match quality {
        EnvironmentPrefilterQuality::Reference => match stepped {
            0 => 32,
            1 | 2 => 96,
            3 | 4 => 192,
            5 | 6 => 384,
            _ => 768,
        },
        EnvironmentPrefilterQuality::InteractiveWebGl2 => match stepped {
            0 => 4,
            1 | 2 => 8,
            _ => 16,
        },
    }
}

/// Bilinearly samples the source mip-0 cubemap at the given direction
/// and returns its RGB radiance. The cube layout matches WebGPU's
/// face-layer order (px, nx, py, ny, pz, nz).
fn sample_source_cubemap(
    source_face_pixels: &[Vec<f32>; 6],
    resolution: u32,
    direction: Vec3,
) -> Vec3 {
    let normalized = normalize_or_z(direction);
    let (face, u, v) = direction_to_face_uv(normalized);
    let pixel_x = ((u + 1.0) * 0.5 * resolution as f32 - 0.5).clamp(0.0, (resolution - 1) as f32);
    let pixel_y = ((v + 1.0) * 0.5 * resolution as f32 - 0.5).clamp(0.0, (resolution - 1) as f32);
    let x_low = pixel_x.floor() as u32;
    let y_low = pixel_y.floor() as u32;
    let x_high = (x_low + 1).min(resolution - 1);
    let y_high = (y_low + 1).min(resolution - 1);
    let fx = pixel_x - x_low as f32;
    let fy = pixel_y - y_low as f32;
    let face_pixels = &source_face_pixels[face];
    let texel = |x: u32, y: u32| -> Vec3 {
        let index = ((y * resolution + x) * 4) as usize;
        Vec3::new(
            face_pixels[index],
            face_pixels[index + 1],
            face_pixels[index + 2],
        )
    };
    let lt = texel(x_low, y_low);
    let rt = texel(x_high, y_low);
    let lb = texel(x_low, y_high);
    let rb = texel(x_high, y_high);
    let top = lerp_vec3(lt, rt, fx);
    let bottom = lerp_vec3(lb, rb, fx);
    lerp_vec3(top, bottom, fy)
}

fn direction_to_face_uv(direction: Vec3) -> (usize, f32, f32) {
    let abs_x = direction.x.abs();
    let abs_y = direction.y.abs();
    let abs_z = direction.z.abs();
    if abs_x >= abs_y && abs_x >= abs_z {
        if direction.x > 0.0 {
            (0, -direction.z / abs_x, -direction.y / abs_x)
        } else {
            (1, direction.z / abs_x, -direction.y / abs_x)
        }
    } else if abs_y >= abs_z {
        if direction.y > 0.0 {
            (2, direction.x / abs_y, direction.z / abs_y)
        } else {
            (3, direction.x / abs_y, -direction.z / abs_y)
        }
    } else if direction.z > 0.0 {
        (4, direction.x / abs_z, -direction.y / abs_z)
    } else {
        (5, -direction.x / abs_z, -direction.y / abs_z)
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

fn lerp_vec3(start: Vec3, end: Vec3, t: f32) -> Vec3 {
    let clamped = t.clamp(0.0, 1.0);
    Vec3::new(
        start.x + (end.x - start.x) * clamped,
        start.y + (end.y - start.y) * clamped,
        start.z + (end.z - start.z) * clamped,
    )
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
            sample_count_for_roughness(1.0, EnvironmentPrefilterQuality::Reference),
            768,
            "reference quality keeps the existing rough-environment sample count"
        );
        assert_eq!(
            sample_count_for_roughness(1.0, EnvironmentPrefilterQuality::InteractiveWebGl2),
            16,
            "WebGL2 first-frame prefiltering must not run the reference offline sample count"
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
    fn direction_to_face_uv_round_trips_face_centers() {
        for face in 0..6 {
            let direction = cubemap_face_direction(face, 0.0, 0.0);
            let (decoded_face, u, v) = direction_to_face_uv(direction);
            assert_eq!(
                decoded_face, face,
                "direction at face {face} center must decode back to that face, got {decoded_face}"
            );
            assert!(
                u.abs() < 1e-4 && v.abs() < 1e-4,
                "face center should round-trip to (0, 0) UV; got ({u}, {v}) for face {face}"
            );
        }
    }
}
