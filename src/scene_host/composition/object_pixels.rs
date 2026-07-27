use serde_json::json;

use super::checks::{CompositionCheckExt, checked_check, error_check, observed_pairs, round3};
use super::helpers::linear_rgba_to_srgb8;
use crate::{CaptureRgba8, CaptureScreenRegion, Color, SceneCompositionCheckV1};

const OBJECT_PIXEL_BACKGROUND_TOLERANCE_RGBA8: u8 = 2;
const OBJECT_LOW_CLIP_LUMA: f32 = 0.04;
const OBJECT_HIGH_CLIP_LUMA: f32 = 0.96;

pub(super) fn object_pixel_quality_check(
    target_path: &str,
    target_id: &str,
    handle: u64,
    capture: &CaptureRgba8,
    region: CaptureScreenRegion,
    background: Color,
    profile: &str,
) -> SceneCompositionCheckV1 {
    let metrics = object_pixel_metrics(capture, region, background);
    let thresholds = ObjectPixelThresholds::for_profile(profile);
    let observed = observed_pairs([
        ("region_pixels", json!(metrics.region_pixels)),
        ("foreground_pixels", json!(metrics.foreground_pixels)),
        (
            "low_clip_fraction",
            json!(round3(metrics.low_clip_fraction)),
        ),
        (
            "high_clip_fraction",
            json!(round3(metrics.high_clip_fraction)),
        ),
        ("mean_luminance", json!(round3(metrics.mean_luminance))),
        (
            "mean_background_delta",
            json!(round3(metrics.mean_background_delta)),
        ),
        (
            "max_background_delta",
            json!(round3(metrics.max_background_delta)),
        ),
        ("profile", json!(profile)),
    ]);
    let mut check = if metrics.low_clip_fraction > thresholds.max_low_clip_fraction {
        error_check(
            format!("{target_path}.pixel_exposure"),
            "lighting_material",
            "subject_black_crushed",
            Some(target_id.to_owned()),
            vec![handle],
            observed,
            (
                "declared object pixels are dominated by near-black values",
                "add light/environment contribution, raise exposure, or use a material/background with visible detail",
            ),
        )
    } else if metrics.high_clip_fraction > thresholds.max_high_clip_fraction {
        error_check(
            format!("{target_path}.pixel_exposure"),
            "lighting_material",
            "subject_blown_out",
            Some(target_id.to_owned()),
            vec![handle],
            observed,
            (
                "declared object pixels are dominated by clipped highlights",
                "lower exposure or light intensity so the subject keeps visible detail",
            ),
        )
    } else if metrics.mean_background_delta < thresholds.min_mean_background_delta {
        error_check(
            format!("{target_path}.pixel_exposure"),
            "visual_salience",
            "subject_salience_too_low",
            Some(target_id.to_owned()),
            vec![handle],
            observed,
            (
                "declared object is too close to the background colour to read clearly",
                "increase material/background contrast, add rim lighting, or choose a clearer background",
            ),
        )
    } else {
        checked_check(
            format!("{target_path}.pixel_exposure"),
            "lighting_material",
            "subject_exposure_sane",
            Some(target_id.to_owned()),
            vec![handle],
            observed,
            (
                "declared object has sane exposure and background separation in its projected native-resolution region",
                "no action needed",
            ),
        )
    };
    check.threshold = Some(json!({
        "max_low_clip_fraction": round3(thresholds.max_low_clip_fraction),
        "max_high_clip_fraction": round3(thresholds.max_high_clip_fraction),
        "min_mean_background_delta": round3(thresholds.min_mean_background_delta)
    }));
    check.with_region_from_screen("node", Some(handle), region)
}

#[derive(Debug, Clone, Copy)]
struct ObjectPixelMetrics {
    region_pixels: u64,
    foreground_pixels: u64,
    low_clip_fraction: f32,
    high_clip_fraction: f32,
    mean_luminance: f32,
    mean_background_delta: f32,
    max_background_delta: f32,
}

fn object_pixel_metrics(
    capture: &CaptureRgba8,
    region: CaptureScreenRegion,
    background: Color,
) -> ObjectPixelMetrics {
    let background = linear_rgba_to_srgb8(background);
    let mut foreground_pixels = 0_u64;
    let mut low_clip = 0_u64;
    let mut high_clip = 0_u64;
    let mut luminance_sum = 0.0_f32;
    let mut background_delta_sum = 0.0_f32;
    let mut max_background_delta = 0.0_f32;
    for y in region.y..region.y.saturating_add(region.height) {
        for x in region.x..region.x.saturating_add(region.width) {
            let offset = ((y as usize) * (capture.descriptor.width as usize) + (x as usize)) * 4;
            let Some(pixel) = capture.rgba8.get(offset..offset + 4) else {
                continue;
            };
            let background_delta = max_rgb_delta(pixel, background);
            if background_delta * 255.0 <= f32::from(OBJECT_PIXEL_BACKGROUND_TOLERANCE_RGBA8) {
                continue;
            }
            foreground_pixels = foreground_pixels.saturating_add(1);
            max_background_delta = max_background_delta.max(background_delta);
            background_delta_sum += background_delta;
            let luminance = srgb_luminance(pixel);
            luminance_sum += luminance;
            if luminance <= OBJECT_LOW_CLIP_LUMA {
                low_clip = low_clip.saturating_add(1);
            }
            if luminance >= OBJECT_HIGH_CLIP_LUMA {
                high_clip = high_clip.saturating_add(1);
            }
        }
    }
    let region_pixels = u64::from(region.width).saturating_mul(u64::from(region.height));
    if foreground_pixels == 0 {
        return ObjectPixelMetrics {
            region_pixels,
            foreground_pixels,
            low_clip_fraction: 0.0,
            high_clip_fraction: 0.0,
            mean_luminance: 0.0,
            mean_background_delta: 0.0,
            max_background_delta,
        };
    }
    ObjectPixelMetrics {
        region_pixels,
        foreground_pixels,
        low_clip_fraction: fraction(low_clip, foreground_pixels),
        high_clip_fraction: fraction(high_clip, foreground_pixels),
        mean_luminance: luminance_sum / foreground_pixels as f32,
        mean_background_delta: background_delta_sum / foreground_pixels as f32,
        max_background_delta,
    }
}

fn max_rgb_delta(pixel: &[u8], background: [u8; 4]) -> f32 {
    (0..3)
        .map(|channel| f32::from(pixel[channel].abs_diff(background[channel])) / 255.0)
        .fold(0.0_f32, f32::max)
}

fn srgb_luminance(pixel: &[u8]) -> f32 {
    let r = f32::from(pixel[0]) / 255.0;
    let g = f32::from(pixel[1]) / 255.0;
    let b = f32::from(pixel[2]) / 255.0;
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

fn fraction(numerator: u64, denominator: u64) -> f32 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f32 / denominator as f32
    }
}

#[derive(Debug, Clone, Copy)]
struct ObjectPixelThresholds {
    max_low_clip_fraction: f32,
    max_high_clip_fraction: f32,
    min_mean_background_delta: f32,
}

impl ObjectPixelThresholds {
    fn for_profile(profile: &str) -> Self {
        match profile {
            "cad" => Self {
                max_low_clip_fraction: 0.90,
                max_high_clip_fraction: 0.90,
                min_mean_background_delta: 0.035,
            },
            "documentation" => Self {
                max_low_clip_fraction: 0.80,
                max_high_clip_fraction: 0.70,
                min_mean_background_delta: 0.045,
            },
            "dashboard" => Self {
                max_low_clip_fraction: 0.75,
                max_high_clip_fraction: 0.65,
                min_mean_background_delta: 0.045,
            },
            // A product still must fail on a crushed subject; it is the
            // profile that gates beauty renders. Kept in step with
            // `RenderQualityProfile::severe_black_crush_max`.
            "product" | "photo_product" => Self {
                max_low_clip_fraction: 0.45,
                max_high_clip_fraction: 0.30,
                min_mean_background_delta: 0.060,
            },
            _ => Self {
                max_low_clip_fraction: 0.80,
                max_high_clip_fraction: 0.30,
                min_mean_background_delta: 0.060,
            },
        }
    }
}
