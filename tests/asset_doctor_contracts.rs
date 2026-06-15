#[cfg(feature = "scene-host")]
use scena::SceneHostCore;
use scena::{ASSET_DOCTOR_REPORT_SCHEMA_V1, AssetDoctorReportV1, AssetDoctorSeverityV1, Assets};

const VALID_ASSET: &str = "tests/assets/gltf/material_variants_scene.gltf";
const BROKEN_ASSET: &str = "tests/assets/gltf/unsupported_required_extension.gltf";

#[test]
fn asset_doctor_reports_load_failure_for_asset_path() {
    let report = pollster::block_on(Assets::new().doctor_asset_path(BROKEN_ASSET));

    assert_eq!(report.schema, ASSET_DOCTOR_REPORT_SCHEMA_V1);
    assert!(!report.ok);
    assert_eq!(report.status, "failed");
    assert_eq!(report.asset, BROKEN_ASSET);
    let finding = report
        .findings
        .iter()
        .find(|finding| finding.code == "unsupported_required_extension")
        .expect("unsupported required extension finding exists");
    assert_eq!(finding.severity, AssetDoctorSeverityV1::Error);
    assert_eq!(finding.path.as_deref(), Some(BROKEN_ASSET));
    assert!(finding.message.contains("KHR_materials_clearcoat"));
    assert!(finding.help.contains("required extension"));
    assert!(finding.suggested_fix.contains("fallback"));
}

#[test]
fn asset_doctor_reports_loaded_asset_extension_findings() {
    let assets = Assets::new();
    let asset = pollster::block_on(assets.load_scene(VALID_ASSET)).expect("asset loads");

    let report = assets.doctor_loaded_asset(&asset);

    assert_eq!(report.schema, ASSET_DOCTOR_REPORT_SCHEMA_V1);
    assert!(report.ok, "{report:#?}");
    assert_eq!(report.asset, VALID_ASSET);
    assert!(report.asset_load_report.is_none());
    let finding = report
        .findings
        .iter()
        .find(|finding| finding.code == "extension_supported")
        .expect("supported extension finding exists");
    assert_eq!(finding.severity, AssetDoctorSeverityV1::Info);
    assert_eq!(finding.path.as_deref(), Some(VALID_ASSET));
    assert_eq!(finding.extension.as_deref(), Some("KHR_materials_variants"));
    assert!(finding.suggested_fix.contains("No action needed"));
}

#[test]
#[cfg(feature = "scene-host")]
fn scene_host_asset_doctor_json_uses_same_report_shape() {
    let host = SceneHostCore::headless(64, 64).expect("host builds");

    let json =
        pollster::block_on(host.asset_doctor_json(BROKEN_ASSET)).expect("doctor json serializes");
    let report: AssetDoctorReportV1 = serde_json::from_str(&json).expect("report decodes");

    assert_eq!(report.schema, ASSET_DOCTOR_REPORT_SCHEMA_V1);
    assert!(!report.ok);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.code == "unsupported_required_extension"),
        "{report:#?}"
    );
}
