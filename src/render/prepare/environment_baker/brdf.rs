use super::*;

#[cfg(test)]
pub(super) fn build_brdf_lut(size: u32) -> Vec<f32> {
    build_brdf_lut_with_sample_count(size, 1024)
}

#[cfg(test)]
pub(super) fn build_brdf_lut_with_sample_count(size: u32, sample_count: u32) -> Vec<f32> {
    build_brdf_lut_with_sample_count_profiled(
        size,
        sample_count,
        1,
        &mut EnvironmentBakeMetrics::default(),
    )
}

pub(super) fn build_brdf_lut_with_sample_count_profiled(
    size: u32,
    sample_count: u32,
    workers: usize,
    metrics: &mut EnvironmentBakeMetrics,
) -> Vec<f32> {
    #[cfg(target_arch = "wasm32")]
    let _ = workers;
    let resolved_size = size.max(1);
    metrics.brdf_lut_texels = metrics
        .brdf_lut_texels
        .saturating_add(u64::from(resolved_size).pow(2));
    metrics.brdf_integration_samples = metrics.brdf_integration_samples.saturating_add(
        u64::from(resolved_size)
            .pow(2)
            .saturating_mul(u64::from(sample_count)),
    );
    let mut pixels = vec![0.0_f32; (resolved_size as usize).pow(2) * 2];
    metrics.parallel_tasks = metrics
        .parallel_tasks
        .saturating_add(u64::from(resolved_size));
    #[cfg(not(target_arch = "wasm32"))]
    if workers > 1 {
        pixels
            .par_chunks_mut(resolved_size as usize * 2)
            .enumerate()
            .for_each(|(y, row)| {
                fill_brdf_lut_row(row, y as u32, resolved_size, sample_count);
            });
        return pixels;
    }
    for (y, row) in pixels.chunks_mut(resolved_size as usize * 2).enumerate() {
        fill_brdf_lut_row(row, y as u32, resolved_size, sample_count);
    }
    pixels
}

fn fill_brdf_lut_row(row: &mut [f32], y: u32, resolved_size: u32, sample_count: u32) {
    let roughness = (y as f32 + 0.5) / resolved_size as f32;
    for x in 0..resolved_size {
        let n_dot_v = (x as f32 + 0.5) / resolved_size as f32;
        let (scale, bias) = integrate_brdf_lut_cell(n_dot_v, roughness, sample_count);
        let pixel_index = (x * 2) as usize;
        row[pixel_index] = scale;
        row[pixel_index + 1] = bias;
    }
}

pub(super) fn integrate_ggx_specular(
    normal: Vec3,
    roughness: f32,
    sample_count: u32,
    source_mips: &[[Vec<f32>; 6]],
    metrics: &mut EnvironmentBakeMetrics,
) -> Vec3 {
    if sample_count == 0 {
        metrics.source_texture_samples = metrics.source_texture_samples.saturating_add(1);
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
        metrics.source_texture_samples = metrics.source_texture_samples.saturating_add(1);
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
        let alpha_roughness = pbr_brdf::alpha_roughness(roughness);
        let visibility = pbr_brdf::ggx_visibility_correlated(n_dot_l, n_dot_v, alpha_roughness)
            * v_dot_h
            * n_dot_l
            / n_dot_h.max(1e-4);
        let fresnel = (1.0 - v_dot_h).clamp(0.0, 1.0).powi(5);
        scale += (1.0 - fresnel) * visibility;
        bias += fresnel * visibility;
    }
    (
        4.0 * scale / sample_count as f32,
        4.0 * bias / sample_count as f32,
    )
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

pub(super) fn hammersley_2d(index: u32, count: u32) -> (f32, f32) {
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

fn ggx_normal_distribution(n_dot_h: f32, roughness: f32) -> f32 {
    pbr_brdf::ggx_normal_distribution(n_dot_h, pbr_brdf::alpha_roughness(roughness)).max(1e-6)
}

fn ggx_sample_pdf(n_dot_h: f32, v_dot_h: f32, roughness: f32) -> f32 {
    let distribution = ggx_normal_distribution(n_dot_h, roughness);
    (distribution * n_dot_h.clamp(0.0, 1.0) / (4.0 * v_dot_h.max(1e-4))).max(1e-6)
}

pub(super) fn source_mip_level_for_sample(
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

pub(super) fn normalize_or_z(value: Vec3) -> Vec3 {
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
pub(super) fn sample_count_for_roughness(
    roughness: f32,
    quality: EnvironmentIblBakeQuality,
) -> u32 {
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
