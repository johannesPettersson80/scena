use crate::app::prelude::*;

#[test]
fn q03_doctor_rejects_release_visual_test_with_only_nonblack_oracle() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/q03-visual-oracle");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(fixture_root.join("tests")).expect("Q03 doctor fixture directory");
    let proof = fixture_root.join("tests/q99_visual_proof.rs");
    fs::write(
        &proof,
        "fn release_visual_proof(frame: &[u8]) { assert!(nonblack_pixel_count(frame) > 0); }",
    )
    .expect("weak Q03 fixture writes");
    let mut findings = Vec::new();

    crate::app::doctor_visual_release::check_feature_specific_visual_oracles(
        &fixture_root,
        &mut findings,
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "Q03-FEATURE-VISUAL-ORACLE"
                && finding.message.contains("tests/q99_visual_proof.rs")
                && finding.message.contains("only a nonblack")
        }),
        "release-named visual proof with only a nonblack count must fail closed: {findings:?}",
    );

    fs::write(
        &proof,
        "fn release_visual_proof(frame: &[u8]) { assert!(nonblack_pixel_count(frame) > 0); let metrics = foreground_metrics(frame); assert_eq!(metrics.component_count, 2); }",
    )
    .expect("strong Q03 fixture writes");
    findings.clear();
    crate::app::doctor_visual_release::check_feature_specific_visual_oracles(
        &fixture_root,
        &mut findings,
    );
    assert_eq!(findings, Vec::new());
}

#[test]
fn q03_doctor_requires_measurement_quality_proof_in_ci_and_release() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/q03-visual-workflows");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(fixture_root.join("tests")).expect("Q03 empty tests fixture");
    fs::create_dir_all(fixture_root.join(".github/workflows"))
        .expect("Q03 workflow fixture directory");
    for workflow in ["ci.yml", "release.yml"] {
        fs::write(
            fixture_root.join(".github/workflows").join(workflow),
            "cargo test\n",
        )
        .expect("weak Q03 workflow fixture writes");
    }
    let mut findings = Vec::new();

    crate::app::doctor_visual_release::check_feature_specific_visual_oracles(
        &fixture_root,
        &mut findings,
    );

    assert_eq!(
        findings
            .iter()
            .filter(|finding| finding.rule == "Q03-FEATURE-VISUAL-ORACLE")
            .count(),
        2,
        "both workflows must require the inspection-backed measurement/callout proof: {findings:?}",
    );
}
