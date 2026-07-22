use crate::app::prelude::*;

pub(crate) fn check_a09_feature_discoverability(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "A09-FEATURE-DISCOVERABILITY";

    for (path, needles) in [
        (
            "Cargo.toml",
            &[
                "default = []",
                "agent = [\"scene-host\"]",
                "scene-host = [\"inspection\"]",
            ][..],
        ),
        (
            "src/bin/scena.rs",
            &[
                "\"agent\": cfg!(feature = \"agent\")",
                "cargo install scena --features {feature}",
                "feature_required(\"recipe build\", \"agent\")",
                "feature_required(\"examples agent\", \"agent\")",
                "feature_required(\"verify interaction\", \"agent\")",
                "feature_required(\"inspect\", \"inspection\")",
            ][..],
        ),
        (
            "docs/specs/feature-ownership.json",
            &[
                "\"name\": \"agent\"",
                "\"kind\": \"feature-composition\"",
                "agent = [\\\"scene-host\\\"]",
                "agent_feature_enables_the_complete_self_verification_surface",
            ][..],
        ),
        (
            "README.md",
            &[
                "cargo install scena --features agent",
                "The default feature set remains empty",
                "`scene-host` | native/browser SceneHost facade; enables `inspection`",
            ][..],
        ),
        (
            "docs/getting-started.md",
            &[
                "cargo install scena --features agent",
                "one-step self-verification surface",
            ][..],
        ),
        (
            "docs/feature-flags.md",
            &[
                "`agent` | complete opt-in self-verification surface",
                "cargo add scena --features agent",
                "The default feature set is exactly empty",
            ][..],
        ),
        (
            "docs/api.md",
            &[
                "complete agent/self-verification build",
                "default builds remain feature-empty",
            ][..],
        ),
        (
            "docs/examples.md",
            &["cargo install scena --features agent"][..],
        ),
        (
            "docs/guides/llm-app-builder.md",
            &[
                "cargo install scena --features agent",
                "cargo run --bin scena --features agent -- <command>",
            ][..],
        ),
        (
            ".codex/skills/scena-app-builder/SKILL.md",
            &[
                "cargo install scena --features agent",
                "cargo run --bin scena --features agent -- <command>",
            ][..],
        ),
        (
            ".codex/skills/scena-app-builder/references/recipe-loop.md",
            &["cargo install scena --features agent"][..],
        ),
        ("CHANGELOG.md", &["Add an opt-in `agent` Cargo feature"][..]),
        (
            "docs/release-notes/v1.8.0.md",
            &["add the opt-in `agent` composition"][..],
        ),
    ] {
        require_contains(root, findings, RULE, path, needles);
    }

    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/a09_feature_discoverability.rs",
        &[
            "manifest_keeps_defaults_empty_and_declares_one_step_agent_composition",
            "agent_feature_enables_the_complete_self_verification_surface",
            "unavailable_agent_commands_name_one_installable_feature_remedy",
        ],
    );

    for path in [
        "docs/getting-started.md",
        "docs/guides/llm-app-builder.md",
        "docs/examples.md",
        ".codex/skills/scena-app-builder/SKILL.md",
        ".codex/skills/scena-app-builder/references/recipe-loop.md",
    ] {
        let Ok(source) = fs::read_to_string(root.join(path)) else {
            continue;
        };
        if source.contains("--features scene-host,inspection") {
            findings.push(Finding::new(
                RULE,
                format!(
                    "{path} redundantly requests scene-host,inspection; use the one-step agent feature"
                ),
            ));
        }
    }
}
