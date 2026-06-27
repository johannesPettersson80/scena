use serde_json::json;

use super::checks::{CompositionCheckExt, checked_check, error_check, observed_pairs};
use super::helpers::{
    clipped_region_from_rect, draws_for_handle, linear_rgba_to_srgb8, projected_node_rect,
    resolve_target_handles,
};
use crate::{
    CaptureRgba8, CaptureScreenRegion, SceneCompositionCheckV1, SceneInspectionReportV1,
    SceneRecipeBuildV1, SceneRecipeExpectV1,
};

const DEFAULT_HELPER_COLOR_TOLERANCE: u8 = 56;
const OCCLUDER_INTERIOR_INSET_PX: u32 = 6;

pub(super) fn composition_helper_layer_checks(
    expect: Option<&SceneRecipeExpectV1>,
    manifest: &SceneRecipeBuildV1,
    inspection: &SceneInspectionReportV1,
    capture: &CaptureRgba8,
) -> Vec<SceneCompositionCheckV1> {
    let mut checks = Vec::new();
    let Some(expect) = expect else {
        return checks;
    };
    for expectation in &expect.expect_helper_occluded {
        let id = format!("expect_helper_occluded.{}", expectation.id);
        let target_id = Some(expectation.id.clone());
        let helper_handles = resolve_target_handles(&expectation.helper, manifest);
        let occluder_handles = resolve_target_handles(&expectation.occluder, manifest);
        let affected = combined_handles(&helper_handles, &occluder_handles);
        if helper_handles.is_empty() || occluder_handles.is_empty() {
            checks.push(error_check(
                id,
                "helper_render_layer",
                "helper_occlusion_target_unresolved",
                target_id,
                affected,
                observed_pairs([
                    ("helper_handles", json!(helper_handles)),
                    ("occluder_handles", json!(occluder_handles)),
                ]),
                (
                    "helper occlusion expectation did not resolve both helper and occluder targets",
                    "fix the helper/occluder target ids or remove the stale helper occlusion expectation",
                ),
            ));
            continue;
        }

        let Some(region) = occluder_interior_region(capture, inspection, &occluder_handles) else {
            checks.push(error_check(
                id,
                "helper_render_layer",
                "helper_occlusion_region_unavailable",
                target_id,
                affected,
                observed_pairs([
                    ("helper_handles", json!(helper_handles)),
                    ("occluder_handles", json!(occluder_handles)),
                ]),
                (
                    "helper occlusion expectation could not derive a stable occluder screen region",
                    "keep the occluder visible and large enough in frame or remove the expectation",
                ),
            ));
            continue;
        };

        let Some(helper_color) = helper_srgb8_color(inspection, &helper_handles) else {
            checks.push(error_check(
                id,
                "helper_render_layer",
                "helper_occlusion_color_unavailable",
                target_id,
                affected,
                observed_pairs([("helper_handles", json!(helper_handles))]),
                (
                    "helper occlusion expectation could not determine the helper draw color",
                    "use an authored helper material with a stable base color",
                ),
            ));
            continue;
        };

        let helper_pixels = count_helper_pixels_in_region(capture, region, helper_color);
        let tolerance = u64::from(expectation.tolerance_pixels.unwrap_or(0));
        let observed = observed_pairs([
            ("helper_handles", json!(helper_handles)),
            ("occluder_handles", json!(occluder_handles)),
            (
                "helper_srgb8",
                json!([helper_color[0], helper_color[1], helper_color[2]]),
            ),
            ("helper_pixels_inside_occluder", json!(helper_pixels)),
            ("tolerance_pixels", json!(tolerance)),
            (
                "occluder_interior_region",
                json!({
                    "x": region.x,
                    "y": region.y,
                    "width": region.width,
                    "height": region.height
                }),
            ),
        ]);
        if helper_pixels <= tolerance {
            checks.push(
                checked_check(
                    id,
                    "helper_render_layer",
                    "helper_layer_occluded_by_subject",
                    target_id,
                    affected,
                    observed,
                    (
                        "helper pixels are absent from the occluder interior, so depth/layer policy is preserved",
                        "no action needed",
                    ),
                )
                .with_region_from_screen("node", first_handle(&occluder_handles), region),
            );
        } else {
            checks.push(
                error_check(
                    id,
                    "helper_render_layer",
                    "helper_layer_overdraws_subject",
                    target_id,
                    affected,
                    observed,
                    (
                        "helper-colored pixels appear inside the occluder interior",
                        "move the helper behind the subject, keep depth-tested helpers out of front layers, or remove the helper occlusion expectation if the overdraw is intentional",
                    ),
                )
                .with_region_from_screen("node", first_handle(&occluder_handles), region),
            );
        }
    }
    checks
}

fn occluder_interior_region(
    capture: &CaptureRgba8,
    inspection: &SceneInspectionReportV1,
    handles: &[u64],
) -> Option<CaptureScreenRegion> {
    let mut rects = Vec::new();
    for handle in handles {
        let draws = draws_for_handle(inspection, *handle);
        if let Some(rect) = projected_node_rect(capture, draws.as_slice()) {
            rects.push(rect);
        }
    }
    let rect = union_rects(rects.as_slice())?;
    shrink_region(
        clipped_region_from_rect(capture, rect)?,
        OCCLUDER_INTERIOR_INSET_PX,
    )
}

fn helper_srgb8_color(inspection: &SceneInspectionReportV1, handles: &[u64]) -> Option<[u8; 4]> {
    handles.iter().find_map(|handle| {
        draws_for_handle(inspection, *handle)
            .into_iter()
            .find_map(|draw| draw.material.as_ref().map(|material| material.base_color))
            .map(linear_rgba_to_srgb8)
    })
}

fn count_helper_pixels_in_region(
    capture: &CaptureRgba8,
    region: CaptureScreenRegion,
    helper_color: [u8; 4],
) -> u64 {
    let mut count = 0_u64;
    for y in region.y..region.y.saturating_add(region.height) {
        for x in region.x..region.x.saturating_add(region.width) {
            let offset = ((y as usize) * (capture.descriptor.width as usize) + (x as usize)) * 4;
            let Some(pixel) = capture.rgba8.get(offset..offset + 4) else {
                continue;
            };
            if color_matches(pixel, helper_color) {
                count = count.saturating_add(1);
            }
        }
    }
    count
}

fn color_matches(pixel: &[u8], helper_color: [u8; 4]) -> bool {
    pixel
        .iter()
        .zip(helper_color)
        .take(3)
        .all(|(sample, target)| sample.abs_diff(target) <= DEFAULT_HELPER_COLOR_TOLERANCE)
}

fn shrink_region(region: CaptureScreenRegion, inset: u32) -> Option<CaptureScreenRegion> {
    if region.width <= inset.saturating_mul(2) || region.height <= inset.saturating_mul(2) {
        return None;
    }
    Some(CaptureScreenRegion {
        x: region.x.saturating_add(inset),
        y: region.y.saturating_add(inset),
        width: region.width.saturating_sub(inset.saturating_mul(2)),
        height: region.height.saturating_sub(inset.saturating_mul(2)),
    })
}

fn union_rects(rects: &[crate::CaptureScreenRect]) -> Option<crate::CaptureScreenRect> {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for rect in rects {
        min_x = min_x.min(rect.min_x);
        min_y = min_y.min(rect.min_y);
        max_x = max_x.max(rect.max_x);
        max_y = max_y.max(rect.max_y);
    }
    if !min_x.is_finite() || !min_y.is_finite() || !max_x.is_finite() || !max_y.is_finite() {
        return None;
    }
    Some(crate::CaptureScreenRect {
        min_x,
        min_y,
        max_x,
        max_y,
        width: (max_x - min_x).max(0.0),
        height: (max_y - min_y).max(0.0),
        center_x: (min_x + max_x) * 0.5,
        center_y: (min_y + max_y) * 0.5,
    })
}

fn combined_handles(left: &[u64], right: &[u64]) -> Vec<u64> {
    let mut handles = left.to_vec();
    handles.extend(right);
    handles.sort_unstable();
    handles.dedup();
    handles
}

fn first_handle(handles: &[u64]) -> Option<u64> {
    handles.first().copied()
}
