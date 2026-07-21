use crate::app::prelude::*;

#[test]
fn c04_release_readiness_requires_nonempty_explicit_artifact_root() {
    let root = repo_root().expect("test runs inside the scena workspace");

    for (cli_root, env_root) in [(None, None), (Some(""), None), (None, Some("  "))] {
        let error = resolve_release_artifact_root(&root, cli_root, env_root)
            .expect_err("missing or empty artifact-root configuration must fail closed");
        assert_eq!(error.rule, "RELEASE-READY-ARTIFACT-ROOT");
        assert!(
            error.message.contains("--artifact-root")
                && error.message.contains("SCENA_RELEASE_ARTIFACT_ROOT"),
            "configuration error must name both supported inputs: {error:?}"
        );
    }

    let from_cli = resolve_release_artifact_root(&root, Some("target/gate-artifacts"), None)
        .expect("CLI artifact root resolves");
    assert_eq!(from_cli.source, "cli");
    assert_eq!(from_cli.path, root.join("target/gate-artifacts"));

    let from_env = resolve_release_artifact_root(&root, None, Some("target/gate-artifacts"))
        .expect("environment artifact root resolves");
    assert_eq!(from_env.source, "environment");
    assert_eq!(from_env.path, root.join("target/gate-artifacts"));
}

#[test]
fn c04_release_readiness_reports_zero_validated_evidence_for_missing_or_incomplete_root() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-release-readiness-test/c04-root-states");
    let missing_root = fixture_root.join("missing");
    let incomplete_root = fixture_root.join("incomplete");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(&incomplete_root).expect("incomplete root creates");

    for artifact_root in [&missing_root, &incomplete_root] {
        let mut findings = Vec::new();
        let summary = check_release_artifact_bundle_with_summary(artifact_root, &mut findings);
        assert_eq!(summary.validated_artifact_count, 0);
        assert_eq!(
            summary.required_artifact_count,
            REQUIRED_RELEASE_ARTIFACT_SUFFIXES.len()
        );
        assert!(!findings.is_empty(), "empty evidence must never pass");
        let report = release_readiness_report(Some(artifact_root), None, summary, &findings);
        assert_eq!(report["schema"], "scena.release_readiness.v1");
        assert_eq!(report["ok"], false);
        assert_eq!(report["validated_artifact_count"], 0);
        assert_eq!(report["artifact_root"], artifact_root.display().to_string());
    }
}

#[cfg(unix)]
#[test]
fn c04_release_readiness_rejects_unreadable_artifact_root() {
    use std::os::unix::fs::PermissionsExt;

    let root = repo_root().expect("test runs inside the scena workspace");
    let artifact_root = root.join("target/xtask-release-readiness-test/c04-unreadable");
    let _ = fs::remove_dir_all(&artifact_root);
    fs::create_dir_all(&artifact_root).expect("unreadable root creates");
    fs::set_permissions(&artifact_root, fs::Permissions::from_mode(0o000))
        .expect("unreadable permissions apply");

    let mut findings = Vec::new();
    let summary = check_release_artifact_bundle_with_summary(&artifact_root, &mut findings);
    fs::set_permissions(&artifact_root, fs::Permissions::from_mode(0o700))
        .expect("fixture permissions restore");

    assert_eq!(summary.validated_artifact_count, 0);
    assert!(
        findings.iter().any(|finding| finding
            .message
            .contains("could not collect release artifacts")),
        "unreadable root must fail closed: {findings:?}"
    );
}

#[test]
fn c04_every_specialized_release_artifact_is_required_for_existence() {
    for suffix in REQUIRED_JSON_TIMESTAMP_ARTIFACT_SUFFIXES
        .iter()
        .chain(REQUIRED_JSON_COMMIT_ARTIFACT_SUFFIXES)
        .chain(REQUIRED_NATIVE_GPU_RENDER_ARTIFACT_SUFFIXES)
        .chain(REQUIRED_RENDERED_OUTPUT_METADATA_ARTIFACT_SUFFIXES)
        .chain(REQUIRED_VISUAL_PROOF_ARTIFACT_SUFFIXES)
    {
        assert!(
            REQUIRED_RELEASE_ARTIFACT_SUFFIXES.contains(suffix),
            "specialized validation artifact must also be existence-required: {suffix}"
        );
    }
    assert!(
        REQUIRED_RELEASE_ARTIFACT_SUFFIXES
            .contains(&"m9-platform/linux-native-vulkan/rendered-output.json")
    );
}

#[test]
fn q04_release_consumer_rejects_known_leak_and_missing_adapter() {
    let valid = required_gpu_resource_lifecycle_fixture();
    assert!(required_gpu_resource_lifecycle_proof_passes(&valid));

    let mut leaked = valid.clone();
    leaked["poll_pending_after"] = serde_json::json!(1);
    assert!(!required_gpu_resource_lifecycle_proof_passes(&leaked));

    let mut missing_adapter = valid;
    missing_adapter["adapter"] = serde_json::Value::Null;
    assert!(!required_gpu_resource_lifecycle_proof_passes(
        &missing_adapter
    ));
}

pub(crate) fn required_gpu_resource_lifecycle_fixture() -> Value {
    serde_json::json!({
        "schema": "scena.q04.required_gpu_resource_lifecycle.v1",
        "status": "passed",
        "proof_class": "physical-hardware-required",
        "adapter": {
            "name": "Mutation Test GPU",
            "device_type": "DiscreteGpu",
            "driver": "test-driver",
            "driver_info": "hardware"
        },
        "baseline": {
            "buffers": 10, "gpu_textures": 20, "render_targets": 4,
            "pipelines": 9, "bind_groups": 6, "shader_modules": 8,
            "pending_destructions": 0
        },
        "prepared": {
            "buffers": 12, "gpu_textures": 27, "render_targets": 11,
            "pipelines": 21, "bind_groups": 12, "shader_modules": 19,
            "pending_destructions": 0
        },
        "released": {
            "buffers": 10, "gpu_textures": 20, "render_targets": 4,
            "pipelines": 9, "bind_groups": 6, "shader_modules": 8,
            "pending_destructions": 72
        },
        "poll_status": "Confirmed",
        "poll_pending_before": 72,
        "poll_destroyed_resources": 72,
        "poll_pending_after": 0,
        "assertions_executed": 12,
        "complete_lifecycle": true
    })
}

#[test]
fn c04_doctor_rejects_fail_open_readiness_drift() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/c04-release-readiness");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        ".github/workflows/ci.yml",
        ".github/workflows/release.yml",
        "CHANGELOG.md",
        "crates/xtask/src/app/core.rs",
        "crates/xtask/src/app/release/readiness.rs",
        "crates/xtask/src/app/release/review_artifacts.rs",
        "crates/xtask/src/app/tests_01.rs",
        "crates/xtask/src/app/tests_08.rs",
        "crates/xtask/src/app/tests_11.rs",
        "crates/xtask/src/app/tests_19.rs",
        "crates/xtask/src/app/tests_20.rs",
        "crates/xtask/src/app/tests_41.rs",
        "docs/specs/release-gates.md",
        "docs/troubleshooting.md",
        "scripts/local_release_readiness.sh",
        "scripts/release_publish_dry_run.sh",
        "src/schema_catalog.rs",
        "src/schema_catalog/entries.rs",
        "tests/assets/cli-golden/schema_list_stdout.json",
        "tests/assets/stable-contracts/schema_catalog.v1.json",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("C04 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("C04 source fixture copies");
    }

    let mut findings = Vec::new();
    check_c04_release_readiness_contract(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let inventory = fixture_root.join("crates/xtask/src/app/release/review_artifacts.rs");
    let source = fs::read_to_string(&inventory).expect("C04 artifact inventory reads");
    let mutated = source.replacen(
        "    \"m9-platform/linux-native-vulkan/rendered-output.json\",\n",
        "",
        1,
    );
    assert_ne!(
        source, mutated,
        "C04 mutation must remove the existence row"
    );
    fs::write(inventory, mutated).expect("C04 artifact inventory mutation writes");
    findings.clear();
    check_c04_release_readiness_contract(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "C04-FAIL-CLOSED-RELEASE-READINESS"
                && finding
                    .message
                    .contains("linux-native-vulkan/rendered-output.json")
        }),
        "removing Linux rendered-output existence must fail doctor: {findings:?}"
    );
}
