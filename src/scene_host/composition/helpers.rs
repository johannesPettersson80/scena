use std::collections::BTreeSet;

use crate::geometry::{Aabb, GeometryTopology};
use crate::scene::view_math::transform_aabb;
use crate::{
    CaptureRgba8, CaptureScreenRect, CaptureScreenRegion, Color, SceneDrawInspectionV1,
    SceneInspectionReportV1, SceneRecipeBuildV1, SceneRecipeExpectV1, SceneRecipeTargetV1,
    project_aabb_from_capture,
};

use super::checks::round3;

const VISIBLE_COVERAGE_BACKGROUND_TOLERANCE_RGBA8: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct VisibleCoverage {
    pub(super) region: CaptureScreenRegion,
    pub(super) region_pixels: u64,
    pub(super) foreground_pixels: u64,
    pub(super) foreground_fraction: f32,
}

pub(super) fn declared_draw_handles(manifest: &SceneRecipeBuildV1) -> BTreeSet<u64> {
    let mut handles = BTreeSet::new();
    for node in &manifest.nodes {
        if matches!(
            node.kind.as_str(),
            "node" | "instance_set" | "particle_set" | "label"
        ) {
            handles.insert(node.handle);
        }
    }
    for import in &manifest.imports {
        handles.extend(import.nodes_by_path.values().copied());
    }
    handles
}

pub(super) fn draws_for_handle(
    inspection: &SceneInspectionReportV1,
    handle: u64,
) -> Vec<&SceneDrawInspectionV1> {
    inspection
        .draw_list
        .iter()
        .filter(|draw| draw.node == handle)
        .collect()
}

pub(super) fn draws_for_handles<'a>(
    inspection: &'a SceneInspectionReportV1,
    handles: &BTreeSet<u64>,
) -> Vec<&'a SceneDrawInspectionV1> {
    inspection
        .draw_list
        .iter()
        .filter(|draw| handles.contains(&draw.node))
        .collect()
}

pub(super) fn projected_node_rect(
    capture: &CaptureRgba8,
    draws: &[&SceneDrawInspectionV1],
) -> Option<CaptureScreenRect> {
    let mut rects = Vec::new();
    for draw in draws {
        if let Some(rect) =
            project_aabb_from_capture(capture, draw.local_bounds, draw.world_transform)
        {
            rects.push(rect);
        }
    }
    union_rects(rects.as_slice())
}

pub(super) fn world_bounds_for_handle(
    inspection: &SceneInspectionReportV1,
    handle: u64,
) -> Option<Aabb> {
    let draws = draws_for_handle(inspection, handle);
    let mut bounds: Option<Aabb> = None;
    for draw in draws {
        let draw_bounds = transform_aabb(draw.local_bounds, draw.world_transform);
        bounds = Some(match bounds {
            Some(bounds) => bounds.union(draw_bounds),
            None => draw_bounds,
        });
    }
    bounds
}

pub(super) fn visible_coverage_for_rect(
    capture: &CaptureRgba8,
    rect: CaptureScreenRect,
    background: Color,
) -> Option<VisibleCoverage> {
    let region = clipped_region_from_rect(capture, rect)?;
    Some(visible_coverage_for_region(capture, region, background))
}

pub(super) fn visible_coverage_for_region(
    capture: &CaptureRgba8,
    region: CaptureScreenRegion,
    background: Color,
) -> VisibleCoverage {
    let background = linear_rgba_to_srgb8(background);
    let mut foreground_pixels = 0_u64;
    for y in region.y..region.y.saturating_add(region.height) {
        for x in region.x..region.x.saturating_add(region.width) {
            let offset = ((y as usize) * (capture.descriptor.width as usize) + (x as usize)) * 4;
            let Some(pixel) = capture.rgba8.get(offset..offset + 4) else {
                continue;
            };
            if pixel_differs_from_background(
                pixel,
                background,
                VISIBLE_COVERAGE_BACKGROUND_TOLERANCE_RGBA8,
            ) {
                foreground_pixels = foreground_pixels.saturating_add(1);
            }
        }
    }
    let region_pixels = u64::from(region.width).saturating_mul(u64::from(region.height));
    VisibleCoverage {
        region,
        region_pixels,
        foreground_pixels,
        foreground_fraction: fraction(foreground_pixels, region_pixels),
    }
}

pub(super) fn clipped_region_from_rect(
    capture: &CaptureRgba8,
    rect: CaptureScreenRect,
) -> Option<CaptureScreenRegion> {
    if !rect.min_x.is_finite()
        || !rect.min_y.is_finite()
        || !rect.max_x.is_finite()
        || !rect.max_y.is_finite()
        || capture.descriptor.width == 0
        || capture.descriptor.height == 0
    {
        return None;
    }
    let min_x = rect.min_x.floor().max(0.0);
    let min_y = rect.min_y.floor().max(0.0);
    let max_x = rect.max_x.ceil().min(capture.descriptor.width as f32);
    let max_y = rect.max_y.ceil().min(capture.descriptor.height as f32);
    if max_x <= min_x || max_y <= min_y {
        return None;
    }
    let x = min_x as u32;
    let y = min_y as u32;
    let max_x = max_x as u32;
    let max_y = max_y as u32;
    let width = max_x.saturating_sub(x);
    let height = max_y.saturating_sub(y);
    (width > 0 && height > 0).then_some(CaptureScreenRegion {
        x,
        y,
        width,
        height,
    })
}

pub(super) fn linear_rgba_to_srgb8(color: Color) -> [u8; 4] {
    [
        linear_channel_to_srgb_u8(color.r),
        linear_channel_to_srgb_u8(color.g),
        linear_channel_to_srgb_u8(color.b),
        (color.a.clamp(0.0, 1.0) * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

fn linear_channel_to_srgb_u8(value: f32) -> u8 {
    let value = value.clamp(0.0, 1.0);
    let encoded = if value <= 0.003_130_8 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0).round().clamp(0.0, 255.0) as u8
}

fn pixel_differs_from_background(pixel: &[u8], background: [u8; 4], tolerance: u8) -> bool {
    (0..3).any(|channel| pixel[channel].abs_diff(background[channel]) > tolerance)
}

fn fraction(numerator: u64, denominator: u64) -> f32 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f32 / denominator as f32
    }
}

fn union_rects(rects: &[CaptureScreenRect]) -> Option<CaptureScreenRect> {
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
    let width = (max_x - min_x).max(0.0);
    let height = (max_y - min_y).max(0.0);
    Some(CaptureScreenRect {
        min_x: round3(min_x),
        min_y: round3(min_y),
        max_x: round3(max_x),
        max_y: round3(max_y),
        width: round3(width),
        height: round3(height),
        center_x: round3((min_x + max_x) * 0.5),
        center_y: round3((min_y + max_y) * 0.5),
    })
}

pub(super) fn expected_color_handles(
    expect: Option<&SceneRecipeExpectV1>,
    manifest: &SceneRecipeBuildV1,
) -> BTreeSet<u64> {
    let mut handles = BTreeSet::new();
    let Some(expect) = expect else {
        return handles;
    };
    for color in &expect.expect_color {
        handles.extend(resolve_target_handles(&color.target, manifest));
    }
    handles
}

pub(super) fn resolve_target_handles(
    target: &SceneRecipeTargetV1,
    manifest: &SceneRecipeBuildV1,
) -> Vec<u64> {
    match target {
        SceneRecipeTargetV1::Node { id } => manifest
            .nodes
            .iter()
            .find(|node| node.id == *id)
            .map(|node| vec![node.handle])
            .unwrap_or_default(),
        SceneRecipeTargetV1::Import { id } => manifest
            .imports
            .iter()
            .find(|import| import.id == *id)
            .map(|import| {
                let mut handles = import.root_handles.clone();
                handles.extend(import.nodes_by_path.values().copied());
                handles.sort_unstable();
                handles.dedup();
                handles
            })
            .unwrap_or_default(),
        SceneRecipeTargetV1::World { .. } => Vec::new(),
    }
}

pub(super) fn unexpected_draw_handles(
    inspection: &SceneInspectionReportV1,
    owned_handles: &BTreeSet<u64>,
) -> Vec<u64> {
    let mut extras = inspection
        .draw_list
        .iter()
        .filter_map(|draw| (!owned_handles.contains(&draw.node)).then_some(draw.node))
        .collect::<Vec<_>>();
    extras.sort_unstable();
    extras.dedup();
    extras
}

pub(super) fn ground_candidate_handles(inspection: &SceneInspectionReportV1) -> Vec<u64> {
    inspection
        .draw_list
        .iter()
        .filter_map(|draw| {
            if draw.topology == GeometryTopology::Lines
                || (draw.local_bounds.max.y - draw.local_bounds.min.y).abs() <= f32::EPSILON
            {
                Some(draw.node)
            } else {
                None
            }
        })
        .collect()
}
