use crate::app::prelude::*;

#[test]
pub(crate) fn doctor_rejects_ndc_smoke_fixture_missing_proof_class_regression() {
    // VISUAL-HARNESS-SMOKE-P0: NDC/fullscreen smoke fixtures must declare
    // proof_class = "harness-smoke" + production_claim = false so they
    // cannot be promoted into renderer/PBR production-proof rows.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/ndc-smoke-missing-proof-class");
    let manifest_rel = "tests/visual/ndc-smoke.toml";
    let manifest_path = fixture_root.join(manifest_rel);
    fs::create_dir_all(manifest_path.parent().expect("manifest parent")).expect("fixture dir");
    fs::write(
        &manifest_path,
        "[[fixture]]\nname = \"ndc_smoke_demo\"\n# missing proof_class and production_claim lines\n",
    )
    .expect("ndc fixture");
    let mut findings = Vec::new();

    check_ndc_smoke_fixture_classification(
        &fixture_root,
        &mut findings,
        manifest_rel,
        &["ndc_smoke_demo"],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "VISUAL-HARNESS-SMOKE-P0"
                && finding.message.contains("proof_class = \"harness-smoke\"")
        }),
        "doctor must reject NDC/fullscreen smoke fixtures missing the \
         harness-smoke proof_class declaration: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_m3a_scene_import_missing_module_export_regression() {
    // ARCH-M3A-SCENE-IMPORT: src/assets.rs must keep the canonical glTF
    // module export wiring; a stripped replacement must fail closed.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/m3a-scene-import-missing-export");
    let assets_path = fixture_root.join("src/assets.rs");
    fs::create_dir_all(assets_path.parent().expect("src dir")).expect("fixture dir");
    fs::write(
        &assets_path,
        "// Stub assets.rs missing the canonical glTF export wiring.\n\
         pub fn placeholder() {}\n",
    )
    .expect("assets fixture");
    let mut findings = Vec::new();

    check_m3a_scene_import_contracts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-M3A-SCENE-IMPORT" && finding.message.contains("mod gltf;")
        }),
        "doctor must reject src/assets.rs that drops the mod gltf; export so \
         scene import wiring cannot silently regress: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_unchecked_production_gltf_parse_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root =
        root.join("target/xtask-doctor-regressions/m3a-scene-import-unchecked-parse");
    let gltf_path = fixture_root.join("src/assets/gltf.rs");
    fs::create_dir_all(gltf_path.parent().expect("glTF source parent")).expect("fixture dir");
    fs::write(
        &gltf_path,
        "fn open(bytes: &[u8]) { let _ = Gltf::from_slice_without_validation(bytes); }\n",
    )
    .expect("glTF source fixture");
    let mut findings = Vec::new();

    check_m3a_scene_import_contracts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-M3A-SCENE-IMPORT"
                && finding.message.contains("from_slice_without_validation")
                && finding.message.contains("forbidden")
        }),
        "doctor must reject unchecked production glTF parsing: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_release_publish_dry_run_helper_missing_strict_mode_regression() {
    // RELEASE-PUBLISH-DRY-RUN-RECORD: scripts/release_publish_dry_run.sh
    // must declare `set -euo pipefail` so a failed git rev-parse /
    // git worktree / tee is not silently ignored before any run_step
    // executes. A helper that drops the strict-mode declaration
    // fails closed.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root =
        root.join("target/xtask-doctor-regressions/release-publish-dry-run-missing-strict");
    let helper_path = fixture_root.join("scripts/release_publish_dry_run.sh");
    fs::create_dir_all(helper_path.parent().expect("scripts dir")).expect("scripts dir");
    fs::write(
        &helper_path,
        "#!/usr/bin/env bash\n\
         set -u\n\
         # missing pipefail; failures in tee or pipeline ahead of run_step\n\
         # would be silently ignored.\n\
         cargo publish --dry-run\n\
         # publish-dry-run.log path mentioned to satisfy other substrings.\n\
         git worktree add --detach /tmp/x\n\
         git worktree remove --force /tmp/x\n",
    )
    .expect("helper fixture");
    let mut findings = Vec::new();

    check_release_publish_dry_run_helper(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RELEASE-PUBLISH-DRY-RUN-RECORD"
                && finding.message.contains("set -euo pipefail")
        }),
        "doctor must reject release_publish_dry_run.sh that drops the \
         strict-mode declaration (set -euo pipefail): {findings:?}",
    );
    for needle in ["AGENTS.md", ".codex/skills", "cmp -s", "diff -qr"] {
        assert!(
            findings.iter().any(|finding| {
                finding.rule == "RELEASE-PUBLISH-DRY-RUN-RECORD" && finding.message.contains(needle)
            }),
            "doctor must reject a publish worktree helper missing {needle}: {findings:?}",
        );
    }
}

#[test]
pub(crate) fn doctor_rejects_missing_packaged_agent_install_gate() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root =
        root.join("target/xtask-doctor-regressions/missing-packaged-agent-install-gate");
    let helper_path = fixture_root.join("scripts/release_publish_dry_run.sh");
    fs::create_dir_all(helper_path.parent().expect("scripts dir")).expect("scripts dir");
    fs::write(
        &helper_path,
        "#!/usr/bin/env bash\n\
         set -euo pipefail\n\
         cargo publish --dry-run\n\
         publish-dry-run.log\n\
         git worktree add --detach /tmp/x\n\
         git worktree remove --force /tmp/x\n\
         AGENTS.md\n\
         .codex/skills\n\
         cmp -s\n\
         diff -qr\n",
    )
    .expect("helper fixture");
    let mut findings = Vec::new();

    check_release_publish_dry_run_helper(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "PACKAGED-AGENT-INSTALL-GATE"
                && finding.message.contains("verify_packaged_agent_install.sh")
        }),
        "doctor must reject release machinery that does not verify the extracted packaged agent install: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_release_readiness_ci_continue_on_error_regression() {
    // RELEASE-READINESS-CI-FAIL-CLOSED: no GHA workflow job that runs
    // release-readiness may set continue-on-error: true. Pre-merge CI may
    // convert ADR-0005 blockers into an explicit report step, but it must
    // not leave a permanently red "allowed failure" job in normal push CI.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root =
        root.join("target/xtask-doctor-regressions/release-readiness-ci-continue-on-error");
    let adr_path =
        fixture_root.join("docs/decisions/ADR-0005-local-release-candidate-deferrals.md");
    let workflow_path = fixture_root.join(".github/workflows/ci.yml");
    fs::create_dir_all(adr_path.parent().expect("adr dir")).expect("adr dir create");
    fs::create_dir_all(workflow_path.parent().expect("workflow dir")).expect("workflow dir create");
    fs::write(&adr_path, "# ADR-0005\n\nStatus: Superseded by ADR-0006.\n").expect("adr fixture");
    fs::write(
        &workflow_path,
        "name: CI\njobs:\n  premerge-release-readiness:\n    \
         runs-on: ubuntu-24.04\n    continue-on-error: true\n    steps:\n      - \
         name: drift\n        run: cargo run -p xtask -- release-readiness\n",
    )
    .expect("workflow fixture");
    let mut findings = Vec::new();

    check_release_readiness_ci_fail_closed(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RELEASE-READINESS-CI-FAIL-CLOSED"
                && finding.message.contains("premerge-release-readiness")
                && finding.message.contains("continue-on-error: true")
        }),
        "doctor must reject continue-on-error: true on release-readiness jobs: {findings:?}",
    );
}

#[test]
pub(crate) fn release_readiness_ci_fail_closed_rejects_continue_on_error_while_adr0005_is_accepted()
{
    // RELEASE-READINESS-CI-FAIL-CLOSED: ADR-0005 may keep pre-merge
    // release-readiness informational, but it may not use continue-on-error
    // because that still leaves a red job in the GitHub Actions UI.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root =
        root.join("target/xtask-doctor-regressions/release-readiness-ci-fail-closed-accepted");
    let adr_path =
        fixture_root.join("docs/decisions/ADR-0005-local-release-candidate-deferrals.md");
    let workflow_path = fixture_root.join(".github/workflows/ci.yml");
    fs::create_dir_all(adr_path.parent().expect("adr dir")).expect("adr dir create");
    fs::create_dir_all(workflow_path.parent().expect("workflow dir")).expect("workflow dir create");
    fs::write(&adr_path, "# ADR-0005\n\nStatus: Accepted.\n").expect("adr fixture");
    fs::write(
        &workflow_path,
        "name: CI\njobs:\n  premerge-release-readiness:\n    \
         runs-on: ubuntu-24.04\n    continue-on-error: true\n    steps:\n      - \
         name: drift\n        run: cargo run -p xtask -- release-readiness\n",
    )
    .expect("workflow fixture");
    let mut findings = Vec::new();

    check_release_readiness_ci_fail_closed(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RELEASE-READINESS-CI-FAIL-CLOSED"
                && finding.message.contains("premerge-release-readiness")
                && finding.message.contains("continue-on-error: true")
        }),
        "doctor must reject continue-on-error: true even while ADR-0005 is Accepted: \
         {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_m5_release_missing_cargo_metadata_regression() {
    // ARCH-M5-RELEASE: Cargo.toml must keep the release-publish metadata
    // (version, rust-version, documentation, keywords, categories,
    // include, crate-type) so the crate cannot ship without the
    // release-readiness contract substrings.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root =
        root.join("target/xtask-doctor-regressions/m5-release-missing-cargo-metadata");
    let cargo_path = fixture_root.join("Cargo.toml");
    fs::create_dir_all(&fixture_root).expect("fixture dir");
    fs::write(
        &cargo_path,
        "[package]\nname = \"scena\"\n# stub Cargo.toml without rust-version, \
         documentation, keywords, categories, include, or crate-type fields.\n",
    )
    .expect("cargo fixture");
    let mut findings = Vec::new();

    check_m5_release_contracts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-M5-RELEASE" && finding.message.contains("rust-version")
        }),
        "doctor must reject Cargo.toml that drops release-publish metadata \
         substrings: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_m6_browser_renderer_probe_missing_cargo_dep_regression() {
    // VISUAL-BROWSER-M6: Cargo.toml must keep the browser-probe feature
    // gate, but the M6 renderer proof must no longer depend on raw Rust
    // WebGL2 program/shader bindings. A Cargo.toml that drops the feature
    // still fails closed; raw render-path WebGl bindings are covered by the
    // source-enforced absence rule.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root =
        root.join("target/xtask-doctor-regressions/m6-browser-probe-missing-cargo-dep");
    let cargo_path = fixture_root.join("Cargo.toml");
    fs::create_dir_all(&fixture_root).expect("fixture dir");
    fs::write(
        &cargo_path,
        "[package]\nname = \"scena\"\n# stub Cargo.toml missing the browser renderer feature gate.\n",
    )
    .expect("cargo fixture");
    let mut findings = Vec::new();

    check_m6_browser_renderer_probe(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "VISUAL-BROWSER-M6"
                && finding.message.contains(
                    "Cargo.toml is missing required contract text \
                         'browser-probe = [\"viewer-element\"]'",
                )
        }),
        "doctor must reject Cargo.toml that drops the browser-probe / \
         wgpu-backed M6 browser renderer probe gate: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_browser_release_probe_without_provenance_contract() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root =
        root.join("target/xtask-doctor-regressions/m6-browser-probe-missing-provenance");
    let probe = fixture_root.join("tests/browser/m6_rust_wasm_renderer_probe.js");
    fs::create_dir_all(probe.parent().expect("probe fixture parent")).expect("fixture dir");
    fs::write(&probe, "// browser probe without release provenance\n").expect("probe fixture");
    let mut findings = Vec::new();

    check_m6_browser_renderer_probe(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "VISUAL-BROWSER-M6"
                && finding.message.contains("attachReleaseArtifactProvenance")
        }),
        "doctor must reject a browser release producer that can emit unattributed JSON: \
         {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_wasm_size_gate_without_provenance_contract() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/m9-wasm-size-missing-provenance");
    let gate = fixture_root.join("tests/release/m9_wasm_size_gate.js");
    fs::create_dir_all(gate.parent().expect("gate fixture parent")).expect("fixture dir");
    fs::write(&gate, "// wasm size gate without release provenance\n").expect("gate fixture");
    let mut findings = Vec::new();

    check_m9_ci_release_lanes(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RELEASE-CI-M9"
                && finding.message.contains("attachReleaseArtifactProvenance")
        }),
        "doctor must reject a WASM-size release producer that can emit unattributed JSON: \
         {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_release_workflow_that_requires_external_review_bundle() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root =
        root.join("target/xtask-doctor-regressions/release-workflow-requires-review-bundle");
    let workflow = fixture_root.join(".github/workflows/release.yml");
    fs::create_dir_all(workflow.parent().expect("workflow fixture parent")).expect("fixture dir");
    fs::write(
        &workflow,
        "name: release\nenv:\n  SCENA_REVIEW_BUNDLE_URL: required\n",
    )
    .expect("workflow fixture");
    let mut findings = Vec::new();

    check_m9_ci_release_lanes(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RELEASE-CI-M9"
                && finding.message.contains("forbidden boundary text")
                && finding.message.contains("SCENA_REVIEW_BUNDLE_URL")
        }),
        "doctor must reject a publish workflow that requires an external review bundle: \
         {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_m1_browser_rendered_output_missing_cargo_dep_regression() {
    // VISUAL-BROWSER-M1: Cargo.toml must keep the wasm-bindgen + ImageData
    // + CanvasRenderingContext2d dependencies that gate the m1 browser
    // rendered-output path. A Cargo.toml that drops any of these fails
    // closed so the M1 browser proof cannot regress silently.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/m1-browser-missing-cargo-dep");
    let cargo_path = fixture_root.join("Cargo.toml");
    fs::create_dir_all(&fixture_root).expect("fixture dir");
    fs::write(
        &cargo_path,
        "[package]\nname = \"scena\"\n# stub Cargo.toml without wasm-bindgen / \
         CanvasRenderingContext2d / ImageData entries.\n",
    )
    .expect("cargo fixture");
    let mut findings = Vec::new();

    check_m1_browser_rendered_output(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "VISUAL-BROWSER-M1" && finding.message.contains("wasm-bindgen")
        }),
        "doctor must reject Cargo.toml that drops the wasm-bindgen / \
         CanvasRenderingContext2d / ImageData wiring required by the M1 \
         browser rendered-output proof: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_manifest_file_hash_mismatch_regression() {
    // VISUAL-DEFAULT-ENV (manifest hash): a manifest entry whose
    // recorded SHA-256 does not match the file on disk fails closed so
    // the asset bundle cannot drift away from the manifest contract.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/manifest-file-hash-mismatch");
    let manifest_rel = "tests/assets/manifest.toml";
    let asset_rel = "tests/assets/some-file.bin";
    let manifest_path = fixture_root.join(manifest_rel);
    let asset_path = fixture_root.join(asset_rel);
    fs::create_dir_all(manifest_path.parent().expect("manifest parent")).expect("manifest dir");
    fs::create_dir_all(asset_path.parent().expect("asset parent")).expect("asset dir");
    fs::write(&manifest_path, "# stub manifest\n").expect("manifest fixture");
    fs::write(&asset_path, b"actual file bytes").expect("asset fixture");
    // 64-hex-character SHA-256 that does not match the file on disk.
    let bogus_sha256 = "0".repeat(64);
    let mut findings = Vec::new();

    check_manifest_file_hash(
        &fixture_root,
        &mut findings,
        manifest_rel,
        asset_rel,
        &bogus_sha256,
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "VISUAL-DEFAULT-ENV" && finding.message.contains("SHA-256 mismatch for")
        }),
        "doctor must reject manifest entries whose recorded SHA-256 does \
         not match the file on disk: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_m10_claim_audit_missing_contract_text_regression() {
    // CLAIM-AUDIT-M10: docs/checklists/m10-threejs-replacement-acceptance.md
    // and docs/api/m10-public-api-diff.md must keep the M10 claim-audit
    // wiring substrings; a stripped replacement must fail the gate.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/m10-claim-audit-missing-text");
    let xtask_path = fixture_root.join("crates/xtask/src/main.rs");
    let m10_checklist_path =
        fixture_root.join("docs/checklists/m10-threejs-replacement-acceptance.md");
    fs::create_dir_all(xtask_path.parent().expect("xtask parent")).expect("xtask dir");
    fs::create_dir_all(m10_checklist_path.parent().expect("m10 parent")).expect("m10 dir");
    fs::write(
        &xtask_path,
        "fn main() { /* stub xtask without claim-audit wiring */ }\n",
    )
    .expect("xtask fixture");
    fs::write(
        &m10_checklist_path,
        "# m10 acceptance stub without claim audit reference\n",
    )
    .expect("m10 fixture");
    let mut findings = Vec::new();

    check_m10_claim_audit_contract(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "CLAIM-AUDIT-M10" && finding.message.contains("claim-audit")
        }),
        "doctor must reject xtask main.rs that drops the M10 claim-audit \
         wiring substrings so the release-acceptance gate cannot ship \
         silently: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_required_doc_missing_substring_regression() {
    // DOCS-PUBLIC-API: docs/api.md must keep the canonical
    // public-API contract substrings; a stripped-down replacement must
    // fail the required-doc-contract gate.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/required-doc-missing-substring");
    let public_api_path = fixture_root.join("docs/api.md");
    fs::create_dir_all(public_api_path.parent().expect("docs")).expect("docs dir");
    fs::write(
        &public_api_path,
        "# Public API\n\nThis stub does not declare the prepare lifecycle, the \
         RendererStats counter, or any material descriptor contract.\n",
    )
    .expect("public-api stub fixture");
    let mut findings = Vec::new();

    check_required_doc_contracts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "DOCS-PUBLIC-API"
                && finding
                    .message
                    .contains("docs/api.md is missing required contract text")
        }),
        "doctor must reject docs/api.md when it drops a required \
         public-API contract substring: {findings:?}",
    );
}

#[test]
pub(crate) fn environment_lifecycle_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_environment_lifecycle_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn equirectangular_hdr_environment_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_equirectangular_hdr_environment_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn environment_ibl_prepare_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_environment_ibl_prepare_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn scene_light_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_scene_light_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn direct_light_shading_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_direct_light_shading_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn directional_shadow_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_directional_shadow_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn shadow_map_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_shadow_map_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn depth_prepass_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_depth_prepass_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn reversed_z_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_reversed_z_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn webgl2_depth_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_webgl2_depth_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn m2_leak_stats_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_m2_leak_stats_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn camera_depth_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_camera_depth_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn origin_shift_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_origin_shift_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn clipping_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_clipping_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());

    let fixture_root = root.join("target/xtask-doctor-regressions/clipping-owner-delegation");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/scene.rs",
        "src/scene/clipping.rs",
        "src/diagnostics.rs",
        "src/lib.rs",
        "src/render.rs",
        "src/render/prepare_lifecycle.rs",
        "src/render/cpu.rs",
        "src/render/cpu_geometry.rs",
        "tests/m2_lighting_depth_clipping.rs",
        "docs/specs/public-api.md",
        "docs/checklists/m2-lighting-depth-clipping.md",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(
            destination
                .parent()
                .expect("clipping fixture path has parent"),
        )
        .expect("clipping fixture directory creates");
        fs::copy(root.join(relative), destination).expect("clipping fixture source copies");
    }

    let cpu = fixture_root.join("src/render/cpu.rs");
    let source = fs::read_to_string(&cpu).expect("CPU clipping source reads");
    let mutated = source.replace(
        "cpu_geometry::point_is_clipped(position, clipping_planes, section_box)",
        "false",
    );
    assert_ne!(
        source, mutated,
        "CPU clipping delegation mutation alters source"
    );
    fs::write(&cpu, mutated).expect("CPU clipping delegation mutation writes");
    findings.clear();
    check_clipping_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-CLIPPING"
                && finding.message.contains("cpu_geometry::point_is_clipped")
        }),
        "removing shared CPU clipping delegation must fail doctor: {findings:?}"
    );

    fs::write(&cpu, source).expect("CPU clipping source restores");
    let owner = fixture_root.join("src/render/cpu_geometry.rs");
    let source = fs::read_to_string(&owner).expect("shared clipping owner reads");
    let mutated = source.replace(
        "signed_distance < -CLIPPING_BOUNDARY_TOLERANCE * scale.max(1.0)",
        "signed_distance < 0.0",
    );
    assert_ne!(
        source, mutated,
        "shared clipping tolerance mutation alters source"
    );
    fs::write(owner, mutated).expect("shared clipping tolerance mutation writes");
    findings.clear();
    check_clipping_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-CLIPPING"
                && finding.message.contains("CLIPPING_BOUNDARY_TOLERANCE")
        }),
        "removing the shared clipping tolerance must fail doctor: {findings:?}"
    );
}

#[test]
pub(crate) fn m3a_scene_import_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_m3a_scene_import_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn default_environment_manifest_is_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_default_environment_manifest(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}
