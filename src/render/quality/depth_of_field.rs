use std::collections::BTreeMap;

use crate::scene::recipe::SceneRecipeQualityDepthOfFieldV1;

use super::metrics::frame_metrics;
use super::types::{
    self, RenderQualityCheckV1, RenderQualityDepthOfFieldMetrics, RenderQualityRegion,
    RenderQualityStatusV1,
};

pub struct DepthOfFieldQualityInput<'a> {
    pub focused_rgba8: &'a [u8],
    pub source_rgba8: &'a [u8],
    pub width: u32,
    pub height: u32,
    pub focal_region: RenderQualityRegion,
    pub background_region: RenderQualityRegion,
    pub expectation: &'a SceneRecipeQualityDepthOfFieldV1,
}

pub fn depth_of_field_metrics(
    focused_rgba8: &[u8],
    source_rgba8: &[u8],
    width: u32,
    height: u32,
    focal_region: RenderQualityRegion,
    background_region: RenderQualityRegion,
) -> RenderQualityDepthOfFieldMetrics {
    let source_background = frame_metrics(source_rgba8, width, height, background_region);
    let focused_background = frame_metrics(focused_rgba8, width, height, background_region);
    let source_background_sobel = source_background.sobel_energy;
    let focused_background_sobel = focused_background.sobel_energy;
    let background_sobel_drop = (source_background_sobel - focused_background_sobel).max(0.0);
    let background_sobel_drop_fraction = if source_background_sobel <= f32::EPSILON {
        0.0
    } else {
        background_sobel_drop / source_background_sobel
    };
    RenderQualityDepthOfFieldMetrics {
        source_background_sobel: types::round3(source_background_sobel),
        focused_background_sobel: types::round3(focused_background_sobel),
        background_sobel_drop: types::round3(background_sobel_drop),
        background_sobel_drop_fraction: types::round3(background_sobel_drop_fraction),
        focal_mean_delta: types::round3(mean_rgb_delta(
            focused_rgba8,
            source_rgba8,
            width,
            height,
            focal_region,
        )),
    }
}

pub fn evaluate_depth_of_field_region_quality(
    id: &str,
    input: DepthOfFieldQualityInput<'_>,
) -> Vec<RenderQualityCheckV1> {
    let focused_rgba8 = input.focused_rgba8;
    let source_rgba8 = input.source_rgba8;
    let width = input.width;
    let height = input.height;
    let focal_region = input.focal_region;
    let background_region = input.background_region;
    let expectation = input.expectation;
    let metrics = depth_of_field_metrics(
        focused_rgba8,
        source_rgba8,
        width,
        height,
        focal_region,
        background_region,
    );
    let min_source_background_sobel =
        expectation.min_source_background_sobel.unwrap_or(0.035) as f32;
    let min_background_sobel_drop = expectation.min_background_sobel_drop.unwrap_or(0.018) as f32;
    let min_background_sobel_drop_fraction = expectation
        .min_background_sobel_drop_fraction
        .unwrap_or(0.18) as f32;
    let max_focal_mean_delta = expectation.max_focal_mean_delta.unwrap_or(0.08) as f32;

    let mut code = "depth_of_field_checked";
    let mut severity = "info";
    let mut fix_hint = "no action needed";
    if metrics.source_background_sobel < min_source_background_sobel {
        code = "depth_of_field_background_detail_missing";
        severity = "error";
        fix_hint = "use a textured or high-frequency background target when proving depth of field, otherwise blur cannot be measured";
    } else if metrics.background_sobel_drop < min_background_sobel_drop
        || metrics.background_sobel_drop_fraction < min_background_sobel_drop_fraction
    {
        code = "depth_of_field_blur_insufficient";
        severity = "error";
        fix_hint = "enable render.depth_of_field, increase radius_px or lower aperture_f_stop, and ensure focus_distance targets the focal subject instead of the background";
    } else if metrics.focal_mean_delta > max_focal_mean_delta {
        code = "depth_of_field_focal_softened";
        severity = "error";
        fix_hint = "set focus_distance to the focal subject distance or reduce depth-of-field radius/aperture so the subject remains sharp";
    }

    vec![RenderQualityCheckV1 {
        id: id.to_owned(),
        code: code.to_owned(),
        status: if severity == "error" {
            RenderQualityStatusV1::Failed
        } else {
            RenderQualityStatusV1::Checked
        },
        severity: severity.to_owned(),
        region: background_region.to_report(),
        observed: BTreeMap::from([
            (
                "background_sobel_drop".to_owned(),
                metrics.background_sobel_drop,
            ),
            (
                "background_sobel_drop_fraction".to_owned(),
                metrics.background_sobel_drop_fraction,
            ),
            ("focal_mean_delta".to_owned(), metrics.focal_mean_delta),
            (
                "focused_background_sobel".to_owned(),
                metrics.focused_background_sobel,
            ),
            (
                "source_background_sobel".to_owned(),
                metrics.source_background_sobel,
            ),
        ]),
        threshold: BTreeMap::from([
            (
                "max_focal_mean_delta".to_owned(),
                types::round3(max_focal_mean_delta),
            ),
            (
                "min_background_sobel_drop".to_owned(),
                types::round3(min_background_sobel_drop),
            ),
            (
                "min_background_sobel_drop_fraction".to_owned(),
                types::round3(min_background_sobel_drop_fraction),
            ),
            (
                "min_source_background_sobel".to_owned(),
                types::round3(min_source_background_sobel),
            ),
        ]),
        fix_hint: fix_hint.to_owned(),
    }]
}

fn mean_rgb_delta(
    left: &[u8],
    right: &[u8],
    width: u32,
    height: u32,
    region: RenderQualityRegion,
) -> f32 {
    if left.len() != right.len() {
        return 1.0;
    }
    let max_x = region.x.saturating_add(region.width).min(width);
    let max_y = region.y.saturating_add(region.height).min(height);
    let mut total = 0.0_f32;
    let mut count = 0_u32;
    for y in region.y..max_y {
        for x in region.x..max_x {
            let offset = ((y as usize) * (width as usize) + x as usize) * 4;
            let Some(left_pixel) = left.get(offset..offset + 3) else {
                continue;
            };
            let Some(right_pixel) = right.get(offset..offset + 3) else {
                continue;
            };
            total += left_pixel
                .iter()
                .zip(right_pixel)
                .map(|(left, right)| f32::from(left.abs_diff(*right)) / 255.0)
                .sum::<f32>()
                / 3.0;
            count = count.saturating_add(1);
        }
    }
    if count == 0 {
        1.0
    } else {
        total / count as f32
    }
}
