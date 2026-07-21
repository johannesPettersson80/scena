pub(super) struct SchemaEntryRow {
    pub(super) schema: &'static str,
    pub(super) owner_module: &'static str,
    pub(super) summary: &'static str,
    pub(super) feature_flag: Option<&'static str>,
    pub(super) fixture_path: Option<&'static str>,
}

pub(super) fn operational_schema_entry_rows() -> &'static [SchemaEntryRow] {
    &[
        SchemaEntryRow {
            schema: "scena.release.findings.v1",
            owner_module: "xtask/release",
            summary: "Independent release-review findings register bound to one source commit.",
            feature_flag: None,
            fixture_path: None,
        },
        SchemaEntryRow {
            schema: "scena.release.staging.v1",
            owner_module: "xtask/release",
            summary: "Release artifact staging metadata kept separate from source evidence provenance.",
            feature_flag: None,
            fixture_path: None,
        },
        SchemaEntryRow {
            schema: "scena.release_readiness.v1",
            owner_module: "xtask/release",
            summary: "Fail-closed staged release-evidence validation result with resolved root and artifact counts.",
            feature_flag: None,
            fixture_path: None,
        },
        SchemaEntryRow {
            schema: "scena.recipe_patch.v1",
            owner_module: "scene/recipe",
            summary: "Source-digest-bound placement update with complete canonical recipe and semantic change summary.",
            feature_flag: None,
            fixture_path: Some("tests/assets/stable-contracts/recipe_patch.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.vocab.v1",
            owner_module: "vocabulary",
            summary: "Closed renderer and recipe vocabularies with stable owners and versions.",
            feature_flag: None,
            fixture_path: Some("tests/assets/stable-contracts/vocab.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.recipe_policy.v1",
            owner_module: "scene/recipe",
            summary: "Effective recipe sandbox roots, URI/network policy, limits, and value sources.",
            feature_flag: None,
            fixture_path: Some("tests/assets/stable-contracts/recipe_policy.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.cli_error.v1",
            owner_module: "bin/scena",
            summary: "Structured CLI dispatch and argument error emitted on stderr.",
            feature_flag: None,
            fixture_path: Some("tests/assets/stable-contracts/cli_error.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.cli_io_error.v1",
            owner_module: "bin/scena",
            summary: "Structured fatal CLI stdout write failure report emitted on stderr.",
            feature_flag: None,
            fixture_path: None,
        },
        SchemaEntryRow {
            schema: "scena.cli_help.v1",
            owner_module: "bin/scena",
            summary: "Machine-readable scena CLI command, option, and guide discovery.",
            feature_flag: None,
            fixture_path: None,
        },
        SchemaEntryRow {
            schema: "scena.cli_version.v1",
            owner_module: "bin/scena",
            summary: "Machine-readable package version, commit, and compiled feature report.",
            feature_flag: None,
            fixture_path: None,
        },
    ]
}
