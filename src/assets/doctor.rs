use serde::{Deserialize, Serialize};

use crate::diagnostics::AssetError;

use super::{
    AssetExternalResourceKind, AssetExternalResourceStatus, AssetFetcher, AssetLoadReport,
    AssetLoadReportV1, AssetLoadWarning, AssetMaterialFallback, AssetPath, Assets,
    GltfExtensionStatus, SceneAsset,
};

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

fn finding_for_load_warning(warning: &AssetLoadWarning) -> AssetDoctorFindingV1 {
    match warning {
        AssetLoadWarning::ExternalBufferMissing {
            path,
            index,
            reason,
        } => AssetDoctorFindingV1 {
            severity: AssetDoctorSeverityV1::Warning,
            code: "external_buffer_missing".to_owned(),
            path: Some(path.as_str().to_owned()),
            field: Some(format!("buffers[{index}]")),
            extension: None,
            message: format!("external buffer {index} was missing: {reason}"),
            help: "the asset can only be trusted when all referenced buffer bytes are available"
                .to_owned(),
            suggested_fix:
                "Serve the buffer next to the glTF, correct the URI, or embed the buffer into a GLB."
                    .to_owned(),
            source: "asset_load_report".to_owned(),
        },
        AssetLoadWarning::ExternalImageMissing { path, reason } => AssetDoctorFindingV1 {
            severity: AssetDoctorSeverityV1::Warning,
            code: "external_image_missing".to_owned(),
            path: Some(path.as_str().to_owned()),
            field: Some("images".to_owned()),
            extension: None,
            message: format!("external image was missing: {reason}"),
            help: "the material will not match the authored asset until referenced image bytes are available".to_owned(),
            suggested_fix: "Serve the image next to the glTF, correct the URI, or embed the image before approval.".to_owned(),
            source: "asset_load_report".to_owned(),
        },
    }
}

fn finding_for_material_fallback(fallback: &AssetMaterialFallback) -> AssetDoctorFindingV1 {
    AssetDoctorFindingV1 {
        severity: AssetDoctorSeverityV1::Warning,
        code: "material_fallback_used".to_owned(),
        path: Some(fallback.source_path.as_str().to_owned()),
        field: Some(fallback.material_slot.clone()),
        extension: Some("KHR_texture_basisu".to_owned()),
        message: format!(
            "{} used fallback texture {}",
            fallback.source_path.as_str(),
            fallback.fallback_path.as_str()
        ),
        help: fallback.reason.clone(),
        suggested_fix:
            "Enable the required decoder feature or keep the fallback texture packaged with the asset."
                .to_owned(),
        source: "asset_load_report".to_owned(),
    }
}

fn finding_for_asset_error(error: &AssetError, fallback_path: &str) -> AssetDoctorFindingV1 {
    let (code, path, field, extension, suggested_fix) = match error {
        AssetError::NotFound { path } => (
            "asset_not_found",
            path.clone(),
            Some("source".to_owned()),
            None,
            "Fix the path, configure the asset fetcher, or serve the asset before rendering.",
        ),
        AssetError::Io { path, .. } => (
            "asset_io",
            path.clone(),
            Some("source".to_owned()),
            None,
            "Fix filesystem or network access, then retry the load.",
        ),
        AssetError::Parse { path, .. } => (
            "asset_parse",
            path.clone(),
            Some("source".to_owned()),
            None,
            "Validate the source file with the glTF validator and re-export valid glTF/GLB.",
        ),
        AssetError::UnsupportedRequiredExtension { path, extension } => (
            "unsupported_required_extension",
            path.clone(),
            Some("extensionsRequired".to_owned()),
            Some(extension.clone()),
            "Remove the required extension, make it optional with a visual fallback, export a fallback material/path, or enable the matching decoder feature when one exists.",
        ),
        AssetError::UnsupportedOptionalExtensionUsed {
            path, extension, ..
        } => (
            "unsupported_optional_extension_used",
            path.clone(),
            Some("extensionsUsed".to_owned()),
            Some(extension.clone()),
            "Inspect extension_diagnostics, then export a fallback or keep the extension optional until the target renderer lane supports it.",
        ),
        AssetError::MissingTexture {
            path,
            material_slot,
            ..
        } => (
            "missing_texture",
            path.clone(),
            Some(material_slot.clone()),
            None,
            "Fix the material texture index or export the referenced image bytes.",
        ),
        AssetError::UnsupportedTextureFormat { path, .. } => (
            "unsupported_texture_format",
            path.clone(),
            Some("images".to_owned()),
            None,
            "Use PNG, JPEG, WebP, or a decoder-backed compressed texture feature supported by the build.",
        ),
        AssetError::Cancelled { path, .. } => (
            "asset_load_cancelled",
            path.clone(),
            Some("source".to_owned()),
            None,
            "Retry the load with a fresh AssetLoadControl when the host still needs this asset.",
        ),
        AssetError::UnsupportedEnvironmentFormat { path, .. } => (
            "unsupported_environment_format",
            path.clone(),
            Some("environment".to_owned()),
            None,
            "Use an equirectangular HDR environment or a bundled supported environment preset.",
        ),
        AssetError::ReloadRequiresRetain { path, .. } => (
            "reload_requires_retain",
            path.clone(),
            Some("retain_policy".to_owned()),
            None,
            "Set RetainPolicy::Always before loading assets that need hot reload.",
        ),
        AssetError::GeometryHandleNotFound { .. }
        | AssetError::MaterialHandleNotFound { .. }
        | AssetError::TextureHandleNotFound { .. }
        | AssetError::EnvironmentHandleNotFound { .. } => (
            "asset_handle_not_found",
            fallback_path.to_owned(),
            Some("handle".to_owned()),
            None,
            "Verify the handle came from the same Assets store and has not been released.",
        ),
    };
    AssetDoctorFindingV1 {
        severity: AssetDoctorSeverityV1::Error,
        code: code.to_owned(),
        path: Some(path),
        field,
        extension,
        message: error.to_string(),
        help: error.help().to_owned(),
        suggested_fix: suggested_fix.to_owned(),
        source: "scena_asset_doctor".to_owned(),
    }
}
