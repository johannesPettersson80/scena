use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy)]
pub(super) struct RenderedNodeCoverage {
    pub(super) centroid_css_px: Option<[f32; 2]>,
    pub(super) coverage_px: u64,
}

pub(super) fn rendered_node_coverages(
    capture: &scena::CaptureRgba8,
    inspection: &scena::SceneInspectionReportV1,
) -> Vec<(u64, RenderedNodeCoverage)> {
    let parents = inspection
        .nodes
        .iter()
        .map(|node| (node.handle, node.parent))
        .collect::<BTreeMap<_, _>>();
    inspection
        .nodes
        .iter()
        .map(|node| {
            (
                node.handle,
                rendered_node_coverage(capture, inspection, &parents, node.handle),
            )
        })
        .collect()
}

pub(super) fn changed_node_coverage(
    capture: &scena::CaptureRgba8,
    inspection: &scena::SceneInspectionReportV1,
    reference: &scena::CaptureRgba8,
    handle: u64,
) -> RenderedNodeCoverage {
    if capture.descriptor.width != reference.descriptor.width
        || capture.descriptor.height != reference.descriptor.height
        || capture.rgba8.len() != reference.rgba8.len()
    {
        return RenderedNodeCoverage {
            centroid_css_px: None,
            coverage_px: 0,
        };
    }
    let parents = inspection
        .nodes
        .iter()
        .map(|node| (node.handle, node.parent))
        .collect::<BTreeMap<_, _>>();
    let Some(region) = projected_node_region(capture, inspection, &parents, handle) else {
        return RenderedNodeCoverage {
            centroid_css_px: None,
            coverage_px: 0,
        };
    };
    changed_pixel_centroid(capture, reference, region)
}

fn rendered_node_coverage(
    capture: &scena::CaptureRgba8,
    inspection: &scena::SceneInspectionReportV1,
    parents: &BTreeMap<u64, Option<u64>>,
    handle: u64,
) -> RenderedNodeCoverage {
    let Some(region) = projected_node_region(capture, inspection, parents, handle) else {
        return RenderedNodeCoverage {
            centroid_css_px: None,
            coverage_px: 0,
        };
    };
    foreground_centroid(capture, region)
}

fn projected_node_region(
    capture: &scena::CaptureRgba8,
    inspection: &scena::SceneInspectionReportV1,
    parents: &BTreeMap<u64, Option<u64>>,
    handle: u64,
) -> Option<scena::CaptureScreenRegion> {
    let mut rects = Vec::new();
    for draw in inspection
        .draw_list
        .iter()
        .filter(|draw| draw.node == handle || is_descendant_of(draw.node, handle, parents))
    {
        if let Some(rect) =
            scena::project_aabb_from_capture(capture, draw.local_bounds, draw.world_transform)
        {
            rects.push(rect);
        }
    }
    union_capture_rects(&rects).and_then(|rect| {
        scena::screen_region_from_rect(rect, capture.descriptor.width, capture.descriptor.height)
    })
}

fn is_descendant_of(mut node: u64, ancestor: u64, parents: &BTreeMap<u64, Option<u64>>) -> bool {
    while let Some(parent) = parents.get(&node).copied().flatten() {
        if parent == ancestor {
            return true;
        }
        node = parent;
    }
    false
}

fn changed_pixel_centroid(
    capture: &scena::CaptureRgba8,
    reference: &scena::CaptureRgba8,
    region: scena::CaptureScreenRegion,
) -> RenderedNodeCoverage {
    let mut coverage_px = 0_u64;
    let mut weighted_x = 0.0_f32;
    let mut weighted_y = 0.0_f32;
    for y in region.y..region.y.saturating_add(region.height) {
        for x in region.x..region.x.saturating_add(region.width) {
            let offset = ((y as usize) * (capture.descriptor.width as usize) + (x as usize)) * 4;
            let Some(pixel) = capture.rgba8.get(offset..offset + 4) else {
                continue;
            };
            let Some(reference_pixel) = reference.rgba8.get(offset..offset + 4) else {
                continue;
            };
            let delta = u16::from(pixel[0].abs_diff(reference_pixel[0]))
                + u16::from(pixel[1].abs_diff(reference_pixel[1]))
                + u16::from(pixel[2].abs_diff(reference_pixel[2]));
            if pixel[3] > 16 && delta > 2 {
                coverage_px = coverage_px.saturating_add(1);
                weighted_x += x as f32 + 0.5;
                weighted_y += y as f32 + 0.5;
            }
        }
    }
    let centroid_css_px = (coverage_px > 0).then(|| {
        [
            round3(weighted_x / coverage_px as f32),
            round3(weighted_y / coverage_px as f32),
        ]
    });
    RenderedNodeCoverage {
        centroid_css_px,
        coverage_px,
    }
}

fn union_capture_rects(rects: &[scena::CaptureScreenRect]) -> Option<scena::CaptureScreenRect> {
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
    Some(scena::CaptureScreenRect {
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

fn foreground_centroid(
    capture: &scena::CaptureRgba8,
    region: scena::CaptureScreenRegion,
) -> RenderedNodeCoverage {
    let background = capture
        .rgba8
        .get(0..4)
        .map(|pixel| [pixel[0], pixel[1], pixel[2]])
        .unwrap_or([0, 0, 0]);
    let mut coverage_px = 0_u64;
    let mut weighted_x = 0.0_f32;
    let mut weighted_y = 0.0_f32;
    for y in region.y..region.y.saturating_add(region.height) {
        for x in region.x..region.x.saturating_add(region.width) {
            let offset = ((y as usize) * (capture.descriptor.width as usize) + (x as usize)) * 4;
            let Some(pixel) = capture.rgba8.get(offset..offset + 4) else {
                continue;
            };
            let delta = u16::from(pixel[0].abs_diff(background[0]))
                + u16::from(pixel[1].abs_diff(background[1]))
                + u16::from(pixel[2].abs_diff(background[2]));
            if pixel[3] > 16 && delta > 18 {
                coverage_px = coverage_px.saturating_add(1);
                weighted_x += x as f32 + 0.5;
                weighted_y += y as f32 + 0.5;
            }
        }
    }
    let centroid_css_px = (coverage_px > 0).then(|| {
        [
            round3(weighted_x / coverage_px as f32),
            round3(weighted_y / coverage_px as f32),
        ]
    });
    RenderedNodeCoverage {
        centroid_css_px,
        coverage_px,
    }
}

fn round3(value: f32) -> f32 {
    if value.is_finite() {
        (value * 1000.0).round() / 1000.0
    } else {
        0.0
    }
}
