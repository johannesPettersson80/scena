use crate::app::prelude::*;

#[test]
pub(crate) fn binary_render_asset_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_binary_render_asset_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

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
pub(crate) fn m7_ergonomics_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_m7_ergonomics_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
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

/// Plan line 1588 / Phase 4: doctor rule regression coverage. Each of the
/// following tests writes a fixture whose required documentation
/// substring is missing and asserts the matching DOCS-* rule fires.
/// Closes the regression gap for the documentation contracts that were
/// previously enforced only by the live tree.
#[test]
pub(crate) fn doctor_rejects_render_lifecycle_doc_missing_substring_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/lifecycle-doc-stub");
    let doc_path = fixture_root.join("docs/specs/render-lifecycle.md");
    fs::create_dir_all(doc_path.parent().expect("lifecycle parent")).expect("fixture dir");
    fs::write(
        &doc_path,
        "# Render lifecycle\n\nThis stub deliberately omits the contract substrings.\n",
    )
    .expect("lifecycle fixture");
    let mut findings = Vec::new();

    check_required_doc_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "DOCS-LIFECYCLE"),
        "doctor must reject docs/specs/render-lifecycle.md when the contract substrings \
         are missing: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_asset_gltf_doc_missing_substring_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/gltf-doc-stub");
    let doc_path = fixture_root.join("docs/specs/asset-gltf-contract.md");
    fs::create_dir_all(doc_path.parent().expect("gltf parent")).expect("fixture dir");
    fs::write(
        &doc_path,
        "# glTF contract\n\nStub that omits the connector and stale-import contract terms.\n",
    )
    .expect("gltf fixture");
    let mut findings = Vec::new();

    check_required_doc_contracts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| finding.rule == "DOCS-GLTF"),
        "doctor must reject docs/specs/asset-gltf-contract.md when its required \
         substrings are missing: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_visual_quality_doc_missing_substring_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/visual-doc-stub");
    let doc_path = fixture_root.join("docs/specs/visual-quality-contract.md");
    fs::create_dir_all(doc_path.parent().expect("visual parent")).expect("fixture dir");
    fs::write(
        &doc_path,
        "# Visual quality\n\nStub without color management or determinism clauses.\n",
    )
    .expect("visual fixture");
    let mut findings = Vec::new();

    check_required_doc_contracts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| finding.rule == "DOCS-VISUAL"),
        "doctor must reject docs/specs/visual-quality-contract.md when its required \
         substrings are missing: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_doctor_contract_doc_missing_substring_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/doctor-doc-stub");
    let doc_path = fixture_root.join("docs/specs/doctor-contract.md");
    fs::create_dir_all(doc_path.parent().expect("doctor parent")).expect("fixture dir");
    fs::write(
        &doc_path,
        "# Doctor contract\n\nStub without the required CLI invocation substrings.\n",
    )
    .expect("doctor fixture");
    let mut findings = Vec::new();

    check_required_doc_contracts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| finding.rule == "DOCS-DOCTOR"),
        "doctor must reject docs/specs/doctor-contract.md when its required CLI \
         invocations are missing: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_release_gates_doc_missing_substring_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/release-gates-doc-stub");
    let doc_path = fixture_root.join("docs/specs/release-gates.md");
    fs::create_dir_all(doc_path.parent().expect("release-gates parent")).expect("fixture dir");
    fs::write(
        &doc_path,
        "# Release gates\n\nStub without the doctor or full-mode contract terms.\n",
    )
    .expect("release-gates fixture");
    let mut findings = Vec::new();

    check_required_doc_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "DOCS-RELEASE-GATES"),
        "doctor must reject docs/specs/release-gates.md when its required substrings \
         are missing: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_public_api_doc_missing_substring_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/public-api-doc-stub");
    let doc_path = fixture_root.join("docs/specs/public-api.md");
    fs::create_dir_all(doc_path.parent().expect("public-api parent")).expect("fixture dir");
    fs::write(
        &doc_path,
        "# Public API\n\nStub without the public-API surface contract terms.\n",
    )
    .expect("public-api fixture");
    let mut findings = Vec::new();

    check_required_doc_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "DOCS-PUBLIC-API"),
        "doctor must reject docs/specs/public-api.md when the public-API surface \
         contract substrings are missing: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_required_docs_missing_file_regression() {
    // DOCS-REQUIRED: the doctor walks `REQUIRED_DOCS` and asserts every
    // file is present + non-empty. A fixture missing one such doc
    // regresses the rule. Picking `docs/decisions/ADR-0005-local-release-candidate-deferrals.md`
    // because it is part of the canonical required set and silent
    // deletion would breach the local-release-candidate paperwork.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/required-doc-missing");
    // Empty fixture root → every required doc is missing.
    fs::create_dir_all(&fixture_root).expect("fixture dir");
    let mut findings = Vec::new();

    require_files(&fixture_root, &mut findings, "DOCS-REQUIRED", REQUIRED_DOCS);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "DOCS-REQUIRED"),
        "doctor must reject the repo when REQUIRED_DOCS files are missing: \
         {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_m5_release_cargo_missing_metadata_regression() {
    // ARCH-M5-RELEASE: Cargo.toml must keep the rust-version, docs.rs
    // documentation pointer, keywords, categories, include list, and
    // hybrid `["rlib", "cdylib"]` crate type. A stub manifest without
    // those entries regresses the v1 release contract.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/m5-release-cargo-stub");
    let manifest_path = fixture_root.join("Cargo.toml");
    fs::create_dir_all(manifest_path.parent().expect("manifest parent")).expect("fixture dir");
    fs::write(
        &manifest_path,
        "[package]\nname = \"scena\"\nversion = \"0.0.0\"\n",
    )
    .expect("manifest fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-M5-RELEASE",
        "Cargo.toml",
        &[
            "version = \"1.1.0\"",
            "rust-version = ",
            "documentation = \"https://docs.rs/scena\"",
            "keywords = [",
            "categories = [",
            "include = [",
            "crate-type = [\"rlib\", \"cdylib\"]",
        ],
    );

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "ARCH-M5-RELEASE"),
        "doctor must reject Cargo.toml stubs that drop the v1 release-metadata \
         surface: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_visual_browser_m1_missing_artifact_regression() {
    // VISUAL-BROWSER-M1: each browser-probe workflow must declare its
    // visual artifact under `target/gate-artifacts/m6-browser-visual/`
    // with a renderer/color/tolerance/source contract; absence regresses
    // the M6 browser parity gate.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/visual-browser-m1-stub");
    let stub_path = fixture_root.join("src/browser_probe/workflows/pbr.rs");
    fs::create_dir_all(stub_path.parent().expect("workflow parent")).expect("fixture dir");
    fs::write(
        &stub_path,
        "// Stub workflow without the visual-artifact declarations.\n",
    )
    .expect("workflow fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "VISUAL-BROWSER-M1",
        "src/browser_probe/workflows/pbr.rs",
        &["pbr-environment-lit", "renderer", "tolerance"],
    );

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "VISUAL-BROWSER-M1"),
        "doctor must reject browser-probe workflows that drop their visual \
         artifact declarations: {findings:?}",
    );
}
