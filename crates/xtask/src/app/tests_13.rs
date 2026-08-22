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
        "src/render/gpu/browser_readback_trace.rs",
        "src/render/offscreen.rs",
        "src/scene_host/capture.rs",
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
        "src/render/gpu/post/pipeline_helpers.rs",
        "src/render/gpu/post/fxaa.rs",
        "src/render/gpu/post/ssao.rs",
        "src/render/gpu/post/dof.rs",
        "src/browser_probe/probes/state_lifecycle.rs",
        "tests/c09_gpu_resource_lifecycle.rs",
        "crates/xtask/src/app/release/review_artifacts.rs",
        "crates/xtask/src/app/release/lane_artifacts.rs",
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
        "docs/lifecycle.md",
        "docs/specs/release-gates.md",
        "README.md",
        "CHANGELOG.md",
        "docs/release-notes/v1.8.0.md",
    ];
    for relative in files {
        let source = root.join(relative);
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("C09 fixture parent"))
            .expect("C09 fixture directory");
        fs::copy(source, destination).expect("copy C09 doctor fixture file");
    }

    let readback = fixture_root.join("src/render/gpu/draw_surface_support.rs");
    let source = fs::read_to_string(&readback).expect("read C09 browser readback fixture");
    let mutated = source.replacen(
        "let slice = readback.buffer.slice(..);",
        "readback.buffer.unmap();\n        let slice = readback.buffer.slice(..);",
        1,
    );
    assert_ne!(
        source, mutated,
        "browser readback mutation must add an invalid pre-map unmap"
    );
    fs::write(&readback, mutated).expect("inject invalid browser readback unmap");
    let mut findings = Vec::new();
    check_c09_gpu_resource_lifecycle_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RENDER-C09"
                && finding.message.contains("must not unmap before map_async")
        }),
        "doctor must reject an unmapped browser buffer unmap: {findings:?}",
    );

    fs::copy(
        root.join("src/render/gpu/draw_surface_support.rs"),
        &readback,
    )
    .expect("restore C09 browser readback fixture");

    super::tests_77::assert_scene_host_capture_readback_is_enforced(&fixture_root, &mut findings);

    let draw = fixture_root.join("src/render/gpu/draw.rs");
    let source = fs::read_to_string(&draw).expect("read C09 draw fixture");
    let mutated =
        format!("{source}\nfn c09_regression() {{ let _ = post::create_resources(); }}\n");
    fs::write(&draw, mutated).expect("inject render-time post allocation");
    findings.clear();

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

    let source = fs::read_to_string(&workflow).expect("read partial hardware workflow fixture");
    let mutated = source.replacen(
        "SCENA_REQUIRE_GPU_RESOURCE_LIFECYCLE: \"1\"",
        "SCENA_REQUIRE_GPU_RESOURCE_LIFECYCLE: \"0\"",
        1,
    );
    assert_ne!(
        source, mutated,
        "required lifecycle mutation must alter workflow"
    );
    fs::write(&workflow, mutated).expect("disable strict GPU lifecycle evidence");
    findings.clear();
    check_c09_gpu_resource_lifecycle_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RENDER-C09"
                && finding
                    .message
                    .contains("SCENA_REQUIRE_GPU_RESOURCE_LIFECYCLE: \"1\"")
        }),
        "doctor must reject disabling the strict GPU lifecycle lane: {findings:?}",
    );

    let lane_artifacts = fixture_root.join("crates/xtask/src/app/release/lane_artifacts.rs");
    let source = fs::read_to_string(&lane_artifacts).expect("read release lane fixture");
    let mutated = source.replace(
        "if lane == \"macos-metal\"",
        "if lane == \"linux-native-vulkan\"",
    );
    assert_ne!(
        source, mutated,
        "lifecycle lane ownership mutation must alter release tooling"
    );
    fs::write(&lane_artifacts, mutated).expect("misroute lifecycle evidence to software Vulkan");
    findings.clear();
    check_c09_gpu_resource_lifecycle_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RENDER-C09" && finding.message.contains("if lane == \"macos-metal\"")
        }),
        "doctor must reject assigning physical lifecycle evidence to the hosted software-Vulkan lane: {findings:?}",
    );

    super::tests_77::assert_browser_backend_selectors_are_enforced(&fixture_root, &mut findings);

    let lifecycle = fixture_root.join("src/render/gpu/lifecycle.rs");
    let source = fs::read_to_string(&lifecycle).expect("read lifecycle completion fixture");
    let mutated = source.replacen(
        "wgpu::Backend::Gl | wgpu::Backend::BrowserWebGpu",
        "wgpu::Backend::Gl",
        1,
    );
    assert_ne!(
        source, mutated,
        "WebGPU-retirement mutation must alter the lifecycle implementation"
    );
    fs::write(&lifecycle, mutated).expect("remove browser WebGPU automatic retirement");
    findings.clear();
    check_c09_gpu_resource_lifecycle_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RENDER-C09"
                && finding
                    .message
                    .contains("wgpu::Backend::Gl | wgpu::Backend::BrowserWebGpu")
        }),
        "doctor must reject removing browser-managed WebGPU retirement: \
         {findings:?}",
    );

    fs::write(&lifecycle, &source).expect("restore lifecycle fixture");
    let mutated = source.replacen(
        "(0, DevicePollStatus::Unsupported)",
        "self.queue.on_submitted_work_done(|| {});\n        \
         (0, DevicePollStatus::Unsupported)",
        1,
    );
    assert_ne!(
        source, mutated,
        "callback-regression mutation must alter the lifecycle implementation"
    );
    fs::write(&lifecycle, mutated).expect("restore callback-dependent retirement");
    findings.clear();
    check_c09_gpu_resource_lifecycle_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RENDER-C09"
                && finding.message.contains(
                    "browser logical resource retirement must not wait on \
                     on_submitted_work_done",
                )
        }),
        "doctor must reject callback-dependent browser retirement: {findings:?}",
    );

    let build = fixture_root.join("src/render/gpu/build.rs");
    let source = fs::read_to_string(&build).expect("read WebGL2 fence-policy fixture");
    let mutated = source.replacen(
        "descriptor.backend_options.gl.fence_behavior = wgpu::GlFenceBehavior::AutoFinish;",
        "descriptor.backend_options.gl.fence_behavior = wgpu::GlFenceBehavior::Normal;",
        1,
    );
    assert_ne!(
        source, mutated,
        "WebGL2 fence-policy mutation must alter the GPU builder"
    );
    fs::write(&build, mutated).expect("restore normal WebGL2 fence behavior");
    findings.clear();
    check_c09_gpu_resource_lifecycle_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RENDER-C09"
                && finding.message.contains(
                    "descriptor.backend_options.gl.fence_behavior = \
                     wgpu::GlFenceBehavior::AutoFinish;",
                )
        }),
        "doctor must reject WebGL2 lifetime tracking that depends on a browser fence: \
         {findings:?}",
    );

    let evidence = fixture_root.join("tests/c09_gpu_resource_lifecycle.rs");
    let source = fs::read_to_string(&evidence).expect("read C09 evidence fixture");
    let mutated = source.replace(
        "required_lifecycle_source_checksums()",
        "removed_lifecycle_source_checksums()",
    );
    assert_ne!(
        source, mutated,
        "source-provenance mutation must alter the C09 evidence producer"
    );
    fs::write(&evidence, mutated).expect("remove C09 source provenance producer");
    findings.clear();
    check_c09_gpu_resource_lifecycle_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RENDER-C09"
                && finding
                    .message
                    .contains("required_lifecycle_source_checksums()")
        }),
        "doctor must reject removal of Q04 source provenance: {findings:?}",
    );
}
