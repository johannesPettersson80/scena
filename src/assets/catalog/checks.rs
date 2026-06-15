use crate::assets::{AssetExternalResourceStatus, AssetLoadReportV1};
use crate::scene::{SourceCoordinateSystem, SourceUnits};

use super::{
    AssetCatalogAssetV1, AssetReadinessFindingV1, AssetReadinessPreviewV1, AssetReadinessSeverityV1,
};

pub(super) fn validate_catalog_identity(
    asset: &AssetCatalogAssetV1,
    findings: &mut Vec<AssetReadinessFindingV1>,
) {
    if asset.id.trim().is_empty() {
        findings.push(finding(
            AssetReadinessSeverityV1::Error,
            "asset_id_missing",
            "asset id must not be empty",
            "assign a stable host-owned asset id",
            None,
            Some("id"),
        ));
    }
    if asset.display_name.trim().is_empty() {
        findings.push(finding(
            AssetReadinessSeverityV1::Error,
            "display_name_missing",
            "asset display name must not be empty",
            "assign a human-readable display name",
            None,
            Some("display_name"),
        ));
    }
    if asset.source.trim().is_empty() {
        findings.push(finding(
            AssetReadinessSeverityV1::Error,
            "source_missing",
            "asset source path must not be empty",
            "supply a glTF/GLB path or URL fetchable by the configured AssetFetcher",
            None,
            Some("source"),
        ));
    }
}

pub(super) fn validate_declared_units(
    asset: &AssetCatalogAssetV1,
    findings: &mut Vec<AssetReadinessFindingV1>,
) {
    match asset.declared_units.as_deref() {
        Some(units) if parse_source_units(units).is_some() => {}
        Some(units) => findings.push(finding(
            AssetReadinessSeverityV1::Error,
            "invalid_source_units",
            format!("declared units '{units}' are not recognized"),
            "use meters, centimeters, millimeters, inches, or feet",
            None,
            Some("declared_units"),
        )),
        None => findings.push(finding(
            AssetReadinessSeverityV1::Error,
            "source_units_unknown",
            "catalog asset does not declare source units",
            "declare units explicitly so scale checks and authored features are unambiguous",
            None,
            Some("declared_units"),
        )),
    }
}

pub(super) fn validate_declared_coordinate_system(
    asset: &AssetCatalogAssetV1,
    findings: &mut Vec<AssetReadinessFindingV1>,
) {
    match asset.source_coordinate_system.as_deref() {
        Some(system) if parse_source_coordinate_system(system).is_some() => {}
        Some(system) => findings.push(finding(
            AssetReadinessSeverityV1::Error,
            "invalid_source_coordinate_system",
            format!("source coordinate system '{system}' is not recognized"),
            "use gltf_y_up_right_handed, z_up_right_handed, y_up_left_handed, or z_up_left_handed",
            None,
            Some("source_coordinate_system"),
        )),
        None => findings.push(finding(
            AssetReadinessSeverityV1::Error,
            "source_coordinate_system_unknown",
            "catalog asset does not declare a source coordinate system",
            "declare the source coordinate system explicitly before approval",
            None,
            Some("source_coordinate_system"),
        )),
    }
}

pub(super) fn validate_preview(
    asset: &AssetCatalogAssetV1,
    findings: &mut Vec<AssetReadinessFindingV1>,
) -> Option<AssetReadinessPreviewV1> {
    let Some(preview) = asset.preview.as_ref() else {
        findings.push(finding(
            AssetReadinessSeverityV1::Error,
            "preview_missing",
            "catalog asset does not declare preview image or generated preview metadata",
            "provide a preview image path or generated preview dimensions",
            None,
            Some("preview"),
        ));
        return None;
    };

    match preview.kind.as_str() {
        "image" => {
            if preview
                .path
                .as_deref()
                .is_none_or(|path| path.trim().is_empty())
            {
                findings.push(finding(
                    AssetReadinessSeverityV1::Error,
                    "preview_path_missing",
                    "image preview requires a non-empty path",
                    "set preview.path to a fetchable image path",
                    None,
                    Some("preview.path"),
                ));
            }
        }
        "generated" => {
            if preview.width.unwrap_or_default() == 0 || preview.height.unwrap_or_default() == 0 {
                findings.push(finding(
                    AssetReadinessSeverityV1::Error,
                    "preview_dimensions_invalid",
                    "generated preview requires positive width and height",
                    "set preview.width and preview.height to deterministic non-zero pixel dimensions",
                    None,
                    Some("preview"),
                ));
            }
        }
        _ => findings.push(finding(
            AssetReadinessSeverityV1::Error,
            "preview_kind_invalid",
            format!("preview kind '{}' is not supported", preview.kind),
            "use image or generated",
            None,
            Some("preview.kind"),
        )),
    }

    Some(AssetReadinessPreviewV1 {
        kind: preview.kind.clone(),
        status: "declared".to_owned(),
        path: preview.path.clone(),
        width: preview.width,
        height: preview.height,
    })
}

pub(super) fn validate_load_report(
    report: &AssetLoadReportV1,
    findings: &mut Vec<AssetReadinessFindingV1>,
) {
    for resource in &report.external_resources {
        if resource.status == AssetExternalResourceStatus::Missing {
            findings.push(finding(
                AssetReadinessSeverityV1::Error,
                "external_resource_missing",
                format!("external resource '{}' is missing", resource.path),
                "serve the external resource next to the glTF or embed it before approval",
                Some(resource.path.clone()),
                Some("required_files"),
            ));
        }
    }
}

pub(super) fn finding(
    severity: AssetReadinessSeverityV1,
    code: impl Into<String>,
    message: impl Into<String>,
    help: impl Into<String>,
    path: Option<String>,
    field: Option<&str>,
) -> AssetReadinessFindingV1 {
    AssetReadinessFindingV1 {
        severity,
        code: code.into(),
        message: message.into(),
        help: help.into(),
        path,
        field: field.map(str::to_owned),
    }
}

fn parse_source_units(value: &str) -> Option<SourceUnits> {
    match value {
        "meter" | "meters" | "m" => Some(SourceUnits::Meters),
        "centimeter" | "centimeters" | "cm" => Some(SourceUnits::Centimeters),
        "millimeter" | "millimeters" | "mm" => Some(SourceUnits::Millimeters),
        "inch" | "inches" | "in" => Some(SourceUnits::Inches),
        "foot" | "feet" | "ft" => Some(SourceUnits::Feet),
        _ => None,
    }
}

fn parse_source_coordinate_system(value: &str) -> Option<SourceCoordinateSystem> {
    match value {
        "gltf_y_up_right_handed" => Some(SourceCoordinateSystem::GltfYUpRightHanded),
        "y_up_left_handed" => Some(SourceCoordinateSystem::YUpLeftHanded),
        "z_up_right_handed" => Some(SourceCoordinateSystem::ZUpRightHanded),
        "z_up_left_handed" => Some(SourceCoordinateSystem::ZUpLeftHanded),
        _ => None,
    }
}
