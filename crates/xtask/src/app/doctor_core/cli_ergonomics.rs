use crate::app::prelude::*;

pub(crate) fn check_a04_cli_ergonomics(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "A04-CLI-ERGONOMICS";

    for (path, needles) in [
        (
            "src/bin/scena.rs",
            &[
                "scena_help::command_help_json(&args)",
                "let mut outcome = success(help)",
                "apply_output_format(&mut outcome, output_format)",
                "examples agent list",
                "[--exit-code]",
                "if error.kind() == io::ErrorKind::BrokenPipe",
            ][..],
        ),
        (
            "src/bin/scena/help.rs",
            &[
                "pub(crate) fn command_help_json",
                "\"scope\": \"command\"",
                "scena examples agent list",
                "scena diff <before.recipe.json>",
                "[--exit-code]",
            ][..],
        ),
        (
            "src/bin/scena/examples_agent.rs",
            &[
                "template_catalog()",
                "deprecated template alias",
                "scena examples agent list",
            ][..],
        ),
        (
            "src/bin/scena/examples_agent/catalog.rs",
            &[
                "AGENT_TEMPLATE_CATALOG_SCHEMA_V1",
                "const TEMPLATE_SPECS",
                "product-configurator-starter",
            ][..],
        ),
        (
            "src/bin/scena/diff.rs",
            &[
                "exit_policy(args.exit_code)",
                "difference_exit_code(args.exit_code, equal)",
                "\"difference_is_failure\"",
                "\"report_only\"",
                "\"--exit-code\"",
            ][..],
        ),
        (
            "src/bin/scena/process_output_shared.rs",
            &[
                "IO_ERROR_EXIT_CODE",
                "buffered_line_writer_preserves_broken_pipe_and_other_errors",
            ][..],
        ),
        (
            "src/schema_catalog/agent_smoke.rs",
            &[
                "AGENT_TEMPLATE_CATALOG_SCHEMA_V1",
                "pub struct AgentTemplateCatalogV1",
                "pub aliases: Vec<String>",
            ][..],
        ),
        (
            "src/schema_catalog.rs",
            &[
                "scena.agent_template_catalog.v1",
                "agent_template_catalog.v1.json",
            ][..],
        ),
        (
            "src/schema_catalog/fixtures.rs",
            &[
                "scena.agent_template_catalog.v1",
                "agent_template_catalog.v1.json",
            ][..],
        ),
        (
            "tests/assets/stable-contracts/agent_template_catalog.v1.json",
            &[
                "\"product-configurator\"",
                "\"product-configurator-starter\"",
                "\"product_configurator\"",
            ][..],
        ),
        (
            "README.md",
            &[
                "scena examples agent list",
                "diff --help --json",
                "--exit-code",
            ][..],
        ),
        (
            "docs/examples.md",
            &[
                "scena.agent_template_catalog.v1",
                "Historical underscore spellings",
            ][..],
        ),
        (
            "docs/schema-contracts.md",
            &[
                "### `scena.agent_template_catalog.v1`",
                "exit_policy:\"report_only\"",
                "difference_is_failure",
            ][..],
        ),
        (
            "docs/troubleshooting.md",
            &[
                "Help or template discovery was treated as an error",
                "`--exit-code`",
            ][..],
        ),
        (
            "CHANGELOG.md",
            &[
                "per-command help request",
                "`scena examples agent list`",
                "`--exit-code`",
            ][..],
        ),
    ] {
        require_contains(root, findings, RULE, path, needles);
    }

    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/a04_cli_ergonomics.rs",
        &[
            "global_and_every_command_help_are_successful_stdout_json",
            "template_catalog_has_one_canonical_name_and_aliases_emit_migration_metadata",
            "diff_reports_inequality_as_data_unless_exit_code_mode_is_requested",
            "unknown_commands_keep_stdout_clean_and_emit_one_stderr_envelope",
            "broken_stdout_pipe_exits_successfully_without_stderr_noise",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/stable_contracts.rs",
        &["agent_template_catalog_golden_matches_live_schema_serialization"],
    );
}
