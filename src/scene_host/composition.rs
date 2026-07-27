use serde_json::json;

use super::SceneHostCore;
mod annotations;
mod backend;
mod checks;
mod clipping;
mod grid;
mod helper_layer;
mod helpers;
mod import_roots;
mod object_depth;
mod object_framing;
mod object_pixels;
mod object_textures;
mod objects;
mod overlays;
mod placement;
mod regions;
mod separation;
mod state;
mod subject;
mod transform;

use crate::diagnostics::Backend;
use crate::{
    AssetFetcher, CaptureRgba8, SceneCompositionReportV1, SceneInspectionReportV1,
    SceneRecipeBuildV1, SceneRecipeExpectV1, SceneRecipeV1,
};
use backend::composition_backend_conformance_checks;
use checks::{CompositionCheckExt, checked_check, error_check, observed_pairs};
use clipping::composition_clipping_checks;
use grid::composition_grid_ownership_checks;
use helper_layer::composition_helper_layer_checks;
use helpers::unexpected_draw_handles;
use object_depth::composition_object_depth_order_checks;
use objects::{ObjectCompositionInput, composition_object_checks, owned_draw_handles};
use overlays::{composition_overlay_collision_checks, overlay_line_regions_by_handle};
use placement::composition_ground_contact_checks;
use separation::composition_separation_checks;
use state::composition_state_checks;
use subject::{SubjectMaskInput, composition_subject_projection_checks, has_declared_subjects};
use transform::composition_transform_checks;

pub use regions::CompositionOverlaySegmentV1;

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn composition_report(
        &self,
        recipe: &SceneRecipeV1,
        manifest: &SceneRecipeBuildV1,
        capture: &CaptureRgba8,
        inspection: &SceneInspectionReportV1,
        introspection: &crate::RenderIntrospectionReportV1,
        expect: Option<&SceneRecipeExpectV1>,
    ) -> SceneCompositionReportV1 {
        let mut checks = Vec::new();
        let label_region_infos = self.composition_label_region_infos(capture);
        let label_regions = label_region_infos
            .iter()
            .map(|(handle, info)| (*handle, info.clipped))
            .collect();
        let overlay_segments = self.composition_overlay_segments(capture);
        let line_regions = overlay_line_regions_by_handle(&overlay_segments);
        checks.extend(self.composition_callout_checks(
            recipe,
            manifest,
            &label_regions,
            &overlay_segments,
        ));
        checks.extend(self.composition_measurement_checks(
            recipe,
            &label_regions,
            &overlay_segments,
        ));
        checks.extend(composition_overlay_collision_checks(
            manifest,
            &label_region_infos,
            &overlay_segments,
            capture.descriptor.width,
            capture.descriptor.height,
        ));
        checks.extend(composition_ground_contact_checks(
            expect, manifest, inspection,
        ));
        checks.extend(composition_helper_layer_checks(
            expect, manifest, inspection, capture,
        ));
        checks.extend(composition_object_depth_order_checks(
            expect, manifest, inspection, capture,
        ));
        checks.extend(composition_backend_conformance_checks(
            recipe,
            self.renderer(),
            introspection,
            expect,
        ));
        checks.extend(composition_clipping_checks(recipe, &self.scene, expect));
        checks.extend(composition_state_checks(manifest, inspection, expect));
        checks.extend(composition_transform_checks(manifest, inspection, expect));
        checks.extend(composition_separation_checks(expect, manifest, inspection));
        let backend = self.backend();
        let mut subject_aov_capture = None;
        let mut subject_aov_error = None;
        if has_declared_subjects(recipe) && backend == Backend::Headless {
            match self.capture_semantic_aovs() {
                Ok(capture) => {
                    subject_aov_capture = Some(capture);
                }
                Err(error) => {
                    subject_aov_error = Some(error.to_string());
                }
            }
        }
        checks.extend(composition_subject_projection_checks(
            recipe,
            manifest,
            capture,
            inspection,
            SubjectMaskInput {
                backend,
                capture: subject_aov_capture.as_ref(),
                capture_error: subject_aov_error.as_deref(),
            },
        ));
        checks.extend(composition_object_checks(ObjectCompositionInput {
            recipe,
            manifest,
            capture,
            inspection,
            expect,
            label_regions: &label_regions,
            line_regions: &line_regions,
            background: self.renderer.background_color(),
        }));
        let mut owned_handles = owned_draw_handles(
            manifest,
            &label_regions,
            overlay_segments.iter().map(|segment| segment.handle),
        );
        let (grid_checks, ground_handles) = composition_grid_ownership_checks(recipe, inspection);
        checks.extend(grid_checks);
        owned_handles.extend(ground_handles);

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

        for draw in &inspection.draw_list {
            let generated = self.resolve_node(draw.node).is_ok_and(|node| {
                self.scene.has_tag(
                    node,
                    super::photographic_surroundings::GENERATED_SURROUNDING_TAG,
                ) || self
                    .scene
                    .has_tag(node, super::photographic_lighting::GENERATED_LIGHT_TAG)
            });
            if generated {
                owned_handles.insert(draw.node);
            }
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
}
