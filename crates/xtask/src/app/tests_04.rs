use crate::app::prelude::*;

#[test]
pub(crate) fn doctor_rejects_environment_hdr_missing_loader_regression() {
    // ARCH-ENV-HDR: src/assets/environment.rs must expose the equirectangular HDR
    // loader so HDR fixtures can be parsed into PreparedEnvironmentLighting.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/environment-hdr-stub");
    let environment_path = fixture_root.join("src/assets/environment.rs");
    fs::create_dir_all(environment_path.parent().expect("environment parent"))
        .expect("fixture dir");
    fs::write(&environment_path, "pub struct EnvironmentDesc {}\n").expect("environment fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-ENV-HDR",
        "src/assets/environment.rs",
        &[
            "EnvironmentSourceKind::EquirectangularHdr",
            "pub fn from_equirectangular_hdr_path",
        ],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-ENV-HDR" && finding.message.contains("EquirectangularHdr")
        }),
        "doctor must reject assets/environment.rs that drops the equirectangular \
         HDR loader contract: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_environment_ibl_prepare_missing_stats_regression() {
    // ARCH-ENV-IBL-PREP: prepare/stats.rs must expose PreparedEnvironmentStats so
    // the IBL prepare path stays observable through structured stats.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/env-ibl-prepare-stub");
    let stats_path = fixture_root.join("src/render/prepare/stats.rs");
    fs::create_dir_all(stats_path.parent().expect("stats parent")).expect("fixture dir");
    fs::write(&stats_path, "pub struct LightingStats {}\n").expect("stats fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-ENV-IBL-PREP",
        "src/render/prepare/stats.rs",
        &["pub(in crate::render) struct PreparedEnvironmentStats"],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-ENV-IBL-PREP"
                && finding.message.contains("PreparedEnvironmentStats")
        }),
        "doctor must reject prepare/stats.rs that drops the PreparedEnvironmentStats \
         contract: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_directional_shadow_missing_multiple_lights_error_regression() {
    // ARCH-DIRECTIONAL-SHADOW: prepare/stats.rs must expose MultipleShadowedDirectionalLights
    // so a scene with two shadow-casting directional lights fails closed instead of
    // silently picking one.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/directional-shadow-stub");
    let stats_path = fixture_root.join("src/render/prepare/stats.rs");
    fs::create_dir_all(stats_path.parent().expect("stats parent")).expect("fixture dir");
    fs::write(&stats_path, "pub struct LightingStats {}\n").expect("stats fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-DIRECTIONAL-SHADOW",
        "src/render/prepare/stats.rs",
        &[
            "pub(in crate::render) fn collect_lighting_stats(",
            "PrepareError::MultipleShadowedDirectionalLights",
        ],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-DIRECTIONAL-SHADOW"
                && finding
                    .message
                    .contains("MultipleShadowedDirectionalLights")
        }),
        "doctor must reject prepare/stats.rs that drops the directional-shadow \
         multiple-lights error: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_m3a_scene_import_missing_dependencies_regression() {
    // ARCH-M3A-SCENE-IMPORT: Cargo.toml must keep the base64/serde_json/wasm-bindgen
    // -futures/Response/obj feature-flag dependencies that the M3a scene importer
    // relies on. A stub Cargo.toml without them regresses the contract.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/m3a-scene-import-stub");
    let cargo_path = fixture_root.join("Cargo.toml");
    fs::create_dir_all(cargo_path.parent().expect("cargo parent")).expect("fixture dir");
    fs::write(
        &cargo_path,
        "[package]\nname = \"scena\"\nversion = \"0.0.0\"\n",
    )
    .expect("cargo fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-M3A-SCENE-IMPORT",
        "Cargo.toml",
        &[
            "base64",
            "serde_json",
            "wasm-bindgen-futures",
            "Response",
            "obj = []",
        ],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-M3A-SCENE-IMPORT" && finding.message.contains("base64")
        }),
        "doctor must reject Cargo.toml that drops the M3a scene-import dependency \
         contract: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_stale_mikktspace_dependency_regression() {
    // ARCH-TANGENT-DEPENDENCY: generated normal-map tangents must use
    // the maintained Rust-native bevy_mikktspace crate rather than the
    // stale mikktspace default feature set that pulls nalgebra 0.26.x.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/stale-mikktspace-dep");
    fs::create_dir_all(&fixture_root).expect("fixture dir");
    fs::write(
        fixture_root.join("Cargo.toml"),
        "[package]\nname = \"scena\"\nversion = \"0.0.0\"\n\n[dependencies]\n\
         mikktspace = \"0.3\"\nnalgebra = \"0.26\"\n",
    )
    .expect("cargo fixture");
    fs::write(
        fixture_root.join("Cargo.lock"),
        "[[package]]\nname = \"mikktspace\"\nversion = \"0.3.0\"\n\n\
         [[package]]\nname = \"nalgebra\"\nversion = \"0.26.2\"\n",
    )
    .expect("lock fixture");
    let mut findings = Vec::new();

    check_tangent_generation_dependency_contracts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-TANGENT-DEPENDENCY" && finding.message.contains("mikktspace")
        }),
        "doctor must reject stale mikktspace / nalgebra 0.26 tangent dependencies: \
         {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_assets_m8_missing_texture_role_imports_regression() {
    // ASSETS-M8: src/assets/gltf/read.rs must parse all five glTF material texture
    // roles plus their KHR_texture_transform variants. A stub that drops them
    // regresses the contract.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/assets-m8-stub");
    let read_path = fixture_root.join("src/assets/gltf/read.rs");
    fs::create_dir_all(read_path.parent().expect("read parent")).expect("fixture dir");
    fs::write(&read_path, "pub fn read_baseColorTexture() {}\n").expect("read fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ASSETS-M8",
        "src/assets/gltf/read.rs",
        &[
            "normalTexture",
            "metallicRoughnessTexture",
            "occlusionTexture",
            "emissiveTexture",
            "with_normal_texture_transform",
        ],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ASSETS-M8" && finding.message.contains("normalTexture")
        }),
        "doctor must reject assets/gltf/read.rs that drops the five glTF texture \
         role imports: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_solid_catch_all_type_regression() {
    // ARCH-SOLID-CATCH-ALL: source modules must not declare catch-all types like
    // Manager, Engine, World, or broad Context. A stub that names one regresses
    // the contract.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/solid-catch-all-stub");
    let scope_path = fixture_root.join("src/scope.rs");
    fs::create_dir_all(scope_path.parent().expect("scope parent")).expect("fixture dir");
    // Use a simple needle the rule will reject; the rule's source scan in
    // check_solid_kiss looks for Manager/Engine/World/broad Context names.
    fs::write(
        &scope_path,
        "pub struct GlobalManager {}\npub struct WorldEngine {}\n",
    )
    .expect("scope fixture");
    let mut findings = Vec::new();

    forbid_contains(
        &fixture_root,
        &mut findings,
        "ARCH-SOLID-CATCH-ALL",
        "src/scope.rs",
        &["GlobalManager"],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-SOLID-CATCH-ALL" && finding.message.contains("GlobalManager")
        }),
        "doctor must reject source modules that name catch-all Manager/Engine \
         types: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_restricted_visibility_catch_all_type_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/restricted-catch-all-stub");
    let scope_path = fixture_root.join("src/render/scope.rs");
    fs::create_dir_all(scope_path.parent().expect("scope parent")).expect("fixture dir");
    fs::write(
        &scope_path,
        "pub(crate) struct GlobalManager {}\npub(super) struct RenderEngine {}\npub(in crate::render) struct HiddenManager {}\n",
    )
    .expect("scope fixture");
    let mut findings = Vec::new();

    check_solid_kiss(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-SOLID-CATCH-ALL" && finding.message.contains("GlobalManager")
        }) && findings.iter().any(|finding| {
            finding.rule == "ARCH-SOLID-CATCH-ALL" && finding.message.contains("RenderEngine")
        }) && findings.iter().any(|finding| {
            finding.rule == "ARCH-SOLID-CATCH-ALL" && finding.message.contains("HiddenManager")
        }),
        "doctor must reject catch-all names behind restricted Rust visibility: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_visual_fixture_metadata_missing_suite_regression() {
    // VISUAL-FIXTURE-METADATA: tests/visual/fixtures/m1-headless-core.toml must
    // declare the [suite] block with the name/format/encoding contract so the
    // doctor can compare each rendered fixture against it.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/visual-fixture-metadata-stub");
    let toml_path = fixture_root.join("tests/visual/fixtures/m1-headless-core.toml");
    fs::create_dir_all(toml_path.parent().expect("toml parent")).expect("fixture dir");
    fs::write(&toml_path, "# placeholder fixture metadata\n").expect("toml fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "VISUAL-FIXTURE-METADATA",
        "tests/visual/fixtures/m1-headless-core.toml",
        &[
            "[suite]",
            "name = \"m1-headless-core\"",
            "format = \"ppm\"",
            "encoding = \"srgb8\"",
        ],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "VISUAL-FIXTURE-METADATA" && finding.message.contains("[suite]")
        }),
        "doctor must reject m1 visual fixture TOML missing the suite contract: \
         {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_release_ci_missing_lane_regression() {
    // RELEASE-CI-M9: ci.yml must list every release lane name. A workflow that
    // drops e.g. macos-metal regresses the contract that release-readiness can
    // expect lane artifacts on every push.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/release-ci-missing-lane");
    let workflow_path = fixture_root.join(".github/workflows/ci.yml");
    fs::create_dir_all(workflow_path.parent().expect("workflow parent")).expect("fixture dir");
    fs::write(
        &workflow_path,
        "jobs:\n  linux-native-vulkan:\n    runs-on: ubuntu-24.04\n",
    )
    .expect("workflow fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "RELEASE-CI-M9",
        ".github/workflows/ci.yml",
        &[
            "linux-native-vulkan",
            "linux-browser-webgl2",
            "macos-metal",
            "windows-dx12",
        ],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RELEASE-CI-M9" && finding.message.contains("macos-metal")
        }),
        "doctor must reject CI workflows that drop a required release lane: \
         {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_ergonomics_m7_missing_controls_contract_regression() {
    // ERGONOMICS-M7: src/controls.rs must expose the orbit-controls contract terms
    // (with_damping, focus, apply_to_scene, damping_factor, TouchEvent, wheel) so
    // controls keep the ergonomic shape examples expect.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/ergonomics-m7-stub");
    let controls_path = fixture_root.join("src/controls.rs");
    fs::create_dir_all(controls_path.parent().expect("controls parent")).expect("fixture dir");
    fs::write(&controls_path, "pub struct OrbitControls {}\n").expect("controls fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ERGONOMICS-M7",
        "src/controls.rs",
        &[
            "with_damping",
            "focus",
            "apply_to_scene",
            "damping_factor",
            "TouchEvent",
            "pub const fn wheel",
        ],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ERGONOMICS-M7" && finding.message.contains("apply_to_scene")
        }),
        "doctor must reject controls.rs that drops the orbit-controls ergonomic \
         contract: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_assets_m8_missing_color_space_regression() {
    // ASSETS-M8 (color space): src/assets/gltf/read.rs must mention both linear and
    // sRGB texture color spaces so glTF imports tag every material texture's color
    // pipeline correctly.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/assets-m8-color-space-stub");
    let read_path = fixture_root.join("src/assets/gltf/read.rs");
    fs::create_dir_all(read_path.parent().expect("read parent")).expect("fixture dir");
    fs::write(
        &read_path,
        "pub fn baseColorTexture() {}\npub fn normalTexture() {}\n\
         pub fn metallicRoughnessTexture() {}\npub fn occlusionTexture() {}\n\
         pub fn emissiveTexture() {}\npub fn with_normal_texture_transform() {}\n\
         pub fn with_metallic_roughness_texture_transform() {}\n\
         pub fn with_occlusion_texture_transform() {}\n\
         pub fn with_emissive_texture_transform() {}\n",
    )
    .expect("read fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ASSETS-M8",
        "src/assets/gltf/read.rs",
        &["TextureColorSpace::Linear", "TextureColorSpace::Srgb"],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ASSETS-M8" && finding.message.contains("TextureColorSpace")
        }),
        "doctor must reject assets/gltf/read.rs that drops the texture color-space \
         contract: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_asset_api_missing_color_space_parameter_regression() {
    // ARCH-ASSET-API: src/assets.rs must keep the explicit
    // load_texture(color_space: TextureColorSpace) signature so callers cannot
    // accidentally load a texture into the wrong color pipeline.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/asset-api-stub");
    let assets_path = fixture_root.join("src/assets.rs");
    fs::create_dir_all(assets_path.parent().expect("assets parent")).expect("fixture dir");
    fs::write(
        &assets_path,
        "pub struct Assets {}\nimpl Assets { pub async fn load_texture(&self) {} }\n",
    )
    .expect("assets fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-ASSET-API",
        "src/assets.rs",
        &[
            "pub async fn load_texture",
            "color_space: TextureColorSpace",
            "Result<TextureHandle, AssetError>",
        ],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-ASSET-API" && finding.message.contains("color_space")
        }),
        "doctor must reject assets.rs that drops the explicit color_space parameter: \
         {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_prepare_assets_missing_collect_call_regression() {
    // ARCH-PREPARE-ASSETS: src/render.rs must route prepare_with_assets through
    // prepare::collect_prepared_primitives so the prepare phase stays the single
    // place that owns asset-aware primitive collection.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/prepare-assets-stub");
    let render_path = fixture_root.join("src/render.rs");
    fs::create_dir_all(render_path.parent().expect("render parent")).expect("fixture dir");
    fs::write(
        &render_path,
        "pub struct Renderer {}\nimpl Renderer { pub fn prepare_with_assets(&self) {} }\n",
    )
    .expect("render fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-PREPARE-ASSETS",
        "src/render.rs",
        &[
            "pub fn prepare_with_assets",
            "prepare::collect_prepared_primitives",
        ],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-PREPARE-ASSETS"
                && finding.message.contains("collect_prepared_primitives")
        }),
        "doctor must reject render.rs that drops the prepare::collect_prepared_primitives \
         routing: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_visual_browser_m6_missing_probe_exports_regression() {
    // VISUAL-BROWSER-M6: src/browser_probe.rs must expose the wasm_bindgen probe
    // entry points (m6Render*Probe) plus the Renderer::from_surface_async +
    // prepare_with_assets + Renderer::render shape that distinguishes Rust/WASM
    // probe proof from JavaScript-only smoke tests.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/visual-browser-m6-stub");
    let probe_path = fixture_root.join("src/browser_probe.rs");
    fs::create_dir_all(probe_path.parent().expect("browser probe parent")).expect("fixture dir");
    fs::write(
        &probe_path,
        "//! Stub browser probe.\npub fn m6_passthrough() {}\n",
    )
    .expect("browser probe fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "VISUAL-BROWSER-M6",
        "src/browser_probe.rs",
        &[
            "m6RenderWebgl2Probe",
            "m6RenderWebgpuProbe",
            "m6RenderWorkflowProbe",
            "Renderer::from_surface_async",
            "scena.m6.browser_renderer_probe.v1",
        ],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "VISUAL-BROWSER-M6" && finding.message.contains("from_surface_async")
        }),
        "doctor must reject browser_probe.rs that drops the Rust/WASM Renderer \
         attached-canvas contract: {findings:?}",
    );
}
