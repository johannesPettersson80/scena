use crate::app::prelude::*;
use crate::app::release::{
    canonical_artifact_tree_digest, require_verified_staging_provenance,
    validate_ci_provenance_manifest,
};

fn context(commit: &str) -> Value {
    json!({
        "repository": "johannesPettersson80/scena",
        "workflow_ref": "johannesPettersson80/scena/.github/workflows/release.yml@refs/tags/v1.9.0",
        "workflow_sha": "89abcdef0123456789abcdef0123456789abcdef",
        "ref": "refs/tags/v1.9.0",
        "run_id": "123456789",
        "run_attempt": 2,
        "job": "publish",
        "source_commit": commit,
    })
}

fn manifest(commit: &str, digest: &str, count: usize, timestamp: u64) -> Value {
    let mut value = context(commit);
    let object = value.as_object_mut().expect("context object");
    object.insert("schema".to_string(), json!("scena.ci_provenance.v1"));
    object.insert("artifact_digest".to_string(), json!(digest));
    object.insert("artifact_file_count".to_string(), json!(count));
    object.insert(
        "issuer".to_string(),
        json!("https://token.actions.githubusercontent.com"),
    );
    object.insert("generated_at_unix_seconds".to_string(), json!(timestamp));
    object.insert("release_evidence".to_string(), json!(false));
    object.insert(
        "release_rejection_codes".to_string(),
        json!(["CI_ATTESTATION_NOT_YET_VERIFIED"]),
    );
    object.insert(
        "attestation".to_string(),
        json!({
            "predicate_type": "https://slsa.dev/provenance/v1",
            "verification_status": "pending",
        }),
    );
    value
}

#[test]
fn q03_ci_provenance_binds_context_and_complete_artifact_tree() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture = root.join("target/xtask-q03-ci-provenance");
    let lane = fixture.join("lane");
    fs::create_dir_all(&lane).expect("fixture dir");
    fs::write(lane.join("result.json"), "{\"status\":\"passed\"}\n").expect("fixture artifact");
    let (digest, count) = canonical_artifact_tree_digest(&fixture).expect("artifact digest");
    let commit = "0123456789abcdef0123456789abcdef01234567";
    let now = 1_800_000_000;
    let expected = context(commit);
    let value = manifest(commit, &digest, count, now);
    validate_ci_provenance_manifest(&fixture, &value, commit, &expected, now)
        .expect("complete signed-manifest payload validates before signature verification");

    for (label, mut mutated) in [
        ("wrong repository", value.clone()),
        ("replayed run", value.clone()),
        ("wrong ref", value.clone()),
        ("missing job", value.clone()),
        ("tampered digest", value.clone()),
    ] {
        match label {
            "wrong repository" => mutated["repository"] = json!("attacker/fork"),
            "replayed run" => mutated["run_id"] = json!("123456788"),
            "wrong ref" => mutated["ref"] = json!("refs/heads/main"),
            "missing job" => mutated["job"] = json!(""),
            "tampered digest" => mutated["artifact_digest"] = json!("0".repeat(64)),
            _ => unreachable!(),
        }
        assert!(
            validate_ci_provenance_manifest(&fixture, &mutated, commit, &expected, now).is_err(),
            "{label} mutation must be rejected"
        );
    }

    fs::write(lane.join("result.json"), "{\"status\":\"tampered\"}\n").expect("tamper fixture");
    assert!(
        validate_ci_provenance_manifest(&fixture, &value, commit, &expected, now).is_err(),
        "post-manifest artifact tampering must be rejected"
    );
}

#[test]
fn q03_release_readiness_requires_verified_ci_provenance() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture = root.join("target/xtask-q03-release-readiness-provenance");
    if fixture.exists() {
        fs::remove_dir_all(&fixture).expect("remove prior Q03 readiness fixture");
    }
    fs::create_dir_all(&fixture).expect("fixture dir");
    let metadata_path = fixture.join("staging-metadata.json");
    let manifest_path = fixture.join("ci-provenance.json");
    let files = vec![metadata_path.clone(), manifest_path.clone()];

    fs::write(
        &metadata_path,
        serde_json::to_vec_pretty(&json!({
            "schema": "scena.release.staging.v1",
            "status": "passed",
            "release_evidence": false,
            "release_rejection_codes": ["CI_PROVENANCE_UNVERIFIED"],
            "ci_provenance": {
                "schema": "scena.ci_provenance.v1",
                "verification_status": "unavailable",
            },
        }))
        .expect("serialize local staging metadata"),
    )
    .expect("write local staging metadata");
    let mut findings = Vec::new();
    require_verified_staging_provenance(&files, &mut findings);
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "RELEASE-CI-PROVENANCE"),
        "local staging must not satisfy release readiness"
    );

    let signed_manifest = json!({
        "schema": "scena.ci_provenance.v1",
        "repository": "johannesPettersson80/scena",
        "workflow_ref": "johannesPettersson80/scena/.github/workflows/release.yml@refs/tags/v1.9.0",
        "workflow_sha": "89abcdef0123456789abcdef0123456789abcdef",
        "ref": "refs/tags/v1.9.0",
        "run_id": "123456789",
        "run_attempt": 2,
        "job": "publish",
        "source_commit": "0123456789abcdef0123456789abcdef01234567",
        "artifact_digest": "a".repeat(64),
        "artifact_file_count": 7,
        "issuer": "https://token.actions.githubusercontent.com",
        "generated_at_unix_seconds": 1_800_000_000_u64,
        "release_evidence": false,
        "release_rejection_codes": ["CI_ATTESTATION_NOT_YET_VERIFIED"],
        "attestation": {
            "predicate_type": "https://slsa.dev/provenance/v1",
            "verification_status": "pending",
        },
    });
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&signed_manifest).expect("serialize signed manifest"),
    )
    .expect("write signed manifest");

    fs::write(
        &metadata_path,
        serde_json::to_vec_pretty(&json!({
            "schema": "scena.release.staging.v1",
            "status": "passed",
            "release_evidence": true,
            "release_rejection_codes": [],
            "ci_provenance": {
                "schema": "scena.ci_provenance.v1",
                "repository": "johannesPettersson80/scena",
                "workflow_ref": "johannesPettersson80/scena/.github/workflows/release.yml@refs/tags/v1.9.0",
                "workflow_sha": "89abcdef0123456789abcdef0123456789abcdef",
                "ref": "refs/tags/v1.9.0",
                "run_id": "123456789",
                "run_attempt": 2,
                "job": "publish",
                "source_commit": "0123456789abcdef0123456789abcdef01234567",
                "artifact_digest": "a".repeat(64),
                "artifact_file_count": 7,
                "issuer": "https://token.actions.githubusercontent.com",
                "generated_at_unix_seconds": 1_800_000_000_u64,
                "attestation": {
                    "predicate_type": "https://slsa.dev/provenance/v1",
                    "verification_status": "verified",
                    "verification_receipt_sha256": "b".repeat(64),
                },
            },
        }))
        .expect("serialize verified staging metadata"),
    )
    .expect("write verified staging metadata");
    findings.clear();
    require_verified_staging_provenance(&files, &mut findings);
    assert!(
        findings.is_empty(),
        "verified staging provenance must satisfy release readiness: {findings:?}"
    );
}

#[test]
fn q03_doctor_rejects_missing_ci_attestation_wiring() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture = root.join("target/xtask-doctor-regressions/q03-ci-provenance");
    if fixture.exists() {
        fs::remove_dir_all(&fixture).expect("remove prior Q03 doctor fixture");
    }
    for relative in [
        ".github/workflows/ci.yml",
        ".github/workflows/release.yml",
        "scripts/ci_provenance.js",
        "crates/xtask/src/app/release/ci_provenance.rs",
        "crates/xtask/src/app/release/bundle_schema.rs",
        "tests/release/ci_provenance_test.js",
        "docs/specs/release-gates.md",
    ] {
        let destination = fixture.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture parent"))
            .expect("create Q03 doctor fixture parent");
        fs::copy(root.join(relative), &destination).expect("copy Q03 doctor contract source");
    }
    let mut findings = Vec::new();
    check_ci_attestation_contracts(&fixture, &mut findings);
    assert!(
        findings.is_empty(),
        "current Q03 CI provenance contract must satisfy doctor: {findings:?}"
    );

    let workflow = fixture.join(".github/workflows/ci.yml");
    let source = fs::read_to_string(&workflow).expect("read CI workflow fixture");
    let mutated = source.replace(
        "actions/attest@f7c74d28b9d84cb8768d0b8ca14a4bac6ef463e6 # v4.2.0",
        "actions/attest@missing-immutable-pin # mutation",
    );
    assert_ne!(
        source, mutated,
        "Q03 mutation must remove attestation action"
    );
    fs::write(workflow, mutated).expect("write Q03 attestation mutation");
    findings.clear();
    check_ci_attestation_contracts(&fixture, &mut findings);
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "RELEASE-CI-PROVENANCE"),
        "doctor must reject missing CI attestation wiring: {findings:?}"
    );
}

#[test]
fn q04_doctor_rejects_smoke_parity_classification_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture = root.join("target/xtask-doctor-regressions/q04-browser-evidence-classification");
    if fixture.exists() {
        fs::remove_dir_all(&fixture).expect("remove prior Q04 doctor fixture");
    }
    for relative in [
        "tests/browser/required_gpu_parity.js",
        "tests/browser/m6_rust_wasm_renderer_probe.js",
        "crates/xtask/src/app/release/required_gpu_parity.rs",
        "crates/xtask/src/app/release/stage_artifacts.rs",
        "tests/browser/browser_evidence_classification_test.js",
        "docs/schema-contracts.md",
    ] {
        let destination = fixture.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture parent"))
            .expect("create Q04 doctor fixture parent");
        fs::copy(root.join(relative), &destination).expect("copy Q04 doctor contract source");
    }
    let mut findings = Vec::new();
    check_q04_browser_evidence_classification(&fixture, &mut findings);
    assert!(
        findings.is_empty(),
        "current Q04 evidence classification must satisfy doctor: {findings:?}"
    );

    let evaluator = fixture.join("tests/browser/required_gpu_parity.js");
    let source = fs::read_to_string(&evaluator).expect("read Q04 evaluator fixture");
    let mutated = source.replace("classifyBrowserEvidence", "classifyBrowserSmokeAsRelease");
    assert_ne!(source, mutated, "Q04 mutation must remove classifier owner");
    fs::write(evaluator, mutated).expect("write Q04 classification mutation");
    findings.clear();
    check_q04_browser_evidence_classification(&fixture, &mut findings);
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "Q04-BROWSER-EVIDENCE-CLASSIFICATION"),
        "doctor must reject smoke/parity classification regression: {findings:?}"
    );
}

#[test]
fn q06_doctor_rejects_missing_full_frame_waterbottle_oracle() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture = root.join("target/xtask-doctor-regressions/q06-waterbottle-full-frame");
    if fixture.exists() {
        fs::remove_dir_all(&fixture).expect("remove prior Q06 doctor fixture");
    }
    let relative = "tests/m8_real_asset_proof.rs";
    let destination = fixture.join(relative);
    fs::create_dir_all(destination.parent().expect("fixture parent"))
        .expect("create Q06 doctor fixture parent");
    fs::copy(root.join(relative), &destination).expect("copy Q06 oracle source");

    let mut findings = Vec::new();
    check_m8_real_asset_dual_lane(&fixture, &mut findings);
    assert!(
        findings.is_empty(),
        "current Q06 full-frame contract must satisfy doctor: {findings:?}"
    );

    let source = fs::read_to_string(&destination).expect("read Q06 source fixture");
    let mutated = source.replace(
        "evaluate_waterbottle_reference_diff",
        "evaluate_sparse_waterbottle_samples",
    );
    assert_ne!(source, mutated, "Q06 mutation must remove full-frame owner");
    fs::write(destination, mutated).expect("write Q06 oracle mutation");
    findings.clear();
    check_m8_real_asset_dual_lane(&fixture, &mut findings);
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "M8-REAL-ASSET-DUAL-LANE"),
        "doctor must reject a missing full-frame WaterBottle oracle: {findings:?}"
    );
}

#[test]
fn q08_doctor_rejects_zero_assertion_physical_parity_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture = root.join("target/xtask-doctor-regressions/q08-physical-parity");
    if fixture.exists() {
        fs::remove_dir_all(&fixture).expect("remove prior Q08 doctor fixture");
    }
    for relative in [
        "tests/support/parity.rs",
        "tests/transmission_parity.rs",
        "tests/c13_depth_clipping_parity.rs",
        "tests/dynamic_transform_parity.rs",
        "tests/pbr_brdf_parity.rs",
        "tests/pf08_texture_bake_parity.rs",
        ".github/workflows/ci.yml",
        ".github/workflows/release.yml",
        "scripts/build_windows_complete_hardware_bundle.sh",
        "scripts/run_windows_complete_hardware_proof.ps1",
        "crates/xtask/src/app/release/q08_parity.rs",
        "tests/release/windows_complete_hardware_proof_validation.js",
    ] {
        let destination = fixture.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture parent"))
            .expect("create Q08 fixture parent");
        fs::copy(root.join(relative), destination).expect("copy Q08 source");
    }
    let mut findings = Vec::new();
    check_q08_required_physical_parity(&fixture, &mut findings);
    assert!(
        findings.is_empty(),
        "current Q08 physical parity contract must satisfy doctor: {findings:?}"
    );

    let consumer = fixture.join("crates/xtask/src/app/release/q08_parity.rs");
    let source = fs::read_to_string(&consumer).expect("read Q08 release consumer");
    let mutated = source.replace("assertions_executed", "reported_assertion_count");
    assert_ne!(
        source, mutated,
        "Q08 mutation must remove assertion binding"
    );
    fs::write(consumer, mutated).expect("write Q08 mutation");
    findings.clear();
    check_q08_required_physical_parity(&fixture, &mut findings);
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "Q08-REQUIRED-PHYSICAL-PARITY"),
        "doctor must reject a Q08 release consumer without assertion binding: {findings:?}"
    );

    let builder = fixture.join("scripts/build_windows_complete_hardware_bundle.sh");
    let source = fs::read_to_string(&builder).expect("read Q08 Windows bundle builder");
    let mutated = source.replace(
        "tests/transmission_parity.rs",
        "tests/omitted_transmission_parity.rs",
    );
    assert_ne!(
        source, mutated,
        "Q08 mutation must remove a runtime producer source"
    );
    fs::write(builder, mutated).expect("write Q08 bundle mutation");
    findings.clear();
    check_q08_required_physical_parity(&fixture, &mut findings);
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "Q08-REQUIRED-PHYSICAL-PARITY"),
        "doctor must reject a Windows bundle missing a Q08 runtime producer source: {findings:?}"
    );

    let portable_output = fixture.join("tests/pbr_brdf_parity.rs");
    let source = fs::read_to_string(&portable_output).expect("read portable Q08 producer");
    let mutated = source.replace(
        "PathBuf::from(\"target/gate-artifacts/pbr-brdf-parity\")",
        "PathBuf::from(env!(\"CARGO_MANIFEST_DIR\"))\n        \
         .join(\"target/gate-artifacts/pbr-brdf-parity\")",
    );
    assert_ne!(
        source, mutated,
        "Q08 mutation must embed the cross-builder source path"
    );
    fs::write(portable_output, mutated).expect("write Q08 portability mutation");
    findings.clear();
    check_q08_required_physical_parity(&fixture, &mut findings);
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "Q08-REQUIRED-PHYSICAL-PARITY"),
        "doctor must reject a cross-compiled proof that writes to the builder path: {findings:?}"
    );
}

#[test]
fn q09_doctor_rejects_display_name_selected_tolerance_policy() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture = root.join("target/xtask-doctor-regressions/q09-adapter-expectations");
    if fixture.exists() {
        fs::remove_dir_all(&fixture).expect("remove prior Q09 doctor fixture");
    }
    for relative in [
        "tests/m8_real_asset_proof.rs",
        "crates/xtask/src/app/release/waterbottle_results.rs",
        "src/render/gpu/build.rs",
        "tests/release/windows_complete_hardware_proof_validation.js",
        "tests/assets/gltf/khronos/WaterBottle/reference_metadata.toml",
    ] {
        let destination = fixture.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture parent"))
            .expect("create Q09 fixture parent");
        fs::copy(root.join(relative), destination).expect("copy Q09 source");
    }
    let mut findings = Vec::new();
    check_q09_structured_adapter_expectations(&fixture, &mut findings);
    assert!(
        findings.is_empty(),
        "current Q09 structured adapter contract must satisfy doctor: {findings:?}"
    );

    let test_source = fixture.join("tests/m8_real_asset_proof.rs");
    let source = fs::read_to_string(&test_source).expect("read Q09 source");
    let mutated = source.replace(
        "GITHUB_MACOS_14_PARAVIRTUAL_METAL_KEY.matches(report)",
        "report.name.contains(\"Apple Paravirtual device\")",
    );
    assert_ne!(
        source, mutated,
        "Q09 mutation must replace structured match"
    );
    fs::write(&test_source, mutated).expect("write Q09 mutation");
    findings.clear();
    check_q09_structured_adapter_expectations(&fixture, &mut findings);
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "Q09-STRUCTURED-ADAPTER-EXPECTATIONS"),
        "doctor must reject display-name-selected WaterBottle policy: {findings:?}"
    );

    fs::write(&test_source, &source).expect("restore Q09 source");
    let mutated = source.replace(
        "portable_waterbottle_regions_leave_full_frame_tolerance_headroom",
        "portable_waterbottle_regions_without_reference_headroom_guard",
    );
    assert_ne!(
        source, mutated,
        "Q09 mutation must remove the portable reference-headroom guard"
    );
    fs::write(&test_source, mutated).expect("write Q09 headroom mutation");
    findings.clear();
    check_q09_structured_adapter_expectations(&fixture, &mut findings);
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "Q09-STRUCTURED-ADAPTER-EXPECTATIONS"),
        "doctor must reject a portable WaterBottle profile without reference-tolerance headroom: \
         {findings:?}"
    );

    fs::write(&test_source, source).expect("restore Q09 source");
    let build_source = fixture.join("src/render/gpu/build.rs");
    let source = fs::read_to_string(&build_source).expect("read native GPU builder");
    let mutated = source.replace("wgpu::Backends::all().with_env()", "wgpu::Backends::all()");
    assert_ne!(
        source, mutated,
        "Q09 mutation must remove WGPU_BACKEND filtering"
    );
    fs::write(&build_source, mutated).expect("write Q09 backend-filter mutation");
    findings.clear();
    check_q09_structured_adapter_expectations(&fixture, &mut findings);
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "Q09-STRUCTURED-ADAPTER-EXPECTATIONS"),
        "doctor must reject native GPU construction that ignores WGPU_BACKEND: {findings:?}"
    );

    fs::write(&build_source, &source).expect("restore native GPU builder");
    let mutated = source.replace(
        "headless_gpu_uses_the_filtered_native_instance_path",
        "headless_gpu_backend_filter_unproven",
    );
    assert_ne!(
        source, mutated,
        "Q09 mutation must remove the headless backend-filter regression proof"
    );
    fs::write(build_source, mutated).expect("write Q09 headless-proof mutation");
    findings.clear();
    check_q09_structured_adapter_expectations(&fixture, &mut findings);
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "Q09-STRUCTURED-ADAPTER-EXPECTATIONS"),
        "doctor must reject missing headless WGPU_BACKEND proof: {findings:?}"
    );
}

#[test]
fn q10_doctor_rejects_post_hoc_wrong_material_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture = root.join("target/xtask-doctor-regressions/q10-rendered-mutations");
    if fixture.exists() {
        fs::remove_dir_all(&fixture).expect("remove prior Q10 doctor fixture");
    }
    for relative in [
        "tests/q01_waterbottle_cpu_reference.rs",
        "crates/xtask/src/app/release/waterbottle_results.rs",
        "crates/xtask/src/app/release/waterbottle_results/reference_stability.rs",
        "tests/assets/gltf/khronos/WaterBottle/reference_metadata.toml",
    ] {
        let destination = fixture.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture parent"))
            .expect("create Q10 fixture parent");
        fs::copy(root.join(relative), destination).expect("copy Q10 source");
    }
    let mut findings = Vec::new();
    check_q10_rendered_waterbottle_mutations(&fixture, &mut findings);
    assert!(
        findings.is_empty(),
        "current Q10 rendered mutations must satisfy doctor: {findings:?}"
    );

    let test_source = fixture.join("tests/q01_waterbottle_cpu_reference.rs");
    let source = fs::read_to_string(&test_source).expect("read Q10 source");
    let mutated = source.replace(
        "let wrong_material_frame = render_wrong_material_scene();",
        "let wrong_material_frame = wrong_material_mutation(&live);",
    );
    assert_ne!(
        source, mutated,
        "Q10 mutation must remove rendered material"
    );
    fs::write(test_source, mutated).expect("write Q10 mutation");
    findings.clear();
    check_q10_rendered_waterbottle_mutations(&fixture, &mut findings);
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "Q10-RENDERED-WATERBOTTLE-MUTATIONS"),
        "doctor must reject a post-hoc wrong-material mutation: {findings:?}"
    );
}

#[test]
fn q11_doctor_rejects_non_deterministic_reference_evidence() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture = root.join("target/xtask-doctor-regressions/q11-reference-stability");
    if fixture.exists() {
        fs::remove_dir_all(&fixture).expect("remove prior Q11 doctor fixture");
    }
    for relative in [
        "tests/q01_waterbottle_cpu_reference.rs",
        ".github/workflows/ci.yml",
        ".github/workflows/release.yml",
        "crates/xtask/src/app/release/review_artifacts.rs",
        "crates/xtask/src/app/release/waterbottle_results.rs",
        "crates/xtask/src/app/release/waterbottle_results/reference_stability.rs",
        "scripts/stage_q01_waterbottle_reference_candidate.sh",
        "scripts/promote_q01_waterbottle_reference.cjs",
        "tests/release/windows_complete_hardware_proof_validation.js",
        "tests/assets/gltf/khronos/WaterBottle/reference_metadata.toml",
    ] {
        let destination = fixture.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture parent"))
            .expect("create Q11 fixture parent");
        fs::copy(root.join(relative), destination).expect("copy Q11 source");
    }
    let mut findings = Vec::new();
    check_q11_reference_stability(&fixture, &mut findings);
    assert!(
        findings.is_empty(),
        "current Q11 reference-stability contract must satisfy doctor: {findings:?}"
    );

    let validator = fixture.join("tests/release/windows_complete_hardware_proof_validation.js");
    let source = fs::read_to_string(&validator).expect("read Q11 Windows validator");
    let mutated = source.replace("report.byte_identical === true", "true");
    assert_ne!(source, mutated, "Q11 mutation must remove byte comparison");
    fs::write(validator, mutated).expect("write Q11 mutation");
    findings.clear();
    check_q11_reference_stability(&fixture, &mut findings);
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "Q11-REFERENCE-STABILITY"),
        "doctor must reject a Q11 validator without byte determinism: {findings:?}"
    );
}
