use std::collections::{BTreeMap, BTreeSet};

use serde_json::json;

use super::checks::{
    CompositionCheckExt, checked_check, error_check, observed_pairs, round3, skip_check,
};
use super::helpers::{draws_for_handles, projected_node_rect, visible_coverage_for_rect};
use super::object_pixels::object_pixel_quality_check;
use super::object_textures::object_texture_result_check;
use super::objects::ObjectCompositionInput;
use crate::{
    CaptureRgba8, CaptureScreenRect, SceneCompositionCheckV1, SceneCompositionStatusV1,
    SceneRecipeTargetV1,
};

pub(super) fn composition_import_checks(
    input: &ObjectCompositionInput<'_>,
    required_profile: Option<&str>,
    require_object_masks: bool,
) -> Vec<SceneCompositionCheckV1> {
    let mut checks = Vec::new();
    let target_fit_import_ids = target_fit_import_ids(input);

    for import in &input.manifest.imports {
        let target_path = format!("import.{}", import.id);
        let target_region_context_crop_allowed = !target_fit_import_ids.is_empty()
            && !target_fit_import_ids.contains(import.id.as_str());
        let mut handles = import
            .nodes_by_path
            .values()
            .copied()
            .collect::<BTreeSet<_>>();
        handles.extend(import.root_handles.iter().copied());
        if let Some(primary_root) = import.primary_root {
            handles.insert(primary_root);
        }
        if handles.is_empty() {
            checks.push(error_check(
                format!("{target_path}.presence"),
                "completeness",
                "declared_import_missing",
                Some(import.id.clone()),
                vec![import.import_handle],
                BTreeMap::new(),
                (
                    "declared recipe import has no manifest node handles",
                    "rebuild the recipe or remove the stale import",
                ),
            ));
            continue;
        }

        let representative_handle = import
            .primary_root
            .or_else(|| import.root_handles.first().copied())
            .or_else(|| handles.iter().next().copied())
            .unwrap_or(import.import_handle);
        let draws = draws_for_handles(input.inspection, &handles);
        if draws.is_empty() {
            checks.push(error_check(
                format!("{target_path}.presence"),
                "completeness",
                "declared_import_not_drawn",
                Some(import.id.clone()),
                handles.iter().copied().collect(),
                observed_pairs([
                    ("node_count", json!(handles.len())),
                    ("draw_count", json!(0_u64)),
                    ("uri", json!(import.uri)),
                ]),
                (
                    "declared import did not produce visible draw output",
                    "make the import visible, move it into frame, or remove the stale recipe import",
                ),
            ));
            continue;
        }

        checks.push(checked_check(
            format!("{target_path}.presence"),
            "completeness",
            "import_visible",
            Some(import.id.clone()),
            handles.iter().copied().collect(),
            observed_pairs([
                ("node_count", json!(handles.len())),
                ("draw_count", json!(draws.len())),
                ("uri", json!(import.uri)),
            ]),
            (
                "declared import is visible and contributes owned draw output",
                "no action needed",
            ),
        ));

        let projected_rect = projected_node_rect(input.capture, draws.as_slice());
        if let Some(rect) = projected_rect.filter(|rect| rect.width > 0.0 && rect.height > 0.0) {
            if target_region_context_crop_allowed && !rect_intersects_capture(input.capture, rect) {
                checks.push(target_region_context_crop_check(
                    format!("{target_path}.projected_bbox"),
                    "placement",
                    Some(import.id.clone()),
                    vec![representative_handle],
                ));
            } else {
                checks.push(
                    checked_check(
                        format!("{target_path}.projected_bbox"),
                        "placement",
                        "projected_bbox_available",
                        Some(import.id.clone()),
                        vec![representative_handle],
                        observed_pairs([
                            ("width_px", json!(round3(rect.width))),
                            ("height_px", json!(round3(rect.height))),
                            ("center_x_px", json!(round3(rect.center_x))),
                            ("center_y_px", json!(round3(rect.center_y))),
                        ]),
                        (
                            "declared import has a projected screen region",
                            "no action needed",
                        ),
                    )
                    .with_region(
                        "import",
                        Some(representative_handle),
                        Some(rect),
                    ),
                );
            }
        } else if target_region_context_crop_allowed {
            checks.push(target_region_context_crop_check(
                format!("{target_path}.projected_bbox"),
                "placement",
                Some(import.id.clone()),
                handles.iter().copied().collect(),
            ));
        } else {
            checks.push(error_check(
                format!("{target_path}.projected_bbox"),
                "placement",
                "projected_bbox_missing",
                Some(import.id.clone()),
                handles.iter().copied().collect(),
                BTreeMap::new(),
                (
                    "declared import draws but its projected screen bounds could not be computed",
                    "check camera metadata and imported node bounds",
                ),
            ));
        }

        if let Some(coverage) = projected_rect
            .and_then(|rect| visible_coverage_for_rect(input.capture, rect, input.background))
        {
            if coverage.foreground_pixels > 0 {
                checks.push(
                    checked_check(
                        format!("{target_path}.visible_coverage"),
                        "occlusion_depth",
                        "visible_pixel_coverage_available",
                        Some(import.id.clone()),
                        vec![representative_handle],
                        observed_pairs([
                            ("region_pixels", json!(coverage.region_pixels)),
                            ("foreground_pixels", json!(coverage.foreground_pixels)),
                            (
                                "foreground_fraction",
                                json!(round3(coverage.foreground_fraction)),
                            ),
                        ]),
                        (
                            "declared import has measured non-background pixels in its projected screen region",
                            "no action needed",
                        ),
                    )
                    .with_region_from_screen("import", Some(representative_handle), coverage.region),
                );
                if let Some(profile) = required_profile
                    && !target_region_context_crop_allowed
                {
                    checks.push(object_pixel_quality_check(
                        &target_path,
                        &import.id,
                        representative_handle,
                        input.capture,
                        coverage.region,
                        input.background,
                        profile,
                    ));
                    if let Some(check) = object_texture_result_check(
                        &target_path,
                        &import.id,
                        representative_handle,
                        input.capture,
                        coverage.region,
                        input.background,
                        draws.as_slice(),
                    ) {
                        checks.push(check);
                    }
                }
            } else if target_region_context_crop_allowed {
                checks.push(target_region_context_crop_check(
                    format!("{target_path}.visible_coverage"),
                    "occlusion_depth",
                    Some(import.id.clone()),
                    vec![representative_handle],
                ));
            } else {
                checks.push(
                    error_check(
                        format!("{target_path}.visible_coverage"),
                        "occlusion_depth",
                        "visible_pixel_coverage_missing",
                        Some(import.id.clone()),
                        vec![representative_handle],
                        observed_pairs([
                            ("region_pixels", json!(coverage.region_pixels)),
                            ("foreground_pixels", json!(coverage.foreground_pixels)),
                            (
                                "foreground_fraction",
                                json!(round3(coverage.foreground_fraction)),
                            ),
                            ("profile", json!(required_profile.unwrap_or_default())),
                        ]),
                        (
                            "declared import projects into the captured frame but contains no measured non-background pixels",
                            "move it into frame, make it visible, change its material/background contrast, or remove the stale import",
                        ),
                    )
                    .with_region_from_screen("import", Some(representative_handle), coverage.region),
                );
            }
        } else if require_object_masks && target_region_context_crop_allowed {
            checks.push(target_region_context_crop_check(
                format!("{target_path}.visible_coverage"),
                "occlusion_depth",
                Some(import.id.clone()),
                vec![representative_handle],
            ));
        } else if require_object_masks {
            checks.push(error_check(
                format!("{target_path}.visible_coverage"),
                "occlusion_depth",
                "visible_pixel_coverage_missing",
                Some(import.id.clone()),
                vec![representative_handle],
                observed_pairs([("profile", json!(required_profile.unwrap_or_default()))]),
                (
                    "declared import has no viewport-clipped projected region for visible-pixel coverage",
                    "move it into the captured frame or remove the stale import",
                ),
            ));
        }
    }

    checks
}

fn target_fit_import_ids(input: &ObjectCompositionInput<'_>) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    let Some(expect) = input.expect else {
        return ids;
    };
    for target_fit in &expect.expect_target_fit {
        if let SceneRecipeTargetV1::Import { id } = &target_fit.target {
            ids.insert(id.clone());
        }
    }
    ids
}

fn rect_intersects_capture(capture: &CaptureRgba8, rect: CaptureScreenRect) -> bool {
    rect.max_x > 0.0
        && rect.max_y > 0.0
        && rect.min_x < capture.descriptor.width as f32
        && rect.min_y < capture.descriptor.height as f32
}

fn target_region_context_crop_check(
    id: String,
    category: &str,
    target_id: Option<String>,
    affected_handles: Vec<u64>,
) -> SceneCompositionCheckV1 {
    skip_check(
        id,
        category,
        "target_region_context_crop_allowed",
        SceneCompositionStatusV1::NotApplicable,
        target_id,
        affected_handles,
        (
            "non-target import is allowed to be cropped by target-region framing",
            "no action needed unless this import should be the framed target",
        ),
    )
}
