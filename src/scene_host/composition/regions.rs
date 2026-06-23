use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::SceneHostCore;
use super::checks::{region_from_screen, round3};
use crate::geometry::GeometryTopology;
use crate::{
    AssetFetcher, CaptureRgba8, CaptureScreenRect, CaptureScreenRegion, SceneCompositionRegionV1,
    screen_region_from_points, screen_region_from_rect, transform_point_for_projection,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositionOverlaySegmentV1 {
    pub handle: Option<u64>,
    pub start_css_px: [f32; 2],
    pub end_css_px: [f32; 2],
    pub region: SceneCompositionRegionV1,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct CompositionLabelRegion {
    pub(super) clipped: CaptureScreenRegion,
    pub(super) unclipped: CaptureScreenRect,
}

impl<F: AssetFetcher> SceneHostCore<F> {
    pub(super) fn composition_label_region_infos(
        &self,
        capture: &CaptureRgba8,
    ) -> BTreeMap<u64, CompositionLabelRegion> {
        let mut regions = BTreeMap::new();
        for (node, _label_key, label, transform) in self.scene.label_nodes() {
            if !self.scene.visible_for_active_camera(node) {
                continue;
            }
            let Some(projected) =
                crate::project_world_point_from_capture(capture, transform.translation)
            else {
                continue;
            };
            let metrics = label.metrics();
            let padding = (label.size() * 0.25).ceil().max(2.0);
            let Some(unclipped) = label_screen_rect_from_center_size(
                projected.x,
                projected.y,
                metrics.width_px,
                metrics.height_px,
                padding,
            ) else {
                continue;
            };
            let Some(clipped) = screen_region_from_rect(
                unclipped,
                capture.descriptor.width,
                capture.descriptor.height,
            ) else {
                continue;
            };
            if let Some(handle) = self.node_handle_map.get(&node).copied() {
                regions.insert(handle, CompositionLabelRegion { clipped, unclipped });
            }
        }
        regions
    }

    pub(super) fn composition_overlay_segments(
        &self,
        capture: &CaptureRgba8,
    ) -> Vec<CompositionOverlaySegmentV1> {
        let mut segments = Vec::new();
        for (node, mesh, transform) in self.scene.mesh_nodes() {
            if !self.scene.visible_for_active_camera(node) {
                continue;
            }
            let Some(geometry) = self.assets.geometry(mesh.geometry()) else {
                continue;
            };
            if geometry.topology() != GeometryTopology::Lines {
                continue;
            }
            let world = self.scene.world_transform(node).unwrap_or(transform);
            let vertices = geometry.vertices();
            let padding = self
                .assets
                .material(mesh.material())
                .and_then(|material| material.stroke_width_px())
                .unwrap_or(1.0)
                .ceil()
                .max(2.0);
            for segment in geometry.indices().chunks_exact(2) {
                let Some(start) = vertices.get(segment[0] as usize) else {
                    continue;
                };
                let Some(end) = vertices.get(segment[1] as usize) else {
                    continue;
                };
                let start = transform_point_for_projection(world, start.position);
                let end = transform_point_for_projection(world, end.position);
                let Some(start) = crate::project_world_point_from_capture(capture, start) else {
                    continue;
                };
                let Some(end) = crate::project_world_point_from_capture(capture, end) else {
                    continue;
                };
                let Some(region) = screen_region_from_points(
                    &[(start.x, start.y), (end.x, end.y)],
                    padding,
                    capture.descriptor.width,
                    capture.descriptor.height,
                ) else {
                    continue;
                };
                segments.push(CompositionOverlaySegmentV1 {
                    handle: self.node_handle_map.get(&node).copied(),
                    start_css_px: [round3(start.x), round3(start.y)],
                    end_css_px: [round3(end.x), round3(end.y)],
                    region: region_from_screen(
                        "line",
                        self.node_handle_map.get(&node).copied(),
                        region,
                    ),
                });
            }
        }
        segments
    }
}

fn label_screen_rect_from_center_size(
    center_x: f32,
    center_y: f32,
    width_px: f32,
    height_px: f32,
    padding_px: f32,
) -> Option<CaptureScreenRect> {
    if !center_x.is_finite()
        || !center_y.is_finite()
        || !width_px.is_finite()
        || !height_px.is_finite()
        || !padding_px.is_finite()
    {
        return None;
    }
    let half_width = (width_px * 0.5).max(0.0);
    let half_height = (height_px * 0.5).max(0.0);
    let min_x = center_x - half_width - padding_px;
    let min_y = center_y - half_height - padding_px;
    let max_x = center_x + half_width + padding_px;
    let max_y = center_y + half_height + padding_px;
    if !min_x.is_finite() || !min_y.is_finite() || !max_x.is_finite() || !max_y.is_finite() {
        return None;
    }
    Some(CaptureScreenRect {
        min_x: round3(min_x),
        min_y: round3(min_y),
        max_x: round3(max_x),
        max_y: round3(max_y),
        width: round3((max_x - min_x).max(0.0)),
        height: round3((max_y - min_y).max(0.0)),
        center_x: round3((min_x + max_x) * 0.5),
        center_y: round3((min_y + max_y) * 0.5),
    })
}
