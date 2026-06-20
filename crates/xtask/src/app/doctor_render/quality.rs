use crate::app::prelude::*;

pub(crate) fn check_render_quality_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-RENDER-QUALITY",
        "src/render/quality.rs",
        &[
            "RENDER_QUALITY_SCHEMA_V1",
            "label_quality_known_bad_fails_exact_reason_and_good_passes",
            "label_quality_known_bad_gpu_eroded_fixture_fails_exact_reason",
            "label-known-bad-eroded.ppm",
            "label-known-bad-gpu-eroded.ppm",
            "label-known-good-antialiased.ppm",
            "label_ink_isolation",
            "label_missing_antialiasing",
            "baseline_quality_fixtures_cover_blank_blown_out_and_tiny_codes",
            "severe_black_crush",
            "severe_blown_out",
            "blank_frame",
            "subject_tiny_in_frame",
            "opt_in_quality_fixtures_cover_exposure_contrast_and_noise_codes",
            "low_clip_fraction_too_high",
            "high_clip_fraction_too_high",
            "contrast_too_flat",
            "edge_energy_too_low",
            "noise_outlier_fraction_too_high",
            "reference_quality_fixtures_cover_abs_delta_e_and_ssim_metrics",
            "grayscale_ssim_matches_reference_anchors",
            "reference_quality_metrics",
            "ssim_grayscale",
            "label_quality_known_bad_bitmap_fixture_fails_antialiasing_reason",
            "label-known-bad-bitmap.ppm",
            "line_quality_known_bad_fails_exact_reason_and_good_passes",
            "line-known-bad-aliased.ppm",
            "line-known-good-antialiased.ppm",
            "line_missing_antialiasing",
            "line_not_straight",
            "geometry_edge_quality_known_bad_fails_exact_reason_and_good_passes",
            "geometry_missing_antialiasing",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-QUALITY",
        "tests/label_text.rs",
        &[
            "label_text_truetype_preserves_antialiasing_coverage_on_cpu",
            "label_text_truetype_preserves_antialiasing_coverage_on_headless_gpu",
            "label_text_truetype_atlas_matches_cpu_and_headless_gpu_with_tolerance",
            "truetype_aa_label_region",
            "frame_delta_in_bounds",
            "full label region",
            "intermediate_gray_pixel_count",
            "truetype-label-aa-cpu",
            "truetype-label-aa-gpu",
            "truetype-label-atlas-parity-cpu-crop",
            "truetype-label-atlas-parity-gpu-crop",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-QUALITY",
        "tests/scena_cli_recipe.rs",
        &[
            "scena_recipe_render_verify_reports_reference_quality_failure",
            "reference_ssim_too_low",
            "wrong-reference-ssim",
            "scena_recipe_render_verify_passes_quality_per_line_region_on_cpu_and_gpu",
            "expect_quality.line.segment",
            "scena_recipe_render_verify_fails_geometry_edge_quality_without_sample_aa_on_cpu_and_gpu",
            "scena_recipe_render_verify_passes_geometry_edge_quality_with_msaa4_on_gpu",
            "scena_recipe_render_supersample_changes_curve_grid_and_specular_pixels_on_cpu_and_gpu",
            "scena_recipe_render_gpu_reconstruction_widens_dashboard_bar_and_grid_edges_without_haloing",
            "dashboard-bar-reconstruction-metrics.json",
            "dashboard-grid-reconstruction-metrics.json",
            "scena_recipe_render_gpu_msaa_grid_floor_is_occluded_by_object",
            "red_grid_pixels_inside_object_interior",
            "scena_recipe_render_verify_emits_composition_report_for_declared_nodes",
            "\"schema\": \"scena.scene_recipe.v1\"",
            "declared_node_not_drawn",
            "source\"] == \"composition\"",
            "\"supersample\": supersample",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-QUALITY",
        "src/render/cpu_render.rs",
        &[
            "downsample_rgba8_reconstruction_filter",
            "rgba8_supersample_downsample_averages_rgb_in_linear_light",
            "srgb8_to_linear",
            "ReconstructionFilter::Gaussian",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-COMPOSITION",
        "src/scene_host/composition.rs",
        &[
            "composition_report",
            "SceneCompositionStatusV1::SkippedNoDeclaredIntent",
            "SceneCompositionStatusV1::SkippedNoBackendSupport",
            "unexpected_draw_output",
            "object_mask_not_available",
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
            "fix_hint",
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
            "\"status\": \"skipped_no_backend_support\"",
            "\"code\": \"declared_node_not_drawn\"",
            "\"fix_hint\"",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-QUALITY",
        "tests/assets/stable-contracts/render_quality.v1.json",
        &[
            "\"schema\": \"scena.render_quality.v1\"",
            "\"code\": \"label_ink_isolation\"",
            "\"code\": \"line_missing_antialiasing\"",
            "\"code\": \"geometry_missing_antialiasing\"",
            "\"fix_hint\"",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-QUALITY",
        "docs/schema-contracts.md",
        &[
            "### `scena.render_quality.v1`",
            "`label_ink_isolation`",
            "`label_missing_antialiasing`",
            "`line_missing_antialiasing`",
            "`geometry_missing_antialiasing`",
            "`reference_ssim_too_low`",
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
            "`declared_node_not_drawn`",
            "`object_mask_not_available`",
        ],
    );
    check_label_atlas_replaced_cell_primitives(root, findings);
}

fn check_label_atlas_replaced_cell_primitives(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-LABEL-ATLAS",
        "src/render/prepare/labels.rs",
        &[
            "prepare_label_atlas",
            "PreparedLabelAtlas",
            "PreparedLabelQuad",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-LABEL-ATLAS",
        "src/render/gpu/labels.rs",
        &[
            "manual_bilinear_coverage",
            "scena.gpu_labels.atlas_texture",
            "scena.gpu_labels.pipeline",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-LABEL-ATLAS",
        "src/render/cpu.rs",
        &[
            "write_label_overlay_pixel",
            "label_overlay_aces_tonemap",
            "blend_display_source_over",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-LABEL-ATLAS",
        "src/scene/labels/font.rs",
        &[
            "DEFAULT_LABEL_FONT_BYTES",
            "include_bytes!(\"fonts/LiberationSans-Regular.ttf\")",
            "default_label_font_face",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-LABEL-ATLAS",
        "src/render/cpu_strokes.rs",
        &[
            "draw_strokes_cpu",
            "stroke_coverage",
            "write_label_overlay_pixel",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-LABEL-ATLAS",
        "src/render/gpu/strokes.wgsl",
        &["stroke_coverage", "distance_px", "half_width_px"],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-LABEL-ATLAS",
        "src/scene/labels.rs",
        &[
            "LabelGlyphCell",
            "glyph_cells",
            "LabelFont::Bitmap",
            "pub fn bitmap",
            "bitmap_glyph_rasters",
            "bitmap_label_metrics",
        ],
    );
    if root.join("src/scene/labels/bitmap.rs").exists() {
        findings.push(Finding::new(
            "ARCH-LABEL-ATLAS",
            "src/scene/labels/bitmap.rs still exists; delete the removed 5x7 bitmap label path",
        ));
    }
    forbid_contains(
        root,
        findings,
        "ARCH-LABEL-ATLAS",
        "src/render/prepare/labels.rs",
        &["append_label_primitives", "push_prepared_label_primitive"],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-LABEL-ATLAS",
        "src/browser_probe/workflows.rs",
        &["browser-bitmap-labels", "bitmap-5x7"],
    );
}
