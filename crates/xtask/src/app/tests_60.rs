use crate::app::prelude::*;

#[test]
fn a02_doctor_rejects_one_recipe_command_dropping_allow_root() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/a02-operator-roots");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/bin/scena/policy.rs",
        "src/bin/scena/args/inspection.rs",
        "src/bin/scena/recipe.rs",
        "src/bin/scena/scene_commands.rs",
        "src/bin/scena/input.rs",
        "src/bin/scena/output.rs",
        "src/bin/scena/help.rs",
        "src/scene/recipe/build.rs",
        "src/scene/recipe/build/sandbox.rs",
        "tests/a02_recipe_policy_cli.rs",
        "README.md",
        "docs/schema-contracts.md",
        "docs/troubleshooting.md",
        "CHANGELOG.md",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("A02 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("A02 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_a02_operator_recipe_roots(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let args = fixture_root.join("src/bin/scena/args/inspection.rs");
    let source = fs::read_to_string(&args).expect("A02 args source reads");
    let mutated = source.replacen(
        "push_allow_root(args, index, &mut allow_roots)?",
        "push_allow_root_removed(args, index, &mut allow_roots)?",
        1,
    );
    assert_ne!(source, mutated, "A02 mutation must remove one command hook");
    fs::write(args, mutated).expect("A02 args mutation writes");
    findings.clear();
    check_a02_operator_recipe_roots(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "A02-OPERATOR-RECIPE-ROOTS" && finding.message.contains("found 4")
        }),
        "dropping one command hook must fail doctor: {findings:?}",
    );
}
