use std::collections::{BTreeMap, BTreeSet};

use serde_json::json;

use super::checks::{
    CompositionCheckExt, checked_check, error_check, observed_pairs, round3, skip_check,
};
use super::helpers::{
    SubjectMaskCoverage, draws_for_handle, expected_color_handles, projected_node_rect,
    visible_coverage_for_rect, visible_coverage_for_region,
};
use super::object_framing::object_framing_check;
use super::object_pixels::object_pixel_quality_check;
use super::object_textures::object_texture_result_check;
use crate::scene::recipe::SceneRecipeAutoExposureV1;
use crate::{
    CaptureRgba8, CaptureScreenRegion, Color, RenderQualityProfile, SceneCompositionCheckV1,
    SceneCompositionStatusV1, SceneInspectionReportV1, SceneRecipeBuildV1, SceneRecipeExpectV1,
    SceneRecipeV1,
};

pub(super) struct ObjectCompositionInput<'a> {
    pub(super) recipe: &'a SceneRecipeV1,
    pub(super) manifest: &'a SceneRecipeBuildV1,
    pub(super) capture: &'a CaptureRgba8,
    pub(super) inspection: &'a SceneInspectionReportV1,
    pub(super) expect: Option<&'a SceneRecipeExpectV1>,
    pub(super) label_regions: &'a BTreeMap<u64, CaptureScreenRegion>,
    pub(super) line_regions: &'a BTreeMap<u64, CaptureScreenRegion>,
    pub(super) background: Color,
    /// The renderer's per-pixel attribution, when a mask was captured. Coverage
    /// falls back to comparing pixels against the clear colour without it, which
    /// cannot separate a subject from a lit floor or backdrop.
    pub(super) subject_aov: Option<&'a crate::SceneHostSemanticAovCaptureV1>,
}

pub(super) fn composition_object_checks(
    input: ObjectCompositionInput<'_>,
) -> Vec<SceneCompositionCheckV1> {
    let explicit_color_handles = expected_color_handles(input.expect, input.manifest);
    let required_profile = required_object_profile(input.recipe, input.expect);
    let require_object_masks = required_profile.is_some();
    let mut checks = Vec::new();

    for target in &input.manifest.nodes {
        if !matches!(
            target.kind.as_str(),
            "node" | "instance_set" | "particle_set" | "label"
        ) {
            continue;
        }
        let target_path = format!("node.{}", target.id);
        let Some(node) = input.inspection.node_by_handle(target.handle) else {
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

        let draws = draws_for_handle(input.inspection, target.handle);
        let label_region = (target.kind == "label")
            .then(|| input.label_regions.get(&target.handle).copied())
            .flatten();
        let line_region = input.line_regions.get(&target.handle).copied();
        let has_draw = !draws.is_empty()
            || (target.kind == "label" && input.label_regions.contains_key(&target.handle));
        let projected_rect = projected_node_rect(input.capture, draws.as_slice());
        if node.visible && (has_draw || line_region.is_some()) {
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

        if let Some(rect) = projected_rect.filter(|rect| rect.width > 0.0 && rect.height > 0.0) {
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
            if let Some(profile) = required_profile
                && matches!(
                    target.kind.as_str(),
                    "node" | "instance_set" | "particle_set"
                )
            {
                checks.push(object_framing_check(
                    &target_path,
                    &target.id,
                    target.handle,
                    input.capture,
                    rect,
                    profile,
                ));
            }
        } else if let Some(region) = label_region.or(line_region) {
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
                        "declared overlay element has an exact projected screen region",
                        "no action needed",
                    ),
                )
                .with_region_from_screen("overlay", Some(target.handle), region),
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
                    "declared overlay element projects to a non-empty screen size",
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
            let material_base_colors = draws
                .iter()
                .filter_map(|draw| draw.material.as_ref().map(|material| material.base_color))
                .collect::<Vec<_>>();
            if !material_base_colors.is_empty() {
                checks.push(checked_check(
                    format!("{target_path}.expected_color"),
                    "color_exposure",
                    "material_base_color_available",
                    Some(target.id.clone()),
                    vec![target.handle],
                    observed_pairs([("base_colors", json!(material_base_colors))]),
                    (
                        "declared node has material base-color intent in the inspected draw output",
                        "use expect_color when the rendered pixel result must match a swatch",
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
        }

        let target_handles = BTreeSet::from([target.handle]);
        let target_mask = input.subject_aov.map(|capture| SubjectMaskCoverage {
            capture,
            handles: &target_handles,
        });
        let visible_coverage = projected_rect
            .and_then(|rect| {
                visible_coverage_for_rect(
                    input.capture,
                    rect,
                    input.background,
                    target_mask.as_ref(),
                )
            })
            .or_else(|| {
                label_region.map(|region| {
                    visible_coverage_for_region(input.capture, region, input.background)
                })
            })
            .or_else(|| {
                line_region.map(|region| {
                    visible_coverage_for_region(input.capture, region, input.background)
                })
            });
        if let Some(coverage) = visible_coverage {
            if coverage.foreground_pixels > 0 {
                checks.push(
                    checked_check(
                        format!("{target_path}.visible_coverage"),
                        "occlusion_depth",
                        "visible_pixel_coverage_available",
                        Some(target.id.clone()),
                        vec![target.handle],
                        observed_pairs([
                            ("region_pixels", json!(coverage.region_pixels)),
                            ("foreground_pixels", json!(coverage.foreground_pixels)),
                            (
                                "foreground_fraction",
                                json!(round3(coverage.foreground_fraction)),
                            ),
                            ("coverage_source", json!(coverage.source)),
                        ]),
                        (
                            "declared node has measured non-background pixels in its projected screen region",
                            "no action needed",
                        ),
                    )
                    .with_region_from_screen("node", Some(target.handle), coverage.region),
                );
                if let Some(profile) = required_profile {
                    checks.push(object_pixel_quality_check(
                        &target_path,
                        &target.id,
                        target.handle,
                        input.capture,
                        coverage.region,
                        input.background,
                        profile,
                    ));
                    if let Some(check) = object_texture_result_check(
                        &target_path,
                        &target.id,
                        target.handle,
                        input.capture,
                        coverage.region,
                        input.background,
                        draws.as_slice(),
                    ) {
                        checks.push(check);
                    }
                }
            } else {
                checks.push(
                    error_check(
                        format!("{target_path}.visible_coverage"),
                        "occlusion_depth",
                        "visible_pixel_coverage_missing",
                        Some(target.id.clone()),
                        vec![target.handle],
                        observed_pairs([
                            ("region_pixels", json!(coverage.region_pixels)),
                            ("foreground_pixels", json!(coverage.foreground_pixels)),
                            (
                                "foreground_fraction",
                                json!(round3(coverage.foreground_fraction)),
                            ),
                            ("coverage_source", json!(coverage.source)),
                            ("profile", json!(required_profile.unwrap_or_default())),
                        ]),
                        (
                            "declared node projects into the captured frame but contains no measured non-background pixels",
                            "move it into frame, make it visible, change its material/background contrast, or remove the stale declaration",
                        ),
                    )
                    .with_region_from_screen("node", Some(target.handle), coverage.region),
                );
            }
        } else if has_draw || require_object_masks {
            checks.push(error_check(
                format!("{target_path}.visible_coverage"),
                "occlusion_depth",
                "visible_pixel_coverage_missing",
                Some(target.id.clone()),
                vec![target.handle],
                observed_pairs([("profile", json!(required_profile.unwrap_or_default()))]),
                (
                    "declared node has no viewport-clipped projected region for visible-pixel coverage",
                    "move it into the captured frame or remove the stale declaration",
                ),
            ));
        } else {
            checks.push(skip_check(
                format!("{target_path}.visible_coverage"),
                "occlusion_depth",
                "visible_coverage_not_applicable",
                SceneCompositionStatusV1::NotApplicable,
                Some(target.id.clone()),
                vec![target.handle],
                (
                    "visible coverage is not applicable because the node did not draw",
                    "fix the presence failure first",
                ),
            ));
        }
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

    checks.extend(super::import_roots::composition_import_checks(
        &input,
        required_profile,
        require_object_masks,
    ));

    checks
}

pub(super) fn required_object_profile<'a>(
    recipe: &'a SceneRecipeV1,
    expect: Option<&'a SceneRecipeExpectV1>,
) -> Option<&'a str> {
    if let Some(profile) = expect.and_then(|expect| {
        expect
            .expect_quality
            .as_ref()
            .map(|quality| quality.profile.as_str())
    }) {
        return Some(profile);
    }
    if let Some(profile) = recipe
        .render
        .as_ref()
        .and_then(|render| render.profile.as_deref())
        && RenderQualityProfile::parse(profile).is_some()
    {
        return Some(profile);
    }
    if recipe.photo.as_ref().is_some_and(|photo| {
        matches!(
            photo.intent.as_str(),
            "camera_behavior" | "camera-behavior" | "product_hero" | "product-hero"
        )
    }) {
        return Some("photo_product");
    }
    let auto_exposure_preset = recipe
        .render
        .as_ref()
        .and_then(|render| render.auto_exposure.as_ref())
        .map(|auto_exposure| match auto_exposure {
            SceneRecipeAutoExposureV1::Preset(preset) => preset.as_str(),
            SceneRecipeAutoExposureV1::Config { preset, .. } => preset.as_str(),
        });
    if matches!(auto_exposure_preset, Some("product_studio")) {
        return Some("product");
    }
    None
}

pub(super) fn owned_draw_handles(
    manifest: &SceneRecipeBuildV1,
    label_regions: &BTreeMap<u64, CaptureScreenRegion>,
    overlay_handles: impl Iterator<Item = Option<u64>>,
) -> BTreeSet<u64> {
    let mut owned_handles = super::helpers::declared_draw_handles(manifest);
    owned_handles.extend(label_regions.keys().copied());
    owned_handles.extend(overlay_handles.flatten());
    owned_handles
}
