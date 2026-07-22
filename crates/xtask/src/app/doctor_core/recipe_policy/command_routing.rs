use crate::app::prelude::*;

/// `C03-CANONICAL-RECIPE-COMMAND-ROUTING`: parsed recipes must never fall back
/// to first-import asset construction in command adapters. Every recipe-aware
/// command enters the policy-aware SceneHost recipe builder.
pub(crate) fn check_c03_canonical_recipe_command_routing(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "C03-CANONICAL-RECIPE-COMMAND-ROUTING";
    for (relative, needles) in [
        (
            "src/bin/scena/input.rs",
            &[
                "pub(crate) const fn is_recipe(&self)",
                "pub(crate) enum ResolvedRecipeBuild",
                "scene_host_build_from_resolved_recipe",
                "SceneHostCore::build_recipe_json(",
                "SceneHostCore::build_recipe_manifest_json(",
            ][..],
        ),
        (
            "src/bin/scena/scene_commands.rs",
            &[
                "if input.is_recipe()",
                "scene_host_build_from_resolved_recipe",
                "scene_host_manifest_from_resolved_recipe",
            ][..],
        ),
        (
            "src/bin/scena/verify.rs",
            &[
                "if input.is_recipe()",
                "scene_host_build_from_resolved_recipe",
            ][..],
        ),
        (
            "src/bin/scena/verify_animation.rs",
            &["if input.is_recipe()", "run_verify_recipe_animation"][..],
        ),
        (
            "src/bin/scena/verify_interaction.rs",
            &[
                "if input.is_recipe()",
                "scene_host_build_from_resolved_recipe",
            ][..],
        ),
        (
            "src/bin/scena/doctor.rs",
            &[
                "if input.is_recipe()",
                "scene_host_manifest_from_resolved_recipe",
            ][..],
        ),
        (
            "tests/scena_cli_recipe.rs",
            &[
                "imports_only_recipe_commands_build_every_import",
                "recipe_verifiers_resolve_capabilities_from_the_second_import",
                "recipe_commands_check_policy_for_every_import",
                "$.imports[1].uri",
            ][..],
        ),
    ] {
        require_contains(root, findings, RULE, relative, needles);
    }

    for relative in [
        "src/bin/scena/input.rs",
        "src/bin/scena/scene_commands.rs",
        "src/bin/scena/verify.rs",
        "src/bin/scena/verify_animation.rs",
        "src/bin/scena/verify_interaction.rs",
        "src/bin/scena/doctor.rs",
    ] {
        forbid_contains(
            root,
            findings,
            RULE,
            relative,
            &["has_scene_host_directives", ".imports.first()"],
        );
    }
}
