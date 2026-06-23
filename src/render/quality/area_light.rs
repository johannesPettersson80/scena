use std::collections::{BTreeMap, BTreeSet};

use crate::scene::recipe::SceneRecipeQualityAreaLightV1;

use super::metrics::{percentile_sorted, pixel_luminance};
use super::types::{
    self, RenderQualityAreaLightMetrics, RenderQualityCheckV1, RenderQualityRegion,
};

pub fn area_light_shadow_metrics(
    rgba8: &[u8],
    width: u32,
    height: u32,
    region: RenderQualityRegion,
) -> RenderQualityAreaLightMetrics {
    let max_x = region.x.saturating_add(region.width).min(width);
    let max_y = region.y.saturating_add(region.height).min(height);
    if region.x >= max_x || region.y >= max_y {
        return RenderQualityAreaLightMetrics {
            shadow_contrast: 0.0,
            penumbra_width_px: 0.0,
            penumbra_luma_levels: 0,
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
        return RenderQualityAreaLightMetrics {
            shadow_contrast: 0.0,
            penumbra_width_px: 0.0,
            penumbra_luma_levels: 0,
        };
    }
    all_luma.sort_by(f32::total_cmp);
    let shadow_contrast =
        (percentile_sorted(&all_luma, 0.90) - percentile_sorted(&all_luma, 0.10)).max(0.0);

    let y0 = region
        .y
        .saturating_add(region.height.saturating_mul(1) / 5)
        .min(max_y);
    let y1 = region
        .y
        .saturating_add(region.height.saturating_mul(4) / 5)
        .min(max_y);
    let mut width_total = 0.0_f32;
    let mut rows = 0_u32;
    let mut luma_levels = BTreeSet::new();
    for y in y0..y1 {
        let mut row_samples = Vec::with_capacity(max_x.saturating_sub(region.x) as usize);
        for x in region.x..max_x {
            row_samples.push((x, pixel_luminance(rgba8, width, x, y).unwrap_or(0.0)));
        }
        if row_samples.len() < 3 {
            continue;
        }
        let mut row_luma = row_samples
            .iter()
            .map(|(_, value)| *value)
            .collect::<Vec<_>>();
        row_luma.sort_by(f32::total_cmp);
        let row_low = percentile_sorted(&row_luma, 0.10);
        let row_high = percentile_sorted(&row_luma, 0.90);
        let row_range = row_high - row_low;
        if row_range < 0.035 {
            continue;
        }

        let mut strongest_index = 1usize;
        let mut strongest_gradient = 0.0_f32;
        for index in 1..row_samples.len() {
            let gradient = (row_samples[index].1 - row_samples[index - 1].1).abs();
            if gradient > strongest_gradient {
                strongest_gradient = gradient;
                strongest_index = index;
            }
        }
        if strongest_gradient <= 0.001 {
            continue;
        }

        let edge_width = (row_range / strongest_gradient).clamp(0.0, row_samples.len() as f32);
        if edge_width < 0.5 {
            continue;
        }
        let radius = ((edge_width * 1.5).ceil() as usize).clamp(3, 24);
        let start = strongest_index.saturating_sub(radius);
        let end = strongest_index
            .saturating_add(radius)
            .min(row_samples.len().saturating_sub(1));
        for (_, value) in &row_samples[start..=end] {
            luma_levels.insert((*value * 255.0).round().clamp(0.0, 255.0) as u8);
        }
        width_total += edge_width;
        rows = rows.saturating_add(1);
    }

    RenderQualityAreaLightMetrics {
        shadow_contrast: types::round3(shadow_contrast),
        penumbra_width_px: types::round3(width_total / rows.max(1) as f32),
        penumbra_luma_levels: luma_levels.len(),
    }
}

pub fn evaluate_area_light_region_quality(
    id: &str,
    rgba8: &[u8],
    width: u32,
    height: u32,
    region: RenderQualityRegion,
    expectation: SceneRecipeQualityAreaLightV1,
    max_emitter_extent_meters: f32,
) -> Vec<RenderQualityCheckV1> {
    let metrics = area_light_shadow_metrics(rgba8, width, height, region);
    let min_shadow_contrast = expectation.min_shadow_contrast.unwrap_or(0.025) as f32;
    let min_penumbra_width_px = expectation.min_penumbra_width_px.unwrap_or(1.5) as f32;
    let min_penumbra_luma_levels = expectation.min_penumbra_luma_levels.unwrap_or(18.0) as f32;
    let min_emitter_extent_meters = expectation.min_emitter_extent_meters.unwrap_or(0.5) as f32;
    let passes = metrics.shadow_contrast >= min_shadow_contrast
        && metrics.penumbra_width_px >= min_penumbra_width_px
        && metrics.penumbra_luma_levels as f32 >= min_penumbra_luma_levels
        && max_emitter_extent_meters >= min_emitter_extent_meters;
    vec![RenderQualityCheckV1 {
        id: id.to_owned(),
        code: if passes {
            "area_light_soft_shadow_checked"
        } else {
            "area_light_soft_shadow_insufficient"
        }
        .to_owned(),
        severity: if passes { "info" } else { "error" }.to_owned(),
        region: region.to_report(),
        observed: BTreeMap::from([
            (
                "penumbra_luma_levels".to_owned(),
                metrics.penumbra_luma_levels as f32,
            ),
            (
                "penumbra_width_px".to_owned(),
                types::round3(metrics.penumbra_width_px),
            ),
            (
                "shadow_contrast".to_owned(),
                types::round3(metrics.shadow_contrast),
            ),
            (
                "soft_emitter_extent_meters".to_owned(),
                types::round3(max_emitter_extent_meters),
            ),
        ]),
        threshold: BTreeMap::from([
            (
                "min_emitter_extent_meters".to_owned(),
                types::round3(min_emitter_extent_meters),
            ),
            (
                "min_penumbra_luma_levels".to_owned(),
                types::round3(min_penumbra_luma_levels),
            ),
            (
                "min_penumbra_width_px".to_owned(),
                types::round3(min_penumbra_width_px),
            ),
            (
                "min_shadow_contrast".to_owned(),
                types::round3(min_shadow_contrast),
            ),
        ]),
        fix_hint: if passes {
            "no action needed"
        } else {
            "use a larger finite area light or softbox, make sure an occluder intersects the emitter cone, and avoid point-like light dimensions when soft shadows are required"
        }
        .to_owned(),
    }]
}
