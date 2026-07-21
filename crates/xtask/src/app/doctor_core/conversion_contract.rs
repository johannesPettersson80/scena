use crate::app::prelude::*;

pub(crate) fn check_a05_scena_convert_contract(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "A05-SCENA-CONVERT-CONTRACT";

    for (path, needles) in [
        (
            "src/bin/scena-convert.rs",
            &[
                "enum OutputMode",
                "\"--json\" => Some(OutputMode::Json)",
                "\"--human\" => Some(OutputMode::Human)",
                "converter_command(options).output()",
                "captured_diagnostics",
                "AssetConversionStatusV1::InvalidRequest",
                "AssetConversionStatusV1::ToolUnavailable",
                "AssetConversionStatusV1::ConversionFailed",
                "io::ErrorKind::BrokenPipe",
            ][..],
        ),
        (
            "src/assets/conversion.rs",
            &[
                "ASSET_CONVERSION_SCHEMA_V1",
                "pub struct AssetConversionReportV1",
                "pub enum AssetConversionStatusV1",
                "pub diagnostics: Vec<AssetConversionDiagnosticV1>",
            ][..],
        ),
        (
            "src/schema_catalog.rs",
            &["ASSET_CONVERSION_SCHEMA_V1", "asset_conversion.v1.json"][..],
        ),
        (
            "src/schema_catalog/fixtures.rs",
            &["scena.asset_conversion.v1", "asset_conversion.v1.json"][..],
        ),
        (
            "tests/assets/stable-contracts/asset_conversion.v1.json",
            &["scena.asset_conversion.v1", "fbx_to_gltf", "planned"][..],
        ),
        (
            "README.md",
            &["scena-convert --json", "scena-convert --human"][..],
        ),
        (
            "docs/assets.md",
            &["## FBX conversion CLI", "diagnostics"][..],
        ),
        (
            "docs/schema-contracts.md",
            &[
                "### `scena.asset_conversion.v1`",
                "conversion_failed",
                "tool_unavailable",
            ][..],
        ),
        (
            "docs/troubleshooting.md",
            &[
                "Converter progress corrupted JSON output",
                "scena-convert --json",
            ][..],
        ),
        (
            "CHANGELOG.md",
            &[
                "Put `scena-convert` under the stable",
                "`--human` explicitly",
            ][..],
        ),
    ] {
        require_contains(root, findings, RULE, path, needles);
    }

    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/a05_scena_convert_contracts.rs",
        &[
            "machine_mode_emits_one_stable_envelope_for_plan_and_argument_failure",
            "machine_mode_captures_tool_progress_warnings_and_failures_inside_the_envelope",
            "machine_mode_reports_an_unavailable_tool_in_the_same_envelope",
            "human_mode_is_explicit_and_never_prints_json",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/stable_contracts.rs",
        &["asset_conversion_golden_matches_live_schema_serialization"],
    );
}
