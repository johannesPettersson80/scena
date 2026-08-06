use crate::app::prelude::*;

pub(crate) fn check_scene_composition_quality_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-SCENE-COMPOSITION",
        "src/scene_host/composition.rs",
        &[
            "composition_report",
            "composition_object_checks",
            "unexpected_draw_output",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-COMPOSITION",
        "src/scene_host/composition/objects.rs",
        &[
            "composition_object_checks",
            "SceneCompositionStatusV1::SkippedNoDeclaredIntent",
            "SceneCompositionStatusV1::NotApplicable",
            "material_base_color_available",
            "visible_pixel_coverage_available",
            "visible_pixel_coverage_missing",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-COMPOSITION",
        "src/scene_host/composition/object_framing.rs",
        &[
            "object_framing_check",
            "subject_fit_sane",
            "subject_too_small_in_frame",
            "subject_too_large_in_frame",
            "ObjectFramingThresholds",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-COMPOSITION",
        "src/scene_host/composition/object_pixels.rs",
        &[
            "object_pixel_quality_check",
            "subject_exposure_sane",
            "subject_black_crushed",
            "subject_blown_out",
            "subject_salience_too_low",
            "OBJECT_LOW_CLIP_LUMA",
            "ObjectPixelThresholds",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-COMPOSITION",
        "src/scene_host/composition/subject.rs",
        &[
            "semantic_color_frame_agreement",
            "subject_color_frame_agreement_below_min",
            "heuristic_local_semantic_boundary",
            "semantic_color_frame_agreement_distinguishes_a_drawn_subject_from_a_stale_aov_mask",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-COMPOSITION",
        "src/render/prepare_lifecycle.rs",
        &[
            "let semantic_aov_target = self.target;",
            "semantic_aov_target,",
        ],
    );
    for path in [
        "src/render/gpu/prepare_resources.rs",
        "src/render/gpu/prepare_resources_wasm.rs",
    ] {
        require_contains(
            root,
            findings,
            "ARCH-SCENE-COMPOSITION",
            path,
            &[
                "semantic_aov_target: RasterTarget",
                "target: semantic_aov_target",
            ],
        );
    }
    require_contains(
        root,
        findings,
        "ARCH-SCENE-COMPOSITION",
        "tests/fr06_semantic_aov.rs",
        &["fr06_headless_gpu_semantic_aov_matches_cpu_center_truth"],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-COMPOSITION",
        "tests/scene_host.rs",
        &["photo_candidate_framing_centers_the_projected_subject_bounds_used_by_the_gate"],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-SCENE-COMPOSITION",
        "src/scene_host/photo.rs",
        &["photographic_visual_center"],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-COMPOSITION",
        "src/scene_host/composition/object_textures.rs",
        &[
            "object_texture_result_check",
            "texture_result_visible",
            "texture_result_flat",
            "texture_result_missing",
            "TextureResultThresholds",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-COMPOSITION",
        "src/scene_host/composition/grid.rs",
        &[
            "composition_grid_ownership_checks",
            "ground_candidate_handles",
            "grid_floor_output_owned",
            "grid_floor_output_missing",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-COMPOSITION",
        "src/scene_host/composition/annotations.rs",
        &[
            "composition_callout_checks",
            "composition_measurement_checks",
            "callout_target_attached",
            "callout_overlay_output_projected",
            "callout_overlay_output_missing",
            "measurement_overlay_output_projected",
            "measurement_overlay_output_missing",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-COMPOSITION",
        "src/scene_host/composition/overlays.rs",
        &[
            "composition_overlay_collision_checks",
            "overlay_line_regions_by_handle",
            "overlay_label_intersects_line",
            "overlay_label_clear_of_lines",
            "overlay_label_intersects_label",
            "overlay_label_clear_of_labels",
            "overlay_label_clipped_by_viewport",
            "overlay_label_inside_viewport",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-COMPOSITION",
        "src/scene_host/composition/placement.rs",
        &[
            "composition_ground_contact_checks",
            "expect_grounded",
            "ground_contact_present",
            "ground_contact_missing",
            "ground_target_unresolved",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-COMPOSITION",
        "src/scene_host/composition/helper_layer.rs",
        &[
            "composition_helper_layer_checks",
            "expect_helper_occluded",
            "helper_layer_occluded_by_subject",
            "helper_layer_overdraws_subject",
            "helper_occlusion_target_unresolved",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-COMPOSITION",
        "src/scene_host/composition/object_depth.rs",
        &[
            "composition_object_depth_order_checks",
            "expect_occlusion",
            "object_depth_order_satisfied",
            "object_depth_order_mismatch",
            "object_depth_order_color_ambiguous",
            "object_depth_order_target_unresolved",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-COMPOSITION",
        "src/scene_host/composition/clipping.rs",
        &[
            "composition_clipping_checks",
            "expect_clipping",
            "clipping_plane_count_satisfied",
            "clipping_plane_count_mismatch",
            "section_box_active",
            "section_box_missing",
            "section_box_inversion_satisfied",
            "section_box_inversion_mismatch",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-COMPOSITION",
        "src/scene_host/composition/state.rs",
        &[
            "composition_state_checks",
            "expect_state",
            "material_variant_state_satisfied",
            "material_variant_state_mismatch",
            "state_import_missing",
            "state_import_not_inspected",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-COMPOSITION",
        "src/scene_host/composition/checks.rs",
        &[
            "SceneCompositionStatusV1::Checked",
            "SceneCompositionStatusV1::Failed",
            "OPTIONAL_COMPOSITION_SKIP_SEVERITY",
            "severity: OPTIONAL_COMPOSITION_SKIP_SEVERITY.to_owned()",
            "fix_hint",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-COMPOSITION",
        "tests/scena_cli_recipe.rs",
        &[
            "optional composition skips must stay informational",
            "grounding_intent_not_declared",
            "scena_recipe_render_verify_rejects_ambiguous_object_depth_colors_on_cpu_and_gpu",
            "object_depth_order_color_ambiguous",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-COMPOSITION",
        "tests/assets/stable-contracts/scene_composition.v1.json",
        &[
            "\"schema\": \"scena.scene_composition.v1\"",
            "\"status\": \"checked\"",
            "\"status\": \"failed\"",
            "\"code\": \"declared_node_not_drawn\"",
            "\"code\": \"visible_pixel_coverage_available\"",
            "\"code\": \"subject_fit_sane\"",
            "\"code\": \"subject_exposure_sane\"",
            "\"code\": \"texture_result_visible\"",
            "\"code\": \"callout_target_attached\"",
            "\"code\": \"callout_overlay_output_projected\"",
            "\"code\": \"grid_floor_output_owned\"",
            "\"code\": \"measurement_overlay_output_projected\"",
            "\"code\": \"overlay_label_clear_of_lines\"",
            "\"code\": \"overlay_label_clear_of_labels\"",
            "\"code\": \"overlay_label_inside_viewport\"",
            "\"code\": \"ground_contact_present\"",
            "\"code\": \"helper_layer_occluded_by_subject\"",
            "\"code\": \"object_depth_order_satisfied\"",
            "\"code\": \"backend_expectation_satisfied\"",
            "\"code\": \"clipping_plane_count_satisfied\"",
            "\"code\": \"section_box_active\"",
            "\"code\": \"section_box_inversion_satisfied\"",
            "\"code\": \"material_variant_state_satisfied\"",
            "\"fix_hint\"",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-COMPOSITION",
        "docs/schema-contracts.md",
        &[
            "### `scena.scene_composition.v1`",
            "`skipped_no_backend_support`",
            "Informational skipped",
            "`declared_node_not_drawn`",
            "`material_base_color_available`",
            "`visible_pixel_coverage_available`",
            "`visible_pixel_coverage_missing`",
            "`subject_fit_sane`",
            "`subject_too_small_in_frame`",
            "`subject_exposure_sane`",
            "`subject_black_crushed`",
            "`subject_blown_out`",
            "`subject_salience_too_low`",
            "`texture_result_visible`",
            "`texture_result_flat`",
            "`texture_result_missing`",
            "`callout_target_attached`",
            "`callout_overlay_output_projected`",
            "`grid_floor_output_owned`",
            "`measurement_overlay_output_projected`",
            "`overlay_label_clear_of_lines`",
            "`overlay_label_intersects_line`",
            "`overlay_label_clear_of_labels`",
            "`overlay_label_intersects_label`",
            "`overlay_label_inside_viewport`",
            "`overlay_label_clipped_by_viewport`",
            "`ground_contact_present`",
            "`ground_contact_missing`",
            "`helper_layer_occluded_by_subject`",
            "`helper_layer_overdraws_subject`",
            "`object_depth_order_satisfied`",
            "`object_depth_order_mismatch`",
            "`object_depth_order_color_ambiguous`",
            "`backend_expectation_satisfied`",
            "`backend_expectation_mismatch`",
            "`clipping_plane_count_satisfied`",
            "`clipping_plane_count_mismatch`",
            "`section_box_active`",
            "`section_box_missing`",
            "`section_box_inversion_satisfied`",
            "`section_box_inversion_mismatch`",
            "`material_variant_state_satisfied`",
            "`material_variant_state_mismatch`",
        ],
    );
}
