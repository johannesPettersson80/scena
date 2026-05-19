use crate::app::prelude::*;

pub(super) fn check_asset_validation_doctor(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ASSET-VALIDATION-DOCTOR",
        "crates/xtask/src/app/core.rs",
        &["AssetDoctor", "asset-doctor", "run_asset_doctor"],
    );
    require_contains(
        root,
        findings,
        "ASSET-VALIDATION-DOCTOR",
        "crates/xtask/src/app/asset_validation.rs",
        &[
            "SCENA_GLTF_VALIDATOR",
            "gltf_validator",
            "official_gltf_validator_args",
            "scena_native_asset_guidance",
            "KHR_materials_clearcoat",
            "export a fallback material",
            "fix",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSET-VALIDATION-DOCTOR",
        "crates/xtask/src/app/tests_15.rs",
        &[
            "parse_command_accepts_asset_doctor_path",
            "asset_doctor_native_guidance_reports_required_clearcoat_with_fix",
            "asset_doctor_official_validator_uses_khronos_stdout_mode",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSET-VALIDATION-DOCTOR",
        "docs/assets.md",
        &[
            "cargo run -p xtask -- asset-doctor",
            "official Khronos glTF Validator",
            "SCENA_GLTF_VALIDATOR",
            "scena.asset_doctor.v1",
            "fix",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSET-VALIDATION-DOCTOR",
        "docs/checklists/next-release-easy-use-and-state-of-the-art.md",
        &[
            "Doctor → official validation + actionable scena guidance",
            "Status: **[shipped]**",
            "ASSET-VALIDATION-DOCTOR",
            "asset-doctor",
        ],
    );
}
