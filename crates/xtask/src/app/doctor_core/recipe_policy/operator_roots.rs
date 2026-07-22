use crate::app::prelude::*;

pub(crate) fn check_a02_operator_recipe_roots(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "A02-OPERATOR-RECIPE-ROOTS";

    for (path, needles) in [
        (
            "src/bin/scena/policy.rs",
            &[
                "pub(crate) fn effective_recipe_policy",
                "pub(crate) fn push_allow_root",
                "root.canonicalize()",
                "if !metadata.is_dir()",
                "policy.with_allowed_root(canonical)",
                "--allow-root applies only to scene-recipe resolution",
            ][..],
        ),
        (
            "src/scene/recipe/build.rs",
            &[
                "pub fn with_allowed_root",
                "allowed_root_operator_overrides",
            ][..],
        ),
        (
            "src/scene/recipe/build/sandbox.rs",
            &["path.canonicalize()", "canonical.starts_with(root)"][..],
        ),
        (
            "src/bin/scena/input.rs",
            &[
                "policy: scena::RecipeBuildPolicy",
                "let policy = input.policy.clone()",
            ][..],
        ),
        (
            "src/bin/scena/output.rs",
            &["add_recipe_policy_to_outcome", "\"policy\".to_owned()"][..],
        ),
        (
            "src/bin/scena/help.rs",
            &[
                "policy recipe [--allow-root <directory>]...",
                "\"result_field\": \"policy\"",
            ][..],
        ),
    ] {
        require_contains(root, findings, RULE, path, needles);
    }

    require_occurrences(
        root,
        findings,
        RULE,
        "src/bin/scena/args/inspection.rs",
        "push_allow_root(args, index, &mut allow_roots)?",
        5,
    );
    require_occurrences(
        root,
        findings,
        RULE,
        "src/bin/scena/recipe.rs",
        "push_allow_root(args, index, &mut allow_roots)?",
        2,
    );
    require_occurrences(
        root,
        findings,
        RULE,
        "src/bin/scena/scene_commands.rs",
        "effective_recipe_policy(&args.allow_roots, None)?",
        4,
    );

    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/a02_recipe_policy_cli.rs",
        &[
            "policy_recipe_discovers_repeatable_canonical_operator_roots",
            "allow_root_is_identical_across_recipe_aware_cli_commands",
            "allow_root_rejects_parent_traversal_and_symlink_escape_after_canonicalization",
        ],
    );

    for (path, needles) in [
        (
            "README.md",
            &[
                "There is no sandbox-disable flag",
                "scena policy recipe --allow-root",
            ][..],
        ),
        (
            "docs/schema-contracts.md",
            &[
                "Resource paths are canonicalized independently",
                "operator_override",
            ][..],
        ),
        (
            "docs/troubleshooting.md",
            &["a symlink or `..` traversal", "Repeat `--allow-root`"][..],
        ),
        (
            "CHANGELOG.md",
            &["repeatable, canonical `--allow-root <directory>`"][..],
        ),
    ] {
        require_contains(root, findings, RULE, path, needles);
    }
}

fn require_occurrences(
    root: &Path,
    findings: &mut Vec<Finding>,
    rule: &'static str,
    relative: &str,
    needle: &str,
    expected: usize,
) {
    let path = root.join(relative);
    let Ok(source) = fs::read_to_string(&path) else {
        findings.push(Finding::new(
            rule,
            format!("missing required file {relative}"),
        ));
        return;
    };
    let actual = source.matches(needle).count();
    if actual != expected {
        findings.push(Finding::new(
            rule,
            format!("{relative} must contain {expected} occurrences of '{needle}', found {actual}"),
        ));
    }
}
