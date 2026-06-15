use crate::assets::{AssetFetcher, Assets, SceneAsset};
use crate::geometry::Aabb;
use crate::scene::Vec3;

use super::checks::finding;
use super::{AssetCatalogAssetV1, AssetReadinessFindingV1, AssetReadinessSeverityV1};

pub(super) fn validate_loaded_asset(
    assets: &Assets<impl AssetFetcher>,
    catalog_asset: &AssetCatalogAssetV1,
    asset: &SceneAsset,
    findings: &mut Vec<AssetReadinessFindingV1>,
) {
    validate_bounds(catalog_asset, asset, findings);
    validate_authored_features(catalog_asset, asset, findings);
    validate_material_requirements(assets, catalog_asset, asset, findings);
}

fn validate_bounds(
    catalog_asset: &AssetCatalogAssetV1,
    asset: &SceneAsset,
    findings: &mut Vec<AssetReadinessFindingV1>,
) {
    let Some(bounds) = asset.bounds() else {
        if catalog_asset.expected_bounds.is_some() {
            findings.push(finding(
                AssetReadinessSeverityV1::Error,
                "bounds_missing",
                "asset has no renderable bounds to compare against expected limits",
                "add renderable geometry or remove the expected bounds requirement",
                Some(catalog_asset.source.clone()),
                Some("expected_bounds"),
            ));
        }
        return;
    };

    if !aabb_is_finite(bounds) {
        findings.push(finding(
            AssetReadinessSeverityV1::Error,
            "bounds_not_finite",
            "asset bounds contain non-finite coordinates",
            "fix the source geometry before catalog approval",
            Some(catalog_asset.source.clone()),
            Some("expected_bounds"),
        ));
        return;
    }

    let Some(expected) = catalog_asset.expected_bounds.as_ref() else {
        return;
    };

    if expected
        .min_extent
        .is_some_and(|value| !valid_extent(value))
        || expected
            .max_extent
            .is_some_and(|value| !valid_extent(value))
    {
        findings.push(finding(
            AssetReadinessSeverityV1::Error,
            "expected_bounds_invalid",
            "expected extent limits must be finite positive numbers",
            "remove invalid extent limits or set positive finite values",
            None,
            Some("expected_bounds"),
        ));
        return;
    }

    if let (Some(min), Some(max)) = (expected.min, expected.max)
        && (bounds.min.x < min.x
            || bounds.min.y < min.y
            || bounds.min.z < min.z
            || bounds.max.x > max.x
            || bounds.max.y > max.y
            || bounds.max.z > max.z)
    {
        findings.push(finding(
            AssetReadinessSeverityV1::Error,
            "bounds_out_of_range",
            "asset bounds fall outside the declared expected bounds",
            "fix the asset scale/origin or widen the expected bounds intentionally",
            Some(catalog_asset.source.clone()),
            Some("expected_bounds"),
        ));
    }

    let extent = max_extent(bounds);
    if expected.min_extent.is_some_and(|min| extent < min)
        || expected.max_extent.is_some_and(|max| extent > max)
    {
        findings.push(finding(
            AssetReadinessSeverityV1::Error,
            "extent_out_of_range",
            "asset maximum extent falls outside the declared scale limits",
            "fix the asset units/scale or update expected extent limits intentionally",
            Some(catalog_asset.source.clone()),
            Some("expected_bounds"),
        ));
    }
}

fn validate_authored_features(
    catalog_asset: &AssetCatalogAssetV1,
    asset: &SceneAsset,
    findings: &mut Vec<AssetReadinessFindingV1>,
) {
    validate_authored_feature_integrity(catalog_asset, asset, findings);
    validate_required_anchors(catalog_asset, asset, findings);
    validate_required_connectors(catalog_asset, asset, findings);
    validate_required_tags(catalog_asset, asset, findings);
}

fn validate_authored_feature_integrity(
    catalog_asset: &AssetCatalogAssetV1,
    asset: &SceneAsset,
    findings: &mut Vec<AssetReadinessFindingV1>,
) {
    for node in asset.nodes() {
        for anchor in node.anchors() {
            if let Some(reason) = anchor.invalid_reason() {
                findings.push(finding(
                    AssetReadinessSeverityV1::Error,
                    "invalid_anchor",
                    format!("anchor '{}' is invalid: {reason}", anchor.name()),
                    "fix the glTF extras.scena.anchors entry",
                    Some(catalog_asset.source.clone()),
                    Some("required_anchors"),
                ));
            }
        }
        for connector in node.connectors() {
            if let Some(reason) = connector.invalid_reason() {
                findings.push(finding(
                    AssetReadinessSeverityV1::Error,
                    "invalid_connector",
                    format!("connector '{}' is invalid: {reason}", connector.name()),
                    "fix the glTF extras.scena.connectors entry",
                    Some(catalog_asset.source.clone()),
                    Some("required_connectors"),
                ));
            }
        }
    }
}

fn validate_required_anchors(
    catalog_asset: &AssetCatalogAssetV1,
    asset: &SceneAsset,
    findings: &mut Vec<AssetReadinessFindingV1>,
) {
    for requirement in &catalog_asset.required_anchors {
        let matching = asset
            .nodes()
            .iter()
            .flat_map(|node| node.anchors())
            .find(|anchor| anchor.name() == requirement.name);
        match matching {
            Some(anchor) => {
                if let Some(missing_tag) = requirement
                    .tags
                    .iter()
                    .find(|tag| !anchor.tags().contains(tag.as_str()))
                {
                    findings.push(finding(
                        AssetReadinessSeverityV1::Error,
                        "required_anchor_tag_missing",
                        format!(
                            "anchor '{}' is missing required tag '{}'",
                            requirement.name, missing_tag
                        ),
                        "add the required tag to the anchor extras or update the manifest",
                        Some(catalog_asset.source.clone()),
                        Some("required_anchors"),
                    ));
                }
            }
            None => findings.push(finding(
                AssetReadinessSeverityV1::Error,
                "required_anchor_missing",
                format!("required anchor '{}' is missing", requirement.name),
                "author the anchor in glTF extras.scena.anchors or remove the requirement",
                Some(catalog_asset.source.clone()),
                Some("required_anchors"),
            )),
        }
    }
}

fn validate_required_connectors(
    catalog_asset: &AssetCatalogAssetV1,
    asset: &SceneAsset,
    findings: &mut Vec<AssetReadinessFindingV1>,
) {
    for requirement in &catalog_asset.required_connectors {
        let matching = asset
            .nodes()
            .iter()
            .flat_map(|node| node.connectors())
            .find(|connector| connector.name() == requirement.name);
        match matching {
            Some(connector) => {
                if let Some(missing_tag) = requirement
                    .tags
                    .iter()
                    .find(|tag| !connector.tags().contains(tag.as_str()))
                {
                    findings.push(finding(
                        AssetReadinessSeverityV1::Error,
                        "required_connector_tag_missing",
                        format!(
                            "connector '{}' is missing required tag '{}'",
                            requirement.name, missing_tag
                        ),
                        "add the required tag to the connector extras or update the manifest",
                        Some(catalog_asset.source.clone()),
                        Some("required_connectors"),
                    ));
                }
            }
            None => findings.push(finding(
                AssetReadinessSeverityV1::Error,
                "required_connector_missing",
                format!("required connector '{}' is missing", requirement.name),
                "author the connector in glTF extras.scena.connectors or remove the requirement",
                Some(catalog_asset.source.clone()),
                Some("required_connectors"),
            )),
        }
    }
}

fn validate_required_tags(
    catalog_asset: &AssetCatalogAssetV1,
    asset: &SceneAsset,
    findings: &mut Vec<AssetReadinessFindingV1>,
) {
    for tag in &catalog_asset.required_tags {
        let found = asset.nodes().iter().any(|node| {
            node.anchors()
                .iter()
                .any(|anchor| anchor.tags().contains(tag.as_str()))
                || node
                    .connectors()
                    .iter()
                    .any(|connector| connector.tags().contains(tag.as_str()))
        });
        if !found {
            findings.push(finding(
                AssetReadinessSeverityV1::Error,
                "required_tag_missing",
                format!("required authored feature tag '{tag}' is missing"),
                "add the tag to an authored anchor/connector or update the manifest",
                Some(catalog_asset.source.clone()),
                Some("required_tags"),
            ));
        }
    }
}

fn validate_material_requirements(
    assets: &Assets<impl AssetFetcher>,
    catalog_asset: &AssetCatalogAssetV1,
    asset: &SceneAsset,
    findings: &mut Vec<AssetReadinessFindingV1>,
) {
    validate_required_material_variants(catalog_asset, asset, findings);
    validate_material_fallback_policy(catalog_asset, asset, findings);
    validate_base_color_texture_requirement(assets, catalog_asset, asset, findings);
}

fn validate_required_material_variants(
    catalog_asset: &AssetCatalogAssetV1,
    asset: &SceneAsset,
    findings: &mut Vec<AssetReadinessFindingV1>,
) {
    for variant in &catalog_asset.material_requirements.required_variants {
        if !asset
            .material_variants()
            .iter()
            .any(|candidate| candidate == variant)
        {
            findings.push(finding(
                AssetReadinessSeverityV1::Error,
                "required_material_variant_missing",
                format!("required material variant '{variant}' is missing"),
                "author the KHR_materials_variants entry or remove the requirement",
                Some(catalog_asset.source.clone()),
                Some("material_requirements.required_variants"),
            ));
        }
    }
}

fn validate_material_fallback_policy(
    catalog_asset: &AssetCatalogAssetV1,
    asset: &SceneAsset,
    findings: &mut Vec<AssetReadinessFindingV1>,
) {
    if !catalog_asset.material_requirements.allow_material_fallbacks
        && !asset.material_fallbacks().is_empty()
    {
        findings.push(finding(
            AssetReadinessSeverityV1::Error,
            "material_fallback_used",
            "asset used one or more material fallback sources",
            "provide the preferred source material/texture or explicitly allow fallbacks",
            Some(catalog_asset.source.clone()),
            Some("material_requirements.allow_material_fallbacks"),
        ));
    }
}

fn validate_base_color_texture_requirement(
    assets: &Assets<impl AssetFetcher>,
    catalog_asset: &AssetCatalogAssetV1,
    asset: &SceneAsset,
    findings: &mut Vec<AssetReadinessFindingV1>,
) {
    if !catalog_asset
        .material_requirements
        .require_base_color_textures
    {
        return;
    }

    for node in asset.nodes() {
        for mesh in node.meshes() {
            let has_base_color_texture = assets
                .material(mesh.material())
                .is_some_and(|material| material.base_color_texture().is_some());
            if !has_base_color_texture {
                findings.push(finding(
                    AssetReadinessSeverityV1::Error,
                    "base_color_texture_missing",
                    "required base-color texture is missing from one or more mesh materials",
                    "author a baseColorTexture or disable material_requirements.require_base_color_textures",
                    Some(catalog_asset.source.clone()),
                    Some("material_requirements.require_base_color_textures"),
                ));
                return;
            }
        }
    }
}

fn aabb_is_finite(bounds: Aabb) -> bool {
    vec3_is_finite(bounds.min) && vec3_is_finite(bounds.max)
}

fn vec3_is_finite(value: Vec3) -> bool {
    value.x.is_finite() && value.y.is_finite() && value.z.is_finite()
}

fn valid_extent(value: f32) -> bool {
    value.is_finite() && value > 0.0
}

fn max_extent(bounds: Aabb) -> f32 {
    let half = bounds.half_extent();
    half.x.max(half.y).max(half.z) * 2.0
}
