use std::collections::BTreeMap;

use crate::scene::recipe::SceneRecipeQualityGroundingV1;

use super::metrics::pixel_luminance;
use super::types::{self, RenderQualityCheckV1, RenderQualityRegion};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderQualityGroundingMetrics {
    pub contact_shadow_delta: f32,
    pub contact_luminance: f32,
    pub reference_luminance: f32,
    pub contact_pixels: u32,
    pub reference_pixels: u32,
}

pub fn contact_shadow_metrics(
    rgba8: &[u8],
    width: u32,
    height: u32,
    target_region: RenderQualityRegion,
) -> RenderQualityGroundingMetrics {
    let Some((contact, reference)) = contact_shadow_regions(width, height, target_region) else {
        return RenderQualityGroundingMetrics {
            contact_shadow_delta: 0.0,
            contact_luminance: 0.0,
            reference_luminance: 0.0,
            contact_pixels: 0,
            reference_pixels: 0,
        };
    };
    let (contact_luminance, contact_pixels) = mean_luminance(rgba8, width, height, contact);
    let (reference_luminance, reference_pixels) = mean_luminance(rgba8, width, height, reference);
    RenderQualityGroundingMetrics {
        contact_shadow_delta: types::round3((reference_luminance - contact_luminance).max(0.0)),
        contact_luminance: types::round3(contact_luminance),
        reference_luminance: types::round3(reference_luminance),
        contact_pixels,
        reference_pixels,
    }
}

pub fn evaluate_grounding_region_quality(
    id: &str,
    rgba8: &[u8],
    width: u32,
    height: u32,
    target_region: RenderQualityRegion,
    expectation: SceneRecipeQualityGroundingV1,
) -> Vec<RenderQualityCheckV1> {
    let metrics = contact_shadow_metrics(rgba8, width, height, target_region);
    let min_contact_shadow_delta = expectation.min_contact_shadow_delta.unwrap_or(0.018) as f32;
    let passes = metrics.contact_pixels > 0
        && metrics.reference_pixels > 0
        && metrics.contact_shadow_delta >= min_contact_shadow_delta;
    vec![RenderQualityCheckV1 {
        id: id.to_owned(),
        code: if passes {
            "contact_shadow_checked"
        } else {
            "contact_shadow_missing"
        }
        .to_owned(),
        severity: if passes { "info" } else { "error" }.to_owned(),
        region: target_region.to_report(),
        observed: BTreeMap::from([
            (
                "contact_luminance".to_owned(),
                types::round3(metrics.contact_luminance),
            ),
            (
                "contact_pixels".to_owned(),
                metrics.contact_pixels as f32,
            ),
            (
                "contact_shadow_delta".to_owned(),
                types::round3(metrics.contact_shadow_delta),
            ),
            (
                "reference_luminance".to_owned(),
                types::round3(metrics.reference_luminance),
            ),
            (
                "reference_pixels".to_owned(),
                metrics.reference_pixels as f32,
            ),
        ]),
        threshold: BTreeMap::from([(
            "min_contact_shadow_delta".to_owned(),
            types::round3(min_contact_shadow_delta),
        )]),
        fix_hint: if passes {
            "no action needed"
        } else {
            "enable SSAO/contact shadows, add a shadow-casting light/floor setup, or adjust the camera so the target's contact region is visible"
        }
        .to_owned(),
    }]
}

fn contact_shadow_regions(
    width: u32,
    height: u32,
    target: RenderQualityRegion,
) -> Option<(RenderQualityRegion, RenderQualityRegion)> {
    if target.width < 4 || target.height < 4 || width == 0 || height == 0 {
        return None;
    }
    let band_height = (target.height / 5).clamp(3, 8);
    let band_pad = (target.width / 5).clamp(2, 12);
    let x = target.x.saturating_sub(band_pad);
    let max_x = target
        .x
        .saturating_add(target.width)
        .saturating_add(band_pad)
        .min(width);
    let band_width = max_x.saturating_sub(x).max(1);
    let target_bottom = target.y.saturating_add(target.height);
    let contact_y = target_bottom.saturating_add(1);
    let reference_y = target_bottom.saturating_add(band_height.saturating_mul(3));
    if reference_y >= height || contact_y >= height {
        return None;
    }
    let contact_height = band_height.min(height.saturating_sub(contact_y)).max(1);
    let reference_height = band_height.min(height.saturating_sub(reference_y)).max(1);
    Some((
        RenderQualityRegion {
            kind: "contact_shadow",
            handle: target.handle,
            x,
            y: contact_y,
            width: band_width,
            height: contact_height,
        },
        RenderQualityRegion {
            kind: "contact_shadow_reference",
            handle: target.handle,
            x,
            y: reference_y,
            width: band_width,
            height: reference_height,
        },
    ))
}

fn mean_luminance(
    rgba8: &[u8],
    width: u32,
    height: u32,
    region: RenderQualityRegion,
) -> (f32, u32) {
    let max_x = region.x.saturating_add(region.width).min(width);
    let max_y = region.y.saturating_add(region.height).min(height);
    let mut total = 0.0_f32;
    let mut count = 0_u32;
    for y in region.y..max_y {
        for x in region.x..max_x {
            let Some(luma) = pixel_luminance(rgba8, width, x, y) else {
                continue;
            };
            total += luma;
            count = count.saturating_add(1);
        }
    }
    if count == 0 {
        (0.0, 0)
    } else {
        (total / count as f32, count)
    }
}
