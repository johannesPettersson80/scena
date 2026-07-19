use crate::app::prelude::*;

#[test]
pub(crate) fn doctor_rejects_feature_gated_contract_suite_without_explicit_command() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/feature-gated-contract-suite");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(fixture_root.join("docs/checklists")).expect("checklist fixture dir");
    fs::create_dir_all(fixture_root.join("tests")).expect("tests fixture dir");
    fs::write(
        fixture_root.join("docs/checklists/application-builder-roadmap.md"),
        "# Roadmap\n\nNo feature-enabled contract command here.\n",
    )
    .expect("roadmap fixture");
    fs::write(
        fixture_root.join("tests/example_contracts.rs"),
        "#![cfg(feature = \"inspection\")]\n\n#[test]\nfn example_contract() {}\n",
    )
    .expect("feature-gated test fixture");
    let mut findings = Vec::new();

    check_feature_gated_contract_tests_documented(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "TESTS-FEATURE-GATED-CONTRACT-SUITES"
                && finding
                    .message
                    .contains("cargo test --features inspection --test example_contracts")
        }),
        "doctor must require explicit feature-enabled commands for gated contract suites: {findings:?}",
    );
}

#[test]
pub(crate) fn feature_gated_contract_suites_are_documented_in_current_roadmap() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_feature_gated_contract_tests_documented(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn doctor_rejects_schema_docs_reference_missing_from_catalog() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/schema-docs-catalog");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(fixture_root.join("docs")).expect("docs fixture dir");
    fs::create_dir_all(fixture_root.join("tests/assets/stable-contracts"))
        .expect("stable contract fixture dir");
    fs::write(fixture_root.join("README.md"), "").expect("readme fixture");
    fs::write(
        fixture_root.join("AGENTS.md"),
        "# AGENTS\n\nNo schema refs here.\n",
    )
    .expect("agents fixture");
    fs::write(
        fixture_root.join("docs/schema-contracts.md"),
        "This doc references `scena.missing_contract.v1` and proof artifact `scena.m6.example_proof.v1`.\n",
    )
    .expect("schema docs fixture");
    fs::write(
        fixture_root.join("tests/assets/stable-contracts/schema_catalog.v1.json"),
        r#"{"schema":"scena.schema_catalog.v1","entries":[]}"#,
    )
    .expect("schema catalog fixture");
    let mut findings = Vec::new();

    crate::app::doctor_docs::schema_references::check_schema_doc_references_listed_in_catalog(
        &fixture_root,
        &mut findings,
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "STABLE-CONTRACT-EVIDENCE"
                && finding.message.contains("scena.missing_contract.v1")
        }),
        "doctor must reject documented schemas missing from the schema catalog: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_requires_canonical_rfc_when_missing() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/missing-canonical-rfc");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(&fixture_root).expect("canonical RFC fixture root");
    let mut findings = Vec::new();

    require_files(&fixture_root, &mut findings, "DOCS-REQUIRED", REQUIRED_DOCS);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "DOCS-REQUIRED"
                && finding.message.contains("docs/RFC-rust-3d-renderer.md")
        }),
        "the canonical RFC must be an unconditional required document: {findings:?}",
    );
}

#[test]
pub(crate) fn require_contains_rejects_missing_active_checklist() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/missing-active-checklist");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(&fixture_root).expect("missing active checklist fixture root");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "DOCS-ACTIVE-PIN",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &["ARCH-CAMERA-DEPTH"],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "DOCS-ACTIVE-PIN"
                && finding.message.contains("m2-lighting-depth-clipping.md")
        }),
        "missing active checklist pins must fail closed: {findings:?}",
    );
}

#[test]
pub(crate) fn forbid_contains_rejects_missing_active_target() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/missing-forbid-target");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(&fixture_root).expect("missing forbid target fixture root");
    let mut findings = Vec::new();

    forbid_contains(
        &fixture_root,
        &mut findings,
        "FORBID-MISSING",
        "src/render/missing.rs",
        &["forbidden"],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "FORBID-MISSING" && finding.message.contains("could not read")
        }),
        "missing forbid-scan targets must fail closed: {findings:?}",
    );
}

#[test]
pub(crate) fn retired_document_allowlist_is_exact() {
    assert!(crate::app::doctor_docs::is_retired_internal_doc(
        "docs/release-notes-template.md"
    ));
    assert!(!crate::app::doctor_docs::is_retired_internal_doc(
        "docs/RFC-rust-3d-renderer.md"
    ));
    assert!(!crate::app::doctor_docs::is_retired_internal_doc(
        "docs/specs/typoed-active-contract.md"
    ));
}

#[test]
pub(crate) fn missing_exact_retired_document_is_permitted_by_contract_pin() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/exact-retired-document");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(&fixture_root).expect("retired document fixture root");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "RETIRED-DOCUMENT",
        "docs/release-notes-template.md",
        &["unused"],
    );

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn env_var_scanner_finds_javascript_process_env_reads() {
    let names = find_env_var_names(
        "const direct = process.env.SCENA_BROWSER_BACKENDS; \
         const indexed = process.env['SCENA_BROWSER_EXECUTABLE'];",
    );

    assert!(names.contains(&"SCENA_BROWSER_BACKENDS".to_string()));
    assert!(names.contains(&"SCENA_BROWSER_EXECUTABLE".to_string()));
}

#[test]
pub(crate) fn env_flag_documentation_scan_is_recursive_and_includes_javascript() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/recursive-js-env");
    let nested = fixture_root.join("tests/browser/nested/probe.js");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(nested.parent().expect("nested JS fixture parent"))
        .expect("nested JS fixture dir");
    fs::write(
        &nested,
        "const value = process.env.SCENA_UNDOCUMENTED_BROWSER_FLAG;\n",
    )
    .expect("nested JS env fixture");
    fs::write(fixture_root.join("CLAUDE.md"), "# Test environment flags\n")
        .expect("fixture CLAUDE.md");
    let mut findings = Vec::new();

    check_tests_env_flags_documented(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "TESTS-ENV-FLAGS-DOCUMENTED"
                && finding.message.contains("SCENA_UNDOCUMENTED_BROWSER_FLAG")
                && finding.message.contains("tests/browser/nested/probe.js")
        }),
        "nested JavaScript env reads must be checked against contributor docs: {findings:?}",
    );
}

#[test]
pub(crate) fn ignored_release_test_scan_is_recursive() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/recursive-ignore");
    let nested = fixture_root.join("tests/external/cardine/proof.rs");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(nested.parent().expect("nested ignored fixture parent"))
        .expect("nested ignored fixture dir");
    fs::write(
        &nested,
        "#[test]\n#[ignore]\nfn external_release_proof() {}\n",
    )
    .expect("nested ignored fixture");
    let mut findings = Vec::new();

    check_no_ignored_release_tests(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "TESTS-NO-IGNORED-RELEASE-PROOF"
                && finding.message.contains("tests/external/cardine/proof.rs")
        }),
        "nested ignored release tests must fail closed: {findings:?}",
    );
}

#[test]
pub(crate) fn arbitrary_rust_marker_cannot_be_satisfied_by_output_shader_companion() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/restricted-wgsl-companion");
    let output = fixture_root.join("src/render/gpu/output.rs");
    let shader = fixture_root.join("src/render/gpu/output_shader.wgsl");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(output.parent().expect("output fixture parent"))
        .expect("output fixture dir");
    fs::write(
        &output,
        "const GPU_TRIANGLE_SHADER: &str = include_str!(\"output_shader.wgsl\");\n",
    )
    .expect("output fixture");
    fs::write(
        &shader,
        "// fn pinned_test_name_only_in_a_shader_comment() {}\n",
    )
    .expect("shader fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "RESTRICTED-WGSL-COMPANION",
        "src/render/gpu/output.rs",
        &["pinned_test_name_only_in_a_shader_comment"],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RESTRICTED-WGSL-COMPANION"
                && finding
                    .message
                    .contains("pinned_test_name_only_in_a_shader_comment")
        }),
        "arbitrary Rust markers must not resolve through a sibling shader: {findings:?}",
    );
}

#[test]
pub(crate) fn required_ci_workflow_rejects_browser_allow_unavailable() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/unsafe-required-webgpu");
    let workflow = fixture_root.join(".github/workflows/ci.yml");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(workflow.parent().expect("workflow fixture parent"))
        .expect("workflow fixture dir");
    fs::write(
        &workflow,
        "jobs:\n  linux-browser-webgpu:\n    env:\n      SCENA_BROWSER_ALLOW_UNAVAILABLE: \"1\"\n",
    )
    .expect("workflow fixture");
    let mut findings = Vec::new();

    forbid_contains(
        &fixture_root,
        &mut findings,
        "REQUIRED-WEBGPU-STRICT",
        ".github/workflows/ci.yml",
        &["SCENA_BROWSER_ALLOW_UNAVAILABLE"],
    );

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "REQUIRED-WEBGPU-STRICT"),
        "required browser lanes must reject allow-unavailable downgrades: {findings:?}",
    );
}

#[test]
pub(crate) fn shipped_feature_cannot_be_marked_deferred_in_current_checklist() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/reverse-status-drift");
    let authority = fixture_root.join("docs/checklists/stunning-renders-and-performance.md");
    let duplicate = fixture_root.join("docs/checklists/current-roadmap.md");
    let source = fixture_root.join("src/render/area_ltc.rs");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(authority.parent().expect("authority fixture parent"))
        .expect("checklist fixture dir");
    fs::create_dir_all(source.parent().expect("source fixture parent"))
        .expect("source fixture dir");
    fs::write(
        &authority,
        "## A3 — Soft area lights (LTC rect/disc/sphere) — [shipped]\n",
    )
    .expect("authority fixture");
    fs::write(
        &duplicate,
        "- **Area lights with LTC**. Status: **[deferred, later-product-lighting]**.\n",
    )
    .expect("duplicate fixture");
    fs::write(&source, "fn sample_ltc_tables() {}\n").expect("shipped source fixture");
    let mut findings = Vec::new();

    check_shipped_feature_status_drift(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "DOCS-REVERSE-STATUS-DRIFT"
                && finding.message.contains("Area lights with LTC")
        }),
        "shipped source plus accepted proof must reject a deferred duplicate: {findings:?}",
    );
}

#[test]
pub(crate) fn public_cli_schema_literal_missing_from_catalog_is_rejected() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/public-schema-discovery");
    let source = fixture_root.join("src/bin/scena.rs");
    let catalog = fixture_root.join("tests/assets/stable-contracts/schema_catalog.v1.json");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(source.parent().expect("source fixture parent"))
        .expect("source fixture dir");
    fs::create_dir_all(catalog.parent().expect("catalog fixture parent"))
        .expect("catalog fixture dir");
    fs::write(
        &source,
        "fn version_json() { let _ = \"scena.cli_version.v1\"; }\n",
    )
    .expect("public schema source fixture");
    fs::write(
        &catalog,
        r#"{"schema":"scena.schema_catalog.v1","entries":[]}"#,
    )
    .expect("catalog fixture");
    let mut findings = Vec::new();

    crate::app::doctor_docs::schema_references::check_public_cli_schemas_listed_in_catalog(
        &fixture_root,
        &mut findings,
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "PUBLIC-SCHEMA-DISCOVERY"
                && finding.message.contains("scena.cli_version.v1")
        }),
        "public CLI contract literals must be cataloged: {findings:?}",
    );
}

#[test]
pub(crate) fn rust_test_name_in_comment_does_not_satisfy_item_contract() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/comment-only-test-name");
    let source = fixture_root.join("tests/proof.rs");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(source.parent().expect("test fixture parent")).expect("test fixture dir");
    fs::write(
        &source,
        "// #[test]\n// fn required_rendered_output_proof() {}\n#[test]\nfn other_test() {}\n",
    )
    .expect("test source fixture");
    let mut findings = Vec::new();

    require_rust_test_functions(
        &fixture_root,
        &mut findings,
        "RUST-TEST-ITEM",
        "tests/proof.rs",
        &["required_rendered_output_proof"],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RUST-TEST-ITEM"
                && finding.message.contains("required_rendered_output_proof")
        }),
        "comment-only test names must not satisfy an item contract: {findings:?}",
    );
}

#[test]
pub(crate) fn recursive_control_flow_scan_rejects_unregistered_early_return() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/unregistered-early-return");
    let source = fixture_root.join("tests/nested/proof.rs");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(source.parent().expect("early-return fixture parent"))
        .expect("early-return fixture dir");
    fs::write(
        &source,
        "#[test]\nfn required_proof() { if adapter_unavailable() { return; } }\n",
    )
    .expect("early-return fixture");
    let mut findings = Vec::new();

    check_test_control_flow_policy(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "TESTS-CONTROL-FLOW-POLICY"
                && finding.message.contains("tests/nested/proof.rs")
        }),
        "unregistered nested early returns must fail closed: {findings:?}",
    );
}

#[test]
pub(crate) fn cfg_test_module_scan_ignores_commented_attribute_and_module() {
    let names = rust_cfg_test_module_names(
        "// #[cfg(test)]\n// mod tests_comment;\n#[cfg(test)]\nmod tests_real;\n",
    );

    assert!(names.contains("tests_real"));
    assert!(!names.contains("tests_comment"));
}
