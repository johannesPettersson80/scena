use crate::app::prelude::*;

#[test]
fn c07_doctor_rejects_post_driven_or_linear_byte_readback() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/c07-target-transfer");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/render/gpu/draw_common.rs",
        "src/render/gpu/post/resources.rs",
        "src/render/gpu/browser_readback.rs",
        "src/render/post_tests.rs",
        "src/browser_probe/workflows.rs",
        "tests/browser/m6_rust_wasm_renderer_probe.js",
        "src/diagnostics/capabilities.rs",
        "src/diagnostics/capabilities/color_formats.rs",
        "docs/specs/color-contract.md",
        "docs/browser.md",
        "docs/rendering.md",
        "docs/capabilities.md",
        "CHANGELOG.md",
        "docs/release-notes/v1.8.0.md",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("C07 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("C07 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_c07_target_transfer_contract(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let draw_common = fixture_root.join("src/render/gpu/draw_common.rs");
    let source = fs::read_to_string(&draw_common).expect("C07 draw-common source reads");
    let mutated = source.replace(
        "target_color_management_uniform",
        "post_color_management_uniform",
    );
    assert_ne!(
        source, mutated,
        "C07 mutation must restore post-driven transfer"
    );
    fs::write(draw_common, mutated).expect("C07 draw-common mutation writes");
    findings.clear();
    check_c07_target_transfer_contract(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "C07-TARGET-DRIVEN-COLOR-TRANSFER"
                && (finding.message.contains("target_color_management_uniform")
                    || finding.message.contains("post_color_management_uniform"))
        }),
        "restoring post-driven transfer must fail doctor: {findings:?}"
    );
}
