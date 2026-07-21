use crate::app::prelude::*;

#[test]
fn a03_doctor_rejects_static_capability_output_presented_as_a_live_probe() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/a03-live-capabilities");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "Cargo.toml",
        "src/bin/scena.rs",
        "src/bin/scena/capabilities.rs",
        "src/bin/scena/help.rs",
        "src/diagnostics/capabilities.rs",
        "src/diagnostics/capabilities/capability_probe.rs",
        "src/render/gpu/lifecycle.rs",
        "tests/a03_capabilities_cli.rs",
        "tests/assets/stable-contracts/capability_report.v1.json",
        "tests/stable_contracts.rs",
        "docs/capabilities.md",
        "docs/guides/llm-app-builder.md",
        "CHANGELOG.md",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("A03 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("A03 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_a03_live_capability_discovery(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let command = fixture_root.join("src/render/gpu/lifecycle.rs");
    let source = fs::read_to_string(&command).expect("A03 probe source reads");
    let mutated = source.replacen(
        "source: \"live_wgpu_adapter\".to_owned()",
        "source: \"compiled_backend_table\".to_owned()",
        1,
    );
    assert_ne!(source, mutated, "A03 mutation must falsify live provenance");
    fs::write(command, mutated).expect("A03 command mutation writes");
    findings.clear();
    check_a03_live_capability_discovery(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "A03-LIVE-CAPABILITY-DISCOVERY"
                && finding.message.contains("live_wgpu_adapter")
        }),
        "static data presented as a live probe must fail doctor: {findings:?}",
    );
}
