use crate::app::prelude::*;

#[test]
fn doctor_rejects_parallel_m9_platform_benchmark_gate() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/m9-parallel-benchmark");
    let _ = fs::remove_dir_all(&fixture_root);
    for directory in [".github/workflows", "tests"] {
        fs::create_dir_all(fixture_root.join(directory)).expect("M9 benchmark fixture directory");
    }
    for workflow in ["ci.yml", "release.yml"] {
        fs::write(
            fixture_root.join(".github/workflows").join(workflow),
            "run: cargo test --test m9_platform_release\n",
        )
        .expect("parallel M9 workflow fixture writes");
    }
    fs::write(
        fixture_root.join("tests/m9_platform_release.rs"),
        r#"
#[test]
fn m9_platform_rendered_output_suite_writes_release_artifacts() {
    write_benchmark_artifact(current_lane());
}
"#,
    )
    .expect("parallel M9 test fixture writes");
    let mut findings = Vec::new();

    check_m9_ci_release_lanes(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RELEASE-CI-M9" && finding.message.contains("--test-threads=1")
        }),
        "doctor must reject performance baselines measured inside the broad parallel M9 test target: {findings:?}",
    );
}

#[test]
fn doctor_rejects_cross_fixture_asset_byte_ordering_as_external_fetch_proof() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/asset-fetch-byte-ordering");
    let test_path = fixture_root.join("tests/m8_assets_materials_ecosystem.rs");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(test_path.parent().expect("asset test fixture parent"))
        .expect("asset test fixture directory");
    fs::write(
        &test_path,
        r#"
fn m8_native_fetcher_cache_dedup_reload_retain_and_external_buffers_are_explicit() {
    let _direct_evidence = "AssetLoadProgress::ExternalBufferFetched";
    let _buffer = "tests/assets/gltf/khronos/TextureTransformTest/TextureTransformTest.bin";
    assert!(external.fetched_bytes() > first.fetched_bytes());
}
"#,
    )
    .expect("asset fetch byte-order fixture");
    let mut findings = Vec::new();

    check_asset_load_test_evidence(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ASSETS-M8"
                && finding
                    .message
                    .contains("external.fetched_bytes() > first.fetched_bytes()")
        }),
        "doctor must reject cross-fixture byte ordering as external-fetch evidence: {findings:?}",
    );
}

#[test]
fn doctor_rejects_webgl2_angle_forced_unroll_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/webgl2-angle-forced-unroll");
    let ltc_path = fixture_root.join("src/render/area_ltc.wgsl");
    let output_path = fixture_root.join("src/render/gpu/output_shader_texture_2d.wgsl");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(ltc_path.parent().expect("LTC shader parent")).expect("fixture dir");
    fs::create_dir_all(output_path.parent().expect("WebGL2 shader parent")).expect("fixture dir");
    fs::write(
        &ltc_path,
        "for (var i = 0u; i < vertex_count; i = i + 1u) {}\n",
    )
    .expect("LTC shader fixture");
    fs::write(
        &output_path,
        "for (var i = 0u; i < MAX_GPU_AREA_LIGHTS; i = i + 1u) {}\n",
    )
    .expect("WebGL2 output shader fixture");
    let mut findings = Vec::new();

    check_renderer_truth_contracts(&fixture_root, &mut findings);

    for forbidden_loop in ["vertex_count", "MAX_GPU_AREA_LIGHTS"] {
        assert!(
            findings.iter().any(|finding| {
                finding.rule == "ARCH-RENDER-TRUTH" && finding.message.contains(forbidden_loop)
            }),
            "doctor must reject the WebGL2 loop that ANGLE's D3D11 backend forces and fails to \
             unroll (HLSL X3511): {forbidden_loop}; findings={findings:?}",
        );
    }
}

#[test]
fn doctor_rejects_post_uniform_overwrite_and_weak_pf01_oracle() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/post-uniform-ordering");
    let post_path = fixture_root.join("src/render/gpu/post/mod.rs");
    let resources_path = fixture_root.join("src/render/gpu/post/resources.rs");
    let harness_path = fixture_root.join("tests/browser/pf01_output_toggle.js");
    let validator_path = fixture_root.join("tests/browser/pf01_output_toggle_validation.js");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(post_path.parent().expect("post module parent")).expect("fixture dir");
    fs::create_dir_all(harness_path.parent().expect("browser harness parent"))
        .expect("fixture dir");
    fs::write(
        &post_path,
        "fn write_uniform(queue: &Queue, resources: &PostResources) {\n\
         queue.write_buffer(&resources.uniform, 0, &[]);\n\
         }\n",
    )
    .expect("post module fixture");
    fs::write(
        &resources_path,
        "struct PostResources { uniform: Buffer }\n",
    )
    .expect("post resource fixture");
    fs::write(
        &harness_path,
        "if (off.fnv1a64 === on.fnv1a64) throw new Error('no delta');\n",
    )
    .expect("PF01 harness fixture");
    fs::write(
        &validator_path,
        "function validateOutputToggleResult(result) { return result; }\n",
    )
    .expect("PF01 validator fixture");
    let mut findings = Vec::new();

    check_c09_gpu_resource_lifecycle_contracts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RENDER-C09"
                && finding.message.contains("command-ordered staging copies")
        }),
        "doctor must reject direct queue writes to a shared post uniform because later pass \
         parameters overwrite earlier passes; findings={findings:?}",
    );
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RENDER-C09" && finding.message.contains("combined-effect collapse")
        }),
        "doctor must reject a PF01 oracle that accepts bloom+FXAA collapsing to either \
         single-effect output; findings={findings:?}",
    );
}

#[test]
fn doctor_rejects_incomplete_windows_complete_hardware_proof_lane() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/windows-complete-hardware-proof");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "examples/native_surface_hardware_proof.rs",
        "tests/fr06_semantic_aov.rs",
        "tests/browser/fr06_semantic_aov.js",
        "tests/release/windows_complete_hardware_proof_validation.js",
        "scripts/run_windows_complete_hardware_proof.ps1",
        "scripts/build_windows_complete_hardware_bundle.sh",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("hardware proof fixture parent"))
            .expect("hardware proof fixture directory");
        fs::copy(root.join(relative), destination).expect("hardware proof fixture source copies");
    }

    let mutations = [
        (
            "examples/native_surface_hardware_proof.rs",
            "native combined output collapsed to FXAA-only",
            "removed native combined-effect oracle",
        ),
        (
            "tests/fr06_semantic_aov.rs",
            "scena.fr06.native_semantic_aov_proof.v1",
            "removed.native.fr06.schema",
        ),
        (
            "tests/browser/fr06_semantic_aov.js",
            "unexpected HTTP failures",
            "ignored HTTP failures",
        ),
        (
            "tests/release/windows_complete_hardware_proof_validation.js",
            "native present-only ${counter} must be zero",
            "native present-only counter was not checked",
        ),
        (
            "scripts/run_windows_complete_hardware_proof.ps1",
            "browser:fr06-semantic-aov",
            "removed-browser-fr06-proof",
        ),
        (
            "scripts/run_windows_complete_hardware_proof.ps1",
            "browser:q01-parity",
            "removed-browser-q01-proof",
        ),
        (
            "scripts/run_windows_complete_hardware_proof.ps1",
            "scena-q04-gpu-resource-lifecycle.exe",
            "removed-q04-lifecycle-proof",
        ),
        (
            "scripts/run_windows_complete_hardware_proof.ps1",
            "scena-p01-shader-module-cache.exe",
            "removed-p01-benchmark-proof",
        ),
        (
            "scripts/build_windows_complete_hardware_bundle.sh",
            "Windows release-evidence bundles require a clean committed checkout",
            "allowed-dirty-bundle",
        ),
    ];
    for (relative, needle, replacement) in mutations {
        let path = fixture_root.join(relative);
        let source = fs::read_to_string(&path).expect("hardware proof fixture reads");
        let mutated = source.replace(needle, replacement);
        assert_ne!(
            source, mutated,
            "hardware proof mutation must alter {relative}"
        );
        fs::write(path, mutated).expect("hardware proof mutation writes");
    }

    let mut findings = Vec::new();
    check_c09_gpu_resource_lifecycle_contracts(&fixture_root, &mut findings);
    check_fr06_semantic_aov_contracts(&fixture_root, &mut findings);

    for (rule, relative, needle) in [
        (
            "RENDER-C09",
            "examples/native_surface_hardware_proof.rs",
            "native combined output collapsed to FXAA-only",
        ),
        (
            "FR06-SEMANTIC-AOV",
            "tests/fr06_semantic_aov.rs",
            "scena.fr06.native_semantic_aov_proof.v1",
        ),
        (
            "FR06-SEMANTIC-AOV",
            "tests/browser/fr06_semantic_aov.js",
            "unexpected HTTP failures",
        ),
        (
            "RENDER-C09",
            "tests/release/windows_complete_hardware_proof_validation.js",
            "native present-only ${counter} must be zero",
        ),
        (
            "RENDER-C09",
            "scripts/run_windows_complete_hardware_proof.ps1",
            "browser:fr06-semantic-aov",
        ),
        (
            "RENDER-C09",
            "scripts/run_windows_complete_hardware_proof.ps1",
            "browser:q01-parity",
        ),
        (
            "RENDER-C09",
            "scripts/run_windows_complete_hardware_proof.ps1",
            "scena-q04-gpu-resource-lifecycle.exe",
        ),
        (
            "RENDER-C09",
            "scripts/run_windows_complete_hardware_proof.ps1",
            "scena-p01-shader-module-cache.exe",
        ),
        (
            "RENDER-C09",
            "scripts/build_windows_complete_hardware_bundle.sh",
            "Windows release-evidence bundles require a clean committed checkout",
        ),
    ] {
        assert!(
            findings.iter().any(|finding| {
                finding.rule == rule
                    && finding.message.contains(relative)
                    && finding.message.contains(needle)
            }),
            "doctor must reject loss of {needle} from {relative}; findings={findings:?}",
        );
    }
}

#[test]
fn q04_macos_lanes_produce_the_required_physical_lifecycle_artifact() {
    let root = repo_root().expect("test runs inside the scena workspace");
    for relative in [".github/workflows/ci.yml", ".github/workflows/release.yml"] {
        let source = fs::read_to_string(root.join(relative)).expect("workflow source reads");
        assert!(
            source.contains("Required physical GPU resource lifecycle")
                && source.contains("SCENA_REQUIRE_GPU_RESOURCE_LIFECYCLE=1")
                && source
                    .contains("required_hardware_gpu_resource_lifecycle_executes_complete_cycle",),
            "{relative} macOS Metal lane must produce the physical Q04 artifact consumed by release staging"
        );
    }
}

#[test]
fn doctor_rejects_missing_investigation_circuit_breakers_and_stale_builder_paths() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/agent-circuit-breakers");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "AGENTS.md",
        ".codex/skills/scena-renderer-quality/SKILL.md",
        ".codex/skills/scena-doctor/SKILL.md",
        ".codex/skills/scena-release-hygiene/SKILL.md",
        ".codex/skills/scena-remote-builder/SKILL.md",
        ".codex/skills/scena-git-github/SKILL.md",
        "scripts/scena_remote_builder_preflight.sh",
        "scripts/collect_ci_failure_evidence.sh",
        "scripts/run_windows_complete_hardware_proof.ps1",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("guardrail fixture parent"))
            .expect("guardrail fixture directory");
        fs::copy(root.join(relative), destination).expect("guardrail fixture source copies");
    }
    for (relative, needle, replacement) in [
        (
            "AGENTS.md",
            "## Investigation Circuit Breakers",
            "## Investigation Notes",
        ),
        (
            "AGENTS.md",
            "A second user-assisted run requires",
            "Another user run may be requested",
        ),
    ] {
        let path = fixture_root.join(relative);
        let source = fs::read_to_string(&path).expect("guardrail fixture reads");
        let mutated = source.replace(needle, replacement);
        assert_ne!(source, mutated, "guardrail mutation must alter {relative}");
        fs::write(path, mutated).expect("guardrail fixture mutation writes");
    }
    let stale_skill = fixture_root.join(".codex/skills/scena-renderer-quality/SKILL.md");
    let mut stale_text = fs::read_to_string(&stale_skill).expect("quality skill fixture reads");
    stale_text.push_str("\nssh scena-builder 'cd \"$HOME/projects/scena\" && cargo test'\n");
    fs::write(stale_skill, stale_text).expect("stale path mutation writes");
    let mut findings = Vec::new();

    check_remote_builder_bootstrap_contracts(&fixture_root, &mut findings);

    for expected in [
        "investigation circuit breaker",
        "second user-assisted run",
        "obsolete shared checkout path",
    ] {
        assert!(
            findings.iter().any(|finding| {
                finding.rule == "O01-REMOTE-BUILDER-BOOTSTRAP" && finding.message.contains(expected)
            }),
            "doctor must reject agent guidance without {expected}; findings={findings:?}",
        );
    }
}

#[test]
fn doctor_rejects_strict_wall_clock_gates_on_github_hosted_runners() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/hosted-timing-policy");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(fixture_root.join(".github/workflows"))
        .expect("hosted timing workflow fixture directory");
    for workflow in ["ci.yml", "release.yml"] {
        fs::write(
            fixture_root.join(".github/workflows").join(workflow),
            r#"jobs:
  windows-dx12:
    runs-on: windows-2025
    steps:
      - run: SCENA_RUN_M9_PLATFORM_BENCHMARK=1 bash scripts/release_lane_command.sh windows-dx12 cargo test --test m9_platform_release m9_platform_benchmark_writes_release_artifact -- --exact --test-threads=1
"#,
        )
        .expect("hosted timing workflow fixture writes");
    }
    let mut findings = Vec::new();

    check_m9_ci_release_lanes(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RELEASE-CI-M9"
                && finding
                    .message
                    .contains("SCENA_M9_TIMING_POLICY=report-only-hosted")
        }),
        "doctor must reject strict wall-clock benchmark gates on shared GitHub-hosted runners: {findings:?}",
    );
}

#[test]
fn doctor_rejects_parallel_heavyweight_agent_template_cli_tests() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/agent-template-cli-isolation");
    let test_path = fixture_root.join("tests/scena_cli_agent_templates.rs");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(test_path.parent().expect("CLI isolation fixture parent"))
        .expect("CLI isolation fixture directory");
    fs::copy(root.join("tests/scena_cli_agent_templates.rs"), &test_path)
        .expect("CLI isolation fixture source copies");
    let source = fs::read_to_string(&test_path).expect("CLI isolation fixture reads");
    let mutated = source
        .replace("static TEMPLATE_CLI_LOCK", "static REMOVED_CLI_LOCK")
        .replace(
            "let _cli_guard = template_cli_guard();",
            "let _removed_guard = ();",
        );
    assert_ne!(source, mutated, "CLI isolation mutation must alter fixture");
    fs::write(&test_path, mutated).expect("CLI isolation mutation writes");
    let mut findings = Vec::new();

    check_m9_ci_release_lanes(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RELEASE-CI-M9" && finding.message.contains("TEMPLATE_CLI_LOCK")
        }),
        "doctor must reject concurrent heavyweight CLI subprocess tests that can exhaust a hosted runner: {findings:?}",
    );
}
