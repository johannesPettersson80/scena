use crate::app::prelude::*;

#[test]
fn c21_doctor_rejects_cad_inspection_without_oriented_studio_rig() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/c21-cad-lighting");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/bin/scena/recipe/cad_inspection/view.rs",
        "tests/scena_cli_recipe.rs",
        "docs/guides/llm-app-builder.md",
        "CHANGELOG.md",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("C21 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("C21 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_c21_cad_inspection_lighting(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let view = fixture_root.join("src/bin/scena/recipe/cad_inspection/view.rs");
    let source = fs::read_to_string(&view).expect("C21 view source reads");
    let mutated = source.replacen("\"kind\": \"studio_rig\"", "\"kind\": \"directional\"", 1);
    assert_ne!(source, mutated, "C21 mutation must remove studio rig kind");
    fs::write(view, mutated).expect("C21 view mutation writes");
    findings.clear();
    check_c21_cad_inspection_lighting(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "C21-CAD-INSPECTION-LIGHTING"
                && finding.message.contains("\"kind\": \"studio_rig\"")
        }),
        "removing the oriented rig must fail doctor: {findings:?}",
    );
}

#[test]
fn q01_doctor_rejects_subject_visibility_reason_contract_drift() {
    // ARCH-SUBJECT-VISIBILITY-REASONS: zero-visible subject diagnostics must
    // stay source-owned, documented, and pinned across all public declaration
    // sources.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/subject-visibility-reasons");
    let source_path = fixture_root.join("src/scene_host/composition/subject.rs");
    let test_path = fixture_root.join("tests/scena_cli_recipe.rs");
    let docs_path = fixture_root.join("docs/schema-contracts.md");
    fs::create_dir_all(source_path.parent().expect("source parent")).expect("source fixture dir");
    fs::create_dir_all(test_path.parent().expect("test parent")).expect("test fixture dir");
    fs::create_dir_all(docs_path.parent().expect("docs parent")).expect("docs fixture dir");
    fs::write(
        &source_path,
        [
            "subject_hidden",
            "subject_outside_viewport",
            "subject_behind_camera",
            "subject_degenerate_geometry",
            "subject_clipped_by_section_box",
            "subject_clipped_by_clipping_plane",
            "subject_transparent_unsupported",
            "subject_visible_mask_empty",
        ]
        .join("\n"),
    )
    .expect("subject source fixture");
    fs::write(
        &test_path,
        [
            "scena_recipe_render_verify_reports_zero_visible_subject_reason_codes",
            "scena_recipe_render_verify_reports_zero_visible_photo_and_focus_subject_reason_codes",
            "subject.photo_subject.visible_mask",
            "subject.render_depth_of_field_focus.visible_mask",
            "subject_hidden",
        ]
        .join("\n"),
    )
    .expect("subject test fixture");
    fs::write(
        &docs_path,
        [
            "subject_hidden",
            "subject_outside_viewport",
            "subject_behind_camera",
            "subject_degenerate_geometry",
            "subject_clipped_by_section_box",
            "subject_clipped_by_clipping_plane",
            "subject_transparent_unsupported",
            "subject_occluded",
            "subject_visible_mask_empty",
            "stale_subject_observation",
        ]
        .join("\n"),
    )
    .expect("subject docs fixture");
    let mut findings = Vec::new();

    check_module_boundaries(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-SUBJECT-VISIBILITY-REASONS"
                && finding.message.contains("subject_occluded")
        }),
        "doctor must reject a source/docs/test drift in zero-visible subject reason codes: {findings:?}",
    );
}
