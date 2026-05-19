use crate::app::prelude::*;

pub(super) fn check_environment_presets(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ENVIRONMENT-PRESETS",
        "src/assets.rs",
        &[
            "mod environment_preset;",
            "EnvironmentPreset",
            "EnvironmentPresetMetadata",
        ],
    );
    require_contains(
        root,
        findings,
        "ENVIRONMENT-PRESETS",
        "src/assets/environment_preset.rs",
        &[
            "pub enum EnvironmentPreset",
            "NeutralStudio",
            "Studio",
            "PACKAGE_SIZE_BUDGET_BYTES",
            "load_environment_preset",
            "source_sha256",
            "source_url",
            "license",
        ],
    );
    require_contains(
        root,
        findings,
        "ENVIRONMENT-PRESETS",
        "src/lib.rs",
        &["EnvironmentPreset", "EnvironmentPresetMetadata"],
    );
    require_contains(
        root,
        findings,
        "ENVIRONMENT-PRESETS",
        "tests/round_c_environment_presets.rs",
        &[
            "environment_preset_catalog_exposes_metadata_and_package_budget",
            "environment_presets_load_without_user_supplied_paths",
            "environment_presets_render_reference_contact_sheet",
            "environment-preset-reference-docs-image.ppm",
        ],
    );
    require_contains(
        root,
        findings,
        "ENVIRONMENT-PRESETS",
        "docs/guides/easy-scene-setup.md",
        &[
            "EnvironmentPreset::Studio",
            "load_environment_preset",
            "EnvironmentPreset::ALL",
            "KTX2 cubemap presets are still future work",
        ],
    );
}
