use crate::app::prelude::*;

#[test]
fn c11_doctor_rejects_prepare_that_polls_before_the_device_loss_guard() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/c11-terminal-device-loss");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/diagnostics.rs",
        "src/diagnostics/help.rs",
        "src/render/surface.rs",
        "src/render/prepare_lifecycle.rs",
        "tests/c11_device_loss_recovery.rs",
        "src/browser_probe/probes.rs",
        "tests/browser/m6_rust_wasm_renderer_probe.js",
        "package.json",
        "docs/lifecycle.md",
        "docs/browser.md",
        "docs/errors.md",
        "CHANGELOG.md",
        "docs/release-notes/v1.8.0.md",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("C11 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("C11 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_c11_terminal_device_loss_contracts(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let prepare = fixture_root.join("src/render/prepare_lifecycle.rs");
    let source = fs::read_to_string(&prepare).expect("C11 prepare source reads");
    let mut mutated = source.replacen("self.prepare_device_ready()?;", "", 1);
    mutated.push_str("\nself.prepare_device_ready()?;\n");
    assert_ne!(
        source, mutated,
        "C11 mutation must poll the dead device first"
    );
    fs::write(prepare, mutated).expect("C11 prepare mutation writes");
    findings.clear();
    check_c11_terminal_device_loss_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "C11-TERMINAL-DEVICE-LOSS" && finding.message.contains("before polling")
        }),
        "polling before the device-loss guard must fail doctor: {findings:?}"
    );
}
