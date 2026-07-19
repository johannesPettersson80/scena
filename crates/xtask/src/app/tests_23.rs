use crate::app::prelude::*;

#[test]
pub(crate) fn c12_doctor_rejects_base_pose_picking_bypass() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut current_findings = Vec::new();
    check_c12_deformed_picking_contracts(&root, &mut current_findings);
    assert!(
        current_findings
            .iter()
            .all(|finding| finding.rule != "SCENE-C12"),
        "current C12 contracts must satisfy doctor before mutation: {current_findings:?}",
    );

    let fixture_root = root.join("target/xtask-doctor-regressions/c12-deformed-picking");
    let files = [
        "src/geometry.rs",
        "src/geometry/deformation.rs",
        "src/render/prepare/primitives.rs",
        "src/render/prepare/shadows.rs",
        "src/picking.rs",
        "src/scene/picking.rs",
        "tests/c12_deformed_picking.rs",
        "docs/api.md",
    ];
    for relative in files {
        let source = root.join(relative);
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("C12 fixture parent"))
            .expect("C12 fixture directory");
        fs::copy(source, destination).expect("copy C12 doctor fixture file");
    }

    let picking = fixture_root.join("src/picking.rs");
    let source = fs::read_to_string(&picking).expect("read C12 picking fixture");
    let mutated = source.replace(
        ".deformed_vertices(scene.morph_weights(node), skin_matrices.as_deref())",
        ".deformed_vertices(None, None)",
    );
    assert_ne!(
        mutated, source,
        "C12 mutation must bypass the live scene deformation pose"
    );
    fs::write(&picking, mutated).expect("write base-pose picking mutation");
    let mut findings = Vec::new();

    check_c12_deformed_picking_contracts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "SCENE-C12"
                && finding.message.contains("src/picking.rs")
                && finding.message.contains("scene.morph_weights(node)")
        }),
        "doctor must reject picking that bypasses the rendered deformation pose: {findings:?}",
    );
}

#[test]
pub(crate) fn c13_doctor_rejects_silent_cpu_fallback_in_strict_constructor() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut current_findings = Vec::new();
    check_c13_strict_gpu_construction_contracts(&root, &mut current_findings);
    assert!(
        current_findings
            .iter()
            .all(|finding| finding.rule != "SCENE-C13"),
        "current C13 contracts must satisfy doctor before mutation: {current_findings:?}",
    );

    let fixture_root = root.join("target/xtask-doctor-regressions/c13-strict-gpu");
    let files = [
        "src/scene_host/construction.rs",
        "src/scene_host/core.rs",
        "src/render/backend_selection.rs",
        "src/viewer.rs",
        "src/viewer/load_progress.rs",
        "src/scene_host/recipe/host.rs",
        "src/scene_host/recipe.rs",
        "src/scene_host/recipe/backend.rs",
        "src/bin/scena/input.rs",
        "src/bin/scena/recipe.rs",
        "src/bin/scena/recipe/quality/depth_of_field.rs",
        "src/scene_host/core_tests.rs",
        "examples/scene_host_contracts.rs",
        "docs/capabilities.md",
        ".github/workflows/ci.yml",
        ".github/workflows/release.yml",
    ];
    for relative in files {
        let source = root.join(relative);
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("C13 fixture parent"))
            .expect("C13 fixture directory");
        fs::copy(source, destination).expect("copy C13 doctor fixture file");
    }

    let core = fixture_root.join("src/scene_host/construction.rs");
    let source = fs::read_to_string(&core).expect("read C13 core fixture");
    let mutated = source.replace(
        "        let renderer = build_gpu(width, height)?;",
        "        let renderer = build_gpu(width, height)\n            .or_else(|_gpu_error| Renderer::headless(width, height))?;",
    );
    assert_ne!(
        mutated, source,
        "C13 mutation must restore the silent CPU fallback"
    );
    fs::write(&core, mutated).expect("write silent GPU fallback mutation");
    let mut findings = Vec::new();

    check_c13_strict_gpu_construction_contracts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "SCENE-C13"
                && finding.message.contains("src/scene_host/construction.rs")
                && (finding
                    .message
                    .contains("let renderer = build_gpu(width, height)?;")
                    || finding.message.contains("or_else(|_gpu_error|"))
        }),
        "doctor must reject strict constructors that silently return CPU: {findings:?}",
    );
}
