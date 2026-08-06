use super::PhotographicSurfaceDesc;
use super::profile::SurfaceProfile;

pub(super) struct GeneratedSurfaceMaps {
    pub(super) base_color: Vec<u8>,
    pub(super) normal: Vec<u8>,
    pub(super) occlusion_roughness_metallic: Vec<u8>,
}

pub(super) fn generate_surface_maps(descriptor: PhotographicSurfaceDesc) -> GeneratedSurfaceMaps {
    let profile = SurfaceProfile::for_kind(descriptor.kind);
    let resolution = descriptor.resolution;
    let pixel_count = (resolution as usize) * (resolution as usize);
    let feature_period = ((descriptor.tile_size_m / descriptor.feature_scale_m).round() as u32)
        .clamp(2, (resolution / 3).max(2));
    let mut heights = Vec::with_capacity(pixel_count);
    let mut roughness_noise = Vec::with_capacity(pixel_count);
    let mut color_noise = Vec::with_capacity(pixel_count);

    for y in 0..resolution {
        for x in 0..resolution {
            let u = x as f32 / resolution as f32;
            let v = y as f32 / resolution as f32;
            let isotropic = fractal_noise(
                u,
                v,
                feature_period,
                feature_period,
                descriptor.seed ^ 0xa93f_5c71_d47b_2109,
            );
            let directional = fractal_noise(
                u,
                v,
                feature_period,
                2,
                descriptor.seed ^ 0x13d7_9b45_8c2e_f601,
            );
            let sparse = sparse_streaks(
                u,
                v,
                feature_period,
                descriptor.seed ^ 0x6f2a_b931_c8d5_047e,
            );
            let height = lerp(isotropic, directional, profile.directionality) * 0.86
                + sparse * descriptor.wear * 0.14;
            heights.push(height.clamp(0.0, 1.0));
            roughness_noise.push(fractal_noise(
                u,
                v,
                (feature_period / 2).max(2),
                (feature_period / 2).max(2),
                descriptor.seed ^ 0xd42c_1f87_7a59_b306,
            ));
            color_noise.push(fractal_noise(
                u,
                v,
                3,
                3,
                descriptor.seed ^ 0x8e61_3ac9_25f4_d7b0,
            ));
        }
    }

    let mut base_color = Vec::with_capacity(pixel_count * 4);
    let mut normal = Vec::with_capacity(pixel_count * 4);
    let mut orm = Vec::with_capacity(pixel_count * 4);
    for y in 0..resolution {
        for x in 0..resolution {
            let index = (y * resolution + x) as usize;
            let left = heights[(y * resolution + (x + resolution - 1) % resolution) as usize];
            let right = heights[(y * resolution + (x + 1) % resolution) as usize];
            let down = heights[(((y + resolution - 1) % resolution) * resolution + x) as usize];
            let up = heights[(((y + 1) % resolution) * resolution + x) as usize];
            let normal_gain =
                profile.height_strength * descriptor.variation * 3.5 + descriptor.wear * 0.25;
            let nx = -(right - left) * normal_gain;
            let ny = -(up - down) * normal_gain;
            let inverse_length = (nx * nx + ny * ny + 1.0).sqrt().recip();
            push_linear_rgba8(
                &mut normal,
                nx * inverse_length * 0.5 + 0.5,
                ny * inverse_length * 0.5 + 0.5,
                inverse_length * 0.5 + 0.5,
                1.0,
            );

            let wear_mask = sparse_streaks(
                x as f32 / resolution as f32,
                y as f32 / resolution as f32,
                feature_period,
                descriptor.seed ^ 0x19b3_e754_6ca8_02fd,
            );
            let roughness = (descriptor.roughness.unwrap_or(profile.roughness)
                + (roughness_noise[index] - 0.5) * profile.roughness_spread * descriptor.variation
                + wear_mask * descriptor.wear * 0.24)
                .clamp(0.045, 0.96);
            let occlusion = (1.0
                - (0.5 - heights[index]).max(0.0)
                    * 0.12
                    * descriptor.variation
                    * profile.occlusion_strength)
                .clamp(0.82, 1.0);
            push_linear_rgba8(
                &mut orm,
                occlusion,
                roughness,
                descriptor.metallic.unwrap_or(profile.metallic),
                1.0,
            );

            let tint = 1.0
                + (color_noise[index] - 0.5) * profile.color_variation * descriptor.variation
                - wear_mask * descriptor.wear * 0.035;
            push_srgb_rgba8(
                &mut base_color,
                descriptor.base_color.r * tint,
                descriptor.base_color.g * tint,
                descriptor.base_color.b * tint,
                descriptor.base_color.a,
            );
        }
    }

    GeneratedSurfaceMaps {
        base_color,
        normal,
        occlusion_roughness_metallic: orm,
    }
}

fn fractal_noise(u: f32, v: f32, period_x: u32, period_y: u32, seed: u64) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 0.55;
    let mut total = 0.0;
    let mut octave_period_x = period_x.max(1);
    let mut octave_period_y = period_y.max(1);
    for octave in 0..4 {
        value += periodic_value_noise(
            u,
            v,
            octave_period_x,
            octave_period_y,
            seed ^ (octave as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15),
        ) * amplitude;
        total += amplitude;
        amplitude *= 0.5;
        octave_period_x = octave_period_x.saturating_mul(2).max(1);
        octave_period_y = octave_period_y.saturating_mul(2).max(1);
    }
    value / total
}

fn periodic_value_noise(u: f32, v: f32, period_x: u32, period_y: u32, seed: u64) -> f32 {
    let x = u * period_x as f32;
    let y = v * period_y as f32;
    let x0 = x.floor() as u32 % period_x;
    let y0 = y.floor() as u32 % period_y;
    let x1 = (x0 + 1) % period_x;
    let y1 = (y0 + 1) % period_y;
    let tx = smoothstep(x.fract());
    let ty = smoothstep(y.fract());
    let bottom = lerp(lattice_noise(x0, y0, seed), lattice_noise(x1, y0, seed), tx);
    let top = lerp(lattice_noise(x0, y1, seed), lattice_noise(x1, y1, seed), tx);
    lerp(bottom, top, ty)
}

fn sparse_streaks(u: f32, v: f32, period: u32, seed: u64) -> f32 {
    let columns = (period / 2).max(3);
    let column_position = u * columns as f32;
    let column = column_position.floor() as u32 % columns;
    let random = lattice_noise(column, 0, seed);
    if random < 0.78 {
        return 0.0;
    }
    let center = lattice_noise(column, 1, seed);
    let width = 0.025 + lattice_noise(column, 2, seed) * 0.065;
    let distance = (column_position.fract() - center).abs();
    let line = (1.0 - distance / width).clamp(0.0, 1.0);
    let breakup = periodic_value_noise(u, v, columns, 5, seed ^ 0xb571_20df_6e9a_438c);
    line * (breakup * 1.35 - 0.35).clamp(0.0, 1.0)
}

fn lattice_noise(x: u32, y: u32, seed: u64) -> f32 {
    let mixed = mix64(
        seed ^ u64::from(x).wrapping_mul(0x9e37_79b1_85eb_ca87)
            ^ u64::from(y).wrapping_mul(0xc2b2_ae3d_27d4_eb4f),
    );
    ((mixed >> 40) as u32) as f32 / 16_777_215.0
}

fn mix64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn push_linear_rgba8(output: &mut Vec<u8>, r: f32, g: f32, b: f32, a: f32) {
    output.extend_from_slice(&[unit_to_u8(r), unit_to_u8(g), unit_to_u8(b), unit_to_u8(a)]);
}

fn push_srgb_rgba8(output: &mut Vec<u8>, r: f32, g: f32, b: f32, a: f32) {
    output.extend_from_slice(&[
        unit_to_u8(linear_to_srgb(r)),
        unit_to_u8(linear_to_srgb(g)),
        unit_to_u8(linear_to_srgb(b)),
        unit_to_u8(a),
    ]);
}

fn linear_to_srgb(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    if value <= 0.003_130_8 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}

fn unit_to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn smoothstep(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}

fn lerp(left: f32, right: f32, amount: f32) -> f32 {
    left + (right - left) * amount
}
