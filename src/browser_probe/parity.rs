use base64::Engine as _;
use serde_json::{Value, json};

use crate::{PixelReadback, fnv1a64_hex};

const SCHEMA: &str = "scena.m6.cpu_webgl2_parity.v1";
const MAX_RMSE: f64 = 0.08;
const MIN_SSIM: f64 = 0.93;
const MAX_P95_CHANNEL_DELTA: u8 = 24;
const MAX_MEAN_CHANNEL_DELTA: f64 = 6.0;
const MIN_FOREGROUND_IOU: f64 = 0.90;
const MAX_FOREGROUND_REGION_RMSE: f64 = 0.13;

pub(super) fn cpu_webgl2_report(cpu: &PixelReadback, gpu: Option<&PixelReadback>) -> Value {
    let cpu = NormalizedFrame::from_readback(cpu);
    let Some(gpu) = gpu.map(NormalizedFrame::from_readback) else {
        return json!({
            "schema": SCHEMA,
            "status": "failed",
            "failure_codes": ["gpu_frame_missing"],
            "cpu_frame": frame_json("renderer-owned-cpu-frame", &cpu),
            "gpu_frame": Value::Null,
            "normalization": normalization_json(cpu.width, cpu.height),
            "thresholds": thresholds_json(),
        });
    };

    let evaluation = evaluate(&cpu, &gpu);
    let mut perturbed_gpu = gpu.clone();
    perturb_gpu_center(&mut perturbed_gpu);
    let mutation_evaluation = evaluate(&cpu, &perturbed_gpu);
    let mutation_rejected = !mutation_evaluation.failure_codes.is_empty();
    let mut failure_codes = evaluation.failure_codes.clone();
    if !mutation_rejected {
        failure_codes.push("known_bad_not_rejected");
    }

    json!({
        "schema": SCHEMA,
        "status": if failure_codes.is_empty() { "passed" } else { "failed" },
        "fixture": {
            "id": "m6-identical-unlit-triangle-v1",
            "scene_builder": "scene_with_triangle",
            "camera_translation": [0.0, 0.0, 2.0],
            "renderer_options": "default",
            "background": "black-opaque",
        },
        "normalization": normalization_json(cpu.width, cpu.height),
        "thresholds": thresholds_json(),
        "cpu_frame": frame_json("renderer-owned-cpu-frame", &cpu),
        "gpu_frame": frame_json("renderer-owned-gpu-copy", &gpu),
        "metrics": evaluation.metrics.to_json(),
        "failure_codes": failure_codes,
        "known_bad_mutation": {
            "kind": "gpu-center-channel-perturbation",
            "rejected": mutation_rejected,
            "failure_codes": mutation_evaluation.failure_codes,
            "metrics": mutation_evaluation.metrics.to_json(),
        },
    })
}

fn normalization_json(width: u32, height: u32) -> Value {
    json!({
        "row_origin": "top-left",
        "transfer": "srgb8",
        "alpha": "straight-opaque",
        "dimensions": "exact",
        "width": width,
        "height": height,
        "comparison_channels": "rgb",
        "ssim_domain": "srgb8-luma",
    })
}

fn thresholds_json() -> Value {
    json!({
        "rmse_max": MAX_RMSE,
        "ssim_min": MIN_SSIM,
        "p95_channel_delta_max": MAX_P95_CHANNEL_DELTA,
        "mean_channel_delta_max": MAX_MEAN_CHANNEL_DELTA,
        "foreground_iou_min": MIN_FOREGROUND_IOU,
        "foreground_region_rmse_max": MAX_FOREGROUND_REGION_RMSE,
        "alpha_deviations_max": 0,
    })
}

fn frame_json(source: &str, frame: &NormalizedFrame) -> Value {
    json!({
        "source": source,
        "width": frame.width,
        "height": frame.height,
        "rgba8_fnv1a64": fnv1a64_hex(&frame.rgba8),
        "rgba8_base64": base64::engine::general_purpose::STANDARD.encode(&frame.rgba8),
        "alpha_deviations": frame.alpha_deviations,
    })
}

#[derive(Clone)]
struct NormalizedFrame {
    width: u32,
    height: u32,
    rgba8: Vec<u8>,
    alpha_deviations: usize,
}

impl NormalizedFrame {
    fn from_readback(readback: &PixelReadback) -> Self {
        let mut rgba8 = readback.rgba8().to_vec();
        let mut alpha_deviations = 0;
        for pixel in rgba8.chunks_exact_mut(4) {
            if pixel[3] != 255 {
                alpha_deviations += 1;
            }
            pixel[3] = 255;
        }
        Self {
            width: readback.width(),
            height: readback.height(),
            rgba8,
            alpha_deviations,
        }
    }
}

struct Evaluation {
    metrics: Metrics,
    failure_codes: Vec<&'static str>,
}

#[derive(Default)]
struct Metrics {
    rmse: f64,
    ssim: f64,
    max_channel_delta: u8,
    p95_channel_delta: u8,
    mean_channel_delta: f64,
    foreground_iou: f64,
    foreground_region_rmse: f64,
    foreground_bounds: Option<[u32; 4]>,
    compared_pixels: usize,
}

impl Metrics {
    fn to_json(&self) -> Value {
        json!({
            "rmse": self.rmse,
            "ssim": self.ssim,
            "max_channel_delta": self.max_channel_delta,
            "p95_channel_delta": self.p95_channel_delta,
            "mean_channel_delta": self.mean_channel_delta,
            "foreground_iou": self.foreground_iou,
            "foreground_region_rmse": self.foreground_region_rmse,
            "foreground_bounds": self.foreground_bounds,
            "compared_pixels": self.compared_pixels,
        })
    }
}

fn evaluate(cpu: &NormalizedFrame, gpu: &NormalizedFrame) -> Evaluation {
    let mut failure_codes = Vec::new();
    if cpu.width != gpu.width || cpu.height != gpu.height || cpu.rgba8.len() != gpu.rgba8.len() {
        failure_codes.push("dimension_mismatch");
        return Evaluation {
            metrics: Metrics::default(),
            failure_codes,
        };
    }

    let foreground_bounds = foreground_union_bounds(cpu, gpu);
    let metrics = Metrics {
        rmse: rmse_rgb(&cpu.rgba8, &gpu.rgba8),
        ssim: ssim_luma(&cpu.rgba8, &gpu.rgba8),
        max_channel_delta: max_channel_delta(&cpu.rgba8, &gpu.rgba8),
        p95_channel_delta: percentile_channel_delta(&cpu.rgba8, &gpu.rgba8, 95),
        mean_channel_delta: mean_channel_delta(&cpu.rgba8, &gpu.rgba8),
        foreground_iou: foreground_iou(&cpu.rgba8, &gpu.rgba8),
        foreground_region_rmse: foreground_bounds
            .map(|bounds| region_rmse_rgb(cpu, gpu, bounds))
            .unwrap_or(1.0),
        foreground_bounds,
        compared_pixels: cpu.rgba8.len() / 4,
    };

    if cpu.alpha_deviations != 0 || gpu.alpha_deviations != 0 {
        failure_codes.push("alpha_not_opaque");
    }
    if metrics.rmse > MAX_RMSE {
        failure_codes.push("rmse");
    }
    if metrics.ssim < MIN_SSIM {
        failure_codes.push("ssim");
    }
    if metrics.p95_channel_delta > MAX_P95_CHANNEL_DELTA {
        failure_codes.push("p95_channel_delta");
    }
    if metrics.mean_channel_delta > MAX_MEAN_CHANNEL_DELTA {
        failure_codes.push("mean_channel_delta");
    }
    if metrics.foreground_iou < MIN_FOREGROUND_IOU {
        failure_codes.push("foreground_iou");
    }
    if metrics.foreground_region_rmse > MAX_FOREGROUND_REGION_RMSE {
        failure_codes.push("foreground_region_rmse");
    }
    Evaluation {
        metrics,
        failure_codes,
    }
}

fn perturb_gpu_center(frame: &mut NormalizedFrame) {
    let min_x = frame.width / 4;
    let max_x = frame.width.saturating_mul(3) / 4;
    let min_y = frame.height / 4;
    let max_y = frame.height.saturating_mul(3) / 4;
    for y in min_y..max_y {
        for x in min_x..max_x {
            let index = ((y * frame.width + x) * 4) as usize;
            frame.rgba8[index..index + 4].copy_from_slice(&[255, 0, 255, 255]);
        }
    }
}

fn rmse_rgb(left: &[u8], right: &[u8]) -> f64 {
    let mut squared = 0.0;
    let mut count = 0_usize;
    for (left, right) in left.chunks_exact(4).zip(right.chunks_exact(4)) {
        for channel in 0..3 {
            let delta = f64::from(left[channel]) - f64::from(right[channel]);
            squared += delta * delta;
            count += 1;
        }
    }
    (squared / count.max(1) as f64).sqrt() / 255.0
}

fn max_channel_delta(left: &[u8], right: &[u8]) -> u8 {
    left.chunks_exact(4)
        .zip(right.chunks_exact(4))
        .flat_map(|(left, right)| (0..3).map(move |channel| left[channel].abs_diff(right[channel])))
        .max()
        .unwrap_or(0)
}

fn percentile_channel_delta(left: &[u8], right: &[u8], percentile: usize) -> u8 {
    let mut deltas = left
        .chunks_exact(4)
        .zip(right.chunks_exact(4))
        .flat_map(|(left, right)| (0..3).map(move |channel| left[channel].abs_diff(right[channel])))
        .collect::<Vec<_>>();
    if deltas.is_empty() {
        return 0;
    }
    deltas.sort_unstable();
    let index = (deltas.len() - 1).saturating_mul(percentile.min(100)) / 100;
    deltas[index]
}

fn mean_channel_delta(left: &[u8], right: &[u8]) -> f64 {
    let mut total = 0_u64;
    let mut count = 0_u64;
    for (left, right) in left.chunks_exact(4).zip(right.chunks_exact(4)) {
        for channel in 0..3 {
            total += u64::from(left[channel].abs_diff(right[channel]));
            count += 1;
        }
    }
    total as f64 / count.max(1) as f64
}

fn ssim_luma(left: &[u8], right: &[u8]) -> f64 {
    let left = luma_values(left);
    let right = luma_values(right);
    let count = left.len().min(right.len()).max(1) as f64;
    let mean_left = left.iter().sum::<f64>() / count;
    let mean_right = right.iter().sum::<f64>() / count;
    let mut variance_left = 0.0;
    let mut variance_right = 0.0;
    let mut covariance = 0.0;
    for (&left, &right) in left.iter().zip(&right) {
        let left_delta = left - mean_left;
        let right_delta = right - mean_right;
        variance_left += left_delta * left_delta;
        variance_right += right_delta * right_delta;
        covariance += left_delta * right_delta;
    }
    variance_left /= count;
    variance_right /= count;
    covariance /= count;
    let c1 = 0.01_f64.powi(2);
    let c2 = 0.03_f64.powi(2);
    ((2.0 * mean_left * mean_right + c1) * (2.0 * covariance + c2))
        / ((mean_left.powi(2) + mean_right.powi(2) + c1) * (variance_left + variance_right + c2))
}

fn luma_values(rgba: &[u8]) -> Vec<f64> {
    rgba.chunks_exact(4)
        .map(|pixel| {
            (0.2126 * f64::from(pixel[0])
                + 0.7152 * f64::from(pixel[1])
                + 0.0722 * f64::from(pixel[2]))
                / 255.0
        })
        .collect()
}

fn foreground(pixel: &[u8]) -> bool {
    pixel[0] > 8 || pixel[1] > 8 || pixel[2] > 8
}

fn foreground_iou(left: &[u8], right: &[u8]) -> f64 {
    let mut intersection = 0_u64;
    let mut union = 0_u64;
    for (left, right) in left.chunks_exact(4).zip(right.chunks_exact(4)) {
        let left = foreground(left);
        let right = foreground(right);
        intersection += u64::from(left && right);
        union += u64::from(left || right);
    }
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

fn foreground_union_bounds(left: &NormalizedFrame, right: &NormalizedFrame) -> Option<[u32; 4]> {
    let mut min_x = left.width;
    let mut min_y = left.height;
    let mut max_x = 0;
    let mut max_y = 0;
    let mut found = false;
    for y in 0..left.height {
        for x in 0..left.width {
            let index = ((y * left.width + x) * 4) as usize;
            if foreground(&left.rgba8[index..index + 4])
                || foreground(&right.rgba8[index..index + 4])
            {
                found = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    found.then_some([min_x, min_y, max_x + 1, max_y + 1])
}

fn region_rmse_rgb(left: &NormalizedFrame, right: &NormalizedFrame, bounds: [u32; 4]) -> f64 {
    let mut squared = 0.0;
    let mut count = 0_usize;
    for y in bounds[1]..bounds[3] {
        for x in bounds[0]..bounds[2] {
            let index = ((y * left.width + x) * 4) as usize;
            for channel in 0..3 {
                let delta = f64::from(left.rgba8[index + channel])
                    - f64::from(right.rgba8[index + channel]);
                squared += delta * delta;
                count += 1;
            }
        }
    }
    (squared / count.max(1) as f64).sqrt() / 255.0
}
