use crate::app::prelude::*;

#[test]
fn c17_doctor_rejects_helper_inclusion_default_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/c17-visible-bounds-framing");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/scene/view.rs",
        "src/scene/view_bounds.rs",
        "src/scene/framing.rs",
        "src/viewer/load_progress.rs",
        "src/viewer/interaction.rs",
        "src/scene_host/camera.rs",
        "tests/c17_visible_bounds_framing.rs",
        "tests/m7_interactive_viewer.rs",
        "docs/api.md",
        "docs/rendering.md",
        "docs/guides/migrating-from-threejs.md",
        "docs/specs/public-api.md",
        "README.md",
        "examples/camera_framing.rs",
        "tests/m5_release.rs",
        "CHANGELOG.md",
        "docs/release-notes/v1.8.0.md",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("C17 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("C17 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_c17_visible_bounds_framing_contract(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let policy = fixture_root.join("src/scene/framing.rs");
    let source = fs::read_to_string(&policy).expect("C17 framing source reads");
    let mutated = source.replacen("include_helpers: false", "include_helpers: true", 1);
    assert_ne!(
        source, mutated,
        "C17 mutation must include helpers by default"
    );
    fs::write(policy, mutated).expect("C17 framing mutation writes");
    findings.clear();
    check_c17_visible_bounds_framing_contract(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "C17-VISIBLE-BOUNDS-FRAMING"
                && finding.message.contains("include_helpers: false")
        }),
        "helper-inclusion regression must fail doctor: {findings:?}",
    );
}
