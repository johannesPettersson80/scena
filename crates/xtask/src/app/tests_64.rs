use crate::app::prelude::*;

#[test]
fn a06_doctor_rejects_ignoring_the_repair_target_again() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/a06-repair-target");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/bin/scena/scene_commands.rs",
        "src/bin/scena/doctor.rs",
        "src/bin/scena/help.rs",
        "tests/a06_repair_doctor_inputs.rs",
        "README.md",
        "docs/guides/llm-app-builder.md",
        "docs/schema-contracts.md",
        "docs/troubleshooting.md",
        ".codex/skills/scena-app-builder/SKILL.md",
        "CHANGELOG.md",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("A06 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("A06 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_a06_repair_and_doctor_inputs(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let command = fixture_root.join("src/bin/scena/scene_commands.rs");
    let source = fs::read_to_string(&command).expect("repair command source reads");
    let mutated = source.replacen("validate_repair_asset_input(&input.asset)?", "None", 1);
    assert_ne!(
        source, mutated,
        "A06 mutation must bypass target validation"
    );
    fs::write(command, mutated).expect("repair target mutation writes");
    findings.clear();
    check_a06_repair_and_doctor_inputs(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "A06-REPAIR-DOCTOR-INPUTS"
                && finding.message.contains("validate_repair_asset_input")
        }),
        "ignoring the repair target must fail doctor: {findings:?}",
    );
}
