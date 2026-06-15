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
