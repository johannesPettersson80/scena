use crate::app::prelude::*;

#[test]
pub(crate) fn doctor_rejects_visual_browser_m1_missing_artifact_regression() {
    // VISUAL-BROWSER-M1: each browser-probe workflow must declare its
    // visual artifact under `target/gate-artifacts/m6-browser-visual/`
    // with a renderer/color/tolerance/source contract; absence regresses
    // the M6 browser parity gate.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/visual-browser-m1-stub");
    let stub_path = fixture_root.join("src/browser_probe/workflows/pbr.rs");
    fs::create_dir_all(stub_path.parent().expect("workflow parent")).expect("fixture dir");
    fs::write(
        &stub_path,
        "// Stub workflow without the visual-artifact declarations.\n",
    )
    .expect("workflow fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "VISUAL-BROWSER-M1",
        "src/browser_probe/workflows/pbr.rs",
        &["pbr-environment-lit", "renderer", "tolerance"],
    );

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "VISUAL-BROWSER-M1"),
        "doctor must reject browser-probe workflows that drop their visual \
         artifact declarations: {findings:?}",
    );
}

#[test]
pub(crate) fn c05_units_doctor_rejects_removed_single_unit_root() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/c05-single-unit-root");
    let files = [
        "src/scene/import/options.rs",
        "src/scene/import.rs",
        "src/scene/import/units.rs",
        "tests/c05_import_unit_contracts.rs",
        "docs/guides/units-axes-handedness.md",
        "docs/assets.md",
    ];
    for relative in files {
        let source = root.join(relative);
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("C05 fixture parent"))
            .expect("C05 fixture directory");
        fs::copy(source, destination).expect("copy C05 doctor fixture file");
    }
    let options = fixture_root.join("src/scene/import/options.rs");
    let source = fs::read_to_string(&options).expect("read C05 options fixture");
    fs::write(
        &options,
        source.replace(
            "Transform::IDENTITY.scale_by(self.source_units.meters_per_unit())",
            "Transform::IDENTITY",
        ),
    )
    .expect("remove the single unit conversion boundary");
    let mut findings = Vec::new();

    check_c05_unit_contracts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ASSETS-C05"
                && finding.message.contains("src/scene/import/options.rs")
                && finding.message.contains("meters_per_unit exactly once")
        }),
        "doctor must reject removal of the single unit root conversion: {findings:?}",
    );
}

#[test]
pub(crate) fn c06_finite_atomic_doctor_rejects_removed_pointer_guard() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/c06-pointer-finite-guard");
    let files = [
        "src/scene/transforms.rs",
        "src/scene.rs",
        "src/scene/view.rs",
        "src/scene/instances.rs",
        "src/controls.rs",
        "src/scene_host/transforms.rs",
        "src/scene_host/instances.rs",
        "src/diagnostics.rs",
        "tests/c06_finite_atomic_transforms.rs",
        "docs/api.md",
        "docs/errors.md",
    ];
    for relative in files {
        let source = root.join(relative);
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("C06 fixture parent"))
            .expect("C06 fixture directory");
        fs::copy(source, destination).expect("copy C06 doctor fixture file");
    }
    let controls = fixture_root.join("src/controls.rs");
    let source = fs::read_to_string(&controls).expect("read C06 controls fixture");
    fs::write(
        &controls,
        source.replace(
            "if !pointer_event_is_finite(event)",
            "if !event.position.0.is_finite()",
        ),
    )
    .expect("disable the pointer finite guard");
    let mut findings = Vec::new();

    check_c06_finite_atomic_contracts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "SCENE-C06"
                && finding.message.contains("src/controls.rs")
                && finding
                    .message
                    .contains("if !pointer_event_is_finite(event)")
        }),
        "doctor must reject a disabled pointer finite guard: {findings:?}",
    );
}

#[test]
pub(crate) fn c07_handle_namespace_doctor_rejects_untagged_import_table() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/c07-untagged-import-table");
    let files = [
        "src/scene_host/handles.rs",
        "src/scene_host/core.rs",
        "src/scene_host/core_handles.rs",
        "src/scene_host/instances.rs",
        "src/scene_host/error.rs",
        "tests/c07_handle_namespaces.rs",
        "tests/browser/scene_host_browser_proof.js",
        "docs/errors.md",
    ];
    for relative in files {
        let source = root.join(relative);
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("C07 fixture parent"))
            .expect("C07 fixture directory");
        fs::copy(source, destination).expect("copy C07 doctor fixture file");
    }
    let core = fixture_root.join("src/scene_host/core.rs");
    let source = fs::read_to_string(&core).expect("read C07 core fixture");
    fs::write(
        &core,
        source.replace(
            "HandleTable::new(HandleKind::Import)",
            "HandleTable::new(HandleKind::Node)",
        ),
    )
    .expect("collapse import table into node namespace");
    let mut findings = Vec::new();

    check_c07_handle_namespace_contracts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "SCENE-C07"
                && finding.message.contains("src/scene_host/core.rs")
                && finding
                    .message
                    .contains("HandleTable::new(HandleKind::Import)")
        }),
        "doctor must reject collapsing imports into the node namespace: {findings:?}",
    );

    fs::copy(root.join("src/scene_host/core.rs"), &core).expect("restore C07 core fixture");
    let browser = fixture_root.join("tests/browser/scene_host_browser_proof.js");
    let source = fs::read_to_string(&browser).expect("read C07 browser fixture");
    fs::write(
        &browser,
        source.replace(
            "addProductGridFloorUnderNode(leftFrameHandle)",
            "addProductGridFloorUnderNode(handleBigInt(leftImportReport.import))",
        ),
    )
    .expect("restore import-as-node browser regression");
    findings.clear();
    check_c07_handle_namespace_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "SCENE-C07"
                && finding
                    .message
                    .contains("addProductGridFloorUnderNode(handleBigInt(leftImportReport.import))")
        }),
        "doctor must reject passing an import handle to a node-only browser binding: {findings:?}",
    );
}

#[test]
pub(crate) fn c08_timeline_doctor_rejects_removed_duration_clamp() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut current_findings = Vec::new();
    check_c08_presentation_timeline_contracts(&root, &mut current_findings);
    assert!(
        current_findings
            .iter()
            .all(|finding| finding.rule != "SCENE-C08"),
        "current C08 contracts must satisfy doctor before mutation: {current_findings:?}",
    );
    let fixture_root = root.join("target/xtask-doctor-regressions/c08-timeline-duration-clamp");
    let files = [
        "src/scene_host/presentation_timeline.rs",
        "src/scene_host/animation.rs",
        "tests/presentation_timeline.rs",
        "docs/schema-contracts.md",
    ];
    for relative in files {
        let source = root.join(relative);
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("C08 fixture parent"))
            .expect("C08 fixture directory");
        fs::copy(source, destination).expect("copy C08 doctor fixture file");
    }
    let timeline = fixture_root.join("src/scene_host/presentation_timeline.rs");
    let source = fs::read_to_string(&timeline).expect("read C08 timeline fixture");
    let mutated = source.replacen(".min(duration_seconds)", "", 1);
    assert_ne!(mutated, source, "C08 mutation must remove the clamp");
    fs::write(&timeline, mutated).expect("remove timeline duration clamp");
    let mut findings = Vec::new();

    check_c08_presentation_timeline_contracts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "SCENE-C08"
                && finding.message.contains("missing required duration clamp")
        }),
        "doctor must reject removal of the clip-duration clamp: {findings:?}",
    );
}

#[test]
pub(crate) fn c09_gpu_lifecycle_doctor_rejects_render_time_output_allocation() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut current_findings = Vec::new();
    check_c09_gpu_resource_lifecycle_contracts(&root, &mut current_findings);
    assert!(
        current_findings
            .iter()
            .all(|finding| finding.rule != "RENDER-C09"),
        "current C09 contracts must satisfy doctor before mutation: {current_findings:?}",
    );

    let fixture_root =
        root.join("target/xtask-doctor-regressions/c09-render-time-output-allocation");
    let files = [
        "src/render/prepare_lifecycle.rs",
        "src/render.rs",
        "src/render/settings.rs",
        "src/render/gpu/prepare_resources.rs",
        "src/render/gpu/headless_target.rs",
        "src/render/gpu/prepare_resources_wasm.rs",
        "src/render/gpu/build.rs",
        "src/render/gpu/draw_surface_support.rs",
        "src/render/gpu/browser_readback.rs",
        "src/render/offscreen.rs",
        "src/scene_host/wasm_capture.rs",
        "src/scene_host/wasm.rs",
        "src/scene_host/wasm_introspection.rs",
        "src/render/gpu/draw.rs",
        "src/render/gpu/draw_surface.rs",
        "src/render/gpu/draw_surface_probe.rs",
        "src/render/gpu/lifecycle.rs",
        "src/render/gpu/stats.rs",
        "src/render/gpu/post/types.rs",
        "src/render/gpu/post/resources.rs",
        "src/render/gpu/post/mod.rs",
        "src/render/gpu/post/fxaa.rs",
        "src/render/gpu/post/ssao.rs",
        "src/render/gpu/post/dof.rs",
        "src/browser_probe/probes/state_lifecycle.rs",
        "tests/c09_gpu_resource_lifecycle.rs",
        "tests/pf01_output_toggle.rs",
        "tests/browser/m6_rust_wasm_renderer_probe.js",
        "tests/browser/pf01_output_toggle.js",
        "tests/browser/scene_host_browser_proof.js",
        "tests/browser/hardware_browser.js",
        "tests/browser/required_gpu_parity.js",
        "tests/browser/fr06_semantic_aov.js",
        "examples/native_surface_hardware_proof.rs",
        "package.json",
        ".github/workflows/hardware-gpu.yml",
        "docs/api.md",
        "docs/browser.md",
    ];
    for relative in files {
        let source = root.join(relative);
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("C09 fixture parent"))
            .expect("C09 fixture directory");
        fs::copy(source, destination).expect("copy C09 doctor fixture file");
    }

    let draw = fixture_root.join("src/render/gpu/draw.rs");
    let source = fs::read_to_string(&draw).expect("read C09 draw fixture");
    let mutated =
        format!("{source}\nfn c09_regression() {{ let _ = post::create_resources(); }}\n");
    fs::write(&draw, mutated).expect("inject render-time post allocation");
    let mut findings = Vec::new();

    check_c09_gpu_resource_lifecycle_contracts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RENDER-C09"
                && finding.message.contains("src/render/gpu/draw.rs")
                && finding.message.contains("post::create_resources")
        }),
        "doctor must reject render-time post allocation: {findings:?}",
    );

    let dof = fixture_root.join("src/render/gpu/post/dof.rs");
    let source = fs::read_to_string(&dof).expect("read C09 DoF fixture");
    fs::write(
        &dof,
        format!("{source}\nfn regression(device: &wgpu::Device) {{ let _ = device.create_bind_group(todo!()); }}\n"),
    )
    .expect("inject render-time depth bind-group allocation");
    let mut findings = Vec::new();
    check_c09_gpu_resource_lifecycle_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RENDER-C09"
                && finding.message.contains("src/render/gpu/post/dof.rs")
                && finding.message.contains("bind group")
        }),
        "doctor must reject render-time depth bind-group allocation: {findings:?}",
    );

    let stats = fixture_root.join("src/render/gpu/stats.rs");
    let source = fs::read_to_string(&stats).expect("read C09 stats fixture");
    fs::write(
        &stats,
        format!("{source}\nfn estimate_prepared_resource_stats() {{}}\n"),
    )
    .expect("restore aggregate resource estimator");
    let mut findings = Vec::new();

    check_c09_gpu_resource_lifecycle_contracts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RENDER-C09"
                && finding.message.contains("src/render/gpu/stats.rs")
                && finding.message.contains("estimate_prepared_resource_stats")
        }),
        "doctor must reject restoration of aggregate estimate-based accounting: {findings:?}",
    );

    let post = fixture_root.join("src/render/gpu/post/mod.rs");
    let source = fs::read_to_string(&post).expect("read shared post-layout fixture");
    fs::write(
        &post,
        format!("{source}\n// mutation: create_pipeline_layout per pipeline\n"),
    )
    .expect("inject per-pipeline layout regression");
    findings.clear();
    check_c09_gpu_resource_lifecycle_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RENDER-C09" && finding.message.contains("shared layouts")
        }),
        "doctor must reject per-pipeline layout recreation: {findings:?}",
    );

    let workflow = fixture_root.join(".github/workflows/hardware-gpu.yml");
    let source = fs::read_to_string(&workflow).expect("read hardware workflow fixture");
    let mutated = source.replace(
        "cargo run --example native_surface_hardware_proof",
        "cargo run --example removed_native_surface_proof",
    );
    assert_ne!(
        source, mutated,
        "hardware proof mutation must alter workflow"
    );
    fs::write(&workflow, mutated).expect("remove attached native surface proof command");
    findings.clear();
    check_c09_gpu_resource_lifecycle_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RENDER-C09"
                && finding
                    .message
                    .contains("cargo run --example native_surface_hardware_proof")
        }),
        "doctor must reject removal of the required attached-surface hardware proof: {findings:?}",
    );

    let source = fs::read_to_string(&workflow).expect("read mutated hardware workflow fixture");
    let mutated = source.replacen(
        "SCENA_REQUIRE_PARITY: \"1\"",
        "SCENA_REQUIRE_PARITY: \"1\"\n      SCENA_ALLOW_PARTIAL_HARDWARE_BACKENDS: \"1\"",
        1,
    );
    assert_ne!(
        source, mutated,
        "partial hardware-backend mutation must alter workflow"
    );
    fs::write(&workflow, mutated).expect("enable partial hardware evidence in required workflow");
    findings.clear();
    check_c09_gpu_resource_lifecycle_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RENDER-C09"
                && finding
                    .message
                    .contains("SCENA_ALLOW_PARTIAL_HARDWARE_BACKENDS")
        }),
        "doctor must reject partial evidence in the required hardware workflow: {findings:?}",
    );

    let browser_selector = fixture_root.join("tests/browser/hardware_browser.js");
    let source = fs::read_to_string(&browser_selector).expect("read browser selector fixture");
    let mutated = source.replacen("gfx.webgpu.force-enabled", "removed-webgpu-force-enable", 1);
    assert_ne!(
        source, mutated,
        "browser selector mutation must remove the Firefox WebGPU preference"
    );
    fs::write(&browser_selector, mutated).expect("remove Firefox WebGPU selector preference");
    findings.clear();
    check_c09_gpu_resource_lifecycle_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RENDER-C09" && finding.message.contains("gfx.webgpu.force-enabled")
        }),
        "doctor must reject loss of the per-backend Firefox WebGPU route: {findings:?}",
    );

    let source = fs::read_to_string(&browser_selector).expect("read mutated browser selector");
    let mutated = source.replacen("platform === \"linux\"", "platform === \"win32\"", 1);
    assert_ne!(
        source, mutated,
        "Windows Chromium backend mutation must alter the platform selector"
    );
    fs::write(&browser_selector, mutated).expect("force Vulkan flags onto Windows Chromium");
    findings.clear();
    check_c09_gpu_resource_lifecycle_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RENDER-C09" && finding.message.contains("platform === \"linux\"")
        }),
        "doctor must reject routing Windows Chromium hardware proof through Vulkan flags: {findings:?}",
    );
}
