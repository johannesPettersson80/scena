use crate::app::prelude::*;

#[test]
pub(crate) fn renderer_truth_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_renderer_truth_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn doctor_rejects_shader_clip_position_passthrough_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/shader-passthrough");
    let shader_path = fixture_root.join("src/render/gpu/output.rs");
    fs::create_dir_all(shader_path.parent().expect("shader parent")).expect("fixture dir");
    fs::write(
        &shader_path,
        "fn vs_main() { out.position = vec4<f32>(in.position, 1.0); }\n",
    )
    .expect("shader fixture");
    let mut findings = Vec::new();

    forbid_contains(
        &fixture_root,
        &mut findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/output.rs",
        &["out.position = vec4<f32>(in.position, 1.0);"],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-RENDER-TRUTH" && finding.message.contains("out.position = vec4")
        }),
        "doctor must reject production shaders that bypass camera projection: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_shader_module_creation_outside_generated_manifest() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/shader-manifest-bypass");
    let bypass_path = fixture_root.join("src/render/gpu/new_pipeline.rs");
    fs::create_dir_all(bypass_path.parent().expect("shader bypass parent")).expect("fixture dir");
    fs::write(
        &bypass_path,
        "fn bypass(device: &wgpu::Device, source: &str) { device.create_shader_module(todo!()); }\n",
    )
    .expect("shader bypass fixture");
    let mut findings = Vec::new();

    check_renderer_truth_contracts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-SHADER-MANIFEST"
                && finding.message.contains("new_pipeline.rs")
                && finding.message.contains("outside")
        }),
        "doctor must reject production shader modules that bypass the generated manifest: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_supported_forward_pbr_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/supported-pbr");
    let capability_status_path = fixture_root.join("src/diagnostics/capability_status.rs");
    fs::create_dir_all(
        capability_status_path
            .parent()
            .expect("capability status parent"),
    )
    .expect("fixture dir");
    fs::write(
        &capability_status_path,
        "const fn forward_pbr_status(_backend: Backend) -> CapabilityStatus {\n    CapabilityStatus::Supported\n}\n",
    )
    .expect("capability fixture");
    let mut findings = Vec::new();

    forbid_contains(
        &fixture_root,
        &mut findings,
        "ARCH-RENDER-TRUTH",
        "src/diagnostics/capability_status.rs",
        &[
            "forward_pbr_status(_backend: Backend) -> CapabilityStatus {\n    CapabilityStatus::Supported",
            "forward_pbr_status(\n    backend: Backend,\n    gpu_device: bool,\n) -> CapabilityStatus {\n    CapabilityStatus::Supported",
        ],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-RENDER-TRUTH"
                && finding.message.contains("CapabilityStatus::Supported")
        }),
        "doctor must reject false forward_pbr support claims: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_unconditional_supported_forward_pbr_with_gpu_device_signature() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/supported-pbr-gpu-device");
    let capability_status_path = fixture_root.join("src/diagnostics/capability_status.rs");
    fs::create_dir_all(
        capability_status_path
            .parent()
            .expect("capability status parent"),
    )
    .expect("fixture dir");
    fs::write(
        &capability_status_path,
        "const fn forward_pbr_status(\n    backend: Backend,\n    gpu_device: bool,\n) -> CapabilityStatus {\n    CapabilityStatus::Supported\n}\n",
    )
    .expect("capability fixture");
    let mut findings = Vec::new();

    forbid_contains(
        &fixture_root,
        &mut findings,
        "ARCH-RENDER-TRUTH",
        "src/diagnostics/capability_status.rs",
        &[
            "forward_pbr_status(\n    backend: Backend,\n    gpu_device: bool,\n) -> CapabilityStatus {\n    CapabilityStatus::Supported",
        ],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-RENDER-TRUTH"
                && finding.message.contains("CapabilityStatus::Supported")
        }),
        "doctor must still reject unconditional forward_pbr support after the status helper gains a gpu_device guard: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_meshless_model_viewer_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/meshless-viewer");
    let example_path = fixture_root.join("examples/glb_model_viewer.rs");
    fs::create_dir_all(example_path.parent().expect("example parent")).expect("fixture dir");
    fs::write(
        &example_path,
        "fn main() { let _path = \"tests/assets/gltf/minimal_scene.gltf\"; }\n",
    )
    .expect("example fixture");
    let mut findings = Vec::new();

    forbid_contains(
        &fixture_root,
        &mut findings,
        "ARCH-RENDER-TRUTH",
        "examples/glb_model_viewer.rs",
        &["minimal_scene.gltf"],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-RENDER-TRUTH" && finding.message.contains("minimal_scene.gltf")
        }),
        "doctor must reject model-viewer examples backed by meshless fixtures: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_oversized_source_module_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/oversized-module");
    let source_path = fixture_root.join("src/render/too_large.rs");
    fs::create_dir_all(source_path.parent().expect("source parent")).expect("fixture dir");
    let mut source = String::new();
    for index in 0..=MAX_SIGNIFICANT_LINES_PER_SOURCE_MODULE {
        source.push_str(&format!("pub fn oversized_fixture_{index}() {{}}\n"));
    }
    fs::write(&source_path, source).expect("oversized source fixture");
    let mut findings = Vec::new();

    check_solid_kiss(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-KISS-SIZE" && finding.message.contains("src/render/too_large.rs")
        }),
        "doctor must reject source modules above the KISS size threshold: {findings:?}",
    );
}

#[test]
pub(crate) fn significant_line_count_counts_product_code_after_test_modules() {
    let mut source = String::from("#[cfg(test)]\nmod tests {\n    fn helper() {}\n}\n");
    for index in 0..=MAX_SIGNIFICANT_LINES_PER_SOURCE_MODULE {
        source.push_str(&format!("pub fn counted_after_tests_{index}() {{}}\n"));
    }

    assert!(
        significant_line_count(&source) > MAX_SIGNIFICANT_LINES_PER_SOURCE_MODULE,
        "the KISS size gate must not stop counting at the first #[cfg(test)] block"
    );
}

#[test]
pub(crate) fn significant_line_count_excludes_individual_cfg_test_functions() {
    let source = "#[cfg(test)]\nfn helper() {\n    assert!(true);\n}\npub fn production() {}\n";
    assert_eq!(significant_line_count(source), 1);
}

#[test]
pub(crate) fn external_cfg_test_module_is_excluded_from_production_size_gate() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/external-cfg-test-module");
    let owner = fixture_root.join("src/render/quality.rs");
    let tests = fixture_root.join("src/render/quality/tests.rs");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(tests.parent().expect("external test fixture parent"))
        .expect("external test fixture dir");
    fs::write(&owner, "#[cfg(test)]\nmod tests;\n").expect("external test owner fixture");
    let mut source = String::new();
    for index in 0..=MAX_SIGNIFICANT_LINES_PER_SOURCE_MODULE {
        source.push_str(&format!("fn test_helper_{index}() {{}}\n"));
    }
    fs::write(&tests, source).expect("external test module fixture");
    let mut findings = Vec::new();

    check_solid_kiss(&fixture_root, &mut findings);

    assert!(
        !findings.iter().any(|finding| {
            finding.rule == "ARCH-KISS-SIZE"
                && finding.message.contains("src/render/quality/tests.rs")
        }),
        "an external module reachable only through #[cfg(test)] is not production code: {findings:?}",
    );
}

#[test]
pub(crate) fn external_non_test_module_remains_in_production_size_gate() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/external-production-module");
    let owner = fixture_root.join("src/render/quality.rs");
    let module = fixture_root.join("src/render/quality/large.rs");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(module.parent().expect("external module fixture parent"))
        .expect("external module fixture dir");
    fs::write(&owner, "mod large;\n").expect("external module owner fixture");
    let mut source = String::new();
    for index in 0..=MAX_SIGNIFICANT_LINES_PER_SOURCE_MODULE {
        source.push_str(&format!("fn production_helper_{index}() {{}}\n"));
    }
    fs::write(&module, source).expect("external production module fixture");
    let mut findings = Vec::new();

    check_solid_kiss(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-KISS-SIZE"
                && finding.message.contains("src/render/quality/large.rs")
        }),
        "an ordinary external module remains production code: {findings:?}",
    );
}

#[test]
pub(crate) fn architecture_m5_contract_does_not_require_generated_gate_artifacts() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/m5-source-only-architecture");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(&fixture_root).expect("M5 source-only fixture root");
    let mut findings = Vec::new();

    check_m5_release_contracts(&fixture_root, &mut findings);

    assert!(
        !findings
            .iter()
            .any(|finding| finding.message.contains("target/gate-artifacts")),
        "architecture mode must validate source contracts, not ignored generated artifacts: {findings:?}",
    );
}

#[test]
pub(crate) fn prepare_asset_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_prepare_asset_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn doctor_rejects_shipped_area_light_claims_without_ltc_or_light_assignment_source() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/area-light-honesty");
    fs::create_dir_all(fixture_root.join("docs/checklists")).expect("checklist fixture dir");
    fs::create_dir_all(fixture_root.join("src/render/prepare")).expect("render fixture dir");
    fs::write(
        fixture_root.join("docs/checklists/stunning-renders-and-performance.md"),
        r#"# Stunning renders + performance

## A3 -- Soft area lights (LTC rect/disc/sphere) -- [shipped]

- [x] LTC (linearly-transformed cosines) rect/disc/sphere area lights, with soft-shadow support.

## B2 -- Clustered / tiled light culling -- [shipped]

- [x] Cluster/tile light assignment so many-light scenes scale.
"#,
    )
    .expect("checklist fixture");
    fs::write(
        fixture_root.join("src/render/prepare/lighting.rs"),
        "pub const AREA_LIGHT_SAMPLE_COUNT: usize = 16;\npub fn finite_emitter_samples() {}\n",
    )
    .expect("finite-emitter render fixture");
    let mut findings = Vec::new();

    check_area_light_acceptance_honesty(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-PREPARE-AREA-LIGHT-HONESTY"
                && finding.message.contains("A3 cannot be marked shipped")
        }),
        "doctor must reject a shipped A3/LTC claim without a dedicated LTC source marker: {findings:?}",
    );
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-PREPARE-AREA-LIGHT-HONESTY"
                && finding.message.contains("B2 cannot be marked shipped")
        }),
        "doctor must reject a shipped B2 clustered/tiled claim without light-assignment source: {findings:?}",
    );
}

#[test]
pub(crate) fn particle_prepare_allocation_contract_is_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_particle_prepare_allocation_contract(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn doctor_rejects_particle_prepare_collect_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/particle-prepare-collect");
    let particles_path = fixture_root.join("src/render/prepare/particles.rs");
    fs::create_dir_all(particles_path.parent().expect("particle prepare parent"))
        .expect("fixture dir");
    fs::write(
        &particles_path,
        "fn append_particle_primitives(scene: &Scene, primitives: &mut Vec<PreparedPrimitive>) {\n    let particles = scene.particle_set_nodes().collect::<Vec<_>>();\n    primitives.reserve(particles.len());\n}\n",
    )
    .expect("particle prepare fixture");
    let mut findings = Vec::new();

    check_particle_prepare_allocation_contract(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-PREPARE-PARTICLES" && finding.message.contains("collect::<Vec")
        }),
        "doctor must reject intermediate Vec collection in particle prepare: {findings:?}",
    );
}

#[test]
pub(crate) fn render_world_bake_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_render_world_bake_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn doctor_rejects_renderer_asset_fetch_regression() {
    // ARCH-RENDER: nothing under src/render/** may name asset fetcher entry points.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/renderer-asset-fetch");
    let render_path = fixture_root.join("src/render/build.rs");
    fs::create_dir_all(render_path.parent().expect("render parent")).expect("fixture dir");
    fs::write(
        &render_path,
        "fn build_renderer() { let _bytes = fetcher.fetch(\"asset\"); }\n",
    )
    .expect("render fixture");
    let mut findings = Vec::new();

    forbid_contains(
        &fixture_root,
        &mut findings,
        "ARCH-RENDER",
        "src/render/build.rs",
        &["fetch("],
    );

    assert!(
        findings
            .iter()
            .any(|finding| { finding.rule == "ARCH-RENDER" && finding.message.contains("fetch(") }),
        "doctor must reject renderer modules that call fetcher entry points: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_renderer_photo_planning_boundary_regression() {
    // ARCH-RENDER-PHOTO-BOUNDARY: photo intent planning is host/CLI work.
    // Renderer render modules must not select candidates, retry exposure, or
    // own photo-report schemas.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/renderer-photo-planning");
    let render_path = fixture_root.join("src/render/frame.rs");
    fs::create_dir_all(render_path.parent().expect("render parent")).expect("fixture dir");
    fs::write(
        &render_path,
        "use crate::PhotoCandidatePlanV1;\nfn render() { let _ = product_hero_candidate_plan; }\n",
    )
    .expect("render fixture");
    let mut findings = Vec::new();

    check_module_boundaries(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-RENDER-PHOTO-BOUNDARY"
                && finding.message.contains("PhotoCandidatePlanV1")
        }),
        "doctor must reject renderer modules that name photo-planning contracts: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_photo_target_resolution_drift_regression() {
    // ARCH-SHARED-TARGET-RESOLVER: all photo/subject consumers must route
    // target handles through scene::recipe::target_resolution instead of
    // reimplementing import/node matching locally.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/photo-target-resolution-drift");
    let photo_path = fixture_root.join("src/bin/scena/photo.rs");
    fs::create_dir_all(photo_path.parent().expect("photo parent")).expect("fixture dir");
    fs::write(
        &photo_path,
        "fn select(target: SceneRecipeTargetV1) { match target { SceneRecipeTargetV1::Import { id } => { let _ = id; }, SceneRecipeTargetV1::Node { id } => { let _ = id; }, _ => {} } }\n",
    )
    .expect("photo target drift fixture");
    let mut findings = Vec::new();

    check_module_boundaries(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-SHARED-TARGET-RESOLVER"
                && finding.message.contains("src/bin/scena/photo.rs")
        }),
        "doctor must reject photo target-resolution drift outside the shared resolver: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_render_phase_pipeline_creation_regression() {
    // ARCH-RENDER-LIFECYCLE: render-phase modules must not allocate shaders or pipelines.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/render-phase-pipeline-creation");
    let draw_path = fixture_root.join("src/render/gpu/draw.rs");
    fs::create_dir_all(draw_path.parent().expect("draw parent")).expect("fixture dir");
    fs::write(
        &draw_path,
        "fn render() { device.create_render_pipeline(&desc); }\n",
    )
    .expect("draw fixture");
    let mut findings = Vec::new();

    forbid_contains(
        &fixture_root,
        &mut findings,
        "ARCH-RENDER-LIFECYCLE",
        "src/render/gpu/draw.rs",
        &["create_render_pipeline"],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-RENDER-LIFECYCLE"
                && finding.message.contains("create_render_pipeline")
        }),
        "doctor must reject GPU render-phase modules that create render pipelines: \
         {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_platform_renderer_pass_regression() {
    // ARCH-PLATFORM: platform stays an adapter layer; pass type names belong in
    // render/**. The canonical forbidden terms are `wgpu::`, `ForwardPass`, `ShadowPass`,
    // and `PostProcessPass`.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/platform-render-pass");
    let platform_path = fixture_root.join("src/platform.rs");
    fs::create_dir_all(platform_path.parent().expect("platform parent")).expect("fixture dir");
    fs::write(
        &platform_path,
        "pub struct ForwardPass; pub fn run(_pass: &mut ForwardPass) {}\n",
    )
    .expect("platform fixture");
    let mut findings = Vec::new();

    forbid_contains(
        &fixture_root,
        &mut findings,
        "ARCH-PLATFORM",
        "src/platform.rs",
        &["ForwardPass"],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-PLATFORM" && finding.message.contains("ForwardPass")
        }),
        "doctor must reject platform.rs that owns renderer pass types: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_assets_wgpu_dependency_regression() {
    // ARCH-ASSETS: assets owns fetch/parse/cache and must not consume wgpu surface types.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/assets-wgpu-dependency");
    let assets_path = fixture_root.join("src/assets.rs");
    fs::create_dir_all(assets_path.parent().expect("assets parent")).expect("fixture dir");
    fs::write(
        &assets_path,
        "fn upload(device: &wgpu::Device) { let _texture = device.create_texture(&desc); }\n",
    )
    .expect("assets fixture");
    let mut findings = Vec::new();

    forbid_contains(
        &fixture_root,
        &mut findings,
        "ARCH-ASSETS",
        "src/assets.rs",
        &["wgpu::"],
    );

    assert!(
        findings
            .iter()
            .any(|finding| { finding.rule == "ARCH-ASSETS" && finding.message.contains("wgpu::") }),
        "doctor must reject assets.rs that pulls in wgpu types: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_output_stage_missing_aces_tonemap_regression() {
    // ARCH-OUTPUT-STAGE: the renderer output stage must implement ACES; a stub
    // src/render/output.rs that drops the tonemap helpers regresses the contract.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/output-stage-no-aces");
    let output_path = fixture_root.join("src/render/output.rs");
    fs::create_dir_all(output_path.parent().expect("output parent")).expect("fixture dir");
    fs::write(
        &output_path,
        "// no aces helpers here\npub fn passthrough() {}\n",
    )
    .expect("output fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-OUTPUT-STAGE",
        "src/render/output.rs",
        &[
            "fn aces_tonemap",
            "fn rrt_and_odt_fit",
            "ACES_INPUT_MATRIX",
            "ACES_OUTPUT_MATRIX",
        ],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-OUTPUT-STAGE" && finding.message.contains("fn aces_tonemap")
        }),
        "doctor must reject output stages that drop ACES tonemap helpers: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_render_alpha_missing_linear_source_over_regression() {
    // ARCH-RENDER-ALPHA: capabilities.rs must expose AlphaPipelineStatus with the
    // LinearSourceOver and BackendPassthrough variants. A stub that drops them
    // regresses the contract.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/render-alpha-stub");
    let capabilities_path = fixture_root.join("src/diagnostics/capabilities.rs");
    fs::create_dir_all(capabilities_path.parent().expect("capabilities parent"))
        .expect("fixture dir");
    fs::write(&capabilities_path, "pub struct Capabilities {}\n").expect("capabilities fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-RENDER-ALPHA",
        "src/diagnostics/capabilities.rs",
        &[
            "pub enum AlphaPipelineStatus",
            "LinearSourceOver",
            "BackendPassthrough",
            "pub alpha_pipeline: AlphaPipelineStatus",
        ],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-RENDER-ALPHA" && finding.message.contains("LinearSourceOver")
        }),
        "doctor must reject capabilities that drop the alpha-pipeline contract: \
         {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_diagnostics_missing_typed_code_regression() {
    // ARCH-DIAGNOSTICS: diagnostic.rs must expose Diagnostic with code, severity,
    // and message. A stub without typed code regresses the contract.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/diagnostics-untyped");
    let diagnostic_path = fixture_root.join("src/diagnostics/diagnostic.rs");
    fs::create_dir_all(diagnostic_path.parent().expect("diagnostic parent")).expect("fixture dir");
    fs::write(
        &diagnostic_path,
        "pub struct Diagnostic { pub message: String }\n",
    )
    .expect("diagnostic fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-DIAGNOSTICS",
        "src/diagnostics/diagnostic.rs",
        &[
            "pub struct Diagnostic",
            "pub code: DiagnosticCode",
            "pub severity: DiagnosticSeverity",
            "pub message: String",
        ],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-DIAGNOSTICS" && finding.message.contains("DiagnosticCode")
        }),
        "doctor must reject Diagnostic types that drop the typed code/severity \
         contract: {findings:?}",
    );
}
