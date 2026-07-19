use crate::app::prelude::*;

pub(crate) fn check_fr05_capture_sequence_contracts(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "FR05-CAPTURE-SEQUENCE";
    let required: &[(&str, &[&str])] = &[
        (
            "src/bin/scena/recipe/capture_sequence.rs",
            &[
                "scena.capture_sequence_result.v1",
                "CanonicalView::Front",
                "CanonicalView::Top",
                "CanonicalView::Right",
                "CanonicalView::Isometric",
                "TAU * sample_index as f32 / sample_count as f32",
                "host.seek_animation(handle, f64::from(time_seconds))",
                "duration_seconds * sample_index as f32 / (sample_count - 1) as f32",
                "if total_frames > MAX_SEQUENCE_FRAMES",
                "png_frames_and_contact_sheet",
            ],
        ),
        (
            "src/bin/scena/recipe/capture_sequence/output.rs",
            &[
                "host.set_camera(camera)",
                "host.prepare()",
                "host.render()",
                ".capture()",
                "compose_contact_sheet_rgba8(&sheet_frames, columns",
                "capture_payload_fnv1a64",
                "safe_file_label(label)",
                "CONTACT_SHEET_TILE_MAX_DIMENSION: u32 = 192",
                "contact_sheet_thumbnail(&capture)",
                "frames[tile.index][\"capture\"][\"payload\"][\"fnv1a64\"]",
            ],
        ),
        (
            "src/bin/scena/recipe/capture_sequence/view.rs",
            &[
                "pub(in crate::scena_recipe) struct SubjectBounds",
                "Self::Front => (0.0, 0.0)",
                "Self::Top => (0.0, FRAC_PI_2 - 0.017_453_292)",
                "Self::Right => (FRAC_PI_2, 0.0)",
                "Self::Isometric => (FRAC_PI_4, (1.0_f32 / 3.0_f32.sqrt()).asin())",
            ],
        ),
        (
            "src/bin/scena/recipe/cad_inspection/view.rs",
            &[
                "capture_sequence::view::subject_bounds",
                "CanonicalView, SubjectBounds",
                "CanonicalView::Top",
                "canonical.ideal_eye_direction()",
                "canonical.screen_up()",
            ],
        ),
        (
            "src/bin/scena/recipe/cad_inspection/image.rs",
            &[
                "compose_contact_sheet_rgba8(&frames, columns",
                "write_png_rgba8(path, sheet.width, sheet.height, &sheet.rgba8)",
            ],
        ),
        (
            "src/bin/scena/recipe/capture_shared.rs",
            &[
                "fn compose_contact_sheet_rgba8",
                ".checked_mul(columns)",
                ".checked_mul(height as usize)",
                "fn write_png_rgba8",
            ],
        ),
        (
            "src/bin/scena/help.rs",
            &[
                "recipe capture <recipe.json> --out-dir <dir>",
                "scena.capture_sequence_result.v1",
            ],
        ),
        (
            "src/schema_catalog.rs",
            &[
                "scena.capture_sequence_result.v1",
                "tests/assets/stable-contracts/capture_sequence_result.v1.json",
            ],
        ),
        (
            "docs/schema-contracts.md",
            &[
                "### `scena.capture_sequence_result.v1`",
                "front, top, right, isometric",
                "set_camera -> prepare -> render -> capture",
                "external GIF/video encoder",
            ],
        ),
    ];
    for (relative, needles) in required {
        require_contains(root, findings, RULE, relative, needles);
    }
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/fr05_capture_sequence.rs",
        &["fr05_recipe_capture_emits_canonical_turntable_and_clip_frames"],
    );
    require_contains(
        root,
        findings,
        RULE,
        "tests/fr05_capture_sequence.rs",
        &[
            "assert_ne!(hashes[4], hashes[5]",
            "assert_ne!(hashes[8], hashes[10]",
            "report[\"contact_sheet\"][\"png\"]",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "src/bin/scena/recipe/capture_sequence/output.rs",
        &[
            "capture_sequence_file_labels_cannot_escape_the_output_directory",
            "capture_sequence_contact_sheet_tiles_have_a_bounded_memory_footprint",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "src/bin/scena/recipe/capture_sequence.rs",
        &["capture_sequence_rejects_a_combined_frame_budget_above_the_limit"],
    );

    if let Ok(source) = fs::read_to_string(root.join("src/bin/scena/recipe/cad_inspection/view.rs"))
        && source.contains("struct SubjectBounds")
    {
        findings.push(Finding::new(
            RULE,
            "CAD inspection must reuse capture_sequence::view::SubjectBounds instead of restoring a parallel bounds implementation",
        ));
    }
}
