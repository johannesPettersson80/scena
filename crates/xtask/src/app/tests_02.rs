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
pub(crate) fn doctor_rejects_supported_forward_pbr_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/supported-pbr");
    let capabilities_path = fixture_root.join("src/diagnostics/capabilities.rs");
    fs::create_dir_all(capabilities_path.parent().expect("capabilities parent"))
        .expect("fixture dir");
    fs::write(
        &capabilities_path,
        "const fn forward_pbr_status(_backend: Backend) -> CapabilityStatus {\n    CapabilityStatus::Supported\n}\n",
    )
    .expect("capability fixture");
    let mut findings = Vec::new();

    forbid_contains(
        &fixture_root,
        &mut findings,
        "ARCH-RENDER-TRUTH",
        "src/diagnostics/capabilities.rs",
        &[
            "forward_pbr_status(_backend: Backend) -> CapabilityStatus {\n    CapabilityStatus::Supported",
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
pub(crate) fn prepare_asset_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_prepare_asset_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
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

#[test]
pub(crate) fn doctor_rejects_renderer_stats_missing_required_counters_regression() {
    // ARCH-RENDER-STATS: diagnostics.rs must expose RendererStats with the required
    // resource-lifetime counters. A stub that drops them regresses the contract.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/renderer-stats-stub");
    let diagnostics_path = fixture_root.join("src/diagnostics.rs");
    fs::create_dir_all(diagnostics_path.parent().expect("diagnostics parent"))
        .expect("fixture dir");
    fs::write(
        &diagnostics_path,
        "pub struct RendererStats { pub frames_rendered: u64 }\n",
    )
    .expect("diagnostics fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-RENDER-STATS",
        "src/diagnostics.rs",
        &[
            "pub struct RendererStats",
            "pub buffers: u64",
            "pub textures: u64",
            "pub materials: u64",
        ],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-RENDER-STATS" && finding.message.contains("pub buffers: u64")
        }),
        "doctor must reject RendererStats that drops the resource-lifetime counter \
         contract: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_camera_depth_missing_perspective_camera_regression() {
    // ARCH-CAMERA-DEPTH: src/scene/camera.rs must expose Camera/PerspectiveCamera/
    // OrthographicCamera/DepthRange. A stub that drops them regresses the contract.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/camera-depth-stub");
    let camera_path = fixture_root.join("src/scene/camera.rs");
    fs::create_dir_all(camera_path.parent().expect("camera parent")).expect("fixture dir");
    fs::write(&camera_path, "pub struct CameraStub {}\n").expect("camera fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-CAMERA-DEPTH",
        "src/scene/camera.rs",
        &[
            "pub enum Camera",
            "pub struct PerspectiveCamera",
            "pub struct OrthographicCamera",
            "pub struct DepthRange",
        ],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-CAMERA-DEPTH" && finding.message.contains("PerspectiveCamera")
        }),
        "doctor must reject camera modules that drop the typed-camera contract: \
         {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_clipping_missing_clipping_plane_key_regression() {
    // ARCH-CLIPPING: src/scene.rs must expose ClippingPlaneKey for typed clipping
    // plane handles.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/clipping-stub");
    let scene_path = fixture_root.join("src/scene.rs");
    fs::create_dir_all(scene_path.parent().expect("scene parent")).expect("fixture dir");
    fs::write(&scene_path, "pub struct Scene {}\n").expect("scene fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-CLIPPING",
        "src/scene.rs",
        &["pub struct ClippingPlaneKey"],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-CLIPPING" && finding.message.contains("ClippingPlaneKey")
        }),
        "doctor must reject scene modules that drop the typed clipping-plane handle: \
         {findings:?}",
    );
}
