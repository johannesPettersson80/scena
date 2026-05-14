use crate::app::prelude::*;

#[test]
pub(crate) fn env_var_scanner_finds_std_env_var_calls() {
    let source = r#"
        fn run() {
            if std::env::var("SCENA_USE_GPU").is_ok() {}
            let _ = env::var("MY_OTHER_FLAG");
            let _ = env::var_os("OS_ONLY");
        }
    "#;
    let names = find_env_var_names(source);
    assert!(names.contains(&"SCENA_USE_GPU".to_string()));
    assert!(names.contains(&"MY_OTHER_FLAG".to_string()));
    assert!(names.contains(&"OS_ONLY".to_string()));
}

#[test]
pub(crate) fn env_var_scanner_deduplicates_repeated_names() {
    let source = r#"
        if env::var("FOO").is_ok() {}
        if env::var("FOO").is_err() {}
    "#;
    let names = find_env_var_names(source);
    assert_eq!(names.iter().filter(|n| *n == "FOO").count(), 1);
}

#[test]
pub(crate) fn cpu_ibl_gap_documented_passes_for_current_repo() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();
    check_cpu_ibl_gap_documented(&root, &mut findings);
    let gap: Vec<_> = findings
        .iter()
        .filter(|f| f.rule == "CPU-IBL-GAP-DOCUMENTED")
        .collect();
    assert!(
        gap.is_empty(),
        "Phase 5.4 cpu-ibl-gap doc must keep doctor green; got: {:?}",
        gap,
    );
}

#[test]
pub(crate) fn m8_real_asset_dual_lane_passes_for_current_test_file() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();
    check_m8_real_asset_dual_lane(&root, &mut findings);
    let dual_lane: Vec<_> = findings
        .iter()
        .filter(|f| f.rule == "M8-REAL-ASSET-DUAL-LANE")
        .collect();
    assert!(
        dual_lane.is_empty(),
        "Phase 3 m8 test split must keep doctor green; got: {:?}",
        dual_lane,
    );
}

#[test]
pub(crate) fn tests_env_flags_documented_passes_when_flag_in_claude_md() {
    let root = repo_root().expect("test runs inside the scena workspace");
    // Sanity: the rule should not fire today (Stage 0 added CLAUDE.md
    // with both SCENA_USE_GPU and VK_ICD_FILENAMES documented).
    let mut findings = Vec::new();
    check_tests_env_flags_documented(&root, &mut findings);
    let flag_findings: Vec<_> = findings
        .iter()
        .filter(|f| f.rule == "TESTS-ENV-FLAGS-DOCUMENTED")
        .collect();
    assert!(
        flag_findings.is_empty(),
        "Stage 0 CLAUDE.md must list every test env flag; got: {:?}",
        flag_findings,
    );
}

#[test]
pub(crate) fn parses_doctor_modes() {
    assert_eq!(
        parse_command(vec!["doctor".into(), "--docs".into()]),
        Ok(Command::Doctor(DoctorMode::Docs))
    );
    assert_eq!(
        parse_command(vec!["doctor".into(), "--architecture".into()]),
        Ok(Command::Doctor(DoctorMode::Architecture))
    );
    assert_eq!(
        parse_command(vec!["doctor".into(), "--full".into()]),
        Ok(Command::Doctor(DoctorMode::Full))
    );
    assert_eq!(
        parse_command(vec!["doctor".into()]),
        Ok(Command::Doctor(DoctorMode::Full))
    );
    assert_eq!(
        parse_command(vec!["claim-audit".into()]),
        Ok(Command::ClaimAudit)
    );
    assert_eq!(
        parse_command(vec!["architecture-map".into()]),
        Ok(Command::ArchitectureMap)
    );
    assert_eq!(
        parse_command(vec!["release-lane-artifact".into(), "macos-metal".into()]),
        Ok(Command::ReleaseLaneArtifact("macos-metal".into()))
    );
    assert_eq!(
        parse_command(vec!["release-readiness".into()]),
        Ok(Command::ReleaseReadiness)
    );
    assert_eq!(
        parse_command(vec![
            "stage-release-artifacts".into(),
            "target/release-artifacts".into(),
            "target/release-bundle".into()
        ]),
        Ok(Command::StageReleaseArtifacts {
            input: "target/release-artifacts".into(),
            output: "target/release-bundle".into()
        })
    );
}

#[test]
pub(crate) fn rejects_unknown_command() {
    assert!(parse_command(vec!["check".into()]).is_err());
}

#[test]
pub(crate) fn release_lane_artifacts_use_release_schema() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let artifact =
        release_lane_artifact(&root, "linux-webgpu-chromium").expect("known lane is accepted");

    assert_eq!(artifact["schema"], "scena.release_lane.v1");
    assert_eq!(artifact["lane"], "linux-webgpu-chromium");
    assert_eq!(artifact["backend"], "WebGpu");
    assert!(
        artifact["generated_at_unix_seconds"].as_u64().is_some(),
        "release lane artifacts must be timestamped"
    );
    assert!(
        artifact["commit"]
            .as_str()
            .is_some_and(|value| !value.is_empty()),
        "release lane artifacts must record the source revision"
    );
    let command_records = artifact["command_records"]
        .as_array()
        .expect("command records are present");
    assert!(
        !command_records.is_empty(),
        "release lane artifacts must include structured command records"
    );
    let first = &command_records[0];
    assert!(first["command"].as_str().is_some());
    assert!(first["duration_ms"].is_null());
    assert_eq!(first["duration_source"], "ci-step-summary-or-wrapper");
    assert!(
        first["failure_log_path"]
            .as_str()
            .is_some_and(|path| path.ends_with("linux-webgpu-chromium.log")),
        "command record must reserve a lane-specific failure log path"
    );
    assert!(
        first["artifact_checksums"].is_array(),
        "command record must carry checksum references for produced artifacts"
    );
    assert!(release_lane_artifact(&root, "unknown").is_err());
}

#[test]
pub(crate) fn claim_categories_map_to_evidence_links() {
    let categories = claim_categories("Scene Renderer glTF WebGPU benchmark doctor");

    assert!(categories.contains(&"public-api"));
    assert!(categories.contains(&"assets-gltf"));
    assert!(categories.contains(&"browser-platform"));
    assert!(categories.contains(&"performance"));
    assert!(categories.contains(&"doctor"));
    assert!(
        evidence_links_for_category("assets-gltf")
            .iter()
            .any(|link| link.ends_with("m8_assets_materials_ecosystem.rs"))
    );
}

#[test]
pub(crate) fn parses_visual_proof_modes() {
    assert_eq!(
        parse_command(vec![
            "visual-proof".to_string(),
            "--all-release-lanes".to_string()
        ]),
        Ok(Command::VisualProof(VisualProofCommand::AllReleaseLanes))
    );
    assert_eq!(
        parse_command(vec![
            "visual-proof".to_string(),
            "waterbottle-gpu".to_string(),
            "--".to_string(),
            "cargo".to_string(),
            "test".to_string(),
            "pbr_contract".to_string(),
        ]),
        Ok(Command::VisualProof(VisualProofCommand::Run {
            lane: "waterbottle-gpu".to_string(),
            command: vec![
                "cargo".to_string(),
                "test".to_string(),
                "pbr_contract".to_string()
            ],
        }))
    );
}

#[test]
pub(crate) fn visual_proof_rejects_cargo_test_without_rust_summary() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/visual-proof-summary");
    let artifact_path = fixture_root.join("visual-proof/waterbottle-gpu.json");
    fs::create_dir_all(artifact_path.parent().expect("visual proof parent"))
        .expect("visual proof fixture dir");
    fs::write(
        &artifact_path,
        r#"{
          "schema": "scena.visual_proof.v1",
          "lane": "waterbottle-gpu",
          "status": "passed",
          "rust_test_command": true,
          "rust_test_output_observed": false,
          "skip_marker_observed": false
        }"#,
    )
    .expect("visual proof fixture");
    let mut findings = Vec::new();

    require_visual_proof_artifact_file(
        &artifact_path,
        "visual-proof/waterbottle-gpu.json",
        &mut findings,
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "VISUAL-PROOF" && finding.message.contains("without Rust test summary")
        }),
        "visual proof must fail closed when cargo test terminates without Rust summary: {findings:?}"
    );
}

#[test]
pub(crate) fn visual_proof_rejects_preview_only_artifact() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/visual-proof-preview");
    let artifact_path = fixture_root.join("visual-proof/waterbottle-gpu.json");
    fs::create_dir_all(artifact_path.parent().expect("visual proof parent"))
        .expect("visual proof fixture dir");
    fs::write(
        &artifact_path,
        r#"{
          "schema": "scena.visual_proof.v1",
          "lane": "waterbottle-gpu",
          "status": "passed",
          "preview_only": true,
          "rust_test_command": false,
          "rust_test_output_observed": true,
          "skip_marker_observed": false
        }"#,
    )
    .expect("visual proof fixture");
    let mut findings = Vec::new();

    require_visual_proof_artifact_file(
        &artifact_path,
        "visual-proof/waterbottle-gpu.json",
        &mut findings,
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "VISUAL-PROOF" && finding.message.contains("preview-only")
        }),
        "visual proof must reject preview-only release evidence: {findings:?}"
    );
}

#[test]
pub(crate) fn release_readiness_rejects_commit_mismatched_json_artifact() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/stale-commit");
    fs::create_dir_all(&fixture_root).expect("stale commit fixture dir");
    let artifact_path = fixture_root.join("rendered-output.json");
    fs::write(
        &artifact_path,
        r#"{
          "schema": "scena.rendered_output.v1",
          "status": "passed",
          "timestamp_unix_seconds": 9999999999,
          "commit_sha": "old-commit"
        }"#,
    )
    .expect("stale commit artifact");
    let mut findings = Vec::new();

    reject_stale_json_commit(
        &artifact_path,
        "m9-platform/headless-cpu/rendered-output.json",
        "expected-commit",
        &mut findings,
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RELEASE-READY-ARTIFACTS"
                && finding.message.contains("old-commit")
                && finding.message.contains("expected-commit")
        }),
        "release readiness must reject stale-commit JSON artifacts: {findings:?}"
    );
}

#[test]
pub(crate) fn visual_proof_pass_summary_ignores_empty_auxiliary_cargo_targets() {
    let output = r#"
running 2 tests
test render::prepare::pbr_contract::tests::light_units_do_not_apply_scene_tuned_divisors_or_clamps ... ok
test render::prepare::pbr_contract::tests::pbr_material_uses_gltf_dielectric_and_metallic_f0 ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 91 filtered out; finished in 0.00s

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
"#;

    assert!(visual_proof_rust_test_nonzero_pass_summary_observed(output));
}

#[test]
pub(crate) fn visual_proof_pass_summary_rejects_all_empty_cargo_targets() {
    let output = r#"
running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 93 filtered out; finished in 0.00s
"#;

    assert!(!visual_proof_rust_test_nonzero_pass_summary_observed(
        output
    ));
}

#[test]
pub(crate) fn extracts_markdown_links() {
    let links = markdown_link_targets("A [doc](docs/a.md) and [external](https://x.test).");
    assert_eq!(links, vec!["docs/a.md", "https://x.test"]);
}

#[test]
pub(crate) fn extracts_declared_type_names() {
    assert_eq!(
        declared_type_name("pub struct Renderer;"),
        Some("Renderer".into())
    );
    assert_eq!(
        declared_type_name("pub(crate) struct StubManager;"),
        Some("StubManager".into())
    );
    assert_eq!(
        declared_type_name("pub(super) struct StubFactory;"),
        Some("StubFactory".into())
    );
    assert_eq!(
        declared_type_name("pub(in crate::render) struct StubEngine;"),
        Some("StubEngine".into())
    );
    assert_eq!(declared_type_name("enum Backend {"), Some("Backend".into()));
    assert_eq!(declared_type_name("// pub struct Ignored;"), None);
}

#[test]
pub(crate) fn catches_broad_type_names() {
    assert!(is_catch_all_type_name("SceneManager"));
    assert!(is_catch_all_type_name("World"));
    assert!(is_catch_all_type_name("AppContext"));
    assert!(!is_catch_all_type_name("InteractionContext"));
    assert!(!is_catch_all_type_name("Renderer"));
}

#[test]
pub(crate) fn doctor_findings_include_contract_reference() {
    let finding = Finding::new("ARCH-RENDER-TRUTH", "shader bypassed camera projection");

    assert!(
        finding
            .message
            .contains("docs/checklists/state-of-art-threejs-replacement-plan.md"),
        "doctor findings must point maintainers at the governing checklist or spec"
    );
}

#[test]
pub(crate) fn unit_test_first_governance_is_declared() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_unit_test_first_governance(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn browser_backend_vocabulary_is_explicit() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_backend_vocabulary(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn module_boundaries_guard_render_phase_side_effects() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_module_boundaries(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn source_files_include_renderer_submodules() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let files = source_files(&root);

    assert!(
        files
            .iter()
            .any(|path| path == Path::new("src/render/gpu.rs"))
    );
}

#[test]
pub(crate) fn source_scope_terms_match_whole_tokens() {
    assert!(contains_scope_term("a robot module", "robot"));
    assert!(!contains_scope_term("a roboticist module", "robot"));
}

#[test]
pub(crate) fn asset_api_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_asset_api_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn render_alpha_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_render_alpha_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn output_stage_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_output_stage_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn fxaa_output_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_fxaa_output_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn diagnostics_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_diagnostics_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn renderer_stats_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_renderer_stats_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}
