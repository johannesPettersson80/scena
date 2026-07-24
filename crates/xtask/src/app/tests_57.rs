use crate::app::prelude::*;

#[test]
fn c20_doctor_rejects_removed_pointer_capture() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/c20-browser-execution");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/viewer_element/element.js",
        "tests/browser/m6_rust_wasm_renderer_probe.js",
        "src/diagnostics/capabilities.rs",
        "src/diagnostics/capabilities/sample_counts.rs",
        "src/render/build.rs",
        "src/render/gpu/prepare_resources_wasm.rs",
        "tests/c20_wasm_capability_contracts.rs",
        "src/bin/scena/input.rs",
        "src/bin/scena/args/inspection.rs",
        "src/bin/scena/recipe.rs",
        "src/bin/scena/recipe/capture_sequence.rs",
        "src/bin/scena/recipe/cad_inspection.rs",
        "src/bin/scena/output.rs",
        "src/bin/scena/scene_commands.rs",
        "src/bin/scena/help.rs",
        "tests/scena_cli_recipe.rs",
        "tests/fr05_capture_sequence.rs",
        "tests/scena_cli_help.rs",
        "README.md",
        "docs/browser.md",
        "docs/capabilities.md",
        "docs/specs/public-api.md",
        "docs/api.md",
        "docs/headless-rendering.md",
        "docs/guides/llm-app-builder.md",
        "docs/troubleshooting.md",
        "docs/schema-contracts.md",
        "CLAUDE.md",
        "CHANGELOG.md",
        "docs/release-notes/v1.8.0.md",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("C20 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("C20 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_c20_browser_execution_ergonomics(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let element = fixture_root.join("src/viewer_element/element.js");
    let source = fs::read_to_string(&element).expect("C20 element source reads");
    let mutated = source.replacen("this.setPointerCapture(event.pointerId);", "", 1);
    assert_ne!(source, mutated, "C20 mutation must remove pointer capture");
    fs::write(&element, mutated).expect("C20 element mutation writes");
    findings.clear();
    check_c20_browser_execution_ergonomics(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "C20-BROWSER-EXECUTION-ERGONOMICS"
                && finding
                    .message
                    .contains("this.setPointerCapture(event.pointerId)")
        }),
        "removing pointer capture must fail doctor: {findings:?}",
    );

    fs::write(&element, source).expect("C20 element source restores");
    let scene_commands = fixture_root.join("src/bin/scena/scene_commands.rs");
    let source = fs::read_to_string(&scene_commands).expect("scene command source reads");
    fs::write(
        &scene_commands,
        format!("{source}\nfn warn_gpu_fallback() {{ eprintln!(\"fallback\"); }}\n"),
    )
    .expect("fallback mutation writes");
    findings.clear();
    check_c20_browser_execution_ergonomics(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "C20-BROWSER-EXECUTION-ERGONOMICS"
                && (finding.message.contains("warn_gpu_fallback")
                    || finding.message.contains("eprintln!("))
        }),
        "unversioned GPU fallback prose must fail doctor: {findings:?}",
    );
}
