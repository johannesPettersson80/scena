use crate::app::prelude::*;

#[test]
fn q01_doctor_rejects_smoke_only_webgpu_parity_contracts() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/q01-webgpu-pixel-parity");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        ".github/workflows/hardware-gpu.yml",
        "package.json",
        "scripts/build_windows_complete_hardware_bundle.sh",
        "scripts/run_windows_complete_hardware_proof.ps1",
        "src/browser_probe.rs",
        "src/browser_probe/parity.rs",
        "src/schema_catalog.rs",
        "src/schema_catalog/fixtures.rs",
        "tests/browser/required_gpu_parity.js",
        "tests/browser/required_gpu_parity_test.js",
        "tests/browser/m6_rust_wasm_renderer_probe.js",
        "tests/m6_browser_webgpu_readback.rs",
        "crates/xtask/src/app/release/required_gpu_parity.rs",
        "crates/xtask/src/app/doctor_docs/stable_fixtures.rs",
        "docs/browser.md",
        "docs/specs/release-gates.md",
        "docs/schema-contracts.md",
        "tests/assets/stable-contracts/required_webgpu_pixel_parity.v1.json",
        "README.md",
        "CHANGELOG.md",
        "docs/release-notes/v1.8.0.md",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("Q01 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("Q01 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_q01_required_webgpu_pixel_parity(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let evaluator = fixture_root.join("tests/browser/required_gpu_parity.js");
    let source = fs::read_to_string(&evaluator).expect("Q01 evaluator reads");
    let mutated = source.replace("\"vertical-flip\"", "\"flipped-image\"");
    assert_ne!(source, mutated, "Q01 mutation must remove vertical flip");
    fs::write(evaluator, mutated).expect("Q01 mutation writes");
    findings.clear();
    check_q01_required_webgpu_pixel_parity(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "Q01-REQUIRED-WEBGPU-PIXEL-PARITY"
                && finding.message.contains("vertical-flip")
        }),
        "removing a known-bad image mutation must fail doctor: {findings:?}",
    );

    let browser_probe = fixture_root.join("tests/browser/m6_rust_wasm_renderer_probe.js");
    let source = fs::read_to_string(&browser_probe).expect("Q01 browser probe reads");
    let mutated = source.replace("collectBrowserGpuEvidence", "ignoreSameBrowserGpuEvidence");
    assert_ne!(
        source, mutated,
        "Q01 mutation must remove same-browser GPU attestation"
    );
    fs::write(browser_probe, mutated).expect("Q01 browser probe mutation writes");
    findings.clear();
    check_q01_required_webgpu_pixel_parity(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "Q01-REQUIRED-WEBGPU-PIXEL-PARITY"
                && finding.message.contains("collectBrowserGpuEvidence")
        }),
        "removing Q01 same-browser GPU attestation must fail doctor: {findings:?}",
    );
}

#[test]
fn q03_doctor_rejects_quadrant_only_structure_evidence() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/q03-m2-local-structure");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "tests/m2_visual_proof.rs",
        "tests/visual/fixtures/m2-headless-core.toml",
        "tests/visual/references/m2-headless-core.toml",
        "tests/visual/references/m2-headless-core-frames.toml",
        "docs/checklists/m2-lighting-depth-clipping.md",
        "README.md",
        "CHANGELOG.md",
        "docs/release-notes/v1.8.0.md",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("Q03 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("Q03 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_q03_m2_local_structure(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let proof = fixture_root.join("tests/m2_visual_proof.rs");
    let source = fs::read_to_string(&proof).expect("Q03 proof reads");
    let mutated = source.replace(
        "sort_each_quadrant_by_luminance",
        "quadrant_means_only_accept_structure",
    );
    assert_ne!(
        source, mutated,
        "Q03 mutation must remove structure sorting"
    );
    fs::write(proof, mutated).expect("Q03 mutation writes");
    findings.clear();
    check_q03_m2_local_structure(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "Q03-M2-LOCAL-STRUCTURE"
                && finding.message.contains("sort_each_quadrant_by_luminance")
        }),
        "removing the mean-preserving collapsed-structure mutation must fail doctor: {findings:?}",
    );
}

#[test]
fn q05_doctor_evaluator_rejects_single_shadow_comparison_tap() {
    let root = repo_root().expect("test runs inside the scena workspace");
    for relative in [
        "src/render/gpu/output_shader.wgsl",
        "src/render/gpu/output_shader_texture_2d.wgsl",
    ] {
        let source = fs::read_to_string(root.join(relative)).expect("Q05 shader reads");
        assert!(directional_shadow_shader_has_pcf3x3(&source));

        let single_tap_mutation = source.replacen(
            "textureSampleCompareLevel(shadow_map, shadow_sampler",
            "removedComparisonTap(",
            8,
        );
        assert_ne!(source, single_tap_mutation);
        assert!(
            !directional_shadow_shader_has_pcf3x3(&single_tap_mutation),
            "Q05 doctor evaluator must reject the reviewed one-tap implementation"
        );
    }
}
