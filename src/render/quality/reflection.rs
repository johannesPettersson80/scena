use std::collections::BTreeMap;

use crate::scene::recipe::SceneRecipeQualityReflectionV1;

use super::metrics::frame_metrics;
use super::types::{self, RenderQualityCheckV1, RenderQualityRegion};

pub fn evaluate_reflection_region_quality(
    id: &str,
    rgba8: &[u8],
    width: u32,
    height: u32,
    region: RenderQualityRegion,
    expectation: SceneRecipeQualityReflectionV1,
) -> Vec<RenderQualityCheckV1> {
    let metrics = frame_metrics(rgba8, width, height, region);
    let min_luminance_range = expectation.min_luminance_range.unwrap_or(0.16) as f32;
    let min_sobel_energy = expectation.min_sobel_energy.unwrap_or(0.03) as f32;
    let min_chroma_range = expectation.min_chroma_range.unwrap_or(0.08) as f32;
    let max_firefly_fraction = expectation.max_firefly_fraction.unwrap_or(0.01) as f32;
    let chroma_range = reflection_chroma_range(rgba8, width, height, region);
    let firefly_fraction = reflection_firefly_fraction(rgba8, width, height, region);
    let mut checks = Vec::new();
    if metrics.luminance_range < min_luminance_range
        || metrics.sobel_energy < min_sobel_energy
        || chroma_range < min_chroma_range
    {
        checks.push(RenderQualityCheckV1 {
            id: id.to_owned(),
            code: "reflection_structure_missing".to_owned(),
            severity: "error".to_owned(),
            region: region.to_report(),
            observed: BTreeMap::from([
                ("chroma_range".to_owned(), types::round3(chroma_range)),
                ("luminance_range".to_owned(), metrics.luminance_range),
                ("sobel_energy".to_owned(), metrics.sobel_energy),
            ]),
            threshold: BTreeMap::from([
                ("min_chroma_range".to_owned(), min_chroma_range),
                ("min_luminance_range".to_owned(), min_luminance_range),
                ("min_sobel_energy".to_owned(), min_sobel_energy),
            ]),
            fix_hint: "enable a reflective floor or SSR/IBL reflection path that shows structured subject or environment detail on the reflective surface".to_owned(),
        });
    }
    if firefly_fraction > max_firefly_fraction {
        checks.push(RenderQualityCheckV1 {
            id: id.to_owned(),
            code: "reflection_firefly_outliers".to_owned(),
            severity: "error".to_owned(),
            region: region.to_report(),
            observed: BTreeMap::from([(
                "firefly_fraction".to_owned(),
                types::round3(firefly_fraction),
            )]),
            threshold: BTreeMap::from([(
                "max_firefly_fraction".to_owned(),
                max_firefly_fraction,
            )]),
            fix_hint: "increase IBL prefilter quality or use source-mip importance sampling so tiny HDR emitters blur into reflection mips instead of isolated bright specks".to_owned(),
        });
    }
    checks
}

fn reflection_chroma_range(
    rgba8: &[u8],
    width: u32,
    height: u32,
    region: RenderQualityRegion,
) -> f32 {
    let max_x = region.x.saturating_add(region.width).min(width);
    let max_y = region.y.saturating_add(region.height).min(height);
    let mut min_chroma = f32::INFINITY;
    let mut max_chroma = f32::NEG_INFINITY;
    for y in region.y..max_y {
        for x in region.x..max_x {
            let offset = ((y as usize) * (width as usize) + x as usize) * 4;
            let Some(pixel) = rgba8.get(offset..offset + 3) else {
                continue;
            };
            let r = f32::from(pixel[0]) / 255.0;
            let g = f32::from(pixel[1]) / 255.0;
            let b = f32::from(pixel[2]) / 255.0;
            let high = r.max(g).max(b);
            let low = r.min(g).min(b);
            let chroma = high - low;
            min_chroma = min_chroma.min(chroma);
            max_chroma = max_chroma.max(chroma);
        }
    }
    if min_chroma.is_finite() && max_chroma.is_finite() {
        types::round3((max_chroma - min_chroma).max(0.0))
    } else {
        0.0
    }
}

fn reflection_firefly_fraction(
    rgba8: &[u8],
    width: u32,
    height: u32,
    region: RenderQualityRegion,
) -> f32 {
    let max_x = region.x.saturating_add(region.width).min(width);
    let max_y = region.y.saturating_add(region.height).min(height);
    if max_x <= region.x || max_y <= region.y {
        return 0.0;
    }
    let mut luminance_values =
        Vec::with_capacity(region.width.saturating_mul(region.height) as usize);
    for y in region.y..max_y {
        for x in region.x..max_x {
            luminance_values.push(pixel_luminance(rgba8, width, x, y));
        }
    }
    if luminance_values.len() < 9 {
        return 0.0;
    }
    luminance_values.sort_by(f32::total_cmp);
    let p95 = percentile(&luminance_values, 0.95);
    let threshold = (p95 + 0.25).clamp(0.74, 0.97);
    let mut candidates = 0usize;
    let mut isolated = 0usize;
    for y in region.y..max_y {
        for x in region.x..max_x {
            let center = pixel_luminance(rgba8, width, x, y);
            if center < threshold {
                continue;
            }
            candidates += 1;
            if bright_neighbor_count(rgba8, width, height, x, y, center) <= 1 {
                isolated += 1;
            }
        }
    }
    if candidates == 0 {
        return 0.0;
    }
    isolated as f32 / (region.width.saturating_mul(region.height).max(1) as f32)
}

fn bright_neighbor_count(
    rgba8: &[u8],
    width: u32,
    height: u32,
    x: u32,
    y: u32,
    center_luminance: f32,
) -> usize {
    let threshold = (center_luminance - 0.12).max(0.0);
    let min_x = x.saturating_sub(1);
    let min_y = y.saturating_sub(1);
    let max_x = (x + 1).min(width.saturating_sub(1));
    let max_y = (y + 1).min(height.saturating_sub(1));
    let mut count = 0usize;
    for ny in min_y..=max_y {
        for nx in min_x..=max_x {
            if nx == x && ny == y {
                continue;
            }
            if pixel_luminance(rgba8, width, nx, ny) >= threshold {
                count += 1;
            }
        }
    }
    count
}

fn pixel_luminance(rgba8: &[u8], width: u32, x: u32, y: u32) -> f32 {
    let offset = ((y as usize) * (width as usize) + x as usize) * 4;
    let Some(pixel) = rgba8.get(offset..offset + 3) else {
        return 0.0;
    };
    (0.2126 * f32::from(pixel[0]) + 0.7152 * f32::from(pixel[1]) + 0.0722 * f32::from(pixel[2]))
        / 255.0
}

fn percentile(sorted_values: &[f32], percentile: f32) -> f32 {
    if sorted_values.is_empty() {
        return 0.0;
    }
    let index = ((sorted_values.len() - 1) as f32 * percentile.clamp(0.0, 1.0)).round() as usize;
    sorted_values[index.min(sorted_values.len() - 1)]
}
