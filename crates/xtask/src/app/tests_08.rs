use crate::app::prelude::*;

#[test]
pub(crate) fn visual_fixture_metadata_is_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_visual_fixture_metadata(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn m2_visual_fixture_metadata_is_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_m2_visual_fixture_metadata(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn m1_browser_rendered_output_is_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_m1_browser_rendered_output(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn m2_browser_rendered_output_is_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_m2_browser_rendered_output(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn m6_browser_renderer_probe_is_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_m6_browser_renderer_probe(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn m9_release_metadata_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_m9_ci_release_lanes(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn xtask_module_split_is_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_xtask_module_split(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn doctor_rejects_xtask_module_split_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/xtask-split-regression");
    let main_path = fixture_root.join("crates/xtask/src/main.rs");
    let app_path = fixture_root.join("crates/xtask/src/app.rs");
    let part_path = fixture_root.join("crates/xtask/src/app/doctor_core/part_01.rs");
    fs::create_dir_all(app_path.parent().expect("app parent")).expect("app dir");
    fs::create_dir_all(part_path.parent().expect("part parent")).expect("part dir");
    fs::write(&main_path, "mod app;\n\nfn main() {\n    app::run();\n}\n").expect("main fixture");
    fs::write(
        &app_path,
        "mod core;\nmod visual_proof;\nmod architecture_map;\nmod release;\nmod visual_artifacts;\nmod doctor_core;\nmod doctor_docs;\nmod doctor_architecture;\nmod doctor_render;\nmod doctor_scene_platform;\nmod doctor_visual_release;\nmod doctor_m7_m8_assets;\n#[cfg(test)]\nmod tests_01;\n",
    )
    .expect("app fixture");
    fs::write(
        &part_path,
        "use crate::app::doctor_render::*;\nmod part_02;\npub(crate) use part_02::*;\n",
    )
    .expect("part fixture");
    let mut findings = Vec::new();

    check_xtask_module_split(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-XTASK-NAMING" && finding.message.contains("part_01.rs")
        }),
        "doctor must reject numeric part_NN xtask filenames: {findings:?}",
    );
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-XTASK-NO-CROSS-GLOB" && finding.message.contains("part_01.rs:1")
        }),
        "doctor must reject xtask cross-module glob imports: {findings:?}",
    );
}

#[test]
pub(crate) fn state_of_art_checklist_links_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_state_of_art_checklist_links(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn m9_release_artifact_uploads_fail_closed() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let workflow = fs::read_to_string(root.join(".github/workflows/ci.yml"))
        .expect("CI workflow must be readable");

    assert!(
        !workflow.contains("if-no-files-found: ignore"),
        "release artifact uploads must fail when required evidence is missing"
    );
    for artifact_name in [
        "linux-native-vulkan-gate-artifacts",
        "linux-browser-webgl2-gate-artifacts",
        "linux-browser-webgpu-gate-artifacts",
        "macos-metal-gate-artifacts",
        "windows-dx12-gate-artifacts",
    ] {
        assert!(
            workflow.contains(artifact_name),
            "missing release artifact upload {artifact_name}"
        );
    }
}

#[test]
pub(crate) fn release_readiness_has_no_open_release_deferrals() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_release_readiness(&root, &mut findings);

    assert!(
        findings.is_empty(),
        "release readiness must not report open release deferrals after final closure: {findings:?}"
    );
}

#[test]
pub(crate) fn release_readiness_reports_missing_downloaded_artifacts() {
    let mut findings = Vec::new();

    check_release_artifact_bundle(
        Path::new("target/xtask-release-readiness-test/missing"),
        &mut findings,
    );

    assert!(
        findings
            .iter()
            .any(|finding| finding.message.contains("missing release artifact root")),
        "downloaded release artifact root must be required when configured"
    );
}

#[test]
pub(crate) fn release_readiness_rejects_unavailable_browser_artifact() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let artifact_root = root.join("target/xtask-release-readiness-test/unavailable-browser");
    let artifact_dir = artifact_root.join("browser");
    fs::create_dir_all(&artifact_dir).expect("test artifact dir");
    fs::write(
        artifact_dir.join("m6-rust-wasm-renderer-probe.json"),
        r#"{"status":"unavailable"}"#,
    )
    .expect("test artifact write");
    let mut findings = Vec::new();

    check_release_artifact_bundle(&artifact_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.message.contains("does not have status 'passed'")),
        "release readiness must reject unavailable browser proof artifacts"
    );
}

#[test]
pub(crate) fn release_readiness_rejects_command_recorded_release_lane_artifact() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let artifact_root = root.join("target/xtask-release-readiness-test/command-recorded");
    let lane_dir = artifact_root.join("release-lanes");
    fs::create_dir_all(&lane_dir).expect("lane artifact dir");
    fs::write(
        lane_dir.join("linux-native-vulkan.json"),
        r#"{"schema":"scena.release_lane.v1","status":"command-recorded"}"#,
    )
    .expect("lane artifact write");
    let mut findings = Vec::new();

    check_release_artifact_bundle(&artifact_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| { finding.message.contains("only records a command") }),
        "release readiness must reject command-recorded lane artifacts: {findings:?}",
    );
}

#[test]
pub(crate) fn release_readiness_rejects_release_lane_artifact_without_measured_command_duration() {
    let artifact = json!({
        "schema": "scena.release_lane.v1",
        "status": "passed",
        "command_records": [
            {
                "command": "cargo test --test m9_platform_release",
                "status": "artifact-evidence-present",
                "duration_ms": null,
                "failure_log_path": "target/gate-artifacts/release-lanes/linux-native-vulkan.log",
                "artifact_checksums": []
            }
        ]
    });
    let mut findings = Vec::new();

    require_release_lane_artifact_evidence(
        &artifact,
        "release-lanes/linux-native-vulkan.json",
        &mut findings,
    );

    assert!(
        findings
            .iter()
            .any(|finding| finding.message.contains("measured command duration")),
        "release readiness must reject lane artifacts without measured durations: {findings:?}",
    );
}

#[test]
pub(crate) fn release_readiness_rejects_release_lane_artifact_with_failed_command_record() {
    let artifact = json!({
        "schema": "scena.release_lane.v1",
        "status": "passed",
        "command_records": [
            {
                "command": "cargo test --test m9_platform_release",
                "status": "failed",
                "duration_ms": 42,
                "failure_log_path": "target/gate-artifacts/release-lanes/linux-native-vulkan.log",
                "failure_log_sha256": "fnv1a64:0000000000000001",
                "artifact_checksums": []
            }
        ]
    });
    let mut findings = Vec::new();

    require_release_lane_artifact_evidence(
        &artifact,
        "release-lanes/linux-native-vulkan.json",
        &mut findings,
    );

    assert!(
        findings
            .iter()
            .any(|finding| finding.message.contains("did not pass")),
        "release readiness must reject failed lane command records: {findings:?}",
    );
}

#[test]
pub(crate) fn release_readiness_rejects_stale_timestamped_artifact() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let artifact_root = root.join("target/xtask-release-readiness-test/stale-artifact");
    let platform_dir = artifact_root.join("m9-platform");
    fs::create_dir_all(&platform_dir).expect("platform artifact dir");
    fs::write(
        platform_dir.join("m9-capability-matrix.json"),
        r#"{
            "schema": "scena.m9.capability_matrix.v1",
            "status": "passed",
            "timestamp_unix_seconds": 1,
            "lanes": []
        }"#,
    )
    .expect("capability matrix write");
    let mut findings = Vec::new();

    check_release_artifact_bundle(&artifact_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| { finding.message.contains("is stale") }),
        "release readiness must reject stale timestamped artifacts: {findings:?}",
    );
}

#[test]
pub(crate) fn release_readiness_rejects_constant_ppm_visual_artifact() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let artifact_root = root.join("target/xtask-release-readiness-test/constant-ppm");
    let lane_dir = artifact_root.join("m9-platform/linux-native-vulkan");
    fs::create_dir_all(&lane_dir).expect("lane artifact dir");
    fs::write(
        lane_dir.join("default-scene.ppm"),
        b"P6\n2 2\n255\n\x20\x40\x60\x20\x40\x60\x20\x40\x60\x20\x40\x60",
    )
    .expect("constant ppm write");
    let mut findings = Vec::new();

    check_release_artifact_bundle(&artifact_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| { finding.message.contains("constant-color") }),
        "release readiness must reject constant-color visual artifacts: {findings:?}",
    );
}

#[test]
pub(crate) fn release_readiness_rejects_factory_contract_capability_rows() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let artifact_root = root.join("target/xtask-release-readiness-test/factory-capability");
    let platform_dir = artifact_root.join("m9-platform");
    fs::create_dir_all(&platform_dir).expect("platform artifact dir");
    fs::write(
        platform_dir.join("m9-capability-matrix.json"),
        format!(
            r#"{{
                "schema": "scena.m9.capability_matrix.v1",
                "status": "passed",
                "timestamp_unix_seconds": {},
                "lanes": [
                    {{
                        "lane": "macos-metal",
                        "measurement_source": "factory-contract"
                    }}
                ]
            }}"#,
            current_unix_seconds()
        ),
    )
    .expect("capability matrix write");
    let mut findings = Vec::new();

    check_release_artifact_bundle(&artifact_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| { finding.message.contains("factory-contract rows") }),
        "release readiness must reject factory-contract capability rows: {findings:?}",
    );
}

#[test]
pub(crate) fn release_readiness_rejects_missing_lane_capability_rows() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let artifact_root = root.join("target/xtask-release-readiness-test/missing-capability");
    let platform_dir = artifact_root.join("m9-platform");
    fs::create_dir_all(&platform_dir).expect("platform artifact dir");
    fs::write(
        platform_dir.join("m9-capability-matrix.json"),
        format!(
            r#"{{
                "schema": "scena.m9.capability_matrix.v1",
                "status": "passed",
                "timestamp_unix_seconds": {},
                "lanes": [
                    {{
                        "lane": "macos-metal",
                        "measurement_source": "missing-lane-artifact"
                    }}
                ]
            }}"#,
            current_unix_seconds()
        ),
    )
    .expect("capability matrix write");
    let mut findings = Vec::new();

    check_release_artifact_bundle(&artifact_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| { finding.message.contains("missing-lane-artifact rows") }),
        "release readiness must reject missing-lane capability rows: {findings:?}",
    );
}

#[test]
pub(crate) fn release_readiness_rejects_benchmark_artifact_without_stored_baseline_comparison() {
    let artifact = json!({
        "schema": "scena.m9.benchmarks.v1",
        "lane": "linux-native-vulkan",
        "rows": [
            {
                "scene": "static-viewer",
                "backend": "Headless",
                "sample_count": 100,
                "p95_frame_ms": 12.0
            }
        ]
    });
    let mut findings = Vec::new();

    require_benchmark_baseline_comparison(
        &artifact,
        "m9-platform/m9-benchmarks.json",
        &mut findings,
    );

    assert!(
        findings
            .iter()
            .any(|finding| finding.message.contains("stored baseline comparison")),
        "release readiness must reject benchmark artifacts without stored-baseline comparison: {findings:?}",
    );
}

#[test]
pub(crate) fn release_readiness_rejects_benchmark_regression_against_stored_baseline() {
    let artifact = json!({
        "schema": "scena.m9.benchmarks.v1",
        "lane": "linux-native-vulkan",
        "baseline_comparison": {
            "status": "failed",
            "baseline_path": "docs/benchmarks/m9-baselines.json",
            "baseline_sha256": "fnv1a64:0000000000000001",
            "metric": "p95_frame_ms"
        },
        "rows": [
            {
                "scene": "static-viewer",
                "backend": "Headless",
                "sample_count": 100,
                "p95_frame_ms": 12.0,
                "baseline_comparison": {
                    "status": "failed",
                    "baseline_p95_frame_ms": 10.0,
                    "allowed_regression_percent": 5.0,
                    "regression_percent": 20.0
                }
            }
        ]
    });
    let mut findings = Vec::new();

    require_benchmark_baseline_comparison(
        &artifact,
        "m9-platform/m9-benchmarks.json",
        &mut findings,
    );

    assert!(
        findings
            .iter()
            .any(|finding| finding.message.contains("benchmark regression")),
        "release readiness must reject statistically significant p95 benchmark regressions: {findings:?}",
    );
}
