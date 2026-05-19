use crate::app::prelude::*;
use crate::app::tests_12::{VALID_GUIDE, write_easy_scene_fixture};

#[test]
pub(crate) fn binary_render_asset_contracts_reject_text_fixtures_with_binary_extensions() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-binary-asset-contract-test");
    let fixture_dir = fixture_root.join("tests/assets/environment/generated");
    fs::create_dir_all(&fixture_dir).expect("fixture dir");
    fs::write(
        fixture_dir.join("fake.ktx2"),
        b"SCENA_CUBEMAP_V1\nencoding = rgba16f-text-fixture\n",
    )
    .expect("fixture write");
    let mut findings = Vec::new();

    check_binary_render_asset_contracts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "BINARY-ASSET-TRUTH-P9"
                && finding.message.contains("fake.ktx2")
                && finding.message.contains("text fixture data")
        }),
        "text fixtures must not be allowed to masquerade as binary render assets: {findings:?}",
    );
}

#[test]
pub(crate) fn public_fields_in_struct_detects_material_desc_visibility_regressions() {
    let source = r#"
        pub struct MaterialDesc {
            kind: MaterialKind,
            pub base_color: Color,
            pub(crate) roughness_factor: f32,
        }
    "#;

    assert_eq!(
        public_fields_in_struct(source, "MaterialDesc"),
        vec!["pub base_color: Color", "pub(crate) roughness_factor: f32"]
    );
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_missing_compressed_asset_visual_proof() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/missing-compressed-asset-proof");
    write_easy_scene_fixture(
        &fixture_root,
        VALID_GUIDE,
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    fs::remove_file(fixture_root.join("tests/m8_compressed_asset_release_proof.rs"))
        .expect("proof fixture removal");
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "PRODUCTION-ASSET-PROFILE"),
        "doctor must reject production-asset profiles without compressed visual proof: {findings:?}",
    );
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_missing_material_variant_visual_proof() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/missing-material-variant-proof");
    write_easy_scene_fixture(
        &fixture_root,
        VALID_GUIDE,
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    fs::write(
        fixture_root.join("tests/examples_visual_proof.rs"),
        "round_b_light_preset_reference_docs_image",
    )
    .expect("visual proof fixture without material variants");
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "VIEWER-MATERIAL-VARIANTS"),
        "doctor must reject viewer material variants without generated visual proof: {findings:?}",
    );
}
