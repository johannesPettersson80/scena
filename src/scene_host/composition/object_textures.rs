use std::collections::BTreeSet;

use serde_json::json;

use super::checks::{CompositionCheckExt, checked_check, error_check, observed_pairs, round3};
use super::helpers::linear_rgba_to_srgb8;
use crate::{
    CaptureRgba8, CaptureScreenRegion, Color, SceneCompositionCheckV1, SceneDrawInspectionV1,
};

const TEXTURE_BACKGROUND_TOLERANCE_RGBA8: u8 = 2;

pub(super) fn object_texture_result_check(
    target_path: &str,
    target_id: &str,
    handle: u64,
    capture: &CaptureRgba8,
    region: CaptureScreenRegion,
    background: Color,
    draws: &[&SceneDrawInspectionV1],
) -> Option<SceneCompositionCheckV1> {
    let texture_slots = decoded_texture_slots(draws);
    if !texture_slots
        .iter()
        .any(|slot| slot.as_str() == "baseColorTexture")
    {
        return None;
    }
    let sample_region = texture_sample_region(region);
    let metrics = texture_result_metrics(capture, sample_region, background);
    let observed = observed_pairs([
        ("texture_slots", json!(texture_slots)),
        ("region_pixels", json!(metrics.region_pixels)),
        ("foreground_pixels", json!(metrics.foreground_pixels)),
        ("luminance_min", json!(round3(metrics.luminance_min))),
        ("luminance_max", json!(round3(metrics.luminance_max))),
        ("luminance_range", json!(round3(metrics.luminance_range))),
        ("luminance_stddev", json!(round3(metrics.luminance_stddev))),
        ("unique_luma_levels", json!(metrics.unique_luma_levels)),
    ]);
    let thresholds = TextureResultThresholds::default();
    let mut check = if metrics.foreground_pixels < thresholds.min_foreground_pixels {
        error_check(
            format!("{target_path}.texture_result"),
            "texture_mapping",
            "texture_result_missing",
            Some(target_id.to_owned()),
            vec![handle],
            observed,
            (
                "decoded base-color texture is declared but the projected node region has too few visible pixels to prove the texture result",
                "move the textured object into frame, increase its on-screen size, or fix material/UV mapping so the texture is visible",
            ),
        )
    } else if metrics.luminance_stddev < thresholds.min_luminance_stddev
        || metrics.luminance_range < thresholds.min_luminance_range
        || metrics.unique_luma_levels < thresholds.min_unique_luma_levels
    {
        error_check(
            format!("{target_path}.texture_result"),
            "texture_mapping",
            "texture_result_flat",
            Some(target_id.to_owned()),
            vec![handle],
            observed,
            (
                "decoded base-color texture is declared but the rendered target region is nearly flat",
                "check UVs, texture transform, sampler/wrap mode, and whether the intended texture is actually mapped onto the target",
            ),
        )
    } else {
        checked_check(
            format!("{target_path}.texture_result"),
            "texture_mapping",
            "texture_result_visible",
            Some(target_id.to_owned()),
            vec![handle],
            observed,
            (
                "decoded base-color texture produces visible luminance variation in the target's native-resolution region",
                "no action needed",
            ),
        )
    };
    check.threshold = Some(json!({
        "min_foreground_pixels": thresholds.min_foreground_pixels,
        "min_luminance_stddev": round3(thresholds.min_luminance_stddev),
        "min_luminance_range": round3(thresholds.min_luminance_range),
        "min_unique_luma_levels": thresholds.min_unique_luma_levels
    }));
    Some(check.with_region_from_screen("node", Some(handle), sample_region))
}

fn decoded_texture_slots(draws: &[&SceneDrawInspectionV1]) -> Vec<String> {
    let mut slots = BTreeSet::new();
    for draw in draws {
        let Some(material) = &draw.material else {
            continue;
        };
        for texture in &material.textures {
            if texture.has_decoded_pixels {
                slots.insert(texture.slot.clone());
            }
        }
    }
    slots.into_iter().collect()
}

#[derive(Debug, Clone, Copy)]
struct TextureResultMetrics {
    region_pixels: u64,
    foreground_pixels: u64,
    luminance_min: f32,
    luminance_max: f32,
    luminance_range: f32,
    luminance_stddev: f32,
    unique_luma_levels: usize,
}

fn texture_result_metrics(
    capture: &CaptureRgba8,
    region: CaptureScreenRegion,
    background: Color,
) -> TextureResultMetrics {
    let background = linear_rgba_to_srgb8(background);
    let mut foreground_pixels = 0_u64;
    let mut sum = 0.0_f32;
    let mut sum_sq = 0.0_f32;
    let mut luminance_min = f32::INFINITY;
    let mut luminance_max = f32::NEG_INFINITY;
    let mut unique = BTreeSet::new();
    for y in region.y..region.y.saturating_add(region.height) {
        for x in region.x..region.x.saturating_add(region.width) {
            let offset = ((y as usize) * (capture.descriptor.width as usize) + (x as usize)) * 4;
            let Some(pixel) = capture.rgba8.get(offset..offset + 4) else {
                continue;
            };
            if !pixel_differs_from_background(pixel, background) {
                continue;
            }
            foreground_pixels = foreground_pixels.saturating_add(1);
            let luminance = srgb_luminance(pixel);
            sum += luminance;
            sum_sq += luminance * luminance;
            luminance_min = luminance_min.min(luminance);
            luminance_max = luminance_max.max(luminance);
            unique.insert((luminance * 255.0).round().clamp(0.0, 255.0) as u8);
        }
    }
    let region_pixels = u64::from(region.width).saturating_mul(u64::from(region.height));
    if foreground_pixels == 0 {
        return TextureResultMetrics {
            region_pixels,
            foreground_pixels,
            luminance_min: 0.0,
            luminance_max: 0.0,
            luminance_range: 0.0,
            luminance_stddev: 0.0,
            unique_luma_levels: 0,
        };
    }
    let mean = sum / foreground_pixels as f32;
    let variance = (sum_sq / foreground_pixels as f32) - mean * mean;
    TextureResultMetrics {
        region_pixels,
        foreground_pixels,
        luminance_min,
        luminance_max,
        luminance_range: luminance_max - luminance_min,
        luminance_stddev: variance.max(0.0).sqrt(),
        unique_luma_levels: unique.len(),
    }
}

fn texture_sample_region(region: CaptureScreenRegion) -> CaptureScreenRegion {
    let inset_x = (region.width / 12).clamp(2, 12);
    let inset_y = (region.height / 12).clamp(2, 12);
    if region.width <= inset_x.saturating_mul(2) || region.height <= inset_y.saturating_mul(2) {
        return region;
    }
    CaptureScreenRegion {
        x: region.x.saturating_add(inset_x),
        y: region.y.saturating_add(inset_y),
        width: region.width.saturating_sub(inset_x.saturating_mul(2)),
        height: region.height.saturating_sub(inset_y.saturating_mul(2)),
    }
}

fn pixel_differs_from_background(pixel: &[u8], background: [u8; 4]) -> bool {
    (0..3).any(|channel| {
        pixel[channel].abs_diff(background[channel]) > TEXTURE_BACKGROUND_TOLERANCE_RGBA8
    })
}

fn srgb_luminance(pixel: &[u8]) -> f32 {
    let r = f32::from(pixel[0]) / 255.0;
    let g = f32::from(pixel[1]) / 255.0;
    let b = f32::from(pixel[2]) / 255.0;
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

#[derive(Debug, Clone, Copy)]
struct TextureResultThresholds {
    min_foreground_pixels: u64,
    min_luminance_stddev: f32,
    min_luminance_range: f32,
    min_unique_luma_levels: usize,
}

impl Default for TextureResultThresholds {
    fn default() -> Self {
        Self {
            min_foreground_pixels: 64,
            min_luminance_stddev: 0.02,
            min_luminance_range: 0.08,
            min_unique_luma_levels: 3,
        }
    }
}
