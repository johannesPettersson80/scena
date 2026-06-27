#![cfg(all(feature = "scene-host", not(target_arch = "wasm32")))]

use scena::{
    AssetPath, Color, PRODUCT_OPTIONS_SCHEMA_V1, ProductOptionGroupV1, ProductOptionV1,
    ProductOptionsV1, SceneHostCameraState, SceneHostCore, SceneHostErrorCode,
    SceneInspectionReportV1, Transform, Vec3, VisualPatchMaterialVariantV1, VisualPatchResultV1,
    VisualPatchTintV1, VisualPatchV1, VisualPatchVisibilityV1,
};

#[test]
fn product_options_apply_visual_patches_and_report_active_choices() {
    let mut host = SceneHostCore::headless(120, 80).expect("host builds");
    let import = pollster::block_on(host.instantiate_url(AssetPath::from(
        "tests/assets/gltf/material_variants_scene.gltf",
    )))
    .expect("variant asset instantiates");
    let mesh = host
        .node_handle(import, "VariantTriangle")
        .expect("variant mesh resolves");
    let accessory = host
        .add_empty(
            Some(host.root_handle()),
            Transform::at(Vec3::new(0.4, 0.0, 0.0)),
            Some("accessory"),
        )
        .expect("accessory node inserts");

    let options = ProductOptionsV1 {
        schema: PRODUCT_OPTIONS_SCHEMA_V1.to_owned(),
        groups: vec![
            ProductOptionGroupV1 {
                id: "finish".to_owned(),
                label: "Finish".to_owned(),
                active: None,
                options: vec![ProductOptionV1 {
                    id: "noon".to_owned(),
                    label: "Noon".to_owned(),
                    patch: VisualPatchV1 {
                        camera: Some(SceneHostCameraState {
                            target: Vec3::new(0.0, 0.0, 0.0),
                            distance: 3.0,
                            yaw_radians: 0.7,
                            pitch_radians: 0.2,
                        }),
                        material_variants: vec![VisualPatchMaterialVariantV1 {
                            import,
                            variant: Some("noon".to_owned()),
                        }],
                        tints: vec![VisualPatchTintV1 {
                            node: mesh,
                            tint: Some(Color::from_linear_rgba(0.2, 0.8, 0.4, 1.0)),
                        }],
                        ..VisualPatchV1::default()
                    },
                    metadata: None,
                }],
            },
            ProductOptionGroupV1 {
                id: "accessory".to_owned(),
                label: "Accessory".to_owned(),
                active: None,
                options: vec![ProductOptionV1 {
                    id: "hidden".to_owned(),
                    label: "Hidden".to_owned(),
                    patch: VisualPatchV1 {
                        visibility: vec![VisualPatchVisibilityV1 {
                            node: accessory,
                            visible: false,
                        }],
                        ..VisualPatchV1::default()
                    },
                    metadata: None,
                }],
            },
        ],
    };

    let stored = host
        .store_product_options(options)
        .expect("product options validate and store");
    assert_eq!(stored.schema, PRODUCT_OPTIONS_SCHEMA_V1);
    assert_eq!(stored.groups[0].active, None);

    let result: VisualPatchResultV1 =
        serde_json::from_str(&host.apply_product_option_json("finish", "noon").unwrap())
            .expect("product option result decodes");
    assert_eq!(result.applied.material_variants, 1);
    assert_eq!(result.applied.tints, 1);
    assert_eq!(result.applied.camera, 1);
    assert!(result.failed.is_empty());
    assert_eq!(host.camera_state().distance, 3.0);
    assert_eq!(
        host.active_material_variant(import)
            .expect("active variant reports"),
        Some("noon".to_owned())
    );

    let result = host
        .apply_product_option("accessory", "hidden")
        .expect("visibility option applies");
    assert_eq!(result.applied.visibility, 1);
    let inspection: SceneInspectionReportV1 =
        serde_json::from_str(&host.inspect_json().expect("inspection serializes"))
            .expect("inspection decodes");
    assert!(
        !inspection
            .node_by_handle(accessory)
            .expect("accessory remains in inspection")
            .visible
    );

    let report = host.product_options();
    assert_eq!(report.groups[0].active, Some("noon".to_owned()));
    assert_eq!(report.groups[1].active, Some("hidden".to_owned()));

    let json_report: ProductOptionsV1 =
        serde_json::from_str(&host.product_options_json().expect("options serialize"))
            .expect("options JSON decodes");
    assert_eq!(json_report.groups[0].active, Some("noon".to_owned()));
}

#[test]
fn product_options_fail_closed_for_unknown_groups_options_and_bad_patches() {
    let mut host = SceneHostCore::headless(120, 80).expect("host builds");
    let options = ProductOptionsV1 {
        schema: PRODUCT_OPTIONS_SCHEMA_V1.to_owned(),
        groups: vec![ProductOptionGroupV1 {
            id: "finish".to_owned(),
            label: "Finish".to_owned(),
            active: None,
            options: vec![ProductOptionV1 {
                id: "bad-import".to_owned(),
                label: "Bad import".to_owned(),
                patch: VisualPatchV1 {
                    material_variants: vec![VisualPatchMaterialVariantV1 {
                        import: u64::MAX,
                        variant: Some("noon".to_owned()),
                    }],
                    ..VisualPatchV1::default()
                },
                metadata: None,
            }],
        }],
    };
    host.store_product_options(options)
        .expect("bad patch entries are validated at apply time");

    let unknown_group = host
        .apply_product_option("missing", "bad-import")
        .expect_err("unknown group fails closed");
    assert_eq!(unknown_group.code(), SceneHostErrorCode::InvalidInput);

    let unknown_option = host
        .apply_product_option("finish", "missing")
        .expect_err("unknown option fails closed");
    assert_eq!(unknown_option.code(), SceneHostErrorCode::InvalidInput);

    let result = host
        .apply_product_option("finish", "bad-import")
        .expect("bad patch reports per-entry failures");
    assert_eq!(result.applied.material_variants, 0);
    assert_eq!(result.failed.len(), 1);
    assert_eq!(host.product_options().groups[0].active, None);
}

#[test]
fn product_options_golden_fixture_matches_live_schema_serialization() {
    let fixture = std::fs::read_to_string("tests/assets/stable-contracts/product_options.v1.json")
        .expect("product options fixture reads");
    let fixture_value: serde_json::Value =
        serde_json::from_str(&fixture).expect("product options fixture parses");
    let decoded: ProductOptionsV1 =
        serde_json::from_str(&fixture).expect("product options fixture decodes");
    let encoded = serde_json::to_value(decoded).expect("product options fixture reserializes");
    assert_eq!(encoded, fixture_value);
}
