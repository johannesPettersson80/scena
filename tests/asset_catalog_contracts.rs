use serde_json::json;

use scena::{ASSET_CATALOG_SCHEMA_V1, ASSET_READINESS_REPORT_SCHEMA_V1, AssetCatalogV1, Assets};

#[test]
fn asset_catalog_v1_is_additive_and_versioned() {
    let catalog: AssetCatalogV1 = serde_json::from_value(json!({
        "schema": ASSET_CATALOG_SCHEMA_V1,
        "assets": []
    }))
    .expect("minimal catalog deserializes");

    assert_eq!(catalog.schema, ASSET_CATALOG_SCHEMA_V1);
    assert!(catalog.assets.is_empty());
    assert_eq!(
        serde_json::to_value(catalog).expect("catalog serializes"),
        json!({
            "schema": ASSET_CATALOG_SCHEMA_V1,
            "assets": []
        })
    );
}

#[test]
fn asset_catalog_validation_reports_ready_and_invalid_assets() {
    let catalog: AssetCatalogV1 =
        serde_json::from_str(include_str!("assets/catalog/readiness_catalog.v1.json"))
            .expect("catalog fixture deserializes");

    let report = pollster::block_on(Assets::new().validate_asset_catalog(&catalog));

    assert_eq!(report.schema, ASSET_READINESS_REPORT_SCHEMA_V1);
    assert!(!report.ok, "invalid catalog entries must fail closed");
    assert_eq!(report.summary.total_assets, 5);
    assert_eq!(report.summary.ready_assets, 1);
    assert_eq!(report.summary.failed_assets, 4);

    let valid = report
        .asset("variant-triangle")
        .expect("valid asset report");
    assert!(valid.ok, "valid catalog asset should pass readiness checks");
    assert_eq!(valid.preview.as_ref().expect("preview").status, "generated");
    assert_eq!(
        valid
            .asset_load_report
            .as_ref()
            .expect("asset load report")
            .schema,
        "scena.asset_load_report.v1"
    );

    assert_codes(
        report.asset("missing-source").expect("missing source"),
        &["load_failed"],
    );
    assert_codes(
        report.asset("bad-contract").expect("bad contract"),
        &[
            "source_units_unknown",
            "source_coordinate_system_unknown",
            "required_anchor_missing",
            "required_connector_missing",
            "required_material_variant_missing",
            "preview_missing",
        ],
    );
    assert_codes(
        report.asset("texture-required").expect("texture required"),
        &["base_color_texture_missing"],
    );
    assert_codes(
        report
            .asset("invalid-authored-features")
            .expect("invalid feature asset"),
        &[
            "invalid_source_coordinate_system",
            "invalid_anchor",
            "invalid_connector",
            "preview_missing",
        ],
    );

    let encoded = serde_json::to_string(&report).expect("readiness report serializes");
    let reparsed: serde_json::Value =
        serde_json::from_str(&encoded).expect("readiness report is JSON");
    assert_eq!(reparsed["schema"], ASSET_READINESS_REPORT_SCHEMA_V1);
}

fn assert_codes(asset: &scena::AssetReadinessAssetReportV1, expected: &[&str]) {
    let codes = asset
        .findings
        .iter()
        .map(|finding| finding.code.as_str())
        .collect::<Vec<_>>();
    for code in expected {
        assert!(codes.contains(code), "expected finding {code} in {codes:?}");
    }
}
