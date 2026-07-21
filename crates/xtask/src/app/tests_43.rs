use crate::app::prelude::*;

#[test]
fn c06_doctor_rejects_silent_or_non_presentable_viewer_defaults() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/c06-presentable-defaults");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/viewer.rs",
        "src/viewer/load_progress.rs",
        "src/diagnostics/diagnostic.rs",
        "tests/first_render_api.rs",
        "tests/scena_cli_recipe.rs",
        "examples/glb_model_viewer.rs",
        "tests/examples_visual_proof.rs",
        "README.md",
        "docs/getting-started.md",
        "docs/guides/easy-scene-setup.md",
        "docs/rendering.md",
        "docs/examples.md",
        "docs/errors.md",
        "docs/troubleshooting.md",
        "docs/api.md",
        "CHANGELOG.md",
        "docs/release-notes/v1.8.0.md",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("C06 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("C06 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_c06_presentable_viewer_defaults(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let viewer = fixture_root.join("src/viewer.rs");
    let source = fs::read_to_string(&viewer).expect("C06 viewer source reads");
    let mutated = source.replace("fallback_lighting: true", "fallback_lighting: false");
    assert_ne!(source, mutated, "C06 mutation must disable the fallback");
    fs::write(viewer, mutated).expect("C06 viewer mutation writes");
    findings.clear();
    check_c06_presentable_viewer_defaults(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "C06-PRESENTABLE-VIEWER-DEFAULTS"
                && finding.message.contains("fallback_lighting: true")
        }),
        "disabling the presentable viewer fallback must fail doctor: {findings:?}"
    );
}
