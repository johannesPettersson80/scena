use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::SceneHostCore;
mod checks;
mod helpers;

use crate::geometry::GeometryTopology;
use crate::{
    AssetFetcher, CaptureRgba8, CaptureScreenRegion, SceneCompositionRegionV1,
    SceneCompositionReportV1, SceneCompositionStatusV1, SceneInspectionReportV1,
    SceneRecipeBuildV1, SceneRecipeExpectV1, SceneRecipeV1, screen_region_from_center_size,
    screen_region_from_points, transform_point_for_projection,
};
use checks::{
    CompositionCheckExt, checked_check, error_check, observed_pairs, region_from_screen, round3,
    skip_check,
};
use helpers::{
    declared_draw_handles, draws_for_handle, expected_color_handles, ground_candidate_handles,
    projected_node_rect, unexpected_draw_handles,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositionOverlaySegmentV1 {
    pub handle: Option<u64>,
    pub start_css_px: [f32; 2],
    pub end_css_px: [f32; 2],
    pub region: SceneCompositionRegionV1,
}

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn composition_report(
        &self,
        recipe: &SceneRecipeV1,
        manifest: &SceneRecipeBuildV1,
        capture: &CaptureRgba8,
        inspection: &SceneInspectionReportV1,
        expect: Option<&SceneRecipeExpectV1>,
    ) -> SceneCompositionReportV1 {
        let mut checks = Vec::new();
        let explicit_color_handles = expected_color_handles(expect, manifest);
        let label_regions = self.composition_label_regions(capture);
        let overlay_segments = self.composition_overlay_segments(capture);
        let mut owned_handles = declared_draw_handles(manifest);
        owned_handles.extend(label_regions.keys().copied());
        owned_handles.extend(overlay_segments.iter().filter_map(|segment| segment.handle));
        if recipe
            .scene
            .as_ref()
            .and_then(|scene| scene.grid.as_ref())
            .is_some()
        {
            owned_handles.extend(ground_candidate_handles(inspection));
        }

        for target in &manifest.nodes {
            if !matches!(
                target.kind.as_str(),
                "node" | "instance_set" | "particle_set" | "label"
            ) {
                continue;
            }
            let target_path = format!("node.{}", target.id);
            let Some(node) = inspection.node_by_handle(target.handle) else {
                checks.push(error_check(
                    format!("{target_path}.presence"),
                    "completeness",
                    "declared_node_missing",
                    Some(target.id.clone()),
                    vec![target.handle],
                    BTreeMap::new(),
                    (
                        "declared recipe node is absent from the inspection report",
                        "rebuild the recipe or remove the stale manifest target",
                    ),
                ));
                continue;
            };

            let draws = draws_for_handle(inspection, target.handle);
            let label_region = (target.kind == "label")
                .then(|| label_regions.get(&target.handle).copied())
                .flatten();
            let has_draw = !draws.is_empty()
                || (target.kind == "label" && label_regions.contains_key(&target.handle));
            if node.visible && has_draw {
                checks.push(checked_check(
                    format!("{target_path}.presence"),
                    "completeness",
                    "node_visible",
                    Some(target.id.clone()),
                    vec![target.handle],
                    observed_pairs([
                        ("visible", json!(true)),
                        ("draw_count", json!(draws.len())),
                        ("kind", json!(target.kind)),
                    ]),
                    (
                        "declared node is visible and contributes owned output",
                        "no action needed",
                    ),
                ));
            } else {
                checks.push(error_check(
                    format!("{target_path}.presence"),
                    "completeness",
                    "declared_node_not_drawn",
                    Some(target.id.clone()),
                    vec![target.handle],
                    observed_pairs([
                        ("visible", json!(node.visible)),
                        ("draw_count", json!(draws.len())),
                        ("kind", json!(target.kind)),
                    ]),
                    (
                        "declared recipe node did not produce visible draw output",
                        "make the node visible, move it into frame, or remove it from the declared recipe",
                    ),
                ));
            }

            if let Some(rect) = projected_node_rect(capture, draws.as_slice()) {
                checks.push(
                    checked_check(
                        format!("{target_path}.projected_bbox"),
                        "placement",
                        "projected_bbox_available",
                        Some(target.id.clone()),
                        vec![target.handle],
                        observed_pairs([
                            ("width_px", json!(round3(rect.width))),
                            ("height_px", json!(round3(rect.height))),
                            ("center_x_px", json!(round3(rect.center_x))),
                            ("center_y_px", json!(round3(rect.center_y))),
                        ]),
                        (
                            "declared node has a projected screen region",
                            "no action needed",
                        ),
                    )
                    .with_region("node", Some(target.handle), Some(rect)),
                );
                checks.push(checked_check(
                    format!("{target_path}.size"),
                    "framing",
                    "projected_size_nonzero",
                    Some(target.id.clone()),
                    vec![target.handle],
                    observed_pairs([
                        ("width_px", json!(round3(rect.width))),
                        ("height_px", json!(round3(rect.height))),
                    ]),
                    (
                        "declared node projects to a non-empty screen size",
                        "no action needed",
                    ),
                ));
            } else if let Some(region) = label_region {
                checks.push(
                    checked_check(
                        format!("{target_path}.projected_bbox"),
                        "placement",
                        "projected_bbox_available",
                        Some(target.id.clone()),
                        vec![target.handle],
                        observed_pairs([
                            ("width_px", json!(region.width)),
                            ("height_px", json!(region.height)),
                            (
                                "center_x_px",
                                json!(round3(region.x as f32 + region.width as f32 * 0.5)),
                            ),
                            (
                                "center_y_px",
                                json!(round3(region.y as f32 + region.height as f32 * 0.5)),
                            ),
                        ]),
                        (
                            "declared label has an exact projected screen region",
                            "no action needed",
                        ),
                    )
                    .with_region_from_screen(
                        "label",
                        Some(target.handle),
                        region,
                    ),
                );
                checks.push(checked_check(
                    format!("{target_path}.size"),
                    "framing",
                    "projected_size_nonzero",
                    Some(target.id.clone()),
                    vec![target.handle],
                    observed_pairs([
                        ("width_px", json!(region.width)),
                        ("height_px", json!(region.height)),
                    ]),
                    (
                        "declared label projects to a non-empty screen size",
                        "no action needed",
                    ),
                ));
            } else if has_draw {
                checks.push(error_check(
                    format!("{target_path}.projected_bbox"),
                    "placement",
                    "projected_bbox_missing",
                    Some(target.id.clone()),
                    vec![target.handle],
                    BTreeMap::new(),
                    (
                        "declared node draws but its projected screen bounds could not be computed",
                        "check camera metadata and node bounds",
                    ),
                ));
            } else {
                checks.push(skip_check(
                    format!("{target_path}.projected_bbox"),
                    "placement",
                    "projected_bbox_not_applicable",
                    SceneCompositionStatusV1::NotApplicable,
                    Some(target.id.clone()),
                    vec![target.handle],
                    (
                        "projected bounds are not applicable because the node did not draw",
                        "fix the presence failure first",
                    ),
                ));
            }

            if explicit_color_handles.contains(&target.handle) {
                checks.push(checked_check(
                    format!("{target_path}.expected_color"),
                    "color_exposure",
                    "expected_color_declared",
                    Some(target.id.clone()),
                    vec![target.handle],
                    BTreeMap::new(),
                    (
                        "recipe declares an expected color for this target; appearance verification owns the pixel assertion",
                        "inspect the appearance report if this color check fails",
                    ),
                ));
            } else {
                checks.push(skip_check(
                    format!("{target_path}.expected_color"),
                    "color_exposure",
                    "expected_color_not_declared",
                    SceneCompositionStatusV1::SkippedNoDeclaredIntent,
                    Some(target.id.clone()),
                    vec![target.handle],
                    (
                        "recipe did not declare a color intent for this node",
                        "add expect_color for nodes whose color matters",
                    ),
                ));
            }

            checks.push(skip_check(
                format!("{target_path}.visible_coverage"),
                "occlusion_depth",
                "object_mask_not_available",
                SceneCompositionStatusV1::SkippedNoBackendSupport,
                Some(target.id.clone()),
                vec![target.handle],
                (
                    "per-object visible-pixel masks are not available in this foundation layer",
                    "add a depth/id-mask backend before treating occlusion coverage as checked",
                ),
            ));
            checks.push(skip_check(
                format!("{target_path}.grounding"),
                "occlusion_depth",
                "grounding_intent_not_declared",
                SceneCompositionStatusV1::SkippedNoDeclaredIntent,
                Some(target.id.clone()),
                vec![target.handle],
                (
                    "recipe did not declare that this node should be grounded",
                    "add a grounding intent before expecting contact or ground-plane checks",
                ),
            ));
        }

        for (handle, region) in label_regions {
            checks.push(
                checked_check(
                    format!("overlay.label.{handle}.rect"),
                    "overlay",
                    "label_rect_projected",
                    None,
                    vec![handle],
                    observed_pairs([
                        ("x", json!(region.x)),
                        ("y", json!(region.y)),
                        ("width", json!(region.width)),
                        ("height", json!(region.height)),
                    ]),
                    (
                        "label overlay has an exact projected region",
                        "no action needed",
                    ),
                )
                .with_region_from_screen("label", Some(handle), region),
            );
        }
        for (index, segment) in overlay_segments.into_iter().enumerate() {
            checks.push(
                checked_check(
                    format!("overlay.line.{index}.segment"),
                    "overlay",
                    "line_segment_projected",
                    None,
                    segment.handle.into_iter().collect(),
                    observed_pairs([
                        ("start_css_px", json!(segment.start_css_px)),
                        ("end_css_px", json!(segment.end_css_px)),
                    ]),
                    (
                        "line overlay has exact projected segment endpoints",
                        "no action needed",
                    ),
                )
                .with_region_value(segment.region),
            );
        }

        let extra_draws = unexpected_draw_handles(inspection, &owned_handles);
        if extra_draws.is_empty() {
            checks.push(checked_check(
                "draw_output.extra".to_owned(),
                "completeness",
                "no_unowned_draw_output",
                None,
                Vec::new(),
                observed_pairs([("extra_draw_handles", json!([]))]),
                (
                    "rendered draw output is owned by declared recipe elements or generated overlays",
                    "no action needed",
                ),
            ));
        } else {
            checks.push(error_check(
                "draw_output.extra".to_owned(),
                "completeness",
                "unexpected_draw_output",
                None,
                extra_draws.clone(),
                observed_pairs([("extra_draw_handles", json!(extra_draws))]),
                (
                    "rendered draw output is not owned by the recipe manifest or generated overlay graph",
                    "remove stale scene content or add explicit ownership for generated content",
                ),
            ));
        }

        SceneCompositionReportV1::new(checks)
    }

    fn composition_label_regions(
        &self,
        capture: &CaptureRgba8,
    ) -> BTreeMap<u64, CaptureScreenRegion> {
        let width = capture.descriptor.width;
        let height = capture.descriptor.height;
        let mut regions = BTreeMap::new();
        if width == 0 || height == 0 {
            return regions;
        }
        for (node, _label_key, label, transform) in self.scene.label_nodes() {
            if !self.scene.visible_for_active_camera(node) {
                continue;
            }
            let Ok(Some(projected)) = self.scene.project_world_point(
                self.active_camera,
                transform.translation,
                width,
                height,
            ) else {
                continue;
            };
            let metrics = label.metrics();
            let padding = (label.size() * 0.25).ceil().max(2.0);
            let Some(region) = screen_region_from_center_size(
                projected.x,
                projected.y,
                metrics.width_px,
                metrics.height_px,
                padding,
                width,
                height,
            ) else {
                continue;
            };
            if let Some(handle) = self.node_handle_map.get(&node).copied() {
                regions.insert(handle, region);
            }
        }
        regions
    }

    fn composition_overlay_segments(
        &self,
        capture: &CaptureRgba8,
    ) -> Vec<CompositionOverlaySegmentV1> {
        let width = capture.descriptor.width;
        let height = capture.descriptor.height;
        let mut segments = Vec::new();
        if width == 0 || height == 0 {
            return segments;
        }
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
            let stroke_padding = self
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
                let Ok(Some(start)) =
                    self.scene
                        .project_world_point(self.active_camera, start, width, height)
                else {
                    continue;
                };
                let Ok(Some(end)) =
                    self.scene
                        .project_world_point(self.active_camera, end, width, height)
                else {
                    continue;
                };
                let Some(region) = screen_region_from_points(
                    &[(start.x, start.y), (end.x, end.y)],
                    stroke_padding,
                    width,
                    height,
                ) else {
                    continue;
                };
                let handle = self.node_handle_map.get(&node).copied();
                segments.push(CompositionOverlaySegmentV1 {
                    handle,
                    start_css_px: [round3(start.x), round3(start.y)],
                    end_css_px: [round3(end.x), round3(end.y)],
                    region: region_from_screen("line", handle, region),
                });
            }
        }
        segments
    }
}
