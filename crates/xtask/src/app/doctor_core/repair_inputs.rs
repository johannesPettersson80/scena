use crate::app::prelude::*;

pub(crate) fn check_a06_repair_and_doctor_inputs(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "A06-REPAIR-DOCTOR-INPUTS";

    for (path, needles) in [
        (
            "src/bin/scena/scene_commands.rs",
            &[
                "validate_repair_asset_input(&input.asset)?",
                "Assets::new().doctor_asset_path(asset)",
                "validate_repair_recipe_input(&input)?",
                "scene_host_manifest_from_resolved_recipe(input)",
            ][..],
        ),
        (
            "src/bin/scena/doctor.rs",
            &[
                "if input.is_recipe()",
                "return run_doctor_recipe(input)",
                "scene_host_manifest_from_resolved_recipe(&input)",
            ][..],
        ),
        (
            "src/bin/scena/help.rs",
            &[
                "the target asset is loaded through asset doctor",
                "a second positional target is invalid",
            ][..],
        ),
        (
            "README.md",
            &[
                "validates that target",
                "a second positional target is an argument error",
            ][..],
        ),
        (
            "docs/schema-contracts.md",
            &[
                "The positional target is not advisory",
                "Assets::doctor_asset_path",
            ][..],
        ),
        (
            "docs/troubleshooting.md",
            &[
                "scena repair target --from report.json",
                "not processed against an unchecked path",
            ][..],
        ),
        (
            "docs/guides/llm-app-builder.md",
            &[
                "The positional target is validated",
                "positional target is invalid",
            ][..],
        ),
        (
            ".codex/skills/scena-app-builder/SKILL.md",
            &[
                "The repair positional is an enforced target",
                "Never supply a second positional target",
            ][..],
        ),
        (
            "CHANGELOG.md",
            &["actually constrain the", "Recipe `doctor` routing"][..],
        ),
    ] {
        require_contains(root, findings, RULE, path, needles);
    }

    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/a06_repair_doctor_inputs.rs",
        &[
            "repair_validates_raw_asset_and_recipe_targets_before_planning",
            "repair_rejects_a_second_positional_target",
            "repair_command_help_explains_target_validation",
            "doctor_routes_valid_missing_malformed_and_policy_rejected_recipe_inputs",
        ],
    );
}
