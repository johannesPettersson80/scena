use crate::app::prelude::*;

const CPU_RENDER_COMMAND: &str = "cargo test --test examples_visual_proof q02_live_cpu_round_e_showcase_emits_shared_evaluator_frame -- --exact";
const CPU_EVALUATOR_COMMAND: &str = "node scripts/evaluate_round_e_cpu_materials.cjs";
const WEBGL_EVALUATOR_COMMAND: &str =
    "npm run cloudflare:materials -- http://127.0.0.1:18104/proof/?sample=material-presets";

#[test]
pub(crate) fn q02_release_lanes_require_live_material_artifacts_and_exact_commands() {
    let headless = release_lane_required_artifacts("headless-cpu");
    for expected in [
        "target/gate-artifacts/round-e-cpu-material-proof/live-frame.png",
        "target/gate-artifacts/round-e-cpu-material-proof/live-cpu-frame.json",
        "target/gate-artifacts/round-e-cpu-material-proof.json",
    ] {
        assert!(
            headless.iter().any(|artifact| artifact == expected),
            "Q02 headless release lane must require {expected}; got {headless:?}"
        );
    }
    let headless_commands = release_lane_expected_commands("headless-cpu");
    for expected in [CPU_RENDER_COMMAND, CPU_EVALUATOR_COMMAND] {
        assert!(
            headless_commands.contains(&expected),
            "Q02 headless release lane must require {expected}; got {headless_commands:?}"
        );
    }

    let webgpu = release_lane_required_artifacts("linux-webgpu-chromium");
    for expected in [
        "target/gate-artifacts/round-e-webgpu-material-proof/live-frame.png",
        "target/gate-artifacts/round-e-webgpu-material-proof/result.json",
    ] {
        assert!(
            webgpu.iter().any(|artifact| artifact == expected),
            "Q02 WebGPU release lane must require {expected}; got {webgpu:?}"
        );
    }
    assert!(
        release_lane_expected_commands("linux-webgpu-chromium")
            .contains(&"npm run browser:q02-materials"),
        "Q02 WebGPU release lane must require its dedicated material-only producer"
    );

    let webgl = release_lane_required_artifacts("linux-webgl2-chromium");
    for expected in [
        "target/gate-artifacts/round-e-cloudflare-material-proof.json",
        "target/gate-artifacts/round-e-cloudflare-material-proof/canvas.png",
        "target/gate-artifacts/round-e-cloudflare-material-proof/chrome.png",
        "target/gate-artifacts/round-e-cloudflare-material-proof/brushed_steel.png",
        "target/gate-artifacts/round-e-cloudflare-material-proof/clearcoat_plastic.png",
        "target/gate-artifacts/round-e-cloudflare-material-proof/clear_glass.png",
        "target/gate-artifacts/round-e-cloudflare-material-proof/frosted_glass.png",
        "target/gate-artifacts/round-e-cloudflare-material-proof/leather.png",
        "target/gate-artifacts/round-e-cloudflare-material-proof/rubber.png",
    ] {
        assert!(
            webgl.iter().any(|artifact| artifact == expected),
            "Q02 WebGL2 release lane must require {expected}; got {webgl:?}"
        );
    }
    assert!(
        release_lane_expected_commands("linux-webgl2-chromium").contains(&WEBGL_EVALUATOR_COMMAND),
        "Q02 WebGL2 release lane must require the live crop/DeltaE/material evaluator command"
    );

    for expected in [
        "round-e-cpu-material-proof/live-frame.png",
        "round-e-cpu-material-proof/live-cpu-frame.json",
        "round-e-cpu-material-proof.json",
        "round-e-cloudflare-material-proof.json",
        "round-e-cloudflare-material-proof/canvas.png",
        "round-e-webgpu-material-proof/live-frame.png",
        "round-e-webgpu-material-proof/result.json",
    ] {
        assert!(
            REQUIRED_RELEASE_ARTIFACT_SUFFIXES.contains(&expected),
            "Q02 staged release bundle must require {expected}"
        );
    }
    for expected in [
        "round-e-cpu-material-proof.json",
        "round-e-cloudflare-material-proof.json",
        "round-e-webgpu-material-proof/result.json",
    ] {
        assert!(
            REQUIRED_PASSED_STATUS_ARTIFACT_SUFFIXES.contains(&expected),
            "Q02 release readiness must require passed status for {expected}"
        );
        assert!(
            REQUIRED_JSON_TIMESTAMP_ARTIFACT_SUFFIXES.contains(&expected),
            "Q02 release readiness must reject stale timestamps for {expected}"
        );
        assert!(
            REQUIRED_JSON_COMMIT_ARTIFACT_SUFFIXES.contains(&expected),
            "Q02 release readiness must reject stale commits for {expected}"
        );
    }
}

#[test]
pub(crate) fn q02_release_lane_content_rejects_missing_surface_specific_material_result() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/q02-release-lane-content");
    let artifact_root = fixture_root.join("target/gate-artifacts");
    let _ = fs::remove_dir_all(&fixture_root);

    let headless = artifact_root.join("m9-platform/headless-cpu");
    fs::create_dir_all(&headless).expect("Q02 headless fixture dir");
    fs::write(
        headless.join("rendered-output.json"),
        r#"{
          "schema":"scena.m9.platform_render.v1",
          "lane":"headless-cpu",
          "backend":"Headless",
          "headless_cpu_proof":true,
          "static_gltf":{
            "proof_class":"cpu-camera-framed-non-ndc",
            "production_claim":true,
            "nonblack_pixels":42
          }
        }"#,
    )
    .expect("Q02 headless rendered output");
    crate::app::tests_24::write_q01_cpu_proof_fixture(
        &artifact_root,
        "0123456789abcdef0123456789abcdef01234567",
    );
    assert!(
        !release_lane_content_ok(&fixture_root, "headless-cpu")
            .expect("Q02 headless content check runs"),
        "headless release content must fail when the Q02 CPU material result is absent"
    );

    fs::create_dir_all(&artifact_root).expect("Q02 browser fixture dir");
    fs::write(
        artifact_root.join("m6-rust-wasm-renderer-probe.json"),
        r#"{
          "gate":"m6-rust-wasm-renderer-probe",
          "status":"passed",
          "results":[
            {"backend":"webgl2","status":"passed","pixels":{"nonblack":42}},
            {"backend":"webgpu","status":"passed","pixels":{"nonblack":42}}
          ]
        }"#,
    )
    .expect("Q02 browser M6 fixture");
    for lane in ["linux-webgl2-chromium", "linux-webgpu-chromium"] {
        assert!(
            !release_lane_content_ok(&fixture_root, lane).expect("Q02 browser content check runs"),
            "{lane} release content must fail when its Q02 material result is absent"
        );
    }
}

#[test]
pub(crate) fn q02_doctor_rejects_shared_evaluator_mutation_or_workflow_drift() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/q02-doctor");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        ".github/workflows/ci.yml",
        ".github/workflows/release.yml",
        "crates/xtask/src/app/release/lane_artifacts.rs",
        "crates/xtask/src/app/release/review_artifacts.rs",
        "crates/xtask/src/app/release/round_e_material_results.rs",
        "scripts/evaluate_round_e_cpu_materials.cjs",
        "scripts/probe_cloudflare_material_presets.mjs",
        "scripts/round_e_material_evaluator.cjs",
        "scripts/tests/round_e_material_evaluator_test.cjs",
        "tests/browser/m6_rust_wasm_renderer_probe.js",
        "tests/examples_visual_proof.rs",
        "tests/visual/references/round_e_material_thresholds.toml",
    ] {
        let source = root.join(relative);
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("Q02 doctor fixture parent"))
            .expect("Q02 doctor fixture directory");
        fs::copy(&source, &destination)
            .unwrap_or_else(|error| panic!("Q02 doctor fixture {relative} copies: {error}"));
    }

    let mut baseline = Vec::new();
    check_q02_round_e_material_proof(&fixture_root, &mut baseline);
    assert_eq!(baseline, Vec::new(), "complete Q02 sources satisfy doctor");

    let mutations = [
        (
            "scripts/tests/round_e_material_evaluator_test.cjs",
            "\"flat chrome\"",
            "\"flattened chrome silently\"",
            "flat chrome",
        ),
        (
            ".github/workflows/ci.yml",
            "bash scripts/release_lane_command.sh headless-cpu node scripts/evaluate_round_e_cpu_materials.cjs",
            "node scripts/evaluate_round_e_cpu_materials.cjs",
            "evaluate_round_e_cpu_materials",
        ),
        (
            "tests/browser/m6_rust_wasm_renderer_probe.js",
            "evaluateRequiredWebgpuMaterialProof(materialPresetResult)",
            "({ status: \"pass\" })",
            "evaluateRequiredWebgpuMaterialProof",
        ),
    ];
    for (relative, needle, replacement, expected) in mutations {
        let path = fixture_root.join(relative);
        let original = fs::read_to_string(&path).expect("Q02 doctor mutation source reads");
        assert!(
            original.contains(needle),
            "Q02 mutation needle exists: {needle}"
        );
        fs::write(&path, original.replacen(needle, replacement, 1))
            .expect("Q02 doctor mutation writes");
        let mut findings = Vec::new();
        check_q02_round_e_material_proof(&fixture_root, &mut findings);
        assert!(
            findings.iter().any(|finding| {
                finding.rule == "Q02-ROUND-E-MATERIALS" && finding.message.contains(expected)
            }),
            "Q02 doctor must reject {expected} drift: {findings:?}"
        );
        fs::write(&path, original).expect("Q02 doctor mutation restores");
    }
}
