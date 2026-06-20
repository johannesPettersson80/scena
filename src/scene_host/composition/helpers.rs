use std::collections::BTreeSet;

use crate::geometry::GeometryTopology;
use crate::{
    CaptureRgba8, CaptureScreenRect, SceneDrawInspectionV1, SceneInspectionReportV1,
    SceneRecipeBuildV1, SceneRecipeExpectV1, SceneRecipeTargetV1, project_aabb_from_capture,
};

use super::checks::round3;

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

fn resolve_target_handles(target: &SceneRecipeTargetV1, manifest: &SceneRecipeBuildV1) -> Vec<u64> {
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
