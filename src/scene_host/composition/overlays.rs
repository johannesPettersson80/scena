use std::collections::BTreeMap;

use serde_json::json;

use super::checks::{CompositionCheckExt, checked_check, error_check, observed_pairs, round3};
use super::regions::{CompositionLabelRegion, CompositionOverlaySegmentV1};
use crate::{CaptureScreenRect, CaptureScreenRegion, SceneCompositionCheckV1, SceneRecipeBuildV1};

pub(super) fn overlay_line_regions_by_handle(
    segments: &[CompositionOverlaySegmentV1],
) -> BTreeMap<u64, CaptureScreenRegion> {
    let mut regions = BTreeMap::new();
    for segment in segments {
        let Some(handle) = segment.handle else {
            continue;
        };
        let Some(region) = segment.region.rect_css_px.as_ref().map(region_from_rect) else {
            continue;
        };
        regions
            .entry(handle)
            .and_modify(|existing| *existing = union_regions(*existing, region))
            .or_insert(region);
    }
    regions
}

pub(super) fn composition_overlay_collision_checks(
    manifest: &SceneRecipeBuildV1,
    label_regions: &BTreeMap<u64, CompositionLabelRegion>,
    overlay_segments: &[CompositionOverlaySegmentV1],
    viewport_width: u32,
    viewport_height: u32,
) -> Vec<SceneCompositionCheckV1> {
    let mut checks = Vec::new();
    let label_ids = label_ids_by_handle(manifest);
    for (label_handle, label_info) in label_regions {
        let label_region = label_info.clipped;
        let label_id = label_check_id(&label_ids, *label_handle);
        checks.push(label_viewport_fit_check(
            &label_ids,
            *label_handle,
            *label_info,
            viewport_width,
            viewport_height,
        ));
        let collisions = overlay_segments
            .iter()
            .filter(|segment| line_intersects_label(label_region, segment))
            .collect::<Vec<_>>();
        if let Some(segment) = collisions.first() {
            let mut affected = vec![*label_handle];
            if let Some(handle) = segment.handle {
                affected.push(handle);
            }
            affected.sort_unstable();
            affected.dedup();
            checks.push(
                error_check(
                    format!("overlay.label.{label_id}.line_clearance"),
                    "overlay",
                    "overlay_label_intersects_line",
                    None,
                    affected,
                    observed_pairs([
                        (
                            "label_rect",
                            json!({
                                "x": label_region.x,
                                "y": label_region.y,
                                "width": label_region.width,
                                "height": label_region.height,
                            }),
                        ),
                        ("line_handle", json!(segment.handle)),
                        ("line_start_css_px", json!(segment.start_css_px)),
                        ("line_end_css_px", json!(segment.end_css_px)),
                        ("collisions", json!(collisions.len())),
                    ]),
                    (
                        "projected line overlay crosses the interior of a label region",
                        "move the label, offset the line endpoint, or shorten the leader/dimension line before the label",
                    ),
                )
                .with_region_from_screen("label", Some(*label_handle), label_region),
            );
        } else {
            checks.push(
                checked_check(
                    format!("overlay.label.{label_id}.line_clearance"),
                    "overlay",
                    "overlay_label_clear_of_lines",
                    None,
                    vec![*label_handle],
                    observed_pairs([
                        (
                            "label_rect",
                            json!({
                                "x": label_region.x,
                                "y": label_region.y,
                                "width": label_region.width,
                                "height": label_region.height,
                            }),
                        ),
                        ("line_count", json!(overlay_segments.len())),
                    ]),
                    (
                        "label region is clear of projected line-overlay interiors",
                        "no action needed",
                    ),
                )
                .with_region_from_screen(
                    "label",
                    Some(*label_handle),
                    label_region,
                ),
            );
        }
        checks.push(label_label_clearance_check(
            &label_ids,
            *label_handle,
            label_region,
            label_regions,
        ));
    }
    checks
}

fn label_viewport_fit_check(
    label_ids: &BTreeMap<u64, String>,
    label_handle: u64,
    label_info: CompositionLabelRegion,
    viewport_width: u32,
    viewport_height: u32,
) -> SceneCompositionCheckV1 {
    let label_id = label_check_id(label_ids, label_handle);
    let inside = label_rect_inside_viewport(label_info.unclipped, viewport_width, viewport_height);
    let observed = observed_pairs([
        ("label_id", json!(label_id.clone())),
        (
            "clipped_rect",
            json!(screen_region_value(label_info.clipped)),
        ),
        (
            "projected_rect",
            json!(screen_rect_value(label_info.unclipped)),
        ),
        (
            "viewport",
            json!({
                "width": viewport_width,
                "height": viewport_height,
            }),
        ),
    ]);
    if inside {
        checked_check(
            format!("overlay.label.{label_id}.viewport_fit"),
            "overlay",
            "overlay_label_inside_viewport",
            None,
            vec![label_handle],
            observed,
            (
                "projected label overlay is fully inside the capture viewport",
                "no action needed",
            ),
        )
        .with_region_from_screen("label", Some(label_handle), label_info.clipped)
    } else {
        error_check(
            format!("overlay.label.{label_id}.viewport_fit"),
            "overlay",
            "overlay_label_clipped_by_viewport",
            None,
            vec![label_handle],
            observed,
            (
                "projected label overlay extends outside the capture viewport",
                "move the label, reduce label size, or frame with overlay-aware margins so the full label stays visible",
            ),
        )
        .with_region_from_screen("label", Some(label_handle), label_info.clipped)
    }
}

fn label_ids_by_handle(manifest: &SceneRecipeBuildV1) -> BTreeMap<u64, String> {
    manifest
        .nodes
        .iter()
        .filter(|node| node.kind == "label")
        .map(|node| (node.handle, node.id.clone()))
        .collect()
}

fn label_check_id(label_ids: &BTreeMap<u64, String>, handle: u64) -> String {
    label_ids
        .get(&handle)
        .cloned()
        .unwrap_or_else(|| handle.to_string())
}

fn label_label_clearance_check(
    label_ids: &BTreeMap<u64, String>,
    label_handle: u64,
    label_region: CaptureScreenRegion,
    label_regions: &BTreeMap<u64, CompositionLabelRegion>,
) -> SceneCompositionCheckV1 {
    let label_id = label_check_id(label_ids, label_handle);
    let first_collision = label_regions
        .iter()
        .filter(|(other_handle, _)| **other_handle != label_handle)
        .filter_map(|(other_handle, other_info)| {
            label_regions_overlap(label_region, other_info.clipped)
                .map(|intersection| (*other_handle, other_info.clipped, intersection))
        })
        .next();

    if let Some((other_handle, other_region, intersection)) = first_collision {
        let other_id = label_check_id(label_ids, other_handle);
        let mut affected = vec![label_handle, other_handle];
        affected.sort_unstable();
        affected.dedup();
        error_check(
            format!("overlay.label.{label_id}.label_clearance"),
            "overlay",
            "overlay_label_intersects_label",
            None,
            affected,
            observed_pairs([
                ("label_id", json!(label_id)),
                ("other_label_id", json!(other_id)),
                ("label_rect", json!(screen_region_value(label_region))),
                ("other_label_rect", json!(screen_region_value(other_region))),
                ("intersection_rect", json!(screen_region_value(intersection))),
                (
                    "intersection_area_px",
                    json!(intersection.width.saturating_mul(intersection.height)),
                ),
            ]),
            (
                "projected label overlay overlaps another label region",
                "move one label, reduce label size, or add explicit layout offsets so labels do not overlap",
            ),
        )
        .with_region_from_screen("label", Some(label_handle), label_region)
    } else {
        checked_check(
            format!("overlay.label.{label_id}.label_clearance"),
            "overlay",
            "overlay_label_clear_of_labels",
            None,
            vec![label_handle],
            observed_pairs([
                ("label_id", json!(label_id)),
                ("label_rect", json!(screen_region_value(label_region))),
                (
                    "other_label_count",
                    json!(label_regions.len().saturating_sub(1)),
                ),
            ]),
            (
                "label region is clear of other projected label regions",
                "no action needed",
            ),
        )
        .with_region_from_screen("label", Some(label_handle), label_region)
    }
}

fn screen_region_value(region: CaptureScreenRegion) -> serde_json::Value {
    json!({
        "x": region.x,
        "y": region.y,
        "width": region.width,
        "height": region.height,
    })
}

fn screen_rect_value(rect: CaptureScreenRect) -> serde_json::Value {
    json!({
        "min_x": round3(rect.min_x),
        "min_y": round3(rect.min_y),
        "max_x": round3(rect.max_x),
        "max_y": round3(rect.max_y),
        "width": round3(rect.width),
        "height": round3(rect.height),
    })
}

fn label_rect_inside_viewport(
    rect: CaptureScreenRect,
    viewport_width: u32,
    viewport_height: u32,
) -> bool {
    rect.min_x >= 0.0
        && rect.min_y >= 0.0
        && rect.max_x <= viewport_width as f32
        && rect.max_y <= viewport_height as f32
}

fn label_regions_overlap(
    a: CaptureScreenRegion,
    b: CaptureScreenRegion,
) -> Option<CaptureScreenRegion> {
    let (a_min_x, a_min_y, a_max_x, a_max_y) = inset_label_rect(a)?;
    let (b_min_x, b_min_y, b_max_x, b_max_y) = inset_label_rect(b)?;
    let min_x = a_min_x.max(b_min_x);
    let min_y = a_min_y.max(b_min_y);
    let max_x = a_max_x.min(b_max_x);
    let max_y = a_max_y.min(b_max_y);
    if max_x <= min_x + 1.0 || max_y <= min_y + 1.0 {
        return None;
    }
    let x = min_x.floor().max(0.0) as u32;
    let y = min_y.floor().max(0.0) as u32;
    let width = (max_x.ceil() as u32).saturating_sub(x).max(1);
    let height = (max_y.ceil() as u32).saturating_sub(y).max(1);
    Some(CaptureScreenRegion {
        x,
        y,
        width,
        height,
    })
}

fn region_from_rect(rect: &crate::RenderIntrospectionRectV1) -> CaptureScreenRegion {
    let min_x = rect.min_x.floor().max(0.0) as u32;
    let min_y = rect.min_y.floor().max(0.0) as u32;
    let max_x = rect.max_x.ceil().max(rect.min_x.floor()) as u32;
    let max_y = rect.max_y.ceil().max(rect.min_y.floor()) as u32;
    CaptureScreenRegion {
        x: min_x,
        y: min_y,
        width: max_x.saturating_sub(min_x).max(1),
        height: max_y.saturating_sub(min_y).max(1),
    }
}

fn union_regions(a: CaptureScreenRegion, b: CaptureScreenRegion) -> CaptureScreenRegion {
    let min_x = a.x.min(b.x);
    let min_y = a.y.min(b.y);
    let max_x = a.x.saturating_add(a.width).max(b.x.saturating_add(b.width));
    let max_y =
        a.y.saturating_add(a.height)
            .max(b.y.saturating_add(b.height));
    CaptureScreenRegion {
        x: min_x,
        y: min_y,
        width: max_x.saturating_sub(min_x),
        height: max_y.saturating_sub(min_y),
    }
}

fn line_intersects_label(
    label_region: CaptureScreenRegion,
    segment: &CompositionOverlaySegmentV1,
) -> bool {
    let Some((min_x, min_y, max_x, max_y)) = inset_label_rect(label_region) else {
        return false;
    };
    segment_intersects_rect(
        segment.start_css_px,
        segment.end_css_px,
        min_x,
        min_y,
        max_x,
        max_y,
    )
}

fn inset_label_rect(region: CaptureScreenRegion) -> Option<(f32, f32, f32, f32)> {
    if region.width < 3 || region.height < 3 {
        return None;
    }
    let width = region.width as f32;
    let height = region.height as f32;
    let inset_x = (width * 0.08).clamp(1.0, 4.0);
    let inset_y = (height * 0.12).clamp(1.0, 4.0);
    if width <= inset_x * 2.0 || height <= inset_y * 2.0 {
        return None;
    }
    Some((
        round3(region.x as f32 + inset_x),
        round3(region.y as f32 + inset_y),
        round3(region.x as f32 + width - inset_x),
        round3(region.y as f32 + height - inset_y),
    ))
}

fn segment_intersects_rect(
    start: [f32; 2],
    end: [f32; 2],
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
) -> bool {
    if point_inside_rect(start, min_x, min_y, max_x, max_y)
        || point_inside_rect(end, min_x, min_y, max_x, max_y)
    {
        return true;
    }

    let dx = end[0] - start[0];
    let dy = end[1] - start[1];
    let mut t0 = 0.0_f32;
    let mut t1 = 1.0_f32;
    clip_segment(-dx, start[0] - min_x, &mut t0, &mut t1)
        && clip_segment(dx, max_x - start[0], &mut t0, &mut t1)
        && clip_segment(-dy, start[1] - min_y, &mut t0, &mut t1)
        && clip_segment(dy, max_y - start[1], &mut t0, &mut t1)
        && t1 > t0
}

fn point_inside_rect(point: [f32; 2], min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> bool {
    point[0] > min_x && point[0] < max_x && point[1] > min_y && point[1] < max_y
}

fn clip_segment(p: f32, q: f32, t0: &mut f32, t1: &mut f32) -> bool {
    if p.abs() <= f32::EPSILON {
        return q >= 0.0;
    }
    let r = q / p;
    if p < 0.0 {
        if r > *t1 {
            return false;
        }
        if r > *t0 {
            *t0 = r;
        }
    } else {
        if r < *t0 {
            return false;
        }
        if r < *t1 {
            *t1 = r;
        }
    }
    true
}
