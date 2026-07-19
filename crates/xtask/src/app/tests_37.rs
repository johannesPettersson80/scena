use crate::app::prelude::*;

#[test]
fn fr06_doctor_rejects_persistent_runtime_handle_claims() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/fr06-semantic-aov");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "docs/specs/semantic-aov-v1.md",
        "src/render/prepare/types.rs",
        "src/render/prepare.rs",
        "src/render/semantic_aov.rs",
        "src/render/settings.rs",
        "src/render/gpu/semantic_aov.rs",
        "src/render/gpu/semantic_aov/capture.rs",
        "src/render/gpu/semantic_aov/webgl2.rs",
        "src/render/gpu/output_shader.wgsl",
        "src/render/gpu/output_shader_texture_2d.wgsl",
        "src/render/gpu/instancing.rs",
        "src/render/gpu/vertices.rs",
        "src/scene_host/semantic_aov.rs",
        "src/scene_host/wasm.rs",
        "src/scene/recipe/types/build_manifest.rs",
        "src/scene_host/recipe/authoring/extras.rs",
        "src/bin/scena/recipe/semantic_aov.rs",
        "src/bin/scena/help.rs",
        "src/schema_catalog.rs",
        "docs/schema-contracts.md",
        "tests/fr06_semantic_aov.rs",
        "tests/browser/fr06_semantic_aov.js",
        "tests/release/windows_complete_hardware_proof_validation.js",
        "scripts/run_windows_complete_hardware_proof.ps1",
        "package.json",
        ".github/workflows/ci.yml",
        ".github/workflows/release.yml",
        ".github/workflows/hardware-gpu.yml",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("FR06 fixture path has parent"))
            .expect("FR06 fixture directory creates");
        fs::copy(root.join(relative), destination).expect("FR06 fixture source copies");
    }

    let mut findings = Vec::new();
    check_fr06_semantic_aov_contracts(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let host = fixture_root.join("src/scene_host/semantic_aov.rs");
    let source = fs::read_to_string(&host).expect("FR06 host fixture reads");
    let mutated = source.replace(
        "identity_scope: \"runtime_scoped\".to_owned()",
        "identity_scope: \"persistent\".to_owned()",
    );
    assert_ne!(source, mutated, "FR06 identity mutation must alter source");
    fs::write(&host, mutated).expect("FR06 identity mutation writes");
    findings.clear();
    check_fr06_semantic_aov_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "FR06-SEMANTIC-AOV" && finding.message.contains("runtime_scoped")
        }),
        "claiming runtime handles are persistent must fail: {findings:?}"
    );

    fs::write(&host, source).expect("FR06 host fixture restores");
    let wasm = fixture_root.join("src/scene_host/wasm.rs");
    let source = fs::read_to_string(&wasm).expect("FR06 wasm fixture reads");
    let mutated = source.replace("captureSemanticAovs", "captureAovsRemoved");
    assert_ne!(
        source, mutated,
        "FR06 GPU browser mutation must alter source"
    );
    fs::write(&wasm, mutated).expect("FR06 GPU browser mutation writes");
    findings.clear();
    check_fr06_semantic_aov_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "FR06-SEMANTIC-AOV" && finding.message.contains("captureSemanticAovs")
        }),
        "removing the GPU browser capture API must fail: {findings:?}"
    );

    fs::write(&wasm, source).expect("FR06 wasm fixture restores");
    let workflow = fixture_root.join(".github/workflows/hardware-gpu.yml");
    let source = fs::read_to_string(&workflow).expect("FR06 hardware workflow fixture reads");
    let mutated = source.replace(
        "cargo test --features scene-host --test fr06_semantic_aov",
        "cargo test --test removed_fr06_hardware_target",
    );
    assert_ne!(
        source, mutated,
        "FR06 hardware mutation must alter workflow"
    );
    fs::write(&workflow, mutated).expect("FR06 hardware mutation writes");
    findings.clear();
    check_fr06_semantic_aov_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "FR06-SEMANTIC-AOV"
                && finding
                    .message
                    .contains("cargo test --features scene-host --test fr06_semantic_aov")
        }),
        "removing the native FR06 hardware command must fail: {findings:?}"
    );
}
