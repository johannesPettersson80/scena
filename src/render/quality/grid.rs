use std::collections::{BTreeMap, BTreeSet};

use super::metrics::{percentile_sorted, pixel_luminance};
use super::types::{self, RenderQualityCheckV1, RenderQualityGridLineMetrics, RenderQualityRegion};

pub fn grid_line_metrics(
    rgba8: &[u8],
    width: u32,
    height: u32,
    region: RenderQualityRegion,
) -> RenderQualityGridLineMetrics {
    let max_x = region.x.saturating_add(region.width).min(width);
    let max_y = region.y.saturating_add(region.height).min(height);
    if region.x >= max_x || region.y >= max_y {
        return RenderQualityGridLineMetrics {
            intermediate_px_per_edge: 0.0,
            unique_luma_levels: 0,
            transition_width_px: 0.0,
            halo_overshoot: 0.0,
            contrast_range: 0.0,
        };
    }

    let mut all_luma =
        Vec::with_capacity((region.width as usize).saturating_mul(region.height as usize));
    for y in region.y..max_y {
        for x in region.x..max_x {
            all_luma.push(pixel_luminance(rgba8, width, x, y).unwrap_or(0.0));
        }
    }
    if all_luma.is_empty() {
        return RenderQualityGridLineMetrics {
            intermediate_px_per_edge: 0.0,
            unique_luma_levels: 0,
            transition_width_px: 0.0,
            halo_overshoot: 0.0,
            contrast_range: 0.0,
        };
    }
    all_luma.sort_by(f32::total_cmp);
    let global_low = percentile_sorted(&all_luma, 0.05);
    let global_high = percentile_sorted(&all_luma, 0.95);
    let contrast_range = (global_high - global_low).max(0.0);

    let y0 = region
        .y
        .saturating_add(region.height.saturating_mul(1) / 5)
        .min(max_y);
    let y1 = region
        .y
        .saturating_add(region.height.saturating_mul(4) / 5)
        .min(max_y);
    let mut transition_total = 0.0_f32;
    let mut edge_rows = 0_u32;
    let mut unique_luma_levels = BTreeSet::new();
    let mut halo_overshoot = 0.0_f32;
    for y in y0..y1 {
        let mut strongest_x = region.x.saturating_add(1).min(max_x.saturating_sub(1));
        let mut strongest_gradient = 0.0_f32;
        let mut previous = pixel_luminance(rgba8, width, region.x, y).unwrap_or(0.0);
        for x in region.x.saturating_add(1)..max_x {
            let current = pixel_luminance(rgba8, width, x, y).unwrap_or(0.0);
            let gradient = (current - previous).abs();
            if gradient > strongest_gradient {
                strongest_gradient = gradient;
                strongest_x = x;
            }
            previous = current;
        }
        if strongest_gradient < 0.08 {
            continue;
        }
        let window_min_x = strongest_x.saturating_sub(10).max(region.x);
        let window_max_x = strongest_x.saturating_add(10).min(max_x.saturating_sub(1));
        let mut row_min = f32::INFINITY;
        let mut row_max = f32::NEG_INFINITY;
        for x in window_min_x..=window_max_x {
            let luma = pixel_luminance(rgba8, width, x, y).unwrap_or(0.0);
            row_min = row_min.min(luma);
            row_max = row_max.max(luma);
        }
        let row_range = row_max - row_min;
        if row_range < 0.18 {
            continue;
        }
        let low_cutoff = row_min + row_range * 0.05;
        let high_cutoff = row_max - row_range * 0.05;
        let mut transition_width = 0_u32;
        for x in window_min_x..=window_max_x {
            let luma = pixel_luminance(rgba8, width, x, y).unwrap_or(0.0);
            if (low_cutoff..high_cutoff).contains(&luma) {
                transition_width = transition_width.saturating_add(1);
                unique_luma_levels.insert((luma * 255.0).round().clamp(0.0, 255.0) as u8);
            }
        }
        halo_overshoot = halo_overshoot
            .max((global_low - row_min).max(0.0))
            .max((row_max - global_high).max(0.0));
        transition_total += transition_width as f32;
        edge_rows = edge_rows.saturating_add(1);
    }

    let intermediate_px_per_edge = transition_total / edge_rows.max(1) as f32;
    RenderQualityGridLineMetrics {
        intermediate_px_per_edge: types::round3(intermediate_px_per_edge),
        unique_luma_levels: unique_luma_levels.len(),
        transition_width_px: types::round3(intermediate_px_per_edge),
        halo_overshoot: types::round3(halo_overshoot),
        contrast_range: types::round3(contrast_range),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn evaluate_grid_line_region_quality(
    id: &str,
    rgba8: &[u8],
    width: u32,
    height: u32,
    region: RenderQualityRegion,
    min_intermediate_px_per_edge: f32,
    min_unique_luma_levels: f32,
    max_halo_overshoot: f32,
    min_contrast_range: f32,
) -> Vec<RenderQualityCheckV1> {
    let metrics = grid_line_metrics(rgba8, width, height, region);
    let passes = metrics.intermediate_px_per_edge >= min_intermediate_px_per_edge
        && metrics.unique_luma_levels as f32 >= min_unique_luma_levels
        && metrics.halo_overshoot <= max_halo_overshoot
        && metrics.contrast_range >= min_contrast_range;
    vec![RenderQualityCheckV1 {
        id: id.to_owned(),
        code: if passes {
            "grid_line_quality_checked"
        } else {
            "grid_line_quality_too_low"
        }
        .to_owned(),
        severity: if passes { "info" } else { "error" }.to_owned(),
        region: region.to_report(),
        observed: BTreeMap::from([
            (
                "contrast_range".to_owned(),
                types::round3(metrics.contrast_range),
            ),
            (
                "halo_overshoot".to_owned(),
                types::round3(metrics.halo_overshoot),
            ),
            (
                "intermediate_px_per_edge".to_owned(),
                types::round3(metrics.intermediate_px_per_edge),
            ),
            (
                "transition_width_px".to_owned(),
                types::round3(metrics.transition_width_px),
            ),
            (
                "unique_luma_levels".to_owned(),
                metrics.unique_luma_levels as f32,
            ),
        ]),
        threshold: BTreeMap::from([
            (
                "max_halo_overshoot".to_owned(),
                types::round3(max_halo_overshoot),
            ),
            (
                "min_contrast_range".to_owned(),
                types::round3(min_contrast_range),
            ),
            (
                "min_intermediate_px_per_edge".to_owned(),
                types::round3(min_intermediate_px_per_edge),
            ),
            (
                "min_unique_luma_levels".to_owned(),
                types::round3(min_unique_luma_levels),
            ),
        ]),
        fix_hint: if passes {
            "no action needed"
        } else {
            "enable MSAA or full-frame supersampling, use reconstruction:\"tent\", increase scene.grid.line_width_px, or raise grid-line contrast enough for native-resolution coverage"
        }
        .to_owned(),
    }]
}
