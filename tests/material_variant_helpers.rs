#![cfg(all(feature = "scene-host", not(target_arch = "wasm32")))]

use scena::{
    AssetPath, SceneHostAssetImportReportV1, SceneHostCore, SceneHostErrorCode,
    SceneInspectionReportV1, VisualPatchMaterialVariantV1, VisualPatchV1,
};

#[test]
fn scene_host_material_variant_reports_include_available_and_active_state() {
    let mut host = SceneHostCore::headless(120, 80).expect("host builds");
    let import_report_json = pollster::block_on(host.instantiate_url_with_report_json(
        AssetPath::from("tests/assets/gltf/material_variants_scene.gltf"),
    ))
    .expect("variant asset instantiates with report");
    let import_report: SceneHostAssetImportReportV1 =
        serde_json::from_str(&import_report_json).expect("import report decodes");

    assert_eq!(
        import_report.material_variants,
        vec!["midnight".to_owned(), "noon".to_owned()]
    );
    assert_eq!(import_report.active_variant, None);

    let import = import_report.import;
    let apply = host
        .apply_patch(&VisualPatchV1 {
            material_variants: vec![VisualPatchMaterialVariantV1 {
                import,
                variant: Some("noon".to_owned()),
            }],
            ..VisualPatchV1::default()
        })
        .expect("variant patch applies");
    assert_eq!(apply.applied.material_variants, 1);

    let inspection: SceneInspectionReportV1 =
        serde_json::from_str(&host.inspect_json().expect("inspection serializes"))
            .expect("inspection decodes");
    let imports = inspection
        .imports
        .expect("scene host inspection reports imports");
    let variant_import = imports
        .iter()
        .find(|entry| entry.handle == import)
        .expect("variant import appears in inspection");
    assert_eq!(
        variant_import.material_variants,
        vec!["midnight".to_owned(), "noon".to_owned()]
    );
    assert_eq!(variant_import.active_variant, Some("noon".to_owned()));

    let clear = host
        .apply_patch(&VisualPatchV1 {
            material_variants: vec![VisualPatchMaterialVariantV1 {
                import,
                variant: None,
            }],
            ..VisualPatchV1::default()
        })
        .expect("variant clear patch applies");
    assert_eq!(clear.applied.material_variants, 1);
    let inspection: SceneInspectionReportV1 =
        serde_json::from_str(&host.inspect_json().expect("inspection serializes"))
            .expect("inspection decodes");
    let cleared_import = inspection
        .imports
        .expect("imports remain reported")
        .into_iter()
        .find(|entry| entry.handle == import)
        .expect("variant import remains reported");
    assert_eq!(cleared_import.active_variant, None);
}

#[test]
fn scene_host_material_variant_patch_fails_for_stale_and_ambiguous_imports() {
    let mut host = SceneHostCore::headless(120, 80).expect("host builds");
    let stale_import = pollster::block_on(host.instantiate_url(AssetPath::from(
        "tests/assets/gltf/material_variants_scene.gltf",
    )))
    .expect("variant asset instantiates");
    host.remove_import(stale_import)
        .expect("import can be removed before stale probe");

    let ambiguous_import = pollster::block_on(host.instantiate_url(AssetPath::from(
        "tests/assets/gltf/material_variants_ambiguous_scene.gltf",
    )))
    .expect("ambiguous variant asset instantiates");

    let result = host
        .apply_patch(&VisualPatchV1 {
            material_variants: vec![
                VisualPatchMaterialVariantV1 {
                    import: stale_import,
                    variant: Some("noon".to_owned()),
                },
                VisualPatchMaterialVariantV1 {
                    import: ambiguous_import,
                    variant: Some("same".to_owned()),
                },
            ],
            ..VisualPatchV1::default()
        })
        .expect("per-entry variant failures are reported");

    assert_eq!(result.applied.material_variants, 0);
    assert_eq!(result.failed.len(), 2);
    assert_eq!(result.failed[0].channel, "material_variants");
    assert_eq!(result.failed[0].handle, Some(stale_import));
    assert_eq!(result.failed[0].code, SceneHostErrorCode::StaleImportHandle);
    assert_eq!(result.failed[1].channel, "material_variants");
    assert_eq!(result.failed[1].handle, Some(ambiguous_import));
    assert_eq!(result.failed[1].code, SceneHostErrorCode::Lookup);
    assert!(result.failed[1].message.contains("ambiguous"));
    assert_eq!(
        host.active_material_variant(ambiguous_import)
            .expect("ambiguous import remains resolvable"),
        None
    );
}
