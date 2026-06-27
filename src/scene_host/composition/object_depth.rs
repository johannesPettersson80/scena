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

const DEFAULT_OBJECT_COLOR_TOLERANCE: u8 = 56;
const COLOR_PROBE_SEPARATION_MARGIN: u16 = (DEFAULT_OBJECT_COLOR_TOLERANCE as u16) * 2;
const FRONT_INTERIOR_INSET_PX: u32 = 6;

pub(super) fn composition_object_depth_order_checks(
    expect: Option<&SceneRecipeExpectV1>,
    manifest: &SceneRecipeBuildV1,
    inspection: &SceneInspectionReportV1,
    capture: &CaptureRgba8,
) -> Vec<SceneCompositionCheckV1> {
    let mut checks = Vec::new();
    let Some(expect) = expect else {
        return checks;
    };
    for expectation in &expect.expect_occlusion {
        let id = format!("expect_occlusion.{}", expectation.id);
        let target_id = Some(expectation.id.clone());
        let front_handles = resolve_target_handles(&expectation.front, manifest);
        let back_handles = resolve_target_handles(&expectation.back, manifest);
        let affected = combined_handles(&front_handles, &back_handles);
        if front_handles.is_empty() || back_handles.is_empty() {
            checks.push(error_check(
                id,
                "occlusion_depth",
                "object_depth_order_target_unresolved",
                target_id,
                affected,
                observed_pairs([
                    ("front_handles", json!(front_handles)),
                    ("back_handles", json!(back_handles)),
                ]),
                (
                    "object depth-order expectation did not resolve both front and back targets",
                    "fix the front/back target ids or remove the stale occlusion expectation",
                ),
            ));
            continue;
        }

        let Some(region) = front_interior_region(capture, inspection, &front_handles) else {
            checks.push(error_check(
                id,
                "occlusion_depth",
                "object_depth_order_region_unavailable",
                target_id,
                affected,
                observed_pairs([
                    ("front_handles", json!(front_handles)),
                    ("back_handles", json!(back_handles)),
                ]),
                (
                    "object depth-order expectation could not derive a stable front-object screen region",
                    "keep the expected front object visible and large enough in frame or remove the expectation",
                ),
            ));
            continue;
        };

        let Some(front_color) = object_srgb8_color(inspection, &front_handles) else {
            checks.push(error_check(
                id,
                "occlusion_depth",
                "object_depth_order_color_unavailable",
                target_id,
                affected,
                observed_pairs([("front_handles", json!(front_handles))]),
                (
                    "object depth-order expectation could not determine the front object's draw color",
                    "use a stable material base color on the expected front object",
                ),
            ));
            continue;
        };

        let Some(back_color) = object_srgb8_color(inspection, &back_handles) else {
            checks.push(error_check(
                id,
                "occlusion_depth",
                "object_depth_order_color_unavailable",
                target_id,
                affected,
                observed_pairs([
                    ("front_handles", json!(front_handles)),
                    ("back_handles", json!(back_handles)),
                    (
                        "front_srgb8",
                        json!([front_color[0], front_color[1], front_color[2]]),
                    ),
                ]),
                (
                    "object depth-order expectation could not determine the back object's draw color",
                    "use a stable material base color on the expected back object",
                ),
            ));
            continue;
        };

        if !colors_are_separable(front_color, back_color) {
            checks.push(
                error_check(
                    id,
                    "occlusion_depth",
                    "object_depth_order_color_ambiguous",
                    target_id,
                    affected,
                    observed_pairs([
                        ("front_handles", json!(front_handles)),
                        ("back_handles", json!(back_handles)),
                        (
                            "front_srgb8",
                            json!([front_color[0], front_color[1], front_color[2]]),
                        ),
                        (
                            "back_srgb8",
                            json!([back_color[0], back_color[1], back_color[2]]),
                        ),
                        (
                            "required_channel_separation",
                            json!(COLOR_PROBE_SEPARATION_MARGIN + 1),
                        ),
                    ]),
                    (
                        "object depth-order expectation uses front/back colours that cannot be separated by the color-probe verifier",
                        "use visually distinct opaque colors for the expected front and back objects, or wait for the exact object-mask verifier before relying on this occlusion expectation",
                    ),
                )
                .with_region_from_screen("node", first_handle(&front_handles), region),
            );
            continue;
        }

        let back_pixels = count_color_pixels_in_region(capture, region, back_color);
        let tolerance = u64::from(expectation.tolerance_pixels.unwrap_or(0));
        let observed = observed_pairs([
            ("front_handles", json!(front_handles)),
            ("back_handles", json!(back_handles)),
            (
                "back_srgb8",
                json!([back_color[0], back_color[1], back_color[2]]),
            ),
            ("method", json!("color_probe")),
            ("back_pixels_inside_front", json!(back_pixels)),
            ("tolerance_pixels", json!(tolerance)),
            (
                "front_interior_region",
                json!({
                    "x": region.x,
                    "y": region.y,
                    "width": region.width,
                    "height": region.height
                }),
            ),
        ]);
        if back_pixels <= tolerance {
            checks.push(
                checked_check(
                    id,
                    "occlusion_depth",
                    "object_depth_order_satisfied",
                    target_id,
                    affected,
                    observed,
                    (
                        "back-object colored pixels are absent from the expected front-object interior",
                        "no action needed",
                    ),
                )
                .with_region_from_screen("node", first_handle(&front_handles), region),
            );
        } else {
            checks.push(
                error_check(
                    id,
                    "occlusion_depth",
                    "object_depth_order_mismatch",
                    target_id,
                    affected,
                    observed,
                    (
                        "back-object colored pixels appear inside the expected front-object interior",
                        "fix the object transforms/depth ordering, make the intended occluder opaque, or update the expectation if the overlap is intentional",
                    ),
                )
                .with_region_from_screen("node", first_handle(&front_handles), region),
            );
        }
    }
    checks
}

fn front_interior_region(
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
        FRONT_INTERIOR_INSET_PX,
    )
}

fn object_srgb8_color(inspection: &SceneInspectionReportV1, handles: &[u64]) -> Option<[u8; 4]> {
    handles.iter().find_map(|handle| {
        draws_for_handle(inspection, *handle)
            .into_iter()
            .find_map(|draw| draw.material.as_ref().map(|material| material.base_color))
            .map(linear_rgba_to_srgb8)
    })
}

fn count_color_pixels_in_region(
    capture: &CaptureRgba8,
    region: CaptureScreenRegion,
    color: [u8; 4],
) -> u64 {
    let mut count = 0_u64;
    for y in region.y..region.y.saturating_add(region.height) {
        for x in region.x..region.x.saturating_add(region.width) {
            let offset = ((y as usize) * (capture.descriptor.width as usize) + (x as usize)) * 4;
            let Some(pixel) = capture.rgba8.get(offset..offset + 4) else {
                continue;
            };
            if color_matches(pixel, color) {
                count = count.saturating_add(1);
            }
        }
    }
    count
}

fn color_matches(pixel: &[u8], color: [u8; 4]) -> bool {
    pixel
        .iter()
        .zip(color)
        .take(3)
        .all(|(sample, target)| sample.abs_diff(target) <= DEFAULT_OBJECT_COLOR_TOLERANCE)
}

fn colors_are_separable(front: [u8; 4], back: [u8; 4]) -> bool {
    (0..3).any(|channel| {
        u16::from(front[channel].abs_diff(back[channel])) > COLOR_PROBE_SEPARATION_MARGIN
    })
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
