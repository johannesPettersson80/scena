use crate::app::prelude::*;
use crate::app::tests_12::{VALID_GUIDE, write_easy_scene_fixture};

pub(crate) fn write_asset_validation_easy_scene_fixture(fixture_root: &Path) {
    fs::create_dir_all(fixture_root.join("crates/xtask/src/app")).expect("xtask fixture dir");
    fs::create_dir_all(fixture_root.join("src/browser_probe")).expect("browser probe fixture dir");
    fs::create_dir_all(fixture_root.join("src/assets")).expect("assets fixture dir");
    fs::create_dir_all(fixture_root.join("src/assets/gltf")).expect("gltf fixture dir");
    fs::create_dir_all(fixture_root.join("src/bin")).expect("bin fixture dir");
    fs::create_dir_all(fixture_root.join("src/bin/scena")).expect("scena bin fixture dir");
    fs::create_dir_all(fixture_root.join("src/scene_host")).expect("scene host fixture dir");
    fs::create_dir_all(fixture_root.join("tests/browser")).expect("browser fixture dir");
    fs::create_dir_all(fixture_root.join("tests")).expect("tests fixture dir");
    fs::write(
        fixture_root.join("docs/assets.md"),
        "cargo run -p xtask -- asset-doctor scena doctor official Khronos glTF Validator SCENA_GLTF_VALIDATOR scena.asset_doctor.v1 Assets::doctor_asset_path SceneHost.assetDoctorJson suggested_fix fix",
    )
    .expect("assets docs fixture");
    fs::write(
        fixture_root.join("crates/xtask/src/app/core.rs"),
        "AssetDoctor asset-doctor run_asset_doctor",
    )
    .expect("xtask core fixture");
    fs::write(
        fixture_root.join("crates/xtask/src/app/asset_validation.rs"),
        "SCENA_GLTF_VALIDATOR gltf_validator official_gltf_validator_args scena_native_asset_guidance KHR_materials_clearcoat export a fallback material fix",
    )
    .expect("asset validation fixture");
    fs::write(
        fixture_root.join("crates/xtask/src/app/tests_15.rs"),
        "parse_command_accepts_asset_doctor_path asset_doctor_native_guidance_reports_required_clearcoat_with_fix asset_doctor_official_validator_uses_khronos_stdout_mode suggested_fix",
    )
    .expect("asset validation test fixture");
    fs::write(
        fixture_root.join("src/assets/gltf/extensions.rs"),
        "GltfExtensionDiagnostic suggested_fix decoder_policy",
    )
    .expect("gltf extension diagnostic fixture");
    fs::write(
        fixture_root.join("src/assets/doctor.rs"),
        "ASSET_DOCTOR_REPORT_SCHEMA_V1 AssetDoctorReportV1 AssetDoctorFindingV1 doctor_asset_path doctor_loaded_asset unsupported_required_extension suggested_fix",
    )
    .expect("asset doctor runtime fixture");
    fs::write(
        fixture_root.join("src/bin/scena/doctor.rs"),
        "run_doctor_command doctor_asset_path failed to serialize asset doctor report",
    )
    .expect("scena cli doctor fixture");
    fs::write(
        fixture_root.join("src/scene_host/assets.rs"),
        "asset_doctor_json doctor_asset_path",
    )
    .expect("scene host asset doctor fixture");
    fs::write(
        fixture_root.join("src/scene_host/wasm_assets.rs"),
        "assetDoctorJson asset_doctor_json",
    )
    .expect("wasm asset doctor fixture");
    fs::write(
        fixture_root.join("tests/asset_doctor_contracts.rs"),
        "doctor_asset_path doctor_loaded_asset asset_doctor_json unsupported_required_extension",
    )
    .expect("asset doctor contracts fixture");
    fs::write(
        fixture_root.join("tests/browser/m6_rust_wasm_renderer_probe_page.js"),
        "scenaAssetDoctorBrowserProbe m6AssetDoctorBrowserProbe asset-doctor-browser unsupported_required_extension",
    )
    .expect("asset doctor browser page fixture");
    fs::write(
        fixture_root.join("src/browser_probe/workflows.rs"),
        r#"asset-catalog-preview AssetCatalogV1 readiness_catalog.v1.json variant-triangle set_active_variant proof_class": "asset-catalog-preview"#,
    )
    .expect("asset catalog browser workflow fixture");
    fs::write(
        fixture_root.join("tests/browser/m6_rust_wasm_renderer_probe.js"),
        r#"asset-catalog-preview assertAssetCatalogPreviewProof scena.asset_catalog.v1 Variant Triangle tests/assets/gltf/material_variants_scene.gltf metadata.active_variant !== "midnight" assertAssetDoctorBrowserProof runAssetDoctorBrowserProof asset-doctor-browser-proof.png scena.asset_doctor.v1"#,
    )
    .expect("asset catalog browser proof fixture");
    fs::write(
        fixture_root.join("docs/checklists/application-builder-roadmap.md"),
        "Browser preview proof for a catalog asset. asset-catalog-preview SCENA_BROWSER_BACKENDS=webgl2 npm run browser:m6 Asset doctor integration API Assets::doctor_asset_path SceneHost.assetDoctorJson asset-doctor-browser-proof.png",
    )
    .expect("application builder roadmap fixture");
    let checklist_path =
        fixture_root.join("docs/checklists/next-release-easy-use-and-state-of-the-art.md");
    let mut checklist =
        fs::read_to_string(&checklist_path).expect("next release checklist fixture");
    checklist.push_str(
        " Doctor → official validation + actionable scena guidance Status: **[shipped]** asset-doctor ASSET-VALIDATION-DOCTOR",
    );
    fs::write(checklist_path, checklist).expect("next release checklist asset fixture");
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_missing_runtime_asset_doctor_api() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/missing-runtime-asset-doctor");
    write_easy_scene_fixture(
        &fixture_root,
        VALID_GUIDE,
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    let _ = fs::remove_file(fixture_root.join("src/assets/doctor.rs"));
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "ASSET-VALIDATION-DOCTOR"),
        "doctor must reject asset-doctor roadmap claims without the runtime API: {findings:?}",
    );
}

#[test]
pub(crate) fn parse_command_accepts_asset_doctor_path() {
    assert_eq!(
        parse_command(vec![
            "asset-doctor".to_string(),
            "tests/assets/gltf/unsupported_required_extension.gltf".to_string(),
        ]),
        Ok(Command::AssetDoctor {
            input: "tests/assets/gltf/unsupported_required_extension.gltf".to_string(),
        })
    );
}

#[test]
pub(crate) fn asset_doctor_native_guidance_reports_required_clearcoat_with_fix() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let path = root.join("tests/assets/gltf/unsupported_required_extension.gltf");

    let guidance = scena_native_asset_guidance(&path).expect("fixture guidance builds");

    assert!(
        guidance.iter().any(|finding| {
            finding.extension == "KHR_materials_clearcoat"
                && finding.required
                && finding.severity == AssetGuidanceSeverity::Error
                && finding.fix.contains("export a fallback material")
                && finding.fix.contains("clearcoat")
        }),
        "asset doctor must report renderer-specific clearcoat guidance with a fix string: {guidance:?}",
    );
}

#[test]
pub(crate) fn asset_doctor_native_guidance_reports_required_transmission_with_capability_fix() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_dir = root.join("target/xtask-asset-guidance");
    fs::create_dir_all(&fixture_dir).expect("asset guidance fixture dir");
    let path = fixture_dir.join("required-transmission.gltf");
    fs::write(
        &path,
        r#"{
            "asset": { "version": "2.0" },
            "extensionsUsed": ["KHR_materials_transmission"],
            "extensionsRequired": ["KHR_materials_transmission"],
            "nodes": [{ "name": "Root" }]
        }"#,
    )
    .expect("required transmission fixture writes");

    let guidance = scena_native_asset_guidance(&path).expect("fixture guidance builds");
    let transmission = guidance
        .iter()
        .find(|finding| finding.extension == "KHR_materials_transmission")
        .expect("transmission guidance is present");

    assert!(transmission.required);
    assert_eq!(transmission.severity, AssetGuidanceSeverity::Error);
    assert_eq!(transmission.status, "degraded");
    assert!(
        transmission.message.contains("physical_glass_transmission")
            && transmission.message.contains("attached GPU"),
        "transmission guidance must point required assets at the backend capability proof: {transmission:?}",
    );
    assert!(
        !transmission.message.contains("not release-proven"),
        "transmission guidance must not carry the stale pre-proof wording: {transmission:?}",
    );
    assert!(
        transmission.fix.contains("capability report")
            && transmission.fix.contains("fallback material"),
        "transmission guidance must give agents a capability-check path and a fallback path: {transmission:?}",
    );
}

#[test]
pub(crate) fn asset_doctor_official_validator_uses_khronos_stdout_mode() {
    let args = official_gltf_validator_args(Path::new("model.glb"));

    assert_eq!(args, vec!["-o".to_string(), "model.glb".to_string()]);
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_missing_asset_validation_doctor() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/missing-asset-validation-doctor");
    write_easy_scene_fixture(
        &fixture_root,
        VALID_GUIDE,
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    let _ = fs::remove_file(fixture_root.join("crates/xtask/src/app/asset_validation.rs"));
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "ASSET-VALIDATION-DOCTOR"),
        "doctor must reject the asset-validation roadmap claim without the official validator wrapper and scena guidance: {findings:?}",
    );
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_missing_asset_catalog_browser_preview_proof() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root =
        root.join("target/xtask-doctor-regressions/missing-asset-catalog-browser-preview");
    write_easy_scene_fixture(
        &fixture_root,
        VALID_GUIDE,
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    fs::write(
        fixture_root.join("tests/browser/m6_rust_wasm_renderer_probe.js"),
        "asset-catalog-preview",
    )
    .expect("browser proof fixture without assertion");
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "ASSET-CATALOG-BROWSER-PREVIEW"),
        "doctor must reject catalog browser-preview claims without the M6 workflow assertion: {findings:?}",
    );
}
