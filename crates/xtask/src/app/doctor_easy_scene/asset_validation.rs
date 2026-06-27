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
        "src/assets/gltf/extensions.rs",
        &["GltfExtensionDiagnostic", "suggested_fix", "decoder_policy"],
    );
    require_contains(
        root,
        findings,
        "ASSET-VALIDATION-DOCTOR",
        "src/assets/doctor.rs",
        &[
            "ASSET_DOCTOR_REPORT_SCHEMA_V1",
            "AssetDoctorReportV1",
            "AssetDoctorFindingV1",
            "doctor_asset_path",
            "doctor_loaded_asset",
            "unsupported_required_extension",
            "suggested_fix",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSET-VALIDATION-DOCTOR",
        "src/bin/scena/doctor.rs",
        &[
            "run_doctor_command",
            "doctor_asset_path",
            "failed to serialize asset doctor report",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSET-VALIDATION-DOCTOR",
        "src/scene_host/assets.rs",
        &["asset_doctor_json", "doctor_asset_path"],
    );
    require_contains(
        root,
        findings,
        "ASSET-VALIDATION-DOCTOR",
        "src/scene_host/wasm_assets.rs",
        &["assetDoctorJson", "asset_doctor_json"],
    );
    require_contains(
        root,
        findings,
        "ASSET-VALIDATION-DOCTOR",
        "tests/asset_doctor_contracts.rs",
        &[
            "doctor_asset_path",
            "doctor_loaded_asset",
            "asset_doctor_json",
            "unsupported_required_extension",
        ],
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
            "suggested_fix",
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
            "scena doctor",
            "Assets::doctor_asset_path",
            "SceneHost.assetDoctorJson",
            "suggested_fix",
            "fix",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSET-VALIDATION-DOCTOR",
        "tests/browser/m6_rust_wasm_renderer_probe_page.js",
        &[
            "scenaAssetDoctorBrowserProbe",
            "m6AssetDoctorBrowserProbe",
            "asset-doctor-browser",
            "unsupported_required_extension",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSET-VALIDATION-DOCTOR",
        "tests/browser/m6_rust_wasm_renderer_probe.js",
        &[
            "assertAssetDoctorBrowserProof",
            "runAssetDoctorBrowserProof",
            "asset-doctor-browser-proof.png",
            "scena.asset_doctor.v1",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSET-VALIDATION-DOCTOR",
        "docs/checklists/application-builder-roadmap.md",
        &[
            "Asset doctor integration API",
            "Assets::doctor_asset_path",
            "SceneHost.assetDoctorJson",
            "asset-doctor-browser-proof.png",
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
    require_contains(
        root,
        findings,
        "ASSET-CATALOG-BROWSER-PREVIEW",
        "src/browser_probe/workflows.rs",
        &[
            "asset-catalog-preview",
            "AssetCatalogV1",
            "readiness_catalog.v1.json",
            "variant-triangle",
            "set_active_variant",
            "proof_class\": \"asset-catalog-preview",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSET-CATALOG-BROWSER-PREVIEW",
        "tests/browser/m6_rust_wasm_renderer_probe.js",
        &[
            "asset-catalog-preview",
            "assertAssetCatalogPreviewProof",
            "scena.asset_catalog.v1",
            "Variant Triangle",
            "tests/assets/gltf/material_variants_scene.gltf",
            "metadata.active_variant !== \"midnight\"",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSET-CATALOG-BROWSER-PREVIEW",
        "docs/checklists/application-builder-roadmap.md",
        &[
            "Browser preview proof for a catalog asset.",
            "asset-catalog-preview",
            "SCENA_BROWSER_BACKENDS=webgl2 npm run browser:m6",
        ],
    );
}
