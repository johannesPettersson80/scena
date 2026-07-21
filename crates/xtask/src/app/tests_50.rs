use crate::app::prelude::*;

#[test]
fn full_review_c13_doctor_rejects_removed_near_plane_clipping() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/full-review-c13-cpu-clipping");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/render/cpu_geometry.rs",
        "src/render/cpu.rs",
        "src/render/cpu_transmission.rs",
        "src/render/semantic_aov.rs",
        "src/render/cpu_render/row_bands.rs",
        "tests/c13_cpu_depth_clipping.rs",
        "tests/c13_depth_clipping_parity.rs",
        "README.md",
        "docs/headless-rendering.md",
        "docs/rendering.md",
        "CHANGELOG.md",
        "docs/release-notes/v1.8.0.md",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("C13 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("C13 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_full_review_cpu_depth_clipping_contracts(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let clipping = fixture_root.join("src/render/cpu_geometry.rs");
    let source = fs::read_to_string(&clipping).expect("C13 clipping source reads");
    let mutated = source.replace(
        "clip_depth_plane(&polygon, polygon_len, &mut scratch, near, true)",
        "clip_depth_plane(&polygon, polygon_len, &mut scratch, far, false)",
    );
    assert_ne!(
        source, mutated,
        "C13 mutation must remove the near-plane clip"
    );
    fs::write(clipping, mutated).expect("C13 clipping mutation writes");
    findings.clear();
    check_full_review_cpu_depth_clipping_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "FULL-REVIEW-C13-CPU-DEPTH-CLIPPING"
                && finding.message.contains("near, true")
        }),
        "removing near-plane clipping must fail doctor: {findings:?}"
    );
}
