use crate::app::prelude::*;

#[test]
fn a09_doctor_rejects_a_redundant_or_default_agent_feature() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/a09-feature-discoverability");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "Cargo.toml",
        "src/bin/scena.rs",
        "tests/a09_feature_discoverability.rs",
        "docs/specs/feature-ownership.json",
        "README.md",
        "docs/getting-started.md",
        "docs/feature-flags.md",
        "docs/api.md",
        "docs/examples.md",
        "docs/guides/llm-app-builder.md",
        ".codex/skills/scena-app-builder/SKILL.md",
        ".codex/skills/scena-app-builder/references/recipe-loop.md",
        "CHANGELOG.md",
        "docs/release-notes/v1.8.0.md",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("A09 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("A09 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_a09_feature_discoverability(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let manifest = fixture_root.join("Cargo.toml");
    let source = fs::read_to_string(&manifest).expect("manifest reads");
    let mutated = source.replacen(
        "agent = [\"scene-host\"]",
        "agent = [\"scene-host\", \"inspection\"]",
        1,
    );
    assert_ne!(
        source, mutated,
        "A09 mutation must add the redundant feature"
    );
    fs::write(manifest, mutated).expect("A09 mutation writes");
    findings.clear();
    check_a09_feature_discoverability(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "A09-FEATURE-DISCOVERABILITY"
                && finding.message.contains("agent = [\"scene-host\"]")
        }),
        "redundant agent composition must fail doctor: {findings:?}",
    );
}
