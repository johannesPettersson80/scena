use crate::capture::{CapturePixelBounds, CaptureRgba8};
use crate::scene::{SceneDrawInspectionV1, SceneNodeInspectionV1};

use super::sampling::ContentSample;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct TargetSample {
    pub(super) kind: &'static str,
    pub(super) region_bbox: Option<CapturePixelBounds>,
    pub(super) content: ContentSample,
}

impl TargetSample {
    pub(super) fn empty() -> Self {
        Self {
            kind: "empty",
            region_bbox: None,
            content: ContentSample::empty(),
        }
    }

    pub(super) fn node_bbox(
        capture: &CaptureRgba8,
        draw: &SceneDrawInspectionV1,
        background: [u8; 4],
        tolerance: u8,
    ) -> Self {
        let region_bbox = projected_draw_bounds(capture, draw);
        let content = region_bbox.map_or_else(ContentSample::empty, |bounds| {
            ContentSample::from_rgba8_bounds(
                capture.descriptor.width,
                capture.rgba8.as_slice(),
                background,
                tolerance,
                bounds,
            )
        });
        Self {
            kind: "node_bbox",
            region_bbox,
            content,
        }
    }

    pub(super) fn node_bounds(
        capture: &CaptureRgba8,
        node: &SceneNodeInspectionV1,
        background: [u8; 4],
        tolerance: u8,
    ) -> Self {
        let region_bbox = node.bounds.and_then(|bounds| {
            super::super::screen_bounds::projected_aabb_bounds(
                capture,
                bounds,
                node.world_transform,
            )
        });
        let content = region_bbox.map_or_else(ContentSample::empty, |bounds| {
            ContentSample::from_rgba8_bounds(
                capture.descriptor.width,
                capture.rgba8.as_slice(),
                background,
                tolerance,
                bounds,
            )
        });
        Self {
            kind: "node_bbox",
            region_bbox,
            content,
        }
    }
}

fn projected_draw_bounds(
    capture: &CaptureRgba8,
    draw: &SceneDrawInspectionV1,
) -> Option<CapturePixelBounds> {
    super::super::screen_bounds::projected_aabb_bounds(
        capture,
        draw.local_bounds,
        draw.world_transform,
    )
}
