use crate::app::prelude::*;

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
