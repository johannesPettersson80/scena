use serde::{Deserialize, Serialize};

use crate::diagnostics::AssetError;

use super::{
    AssetExternalResourceKind, AssetExternalResourceStatus, AssetFetcher, AssetLoadReport,
    AssetLoadReportV1, AssetLoadWarning, AssetMaterialFallback, AssetPath, Assets,
    GltfExtensionStatus, SceneAsset,
};

mod findings;
use findings::{finding_for_asset_error, finding_for_load_warning, finding_for_material_fallback};

pub const ASSET_DOCTOR_REPORT_SCHEMA_V1: &str = "scena.asset_doctor.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetDoctorReportV1 {
    pub schema: String,
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub asset: String,
    #[serde(default)]
    pub summary: AssetDoctorSummaryV1,
    #[serde(default)]
    pub asset_load_report: Option<AssetLoadReportV1>,
    #[serde(default)]
    pub findings: Vec<AssetDoctorFindingV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AssetDoctorSummaryV1 {
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetDoctorFindingV1 {
    pub severity: AssetDoctorSeverityV1,
    pub code: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub field: Option<String>,
    #[serde(default)]
    pub extension: Option<String>,
    pub message: String,
    pub help: String,
    pub suggested_fix: String,
    pub source: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetDoctorSeverityV1 {
    Error,
    Warning,
    Info,
}

impl<F: AssetFetcher> Assets<F> {
    pub async fn doctor_asset_path(&self, path: impl Into<AssetPath>) -> AssetDoctorReportV1 {
        let path = path.into();
        match self.load_scene_with_report(path.clone()).await {
            Ok(load_report) => {
                let asset_load_report = load_report.to_schema_report();
                let mut findings = findings_for_loaded_asset(load_report.asset());
                findings.extend(findings_for_load_report(&load_report));
                AssetDoctorReportV1::new(
                    path.as_str().to_owned(),
                    Some(asset_load_report),
                    findings,
                )
            }
            Err(error) => {
                let finding = finding_for_asset_error(&error, path.as_str());
                AssetDoctorReportV1::new(path.as_str().to_owned(), None, vec![finding])
            }
        }
    }

    pub fn doctor_loaded_asset(&self, asset: &SceneAsset) -> AssetDoctorReportV1 {
        AssetDoctorReportV1::new(
            asset.path().as_str().to_owned(),
            None,
            findings_for_loaded_asset(asset),
        )
    }
}

impl AssetDoctorReportV1 {
    fn new(
        asset: String,
        asset_load_report: Option<AssetLoadReportV1>,
        mut findings: Vec<AssetDoctorFindingV1>,
    ) -> Self {
        findings.sort_by(|left, right| {
            severity_rank(left.severity)
                .cmp(&severity_rank(right.severity))
                .then_with(|| left.code.cmp(&right.code))
                .then_with(|| left.path.cmp(&right.path))
                .then_with(|| left.extension.cmp(&right.extension))
                .then_with(|| left.field.cmp(&right.field))
        });
        let summary = AssetDoctorSummaryV1::from_findings(&findings);
        let ok = summary.error_count == 0;
        Self {
            schema: ASSET_DOCTOR_REPORT_SCHEMA_V1.to_owned(),
            ok,
            status: if ok { "passed" } else { "failed" }.to_owned(),
            asset,
            summary,
            asset_load_report,
            findings,
        }
    }
}

impl AssetDoctorSummaryV1 {
    fn from_findings(findings: &[AssetDoctorFindingV1]) -> Self {
        let mut summary = Self::default();
        for finding in findings {
            match finding.severity {
                AssetDoctorSeverityV1::Error => summary.error_count += 1,
                AssetDoctorSeverityV1::Warning => summary.warning_count += 1,
                AssetDoctorSeverityV1::Info => summary.info_count += 1,
            }
        }
        summary
    }
}

fn severity_rank(severity: AssetDoctorSeverityV1) -> u8 {
    match severity {
        AssetDoctorSeverityV1::Error => 0,
        AssetDoctorSeverityV1::Warning => 1,
        AssetDoctorSeverityV1::Info => 2,
    }
}

fn findings_for_loaded_asset(asset: &SceneAsset) -> Vec<AssetDoctorFindingV1> {
    let mut findings = Vec::new();
    let path = asset.path().as_str().to_owned();
    for diagnostic in asset.extension_diagnostics() {
        let (severity, code, message) = match diagnostic.status() {
            GltfExtensionStatus::Supported => (
                AssetDoctorSeverityV1::Info,
                "extension_supported",
                format!("{} is supported by scena", diagnostic.extension()),
            ),
            GltfExtensionStatus::Degraded => (
                AssetDoctorSeverityV1::Warning,
                "extension_degraded",
                format!("{} is degraded by scena", diagnostic.extension()),
            ),
        };
        findings.push(AssetDoctorFindingV1 {
            severity,
            code: code.to_owned(),
            path: Some(path.clone()),
            field: Some("extensionsUsed".to_owned()),
            extension: Some(diagnostic.extension().to_owned()),
            message,
            help: diagnostic.help().to_owned(),
            suggested_fix: diagnostic.suggested_fix().to_owned(),
            source: "scena_asset_doctor".to_owned(),
        });
    }
    for fallback in asset.material_fallbacks() {
        findings.push(finding_for_material_fallback(fallback));
    }
    findings
}

fn findings_for_load_report(
    report: &AssetLoadReport<SceneAsset>,
) -> impl Iterator<Item = AssetDoctorFindingV1> + '_ {
    report
        .warnings()
        .iter()
        .map(finding_for_load_warning)
        .chain(
            report
                .external_resources()
                .iter()
                .filter(|resource| resource.status != AssetExternalResourceStatus::Fetched)
                .map(|resource| {
                    let (code, help, suggested_fix) = match resource.status {
                        AssetExternalResourceStatus::Fetched => unreachable!(),
                        AssetExternalResourceStatus::Missing => (
                            "external_resource_missing",
                            "an external buffer or image referenced by the asset could not be fetched",
                            "Fix the referenced URI, serve it next to the glTF, or embed the resource before relying on the asset.",
                        ),
                        AssetExternalResourceStatus::SkippedUnsupportedFormat => (
                            "external_resource_unsupported_format",
                            "an external image was present but skipped because the format is unsupported",
                            "Use PNG, JPEG, WebP, or a decoder-backed compressed texture path supported by the build.",
                        ),
                    };
                    let field = match resource.kind {
                        AssetExternalResourceKind::Buffer => "buffers",
                        AssetExternalResourceKind::Image => "images",
                    };
                    AssetDoctorFindingV1 {
                        severity: AssetDoctorSeverityV1::Warning,
                        code: code.to_owned(),
                        path: Some(resource.path.as_str().to_owned()),
                        field: Some(field.to_owned()),
                        extension: None,
                        message: resource.reason.clone().unwrap_or_else(|| {
                            format!(
                                "{field} external resource {} was {:?}",
                                resource.path.as_str(),
                                resource.status
                            )
                        }),
                        help: help.to_owned(),
                        suggested_fix: suggested_fix.to_owned(),
                        source: "asset_load_report".to_owned(),
                    }
                }),
        )
}
