use crate::app::prelude::*;

#[test]
fn a05_doctor_rejects_leaking_converter_output_into_machine_stdout() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/a05-scena-convert");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/bin/scena-convert.rs",
        "src/bin/scena/process_output_shared.rs",
        "src/assets/conversion.rs",
        "src/assets.rs",
        "src/lib.rs",
        "src/schema_catalog.rs",
        "src/schema_catalog/fixtures.rs",
        "tests/a05_scena_convert_contracts.rs",
        "tests/assets/stable-contracts/asset_conversion.v1.json",
        "tests/stable_contracts.rs",
        "README.md",
        "docs/assets.md",
        "docs/schema-contracts.md",
        "docs/troubleshooting.md",
        "CHANGELOG.md",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("A05 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("A05 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_a05_scena_convert_contract(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let binary = fixture_root.join("src/bin/scena-convert.rs");
    let source = fs::read_to_string(&binary).expect("converter source reads");
    let mutated = source.replacen(
        "converter_command(options).output()",
        "converter_command(options).status().map(|status| std::process::Output { status, stdout: Vec::new(), stderr: Vec::new() })",
        1,
    );
    assert_ne!(source, mutated, "A05 mutation must remove captured output");
    fs::write(binary, mutated).expect("converter mutation writes");
    findings.clear();
    check_a05_scena_convert_contract(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "A05-SCENA-CONVERT-CONTRACT"
                && finding
                    .message
                    .contains("converter_command(options).output()")
        }),
        "machine-mode child output leakage must fail doctor: {findings:?}",
    );
}
