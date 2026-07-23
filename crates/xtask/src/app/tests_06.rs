use crate::app::prelude::*;

#[test]
pub(crate) fn doctor_rejects_public_example_gate_without_all_features() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/public-example-features");
    let workflow = fixture_root.join(".github/workflows/ci.yml");
    fs::create_dir_all(workflow.parent().expect("workflow parent")).expect("fixture dir");
    fs::write(&workflow, "run: cargo check --examples\n").expect("workflow fixture");
    let mut findings = Vec::new();

    crate::app::doctor_scene_platform::check_public_example_compile_coverage(
        &fixture_root,
        &mut findings,
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-PUBLIC-EXAMPLE-COVERAGE"
                && finding.message.contains("--all-features")
        }),
        "doctor must reject a public-example gate that skips required-feature examples: {findings:?}"
    );
}

#[test]
pub(crate) fn public_example_compile_coverage_is_green_for_the_real_tree() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();
    crate::app::doctor_scene_platform::check_public_example_compile_coverage(&root, &mut findings);
    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn doctor_rejects_world_baked_prepare_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/world-baked-prepare");
    let prepare_path = fixture_root.join("src/render/prepare.rs");
    fs::create_dir_all(prepare_path.parent().expect("prepare parent")).expect("fixture dir");
    fs::write(
        &prepare_path,
        "fn collect() { let _ = transform_primitive(primitive, transform, origin_shift); }\n",
    )
    .expect("prepare fixture");
    let mut findings = Vec::new();

    forbid_contains(
        &fixture_root,
        &mut findings,
        "ARCH-RENDER-WORLD-BAKE",
        "src/render/prepare.rs",
        &["transform_primitive(primitive, transform, origin_shift)"],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-RENDER-WORLD-BAKE"
                && finding.message.contains("transform_primitive")
        }),
        "doctor must reject prepare.rs that bakes per-renderable world transforms into \
         vertex positions instead of stamping them through prepared_primitive(...): \
         {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_agents_md_missing_doctor_runbook_regression() {
    // AGENTS-VALIDATION: AGENTS.md must instruct contributors to run
    // `cargo run -p xtask -- doctor --full` and reference the
    // scena-doctor skill. A workspace whose AGENTS.md drops either
    // contract must surface a finding so the doctor entrypoint never
    // becomes invisible to new agents.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/agents-md-missing-doctor");
    fs::create_dir_all(&fixture_root).expect("fixture dir");
    fs::write(
        fixture_root.join("AGENTS.md"),
        "# Stub AGENTS\n\nContributors should run tests.\n",
    )
    .expect("agents stub");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "AGENTS-VALIDATION",
        "AGENTS.md",
        &["cargo run -p xtask -- doctor --full", "Use `scena-doctor`"],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "AGENTS-VALIDATION"
                && finding
                    .message
                    .contains("cargo run -p xtask -- doctor --full")
        }),
        "doctor must reject AGENTS.md that drops the doctor runbook \
         reference: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_source_file_with_out_of_scope_term_regression() {
    // ARCH-SCOPE: scena is a renderer, not a domain engine. Source
    // files referencing domain-specific terms (plc, robotics, robot,
    // etc.) drift the project outside its non-goals. The fixture
    // writes a source file containing "plc" and asserts the
    // architecture doctor surfaces a renderer-forbidden-term finding.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/source-scope-out-of-scope");
    let src_dir = fixture_root.join("src");
    fs::create_dir_all(&src_dir).expect("src dir");
    fs::write(
        src_dir.join("foo.rs"),
        "// Wires plc telemetry into the renderer.\npub fn run() {}\n",
    )
    .expect("foo source");
    let mut findings = Vec::new();

    check_source_scope(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-SCOPE"
                && finding.message.contains("src/foo.rs")
                && finding.message.contains("plc")
        }),
        "doctor must reject source files containing renderer-forbidden \
         scope terms like 'plc': {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_public_contract_forbidden_vocab_regression() {
    // Public schema/API contract docs must stay renderer-neutral. This fixture
    // plants terms from the WASM scene-host denylist in the schema contract
    // surface and expects doctor to fail closed.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/public-contract-vocab");
    let docs_dir = fixture_root.join("docs");
    fs::create_dir_all(&docs_dir).expect("docs dir");
    fs::write(
        docs_dir.join("schema-contracts.md"),
        "# Stable JSON contract policy\n\nDo not add joint or urdf fields here.\n",
    )
    .expect("schema contract fixture");
    let scene_host_dir = fixture_root.join("src/scene_host");
    fs::create_dir_all(&scene_host_dir).expect("scene host dir");
    fs::write(
        scene_host_dir.join("handles.rs"),
        "pub fn urdf_joint_handle_name() -> &'static str { \"bad\" }\n",
    )
    .expect("scene host fixture");
    let mut findings = Vec::new();

    check_source_scope(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-PUBLIC-CONTRACT-VOCAB"
                && finding.message.contains("docs/schema-contracts.md")
                && (finding.message.contains("joint") || finding.message.contains("urdf"))
        }),
        "doctor must reject public contract docs containing domain-specific \
         vocabulary such as 'joint' or 'urdf': {findings:?}",
    );
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-PUBLIC-CONTRACT-VOCAB"
                && finding.message.contains("src/scene_host/handles.rs")
                && (finding.message.contains("joint") || finding.message.contains("urdf"))
        }),
        "doctor must reject new public contract submodules under \
         src/scene_host/: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_missing_stable_contract_release_evidence_regression() {
    // STABLE-CONTRACT-EVIDENCE: public JSON contracts must keep docs,
    // examples, and golden fixtures together. This fixture plants only the
    // schema docs and expects the docs doctor to reject the missing example
    // and fixture evidence.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root =
        root.join("target/xtask-doctor-regressions/stable-contract-evidence-missing");
    let docs_dir = fixture_root.join("docs");
    fs::create_dir_all(&docs_dir).expect("docs dir");
    fs::write(
        docs_dir.join("schema-contracts.md"),
        "# Stable JSON contract policy\n\n\
         scena.scene_inspection.v1\n\
         scena.capability_report.v1\n\
         scena.capture.v1\n\
         scena.annotation_projection.v1\n\
         scena.asset_geometry_summary.v1\n\
         scena.asset_load_report.v1\n\
         scena.scene_host_asset_import.v1\n\
         scena.visual_patch.v1\n\
         visual_patch_result.v1.json\n\
         tests/assets/stable-contracts\n",
    )
    .expect("schema contract fixture");
    let mut findings = Vec::new();

    check_stable_contract_release_evidence(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "STABLE-CONTRACT-EVIDENCE"
                && finding.message.contains("examples/scene_host_contracts.rs")
        }),
        "doctor must reject missing native contract example: {findings:?}",
    );
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "STABLE-CONTRACT-EVIDENCE"
                && finding
                    .message
                    .contains("tests/assets/stable-contracts/capture.v1.json")
        }),
        "doctor must reject missing golden JSON fixtures: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_schema_catalog_missing_stable_fixture_schema_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root =
        root.join("target/xtask-doctor-regressions/stable-contract-schema-catalog-missing");
    let contracts_dir = fixture_root.join("tests/assets/stable-contracts");
    fs::create_dir_all(&contracts_dir).expect("stable-contracts dir");
    fs::write(
        contracts_dir.join("schema_catalog.v1.json"),
        r#"{
  "schema": "scena.schema_catalog.v1",
  "entries": [
    {
      "schema": "scena.schema_catalog.v1",
      "owner_module": "schema_catalog",
      "summary": "fixture",
      "fixture_path": "tests/assets/stable-contracts/schema_catalog.v1.json"
    }
  ]
}"#,
    )
    .expect("schema catalog fixture");
    let mut findings = Vec::new();

    check_stable_contract_release_evidence(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "STABLE-CONTRACT-EVIDENCE"
                && finding
                    .message
                    .contains("must list stable fixture schema scena.render_introspection.v1")
        }),
        "doctor must reject stable fixtures missing from schema catalog: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_required_module_layout_with_missing_files_regression() {
    // ARCH-REQUIRED: the architecture doctor must reject any workspace
    // checkout missing one of the canonical source modules listed in
    // `REQUIRED_SOURCE_MODULES`. We simulate a fresh-clone-with-missing-
    // files scenario by pointing `require_files` at an empty fixture
    // root and asserting the helper surfaces a per-path finding.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/required-modules-missing");
    fs::create_dir_all(&fixture_root).expect("fixture dir");
    let mut findings = Vec::new();

    require_files(
        &fixture_root,
        &mut findings,
        "ARCH-REQUIRED",
        &["src/lib.rs", "src/render.rs"],
    );

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "ARCH-REQUIRED"
                && finding.message.contains("src/lib.rs")
                && finding.message.contains("missing required file")),
        "doctor must reject a checkout missing src/lib.rs: {findings:?}",
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "ARCH-REQUIRED"
                && finding.message.contains("src/render.rs")),
        "doctor must reject a checkout missing src/render.rs: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_markdown_with_stale_doc_terms_regression() {
    // DOCS-STALE-TERM: any markdown document containing "TODO", "FIXME",
    // "TBD", or other documented stale-author markers must surface a
    // per-file finding so doc rot can never silently land. Mirrors the
    // pattern exercised by `doctor_rejects_markdown_link_to_missing_target_regression`.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/markdown-stale-terms");
    let docs_dir = fixture_root.join("docs/specs");
    fs::create_dir_all(&docs_dir).expect("docs dir");
    fs::write(fixture_root.join("README.md"), "# Fixture readme\n").expect("readme stub");
    fs::write(fixture_root.join("AGENTS.md"), "# Fixture agents\n").expect("agents stub");
    fs::write(
        docs_dir.join("stale.md"),
        "# Stale-term fixture\n\nTODO: finish this document before shipping.\n",
    )
    .expect("stale-term fixture");
    let mut findings = Vec::new();

    check_for_stale_doc_terms(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "DOCS-STALE-TERM"
                && finding.message.contains("stale.md")
                && finding.message.contains("TODO")
        }),
        "doctor must reject markdown documents that retain author-stale \
         markers like TODO/FIXME/TBD so doc rot cannot ship: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_markdown_link_to_missing_target_regression() {
    // DOCS-LINKS: a markdown link to a missing relative target must surface
    // a finding so broken intra-doc references can never silently ship.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/markdown-broken-link");
    let docs_dir = fixture_root.join("docs/specs");
    fs::create_dir_all(&docs_dir).expect("docs dir");
    fs::write(fixture_root.join("README.md"), "# Fixture readme\n").expect("readme stub");
    fs::write(fixture_root.join("AGENTS.md"), "# Fixture agents\n").expect("agents stub");
    fs::write(
        docs_dir.join("broken.md"),
        "# Broken link fixture\n\nSee [docs that do not exist](does-not-exist.md).\n",
    )
    .expect("broken-link fixture");
    let mut findings = Vec::new();

    check_markdown_links(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "DOCS-LINKS" && finding.message.contains("does-not-exist.md")
        }),
        "doctor must reject markdown documents that link to missing relative \
         targets so broken intra-doc references can never ship: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_material_desc_public_field_regression() {
    // ARCH-ASSET-API: src/material.rs MaterialDesc must keep its fields
    // private so the descriptor stays an opaque builder-only value.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/material-desc-public-field");
    let material_path = fixture_root.join("src/material.rs");
    fs::create_dir_all(material_path.parent().expect("src dir")).expect("fixture dir");
    fs::write(
        &material_path,
        "pub struct MaterialDesc {\n    pub leaked_field: u32,\n}\n",
    )
    .expect("material fixture");
    let mut findings = Vec::new();

    check_material_desc_fields_private(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-ASSET-API" && finding.message.contains("leaked_field")
        }),
        "doctor must reject src/material.rs MaterialDesc declaring a public \
         field so the descriptor stays an opaque builder-only value: {findings:?}",
    );
}
