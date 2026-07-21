use crate::app::prelude::*;

#[test]
fn c12_doctor_rejects_a_second_recoverable_surface_retry() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/c12-surface-acquisition");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/render/gpu/surface_frame.rs",
        "src/render/gpu/draw.rs",
        "src/render/gpu/draw_surface.rs",
        "src/render/gpu/draw_surface_support.rs",
        "src/render/frame.rs",
        "src/render/frame/surface.rs",
        "src/diagnostics.rs",
        "src/diagnostics/stats.rs",
        "src/scene_host/reporting.rs",
        "README.md",
        "docs/lifecycle.md",
        "docs/platforms.md",
        "docs/errors.md",
        "docs/api.md",
        "CHANGELOG.md",
        "docs/release-notes/v1.8.0.md",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("C12 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("C12 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_full_review_surface_acquisition_contracts(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let policy = fixture_root.join("src/render/gpu/surface_frame.rs");
    let source = fs::read_to_string(&policy).expect("C12 policy source reads");
    let mutated = source.replace(
        "SurfaceAcquireAction::FailAfterRetry(status)",
        "SurfaceAcquireAction::ReconfigureAndRetry",
    );
    assert_ne!(
        source, mutated,
        "C12 mutation must permit an unbounded retry"
    );
    fs::write(policy, mutated).expect("C12 policy mutation writes");
    findings.clear();
    check_full_review_surface_acquisition_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "C12-SURFACE-ACQUISITION"
                && finding.message.contains("FailAfterRetry(status)")
        }),
        "unbounded recoverable surface retry must fail doctor: {findings:?}"
    );
}
