use std::collections::BTreeSet;

use serde_json::json;

use super::checks::{
    CompositionCheckExt, checked_check, error_check, observed_pairs, round3, skip_check,
};
use super::helpers::{clipped_region_from_rect, draws_for_handles, projected_node_rect};
use crate::{
    CaptureRgba8, CaptureScreenRect, CaptureScreenRegion, SceneCompositionCheckV1,
    SceneCompositionStatusV1, SceneDrawInspectionV1, SceneHostSemanticAovCaptureV1,
    SceneInspectionReportV1, SceneNodeInspectionV1, SceneRecipeBuildV1,
    SceneRecipeTargetResolutionMode, SceneRecipeTargetV1, SceneRecipeV1,
    SubjectObservationPixelQualityV1, resolve_scene_recipe_target_handles,
};

const MIN_SUBJECT_COLOR_FRAME_BOUNDARY_DELTA_SRGB8: f64 = 2.0;
const MIN_SUBJECT_BEAUTY_SEMANTIC_MATCH_FRACTION: f64 = 0.5;

#[derive(Debug, Clone, Copy)]
pub(super) struct SubjectMaskInput<'a> {
    pub(super) capture: Option<&'a SceneHostSemanticAovCaptureV1>,
    pub(super) capture_error: Option<&'a str>,
}

pub(super) fn composition_subject_projection_checks(
    recipe: &SceneRecipeV1,
    manifest: &SceneRecipeBuildV1,
    capture: &CaptureRgba8,
    inspection: &SceneInspectionReportV1,
    mask_input: SubjectMaskInput<'_>,
) -> Vec<SceneCompositionCheckV1> {
    let mut checks = Vec::new();
    for subject in declared_subjects(recipe) {
        checks.push(subject_projection_check(
            subject, manifest, capture, inspection,
        ));
        checks.push(subject_visible_mask_check(
            subject, recipe, manifest, capture, inspection, mask_input,
        ));
    }
    checks
}

pub(super) fn has_declared_subjects(recipe: &SceneRecipeV1) -> bool {
    !declared_subjects(recipe).is_empty()
}

#[derive(Debug, Clone, Copy)]
struct DeclaredSubject<'a> {
    slug: &'static str,
    source: &'static str,
    target: &'a SceneRecipeTargetV1,
}

fn declared_subjects(recipe: &SceneRecipeV1) -> Vec<DeclaredSubject<'_>> {
    let mut subjects = Vec::new();
    if let Some(metering) = recipe
        .render
        .as_ref()
        .and_then(|render| render.metering.as_ref())
        && metering.mode == "subject"
        && let Some(target) = metering.target.as_ref()
    {
        subjects.push(DeclaredSubject {
            slug: "render_metering",
            source: "render.metering",
            target,
        });
    }
    if let Some(focus) = recipe
        .render
        .as_ref()
        .and_then(|render| render.depth_of_field.as_ref())
        .and_then(|depth_of_field| depth_of_field.focus.as_ref())
        && focus.mode == "subject"
    {
        subjects.push(DeclaredSubject {
            slug: "render_depth_of_field_focus",
            source: "render.depth_of_field.focus",
            target: &focus.target,
        });
    }
    if let Some(subject) = recipe
        .photo
        .as_ref()
        .and_then(|photo| photo.subject.as_ref())
    {
        subjects.push(DeclaredSubject {
            slug: "photo_subject",
            source: "photo.subject",
            target: subject.target(),
        });
    }
    subjects
}

fn subject_projection_check(
    subject: DeclaredSubject<'_>,
    manifest: &SceneRecipeBuildV1,
    capture: &CaptureRgba8,
    inspection: &SceneInspectionReportV1,
) -> SceneCompositionCheckV1 {
    let id = format!("subject.{}.projected_bounds", subject.slug);
    let target_id = subject_target_id(subject.target);
    let mut base_observed = observed_pairs([
        ("source", json!(subject.source)),
        ("target_kind", json!(subject_target_kind(subject.target))),
        ("viewport_width", json!(capture.descriptor.width)),
        ("viewport_height", json!(capture.descriptor.height)),
        ("confidence", json!("projected_only")),
    ]);
    if let Some(target_id) = target_id.as_deref() {
        base_observed.insert("target_id".to_owned(), json!(target_id));
    }

    let handles = match resolve_scene_recipe_target_handles(
        manifest,
        subject.target,
        SceneRecipeTargetResolutionMode::SubjectIncludingHidden,
    ) {
        Ok(handles) => handles,
        Err(error) => {
            base_observed.insert("resolution_error".to_owned(), json!(error.message));
            base_observed.insert("candidates".to_owned(), json!(error.candidates));
            return error_check(
                id,
                "framing",
                "subject_target_unresolved",
                target_id,
                Vec::new(),
                base_observed,
                (
                    "declared subject target could not be resolved from the recipe build manifest",
                    "use a declared node id, imported node path, or whole-import subject target that exists in the recipe",
                ),
            );
        }
    };

    let handle_set = handles.iter().copied().collect::<BTreeSet<_>>();
    let draws = draws_for_handles(inspection, &handle_set);
    let projected_rect = projected_node_rect(capture, draws.as_slice());
    let representative_handle = handles.first().copied();
    base_observed.insert("handle_count".to_owned(), json!(handles.len()));
    base_observed.insert("draw_count".to_owned(), json!(draws.len()));

    let Some(rect) = projected_rect.filter(has_nonzero_area) else {
        return error_check(
            id,
            "framing",
            "subject_projected_bounds_missing",
            target_id,
            handles,
            base_observed,
            (
                "declared subject resolved to scene handles but has no projected bounds in the current camera/viewport",
                "move the subject into frame, choose a visible subject target, or frame the camera before rendering",
            ),
        );
    };

    add_rect_observations(&mut base_observed, capture, rect);
    checked_check(
        id,
        "framing",
        "subject_projected_bounds_available",
        target_id,
        handles,
        base_observed,
        (
            "declared subject has projected bounds in the current camera and viewport",
            "no action needed",
        ),
    )
    .with_region("subject", representative_handle, Some(rect))
}

fn subject_visible_mask_check(
    subject: DeclaredSubject<'_>,
    recipe: &SceneRecipeV1,
    manifest: &SceneRecipeBuildV1,
    capture: &CaptureRgba8,
    inspection: &SceneInspectionReportV1,
    mask_input: SubjectMaskInput<'_>,
) -> SceneCompositionCheckV1 {
    let id = format!("subject.{}.visible_mask", subject.slug);
    let target_id = subject_target_id(subject.target);
    let mut observed = subject_observed_base(subject, capture);
    observed.insert("mask_source".to_owned(), json!("semantic_aov"));

    let handles = match resolve_scene_recipe_target_handles(
        manifest,
        subject.target,
        SceneRecipeTargetResolutionMode::SubjectIncludingHidden,
    ) {
        Ok(handles) => handles,
        Err(error) => {
            observed.insert("resolution_error".to_owned(), json!(error.message));
            observed.insert("candidates".to_owned(), json!(error.candidates));
            return error_check(
                id,
                "occlusion_depth",
                "subject_visible_mask_target_unresolved",
                target_id,
                Vec::new(),
                observed,
                (
                    "declared subject target could not be resolved for semantic visible-mask measurement",
                    "use a declared node id, imported node path, or whole-import subject target that exists in the recipe",
                ),
            );
        }
    };
    observed.insert("handle_count".to_owned(), json!(handles.len()));

    // Key the skip on whether a mask was actually captured, not on the backend.
    // The GPU backends emit the same AOVs; asserting otherwise skipped the exact
    // check on every GPU verification even when a mask was available.
    if mask_input.capture.is_none() && mask_input.capture_error.is_none() {
        return skip_check(
            id,
            "occlusion_depth",
            "subject_visible_mask_backend_unsupported",
            SceneCompositionStatusV1::SkippedNoBackendSupport,
            target_id,
            handles,
            (
                "exact semantic subject masks are currently emitted by the CPU Headless composition path",
                "run headless recipe verification for exact subject-mask proof, or use backend-specific semantic AOV proof before claiming exact backend masks",
            ),
        );
    }

    let Some(aov) = mask_input.capture else {
        if let Some(error) = mask_input.capture_error {
            observed.insert("capture_error".to_owned(), json!(error));
        }
        return error_check(
            id,
            "occlusion_depth",
            "subject_visible_mask_unavailable",
            target_id,
            handles,
            observed,
            (
                "semantic AOV capture was unavailable for the declared subject mask",
                "verify that the headless scene has an active camera and can prepare semantic AOV attribution",
            ),
        );
    };

    if aov.width != capture.descriptor.width
        || aov.height != capture.descriptor.height
        || aov.id_indices.len() != (aov.width as usize).saturating_mul(aov.height as usize)
    {
        observed.insert("aov_width".to_owned(), json!(aov.width));
        observed.insert("aov_height".to_owned(), json!(aov.height));
        observed.insert("aov_id_samples".to_owned(), json!(aov.id_indices.len()));
        return error_check(
            id,
            "occlusion_depth",
            "subject_visible_mask_dimension_mismatch",
            target_id,
            handles,
            observed,
            (
                "semantic AOV dimensions do not match the rendered capture dimensions",
                "re-render and recapture semantic AOVs from the same prepared frame before using subject-mask metrics",
            ),
        );
    }

    let handle_set = handles.iter().copied().collect::<BTreeSet<_>>();
    let draws = draws_for_handles(inspection, &handle_set);
    let projected_rect = projected_node_rect(capture, draws.as_slice());
    let projected_area_px = projected_rect
        .and_then(|rect| clipped_region_from_rect(capture, rect))
        .map(region_area)
        .unwrap_or(0);
    let transparent_target_draws = draws
        .iter()
        .filter(|draw| {
            draw.material
                .as_ref()
                .is_some_and(|material| material.alpha_mode != "opaque")
        })
        .count();
    let target_palette_indices = target_palette_indices(aov, &handle_set);
    observed.insert("draw_count".to_owned(), json!(draws.len()));
    observed.insert("projected_area_px".to_owned(), json!(projected_area_px));
    observed.insert(
        "target_palette_count".to_owned(),
        json!(target_palette_indices.len()),
    );
    observed.insert(
        "transparent_target_draws".to_owned(),
        json!(transparent_target_draws),
    );
    observed.insert(
        "transparency_support".to_owned(),
        json!(if transparent_target_draws == 0 {
            "supported_opaque"
        } else {
            "degraded_transparent_excluded"
        }),
    );
    observed.insert("overlay_behavior".to_owned(), json!("excluded"));
    observed.insert("label_behavior".to_owned(), json!("excluded"));
    observed.insert(
        "excluded_transparent_triangle_count".to_owned(),
        json!(aov.exclusions.transparent_triangle_count),
    );
    observed.insert(
        "excluded_overlay_triangle_count".to_owned(),
        json!(aov.exclusions.overlay_triangle_count),
    );
    observed.insert(
        "excluded_label_quad_count".to_owned(),
        json!(aov.exclusions.label_quad_count),
    );
    observed.insert(
        "excluded_unattributed_triangle_count".to_owned(),
        json!(aov.exclusions.unattributed_triangle_count),
    );
    if target_palette_indices.is_empty() {
        let reason = subject_zero_visible_reason(SubjectZeroVisibleInput {
            recipe,
            inspection,
            handles: &handle_set,
            draws: draws.as_slice(),
            projected_rect,
            projected_area_px,
            other_visible_pixels: 0,
            target_palette_missing: true,
            transparent_target_draws,
        });
        observed.insert("zero_visible_reason".to_owned(), json!(reason.code));
        return error_check(
            id,
            "occlusion_depth",
            reason.code,
            target_id,
            handles,
            observed,
            (reason.message, reason.fix_hint),
        );
    }

    let metrics = semantic_mask_metrics(aov, &target_palette_indices);
    observed.insert("visible_pixels".to_owned(), json!(metrics.visible_pixels));
    observed.insert(
        "other_visible_pixels".to_owned(),
        json!(metrics.other_visible_pixels),
    );
    let Some(region) = metrics.region else {
        let reason = subject_zero_visible_reason(SubjectZeroVisibleInput {
            recipe,
            inspection,
            handles: &handle_set,
            draws: draws.as_slice(),
            projected_rect,
            projected_area_px,
            other_visible_pixels: metrics.other_visible_pixels,
            target_palette_missing: false,
            transparent_target_draws,
        });
        observed.insert("zero_visible_reason".to_owned(), json!(reason.code));
        return error_check(
            id,
            "occlusion_depth",
            reason.code,
            target_id,
            handles,
            observed,
            (reason.message, reason.fix_hint),
        );
    };
    if let Some(pixel_quality) =
        semantic_subject_pixel_quality(capture, aov, &target_palette_indices)
    {
        observed.insert(
            "mean_luminance_srgb8".to_owned(),
            json!(round2_f64(pixel_quality.mean_luminance_srgb8)),
        );
        observed.insert(
            "luminance_stddev_srgb8".to_owned(),
            json!(round2_f64(pixel_quality.luminance_stddev_srgb8)),
        );
        observed.insert(
            "luminance_range_srgb8".to_owned(),
            json!(round2_f64(pixel_quality.luminance_range_srgb8)),
        );
        observed.insert(
            "low_clip_fraction".to_owned(),
            json!(round4_f64(pixel_quality.low_clip_fraction)),
        );
        observed.insert(
            "high_clip_fraction".to_owned(),
            json!(round4_f64(pixel_quality.high_clip_fraction)),
        );
        observed.insert(
            "subject_pixel_sample_count".to_owned(),
            json!(pixel_quality.sample_count),
        );
    }
    if let Some(beauty_ids) = aov.beauty_id_indices.as_deref() {
        let Some(agreement) =
            semantic_beauty_pass_agreement(&aov.id_indices, beauty_ids, &target_palette_indices)
        else {
            observed.insert(
                "color_frame_agreement_confidence".to_owned(),
                json!("unavailable"),
            );
            observed.insert(
                "beauty_semantic_samples".to_owned(),
                json!(beauty_ids.len()),
            );
            return error_check(
                id,
                "completeness",
                "subject_color_frame_agreement_unavailable",
                target_id,
                handles,
                observed,
                (
                    "the same-pass beauty semantic witness dimensions do not match the semantic AOV",
                    "re-render and capture beauty and semantic attachments from the same prepared target",
                ),
            );
        };
        observed.insert(
            "color_frame_agreement_confidence".to_owned(),
            json!("same_pass_beauty_semantic_mrt"),
        );
        observed.insert(
            "beauty_semantic_aov_target_pixels".to_owned(),
            json!(agreement.aov_target_pixels),
        );
        observed.insert(
            "beauty_semantic_matching_target_pixels".to_owned(),
            json!(agreement.matching_target_pixels),
        );
        observed.insert(
            "beauty_semantic_match_fraction".to_owned(),
            json!(round4_f64(agreement.match_fraction)),
        );
        if !agreement.subject_visible_in_beauty_pass() {
            return error_check(
                id,
                "completeness",
                "subject_color_frame_agreement_below_min",
                target_id,
                handles,
                observed,
                (
                    "the separate semantic AOV reports a visible subject but the same fragment invocations that wrote the beauty frame did not emit matching subject coverage",
                    "treat the render as failed and inspect beauty-pass draw completion; do not infer subject presence from the separate AOV",
                ),
            );
        }
    } else {
        let Some(color_frame_agreement) = semantic_color_frame_agreement(
            capture.descriptor.width,
            capture.descriptor.height,
            &capture.rgba8,
            &aov.id_indices,
            &target_palette_indices,
        ) else {
            observed.insert(
                "color_frame_agreement_confidence".to_owned(),
                json!("unavailable"),
            );
            return error_check(
                id,
                "completeness",
                "subject_color_frame_agreement_unavailable",
                target_id,
                handles,
                observed,
                (
                    "the semantic subject mask could not be compared with the rendered color frame",
                    "re-render and capture the color frame and semantic AOV at matching dimensions from the same prepared state",
                ),
            );
        };
        observed.insert(
            "color_frame_agreement_confidence".to_owned(),
            json!("heuristic_local_semantic_boundary"),
        );
        observed.insert(
            "color_frame_boundary_mean_delta_srgb8".to_owned(),
            json!(round2_f64(color_frame_agreement.boundary_mean_delta_srgb8)),
        );
        observed.insert(
            "color_frame_boundary_sample_count".to_owned(),
            json!(color_frame_agreement.boundary_sample_count),
        );
        let heuristic_is_blocking = capture.descriptor.backend != crate::Backend::Headless;
        observed.insert(
            "color_frame_agreement_enforcement".to_owned(),
            json!(if heuristic_is_blocking {
                "blocking_without_mrt"
            } else {
                "report_only_cpu"
            }),
        );
        if heuristic_is_blocking && !color_frame_agreement.subject_visible_in_color_frame() {
            return error_check(
                id,
                "completeness",
                "subject_color_frame_agreement_below_min",
                target_id,
                handles,
                observed,
                (
                    "the semantic AOV reports a visible subject but the rendered color frame has no corresponding local silhouette signal",
                    "treat the render as failed; inspect beauty-pass draw completion and add a same-pass semantic coverage witness before trusting this backend",
                ),
            );
        }
    }
    let viewport_area = (aov.width as u64).saturating_mul(aov.height as u64).max(1);
    observed.insert(
        "visible_fill_fraction".to_owned(),
        json!(round3(metrics.visible_pixels as f32 / viewport_area as f32)),
    );
    let visible_fraction_of_projected = if projected_area_px > 0 {
        (metrics.visible_pixels as f32 / projected_area_px as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };
    observed.insert(
        "visible_fraction_of_projected".to_owned(),
        json!(round3(visible_fraction_of_projected)),
    );
    observed.insert(
        "occlusion_estimate".to_owned(),
        json!(round3(1.0 - visible_fraction_of_projected)),
    );
    observed.insert(
        "confidence".to_owned(),
        json!(if transparent_target_draws == 0 {
            "exact_opaque_semantic_aov"
        } else {
            "degraded_transparent_excluded"
        }),
    );
    if let Some(depth) = metrics.depth {
        observed.insert("depth_near_m".to_owned(), json!(round3(depth.near_m)));
        observed.insert("depth_p50_m".to_owned(), json!(round3(depth.p50_m)));
        observed.insert("depth_far_m".to_owned(), json!(round3(depth.far_m)));
        observed.insert("depth_sample_count".to_owned(), json!(depth.sample_count));
        observed.insert(
            "depth_confidence".to_owned(),
            json!(round3(depth.confidence)),
        );
    }

    checked_check(
        id,
        "occlusion_depth",
        "subject_visible_mask_available",
        target_id,
        handles,
        observed,
        (
            "declared subject has exact visible-pixel bounds from semantic AOV attribution",
            "no action needed",
        ),
    )
    .with_region_from_screen("subject_visible", region.handle, region.region)
}

fn add_rect_observations(
    observed: &mut std::collections::BTreeMap<String, serde_json::Value>,
    capture: &CaptureRgba8,
    rect: CaptureScreenRect,
) {
    let viewport_area =
        (capture.descriptor.width as f32).max(1.0) * (capture.descriptor.height as f32).max(1.0);
    let area_px = rect.width.max(0.0) * rect.height.max(0.0);
    observed.insert("width_px".to_owned(), json!(round3(rect.width)));
    observed.insert("height_px".to_owned(), json!(round3(rect.height)));
    observed.insert("center_x_px".to_owned(), json!(round3(rect.center_x)));
    observed.insert("center_y_px".to_owned(), json!(round3(rect.center_y)));
    observed.insert("area_px".to_owned(), json!(round3(area_px)));
    observed.insert(
        "fill_fraction".to_owned(),
        json!(round3((area_px / viewport_area).clamp(0.0, 1.0))),
    );
}

fn has_nonzero_area(rect: &CaptureScreenRect) -> bool {
    rect.width.is_finite() && rect.height.is_finite() && rect.width > 0.0 && rect.height > 0.0
}

#[derive(Debug, Clone, Copy)]
struct SubjectZeroVisibleInput<'a> {
    recipe: &'a SceneRecipeV1,
    inspection: &'a SceneInspectionReportV1,
    handles: &'a BTreeSet<u64>,
    draws: &'a [&'a SceneDrawInspectionV1],
    projected_rect: Option<CaptureScreenRect>,
    projected_area_px: u64,
    other_visible_pixels: u64,
    target_palette_missing: bool,
    transparent_target_draws: usize,
}

#[derive(Debug, Clone, Copy)]
struct SubjectZeroVisibleReason {
    code: &'static str,
    message: &'static str,
    fix_hint: &'static str,
}

fn subject_zero_visible_reason(input: SubjectZeroVisibleInput<'_>) -> SubjectZeroVisibleReason {
    if subject_hidden_by_visibility(input.inspection, input.handles) {
        return SubjectZeroVisibleReason {
            code: "subject_hidden",
            message: "declared subject target is hidden by node visibility",
            fix_hint: "set the subject node and its ancestors visible before rendering",
        };
    }
    if input.transparent_target_draws > 0 && input.target_palette_missing {
        return SubjectZeroVisibleReason {
            code: "subject_transparent_unsupported",
            message: "declared subject is transparent and excluded from exact semantic subject masks",
            fix_hint: "choose an opaque subject mesh or use a backend proof that supports transparent subject attribution",
        };
    }
    if input.recipe.section_box.is_some()
        && input.projected_rect.is_some()
        && input.projected_area_px > 0
    {
        return SubjectZeroVisibleReason {
            code: "subject_clipped_by_section_box",
            message: "declared subject projects into the frame but is removed by the active section box",
            fix_hint: "widen or disable the section box, or choose a subject inside the section volume",
        };
    }
    if input
        .recipe
        .clipping_planes
        .iter()
        .any(|plane| plane.active)
        && input.projected_rect.is_some()
        && input.projected_area_px > 0
    {
        return SubjectZeroVisibleReason {
            code: "subject_clipped_by_clipping_plane",
            message: "declared subject projects into the frame but is removed by active clipping planes",
            fix_hint: "clear, move, or disable the clipping planes that cut away the subject",
        };
    }
    if subject_has_degenerate_geometry(input.inspection, input.handles, input.projected_rect) {
        return SubjectZeroVisibleReason {
            code: "subject_degenerate_geometry",
            message: "declared subject resolved but has degenerate geometry or transform in this view",
            fix_hint: "choose a subject with non-degenerate opaque geometry and a finite nonzero transform",
        };
    }
    if input.projected_rect.is_some() && input.projected_area_px == 0 {
        return SubjectZeroVisibleReason {
            code: "subject_outside_viewport",
            message: "declared subject is outside the current viewport",
            fix_hint: "frame the subject or move the camera so the subject projects inside the viewport",
        };
    }
    if input.projected_rect.is_none() && !input.draws.is_empty() {
        return SubjectZeroVisibleReason {
            code: "subject_behind_camera",
            message: "declared subject has drawable geometry but no forward camera projection",
            fix_hint: "move the camera or subject so the subject is in front of the active camera",
        };
    }
    if input.other_visible_pixels > 0 {
        return SubjectZeroVisibleReason {
            code: "subject_occluded",
            message: "declared subject is fully occluded by other visible geometry",
            fix_hint: "move the camera, move occluding geometry, or choose a visible subject target",
        };
    }
    if input.draws.is_empty() || input.target_palette_missing {
        return SubjectZeroVisibleReason {
            code: "subject_degenerate_geometry",
            message: "declared subject resolved but produced no attributable drawable subject pixels",
            fix_hint: "choose a subject with non-degenerate opaque geometry and a finite transform",
        };
    }
    SubjectZeroVisibleReason {
        code: "subject_visible_mask_empty",
        message: "declared subject has semantic palette entries but no visible subject pixels in the current frame",
        fix_hint: "move the subject into frame, remove occluders, or choose a subject target with visible opaque geometry",
    }
}

fn subject_hidden_by_visibility(
    inspection: &SceneInspectionReportV1,
    handles: &BTreeSet<u64>,
) -> bool {
    handles
        .iter()
        .any(|handle| node_hidden_by_visibility(inspection, *handle))
}

fn node_hidden_by_visibility(inspection: &SceneInspectionReportV1, handle: u64) -> bool {
    let Some(node) = inspection.node_by_handle(handle) else {
        return false;
    };
    if !node.visible {
        return true;
    }
    hidden_ancestor(inspection, node)
}

fn hidden_ancestor(inspection: &SceneInspectionReportV1, node: &SceneNodeInspectionV1) -> bool {
    let mut parent = node.parent;
    while let Some(handle) = parent {
        let Some(parent_node) = inspection.node_by_handle(handle) else {
            return false;
        };
        if !parent_node.visible {
            return true;
        }
        parent = parent_node.parent;
    }
    false
}

fn subject_has_degenerate_geometry(
    inspection: &SceneInspectionReportV1,
    handles: &BTreeSet<u64>,
    projected_rect: Option<CaptureScreenRect>,
) -> bool {
    if projected_rect.is_some_and(|rect| {
        !rect.width.is_finite()
            || !rect.height.is_finite()
            || rect.width <= 0.0
            || rect.height <= 0.0
    }) {
        return true;
    }
    handles.iter().any(|handle| {
        inspection.node_by_handle(*handle).is_some_and(|node| {
            node.local_transform.scale.x == 0.0
                || node.local_transform.scale.y == 0.0
                || node.local_transform.scale.z == 0.0
                || node.world_transform.scale.x == 0.0
                || node.world_transform.scale.y == 0.0
                || node.world_transform.scale.z == 0.0
        })
    })
}

fn subject_observed_base(
    subject: DeclaredSubject<'_>,
    capture: &CaptureRgba8,
) -> std::collections::BTreeMap<String, serde_json::Value> {
    let mut observed = observed_pairs([
        ("source", json!(subject.source)),
        ("target_kind", json!(subject_target_kind(subject.target))),
        ("viewport_width", json!(capture.descriptor.width)),
        ("viewport_height", json!(capture.descriptor.height)),
    ]);
    if let Some(target_id) = subject_target_id(subject.target) {
        observed.insert("target_id".to_owned(), json!(target_id));
    }
    observed
}

fn target_palette_indices(
    aov: &SceneHostSemanticAovCaptureV1,
    target_handles: &BTreeSet<u64>,
) -> BTreeSet<u32> {
    aov.legend
        .iter()
        .filter(|entry| {
            target_handles.contains(&entry.node_handle)
                || entry
                    .instance_handle
                    .is_some_and(|handle| target_handles.contains(&handle))
        })
        .map(|entry| entry.palette_index)
        .collect()
}

#[derive(Debug, Clone, Copy)]
struct VisibleMaskRegion {
    handle: Option<u64>,
    region: CaptureScreenRegion,
}

#[derive(Debug, Clone, Copy)]
struct SemanticMaskMetrics {
    visible_pixels: u64,
    other_visible_pixels: u64,
    region: Option<VisibleMaskRegion>,
    depth: Option<SemanticDepthMetrics>,
}

#[derive(Debug, Clone, Copy)]
struct SemanticDepthMetrics {
    near_m: f32,
    p50_m: f32,
    far_m: f32,
    sample_count: u64,
    confidence: f32,
}

fn semantic_mask_metrics(
    aov: &SceneHostSemanticAovCaptureV1,
    target_palette_indices: &BTreeSet<u32>,
) -> SemanticMaskMetrics {
    let mut visible_pixels = 0_u64;
    let mut other_visible_pixels = 0_u64;
    let mut depths = Vec::new();
    let mut min_x = u32::MAX;
    let mut min_y = u32::MAX;
    let mut max_x = 0_u32;
    let mut max_y = 0_u32;
    for (offset, palette_index) in aov.id_indices.iter().copied().enumerate() {
        if palette_index == 0 {
            continue;
        }
        if !target_palette_indices.contains(&palette_index) {
            other_visible_pixels = other_visible_pixels.saturating_add(1);
            continue;
        }
        visible_pixels = visible_pixels.saturating_add(1);
        let x = (offset % aov.width as usize) as u32;
        let y = (offset / aov.width as usize) as u32;
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
        let depth = aov.depth_meters.get(offset).copied().unwrap_or(f32::NAN);
        if depth.is_finite() && depth > 0.0 {
            depths.push(depth);
        }
    }
    let region = (visible_pixels > 0).then_some(VisibleMaskRegion {
        handle: None,
        region: CaptureScreenRegion {
            x: min_x,
            y: min_y,
            width: max_x.saturating_sub(min_x).saturating_add(1),
            height: max_y.saturating_sub(min_y).saturating_add(1),
        },
    });
    let depth = depth_metrics(&mut depths, visible_pixels);
    SemanticMaskMetrics {
        visible_pixels,
        other_visible_pixels,
        region,
        depth,
    }
}

fn semantic_subject_pixel_quality(
    capture: &CaptureRgba8,
    aov: &SceneHostSemanticAovCaptureV1,
    target_palette_indices: &BTreeSet<u32>,
) -> Option<SubjectObservationPixelQualityV1> {
    let mut sample_count = 0_u64;
    let mut low_clip = 0_u64;
    let mut high_clip = 0_u64;
    let mut sum = 0.0_f64;
    let mut sum_sq = 0.0_f64;
    let mut min_luma = f64::INFINITY;
    let mut max_luma = f64::NEG_INFINITY;
    for (offset, palette_index) in aov.id_indices.iter().copied().enumerate() {
        if !target_palette_indices.contains(&palette_index) {
            continue;
        }
        let pixel_offset = offset.saturating_mul(4);
        let pixel = capture.rgba8.get(pixel_offset..pixel_offset + 4)?;
        if pixel[3] == 0 {
            continue;
        }
        let luma = 0.2126 * f64::from(pixel[0])
            + 0.7152 * f64::from(pixel[1])
            + 0.0722 * f64::from(pixel[2]);
        sum += luma;
        sum_sq += luma * luma;
        min_luma = min_luma.min(luma);
        max_luma = max_luma.max(luma);
        if luma <= 10.0 {
            low_clip = low_clip.saturating_add(1);
        }
        if luma >= 245.0 {
            high_clip = high_clip.saturating_add(1);
        }
        sample_count = sample_count.saturating_add(1);
    }
    if sample_count == 0 {
        return None;
    }
    let mean = sum / sample_count as f64;
    let variance = (sum_sq / sample_count as f64 - mean * mean).max(0.0);
    Some(SubjectObservationPixelQualityV1 {
        mean_luminance_srgb8: mean,
        luminance_stddev_srgb8: variance.sqrt(),
        luminance_range_srgb8: max_luma - min_luma,
        low_clip_fraction: low_clip as f64 / sample_count as f64,
        high_clip_fraction: high_clip as f64 / sample_count as f64,
        sample_count,
    })
}

#[derive(Debug, Clone, Copy)]
struct SemanticColorFrameAgreement {
    boundary_mean_delta_srgb8: f64,
    boundary_sample_count: u64,
}

#[derive(Debug, Clone, Copy)]
struct SemanticBeautyPassAgreement {
    aov_target_pixels: u64,
    matching_target_pixels: u64,
    match_fraction: f64,
}

impl SemanticBeautyPassAgreement {
    fn subject_visible_in_beauty_pass(self) -> bool {
        self.aov_target_pixels > 0
            && self.matching_target_pixels > 0
            && self.match_fraction >= MIN_SUBJECT_BEAUTY_SEMANTIC_MATCH_FRACTION
    }
}

fn semantic_beauty_pass_agreement(
    aov_ids: &[u32],
    beauty_ids: &[u32],
    target_palette_indices: &BTreeSet<u32>,
) -> Option<SemanticBeautyPassAgreement> {
    if aov_ids.len() != beauty_ids.len() {
        return None;
    }
    let mut aov_target_pixels = 0_u64;
    let mut matching_target_pixels = 0_u64;
    for (aov_id, beauty_id) in aov_ids.iter().zip(beauty_ids) {
        if !target_palette_indices.contains(aov_id) {
            continue;
        }
        aov_target_pixels = aov_target_pixels.saturating_add(1);
        if aov_id == beauty_id {
            matching_target_pixels = matching_target_pixels.saturating_add(1);
        }
    }
    Some(SemanticBeautyPassAgreement {
        aov_target_pixels,
        matching_target_pixels,
        match_fraction: matching_target_pixels as f64 / aov_target_pixels.max(1) as f64,
    })
}

impl SemanticColorFrameAgreement {
    fn subject_visible_in_color_frame(self) -> bool {
        self.boundary_sample_count > 0
            && self.boundary_mean_delta_srgb8 >= MIN_SUBJECT_COLOR_FRAME_BOUNDARY_DELTA_SRGB8
    }
}

fn semantic_color_frame_agreement(
    width: u32,
    height: u32,
    rgba8: &[u8],
    id_indices: &[u32],
    target_palette_indices: &BTreeSet<u32>,
) -> Option<SemanticColorFrameAgreement> {
    let pixel_count = (width as usize).checked_mul(height as usize)?;
    if id_indices.len() != pixel_count || rgba8.len() != pixel_count.checked_mul(4)? {
        return None;
    }
    let width = width as usize;
    let height = height as usize;
    let is_subject = |index: usize| target_palette_indices.contains(&id_indices[index]);
    let mut boundary_delta_sum = 0_u64;
    let mut boundary_sample_count = 0_u64;
    for index in 0..pixel_count {
        if !is_subject(index) {
            continue;
        }
        let x = index % width;
        let y = index / width;
        let pixel = &rgba8[index * 4..index * 4 + 4];
        let boundary_delta = [
            (x > 0).then_some(index - 1),
            (x + 1 < width).then_some(index + 1),
            (y > 0).then_some(index - width),
            (y + 1 < height).then_some(index + width),
        ]
        .into_iter()
        .flatten()
        .filter(|neighbor| !is_subject(*neighbor))
        .map(|neighbor| {
            let neighbor = &rgba8[neighbor * 4..neighbor * 4 + 4];
            (0..3)
                .map(|channel| pixel[channel].abs_diff(neighbor[channel]))
                .max()
                .unwrap_or(0)
        })
        .max();
        if let Some(delta) = boundary_delta {
            boundary_delta_sum = boundary_delta_sum.saturating_add(u64::from(delta));
            boundary_sample_count = boundary_sample_count.saturating_add(1);
        }
    }
    Some(SemanticColorFrameAgreement {
        boundary_mean_delta_srgb8: boundary_delta_sum as f64 / boundary_sample_count.max(1) as f64,
        boundary_sample_count,
    })
}

const fn region_area(region: CaptureScreenRegion) -> u64 {
    (region.width as u64) * (region.height as u64)
}

fn depth_metrics(depths: &mut [f32], visible_pixels: u64) -> Option<SemanticDepthMetrics> {
    if depths.is_empty() {
        return None;
    }
    depths.sort_by(|left, right| left.total_cmp(right));
    let last = depths.len() - 1;
    Some(SemanticDepthMetrics {
        near_m: depths[0],
        p50_m: depths[last / 2],
        far_m: depths[last],
        sample_count: depths.len() as u64,
        confidence: (depths.len() as f32 / visible_pixels.max(1) as f32).clamp(0.0, 1.0),
    })
}

fn subject_target_kind(target: &SceneRecipeTargetV1) -> &'static str {
    match target {
        SceneRecipeTargetV1::Node { .. } => "node",
        SceneRecipeTargetV1::Import { .. } => "import",
        SceneRecipeTargetV1::World { .. } => "world",
    }
}

fn subject_target_id(target: &SceneRecipeTargetV1) -> Option<String> {
    match target {
        SceneRecipeTargetV1::Node { id } | SceneRecipeTargetV1::Import { id } => Some(id.clone()),
        SceneRecipeTargetV1::World { .. } => None,
    }
}

fn round2_f64(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn round4_f64(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_color_frame_agreement_distinguishes_a_drawn_subject_from_a_stale_aov_mask() {
        let width = 4;
        let height = 4;
        let ids = vec![
            0, 0, 0, 0, //
            0, 1, 1, 0, //
            0, 1, 1, 0, //
            0, 0, 0, 0,
        ];
        let target = BTreeSet::from([1]);
        let flat = [96, 96, 96, 255].repeat((width * height) as usize);
        let stale = semantic_color_frame_agreement(width, height, &flat, &ids, &target)
            .expect("well-formed flat capture has boundary samples");
        assert_eq!(stale.boundary_mean_delta_srgb8, 0.0);
        assert!(!stale.subject_visible_in_color_frame());

        let mut drawn = flat;
        for offset in [5_usize, 6, 9, 10] {
            drawn[offset * 4..offset * 4 + 4].copy_from_slice(&[24, 32, 40, 255]);
        }
        let agreement = semantic_color_frame_agreement(width, height, &drawn, &ids, &target)
            .expect("well-formed drawn capture has boundary samples");
        assert!(agreement.boundary_mean_delta_srgb8 >= 56.0);
        assert!(agreement.subject_visible_in_color_frame());
    }

    #[test]
    fn semantic_beauty_pass_agreement_rejects_a_stale_aov_without_rgb_guessing() {
        let aov_ids = vec![
            0, 0, 0, 0, //
            0, 7, 7, 0, //
            0, 7, 7, 0, //
            0, 0, 0, 0,
        ];
        let target = BTreeSet::from([7]);
        let missing_beauty = vec![0; aov_ids.len()];
        let missing =
            semantic_beauty_pass_agreement(&aov_ids, &missing_beauty, &target).expect("same size");
        assert_eq!(missing.aov_target_pixels, 4);
        assert_eq!(missing.matching_target_pixels, 0);
        assert!(!missing.subject_visible_in_beauty_pass());

        let drawn = semantic_beauty_pass_agreement(&aov_ids, &aov_ids, &target)
            .expect("same-size matching witness");
        assert_eq!(drawn.matching_target_pixels, 4);
        assert_eq!(drawn.match_fraction, 1.0);
        assert!(drawn.subject_visible_in_beauty_pass());
    }
}
