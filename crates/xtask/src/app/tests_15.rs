use crate::app::prelude::*;
use crate::app::tests_12::{VALID_GUIDE, write_easy_scene_fixture};

pub(crate) fn write_asset_validation_easy_scene_fixture(fixture_root: &Path) {
    fs::create_dir_all(fixture_root.join("crates/xtask/src/app")).expect("xtask fixture dir");
    fs::write(
        fixture_root.join("docs/assets.md"),
        "cargo run -p xtask -- asset-doctor official Khronos glTF Validator SCENA_GLTF_VALIDATOR scena.asset_doctor.v1 fix",
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
        "parse_command_accepts_asset_doctor_path asset_doctor_native_guidance_reports_required_clearcoat_with_fix asset_doctor_official_validator_uses_khronos_stdout_mode",
    )
    .expect("asset validation test fixture");
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
