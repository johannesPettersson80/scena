use super::metrics::luminance_from_srgb8;
use super::types::{ReferenceQualityMetrics, round3};

pub fn reference_quality_metrics(
    actual: &[u8],
    expected: &[u8],
    width: u32,
    height: u32,
) -> Option<ReferenceQualityMetrics> {
    let expected_len = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    if actual.len() != expected_len || expected.len() != expected_len {
        return None;
    }
    let mut total_abs = 0u64;
    let mut total_delta_e = 0.0f32;
    for (actual_pixel, expected_pixel) in actual.chunks_exact(4).zip(expected.chunks_exact(4)) {
        for (actual_channel, expected_channel) in actual_pixel.iter().zip(expected_pixel) {
            total_abs =
                total_abs.saturating_add(u64::from(actual_channel.abs_diff(*expected_channel)));
        }
        total_delta_e += delta_e2000(
            lab_from_srgb8(actual_pixel[0], actual_pixel[1], actual_pixel[2]),
            lab_from_srgb8(expected_pixel[0], expected_pixel[1], expected_pixel[2]),
        );
    }
    let channel_count = expected_len as f32;
    let pixel_count = (width as usize).saturating_mul(height as usize).max(1) as f32;
    Some(ReferenceQualityMetrics {
        mean_abs_diff: round3(total_abs as f32 / channel_count),
        mean_delta_e2000: round3(total_delta_e / pixel_count),
        ssim: round3(ssim_grayscale(actual, expected, width, height)?),
    })
}

pub fn ssim_grayscale(actual: &[u8], expected: &[u8], width: u32, height: u32) -> Option<f32> {
    let expected_len = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    if actual.len() != expected_len || expected.len() != expected_len {
        return None;
    }
    let mut actual_values = Vec::with_capacity((width as usize).saturating_mul(height as usize));
    let mut expected_values = Vec::with_capacity(actual_values.capacity());
    for (actual_pixel, expected_pixel) in actual.chunks_exact(4).zip(expected.chunks_exact(4)) {
        actual_values.push(luminance_from_srgb8(
            actual_pixel[0],
            actual_pixel[1],
            actual_pixel[2],
        ));
        expected_values.push(luminance_from_srgb8(
            expected_pixel[0],
            expected_pixel[1],
            expected_pixel[2],
        ));
    }
    let mean_a = mean(&actual_values);
    let mean_b = mean(&expected_values);
    let mut var_a = 0.0;
    let mut var_b = 0.0;
    let mut cov = 0.0;
    for (a, b) in actual_values.iter().zip(&expected_values) {
        var_a += (*a - mean_a) * (*a - mean_a);
        var_b += (*b - mean_b) * (*b - mean_b);
        cov += (*a - mean_a) * (*b - mean_b);
    }
    let count = actual_values.len().max(1) as f32;
    var_a /= count;
    var_b /= count;
    cov /= count;
    let c1 = 0.01 * 0.01;
    let c2 = 0.03 * 0.03;
    let numerator = (2.0 * mean_a * mean_b + c1) * (2.0 * cov + c2);
    let denominator = (mean_a * mean_a + mean_b * mean_b + c1) * (var_a + var_b + c2);
    Some(if denominator <= f32::EPSILON {
        1.0
    } else {
        (numerator / denominator).clamp(0.0, 1.0)
    })
}

fn mean(values: &[f32]) -> f32 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f32>() / values.len() as f32
    }
}

fn lab_from_srgb8(r: u8, g: u8, b: u8) -> [f32; 3] {
    let srgb_to_linear = |value: u8| {
        let value = f32::from(value) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    let r = srgb_to_linear(r);
    let g = srgb_to_linear(g);
    let b = srgb_to_linear(b);
    let x = (0.412_456_4 * r + 0.357_576_1 * g + 0.180_437_5 * b) / 0.95047;
    let y = 0.212_672_9 * r + 0.715_152_2 * g + 0.072_175 * b;
    let z = (0.019_333_9 * r + 0.119_192 * g + 0.950_304_1 * b) / 1.08883;
    let f = |value: f32| {
        if value > 0.008_856 {
            value.cbrt()
        } else {
            (7.787 * value) + (16.0 / 116.0)
        }
    };
    let fx = f(x);
    let fy = f(y);
    let fz = f(z);
    [(116.0 * fy) - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

fn delta_e2000(lab1: [f32; 3], lab2: [f32; 3]) -> f32 {
    let (l1, a1, b1) = (lab1[0], lab1[1], lab1[2]);
    let (l2, a2, b2) = (lab2[0], lab2[1], lab2[2]);
    let avg_lp = (l1 + l2) * 0.5;
    let c1 = (a1 * a1 + b1 * b1).sqrt();
    let c2 = (a2 * a2 + b2 * b2).sqrt();
    let avg_c = (c1 + c2) * 0.5;
    let g = 0.5 * (1.0 - (avg_c.powi(7) / (avg_c.powi(7) + 25.0_f32.powi(7))).sqrt());
    let a1p = (1.0 + g) * a1;
    let a2p = (1.0 + g) * a2;
    let c1p = (a1p * a1p + b1 * b1).sqrt();
    let c2p = (a2p * a2p + b2 * b2).sqrt();
    let h1p = hue_angle(a1p, b1);
    let h2p = hue_angle(a2p, b2);
    let dlp = l2 - l1;
    let dcp = c2p - c1p;
    let dhp = if c1p * c2p == 0.0 {
        0.0
    } else if (h2p - h1p).abs() <= 180.0 {
        h2p - h1p
    } else if h2p <= h1p {
        h2p - h1p + 360.0
    } else {
        h2p - h1p - 360.0
    };
    let dhp = 2.0 * (c1p * c2p).sqrt() * degrees_to_radians(dhp * 0.5).sin();
    let avg_cp = (c1p + c2p) * 0.5;
    let avg_hp = if c1p * c2p == 0.0 {
        h1p + h2p
    } else if (h1p - h2p).abs() <= 180.0 {
        (h1p + h2p) * 0.5
    } else if h1p + h2p < 360.0 {
        (h1p + h2p + 360.0) * 0.5
    } else {
        (h1p + h2p - 360.0) * 0.5
    };
    let t = 1.0 - 0.17 * degrees_to_radians(avg_hp - 30.0).cos()
        + 0.24 * degrees_to_radians(2.0 * avg_hp).cos()
        + 0.32 * degrees_to_radians(3.0 * avg_hp + 6.0).cos()
        - 0.20 * degrees_to_radians(4.0 * avg_hp - 63.0).cos();
    let delta_theta = 30.0 * (-((avg_hp - 275.0) / 25.0).powi(2)).exp();
    let rc = 2.0 * (avg_cp.powi(7) / (avg_cp.powi(7) + 25.0_f32.powi(7))).sqrt();
    let sl = 1.0 + (0.015 * (avg_lp - 50.0).powi(2)) / (20.0 + (avg_lp - 50.0).powi(2)).sqrt();
    let sc = 1.0 + 0.045 * avg_cp;
    let sh = 1.0 + 0.015 * avg_cp * t;
    let rt = -degrees_to_radians(2.0 * delta_theta).sin() * rc;
    let l_term = dlp / sl;
    let c_term = dcp / sc;
    let h_term = dhp / sh;
    (l_term * l_term + c_term * c_term + h_term * h_term + rt * c_term * h_term).sqrt()
}

fn hue_angle(a: f32, b: f32) -> f32 {
    if a == 0.0 && b == 0.0 {
        0.0
    } else {
        b.atan2(a).to_degrees().rem_euclid(360.0)
    }
}

fn degrees_to_radians(value: f32) -> f32 {
    value * std::f32::consts::PI / 180.0
}
