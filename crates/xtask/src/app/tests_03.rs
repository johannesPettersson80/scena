use crate::app::prelude::*;

#[test]
pub(crate) fn doctor_rejects_depth_prepass_missing_counters_regression() {
    // ARCH-DEPTH-PREPASS: diagnostics.rs must expose the depth-prepass counter
    // contract so the doctor can prove the prepass actually executed.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/depth-prepass-stub");
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
        "ARCH-DEPTH-PREPASS",
        "src/diagnostics.rs",
        &[
            "pub depth_prepass_passes: u64",
            "pub depth_prepass_draws: u64",
        ],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-DEPTH-PREPASS" && finding.message.contains("depth_prepass_passes")
        }),
        "doctor must reject diagnostics.rs that drops the depth-prepass counter \
         contract: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_origin_shift_missing_field_regression() {
    // ARCH-ORIGIN-SHIFT: src/scene.rs must expose origin_shift as a Vec3 field so
    // large-scene precision shifts stay observable.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/origin-shift-stub");
    let scene_path = fixture_root.join("src/scene.rs");
    fs::create_dir_all(scene_path.parent().expect("scene parent")).expect("fixture dir");
    fs::write(&scene_path, "pub struct Scene { pub root: NodeKey }\n").expect("scene fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-ORIGIN-SHIFT",
        "src/scene.rs",
        &["origin_shift: Vec3"],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-ORIGIN-SHIFT" && finding.message.contains("origin_shift: Vec3")
        }),
        "doctor must reject scene.rs that drops the origin_shift Vec3 contract: \
         {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_environment_lifecycle_missing_revision_regression() {
    // ARCH-ENVIRONMENT-LIFECYCLE: src/render.rs must track the bound environment plus
    // its revision so reload/dirty propagation stays observable.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/environment-lifecycle-stub");
    let render_path = fixture_root.join("src/render.rs");
    fs::create_dir_all(render_path.parent().expect("render parent")).expect("fixture dir");
    fs::write(&render_path, "pub struct Renderer {}\n").expect("render fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-ENVIRONMENT-LIFECYCLE",
        "src/render.rs",
        &[
            "environment: Option<EnvironmentHandle>",
            "environment_revision: u64",
        ],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-ENVIRONMENT-LIFECYCLE"
                && finding.message.contains("environment_revision")
        }),
        "doctor must reject Renderer types that drop the environment revision \
         contract: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_scene_lights_missing_typed_key_regression() {
    // ARCH-SCENE-LIGHTS: src/scene.rs must expose the typed LightKey handle plus
    // the lights submodule so light entries do not become string lookups.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/scene-lights-stub");
    let scene_path = fixture_root.join("src/scene.rs");
    fs::create_dir_all(scene_path.parent().expect("scene parent")).expect("fixture dir");
    fs::write(&scene_path, "pub struct Scene { pub root: NodeKey }\n").expect("scene fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-SCENE-LIGHTS",
        "src/scene.rs",
        &["pub struct LightKey", "mod lights;"],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-SCENE-LIGHTS" && finding.message.contains("LightKey")
        }),
        "doctor must reject scene.rs that drops the typed light-key contract: \
         {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_shadow_map_missing_counter_regression() {
    // ARCH-SHADOW-MAP: diagnostics.rs must expose shadow_maps and the directional
    // shadow-map resolution metadata so missing shadow infrastructure stays visible.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/shadow-map-stub");
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
        "ARCH-SHADOW-MAP",
        "src/diagnostics.rs",
        &[
            "pub shadow_maps: u64",
            "pub directional_shadow_map_resolution: Option<u32>",
        ],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-SHADOW-MAP" && finding.message.contains("shadow_maps: u64")
        }),
        "doctor must reject diagnostics.rs that drops the shadow-map counter \
         contract: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_m3b_animation_missing_typed_keys_regression() {
    // ARCH-M3B-ANIMATION: src/animation.rs must expose the typed animation handle and
    // playback-state enums so animation lookups stay typed instead of stringly.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/m3b-animation-stub");
    let animation_path = fixture_root.join("src/animation.rs");
    fs::create_dir_all(animation_path.parent().expect("animation parent")).expect("fixture dir");
    fs::write(&animation_path, "pub struct AnimationMixer {}\n").expect("animation fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-M3B-ANIMATION",
        "src/animation.rs",
        &[
            "pub struct AnimationMixerKey",
            "pub enum AnimationPlaybackState",
            "pub enum AnimationLoopMode",
            "pub enum AnimationTarget",
        ],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-M3B-ANIMATION" && finding.message.contains("AnimationMixerKey")
        }),
        "doctor must reject animation.rs that drops the typed mixer-key contract: \
         {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_m4_platform_missing_dirty_state_regression() {
    // ARCH-M4-PLATFORM: src/scene/dirty.rs must expose SceneDirtyState plus the
    // transform_revision counter so dirty propagation stays observable to render-
    // on-change consumers.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/m4-platform-stub");
    let dirty_path = fixture_root.join("src/scene/dirty.rs");
    fs::create_dir_all(dirty_path.parent().expect("dirty parent")).expect("fixture dir");
    fs::write(&dirty_path, "pub struct DirtyState {}\n").expect("dirty fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-M4-PLATFORM",
        "src/scene/dirty.rs",
        &[
            "pub struct SceneDirtyState",
            "transform_revision",
            "pub fn dirty_state",
        ],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-M4-PLATFORM" && finding.message.contains("SceneDirtyState")
        }),
        "doctor must reject scene/dirty.rs that drops the SceneDirtyState contract: \
         {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_release_ci_silent_artifact_upload_regression() {
    // RELEASE-CI-M9: CI workflows must use `if-no-files-found: error` on artifact
    // upload so a silent missing-artifacts upload doesn't pretend the lane passed.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/release-ci-silent-upload");
    let workflow_path = fixture_root.join(".github/workflows/ci.yml");
    fs::create_dir_all(workflow_path.parent().expect("workflow parent")).expect("fixture dir");
    fs::write(
        &workflow_path,
        "jobs:\n  some-lane:\n    steps:\n      - uses: actions/upload-artifact@v4\n        with:\n          name: gate-artifacts\n          path: target/gate-artifacts/**\n          if-no-files-found: ignore\n",
    )
    .expect("workflow fixture");
    let mut findings = Vec::new();

    forbid_contains(
        &fixture_root,
        &mut findings,
        "RELEASE-CI-M9",
        ".github/workflows/ci.yml",
        &["if-no-files-found: ignore"],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RELEASE-CI-M9" && finding.message.contains("if-no-files-found: ignore")
        }),
        "doctor must reject CI workflows that silently ignore missing artifacts: \
         {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_fxaa_missing_pass_counter_regression() {
    // ARCH-FXAA-OUTPUT: diagnostics.rs must expose fxaa_passes: u64 so the FXAA
    // pass invocation count stays observable to release-readiness.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/fxaa-output-stub");
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
        "ARCH-FXAA-OUTPUT",
        "src/diagnostics.rs",
        &["pub fxaa_passes: u64"],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-FXAA-OUTPUT" && finding.message.contains("fxaa_passes: u64")
        }),
        "doctor must reject diagnostics.rs that drops the FXAA pass counter: \
         {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_reversed_z_missing_capability_field_regression() {
    // ARCH-REVERSED-Z: capabilities.rs must expose reversed_z_depth as a typed
    // CapabilityStatus and the const status helper that downgrades on WebGL2.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/reversed-z-stub");
    let capabilities_path = fixture_root.join("src/diagnostics/capabilities.rs");
    fs::create_dir_all(capabilities_path.parent().expect("capabilities parent"))
        .expect("fixture dir");
    fs::write(&capabilities_path, "pub struct Capabilities {}\n").expect("capabilities fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-REVERSED-Z",
        "src/diagnostics/capabilities.rs",
        &[
            "pub reversed_z_depth: CapabilityStatus",
            "const fn reversed_z_depth_status",
        ],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-REVERSED-Z" && finding.message.contains("reversed_z_depth")
        }),
        "doctor must reject capabilities.rs that drops the reversed_z_depth typed \
         status contract: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_backend_vocabulary_missing_browser_canvas_regression() {
    // ARCH-BACKEND-VOCAB: src/platform.rs must expose browser_webgpu_canvas /
    // browser_webgl2_canvas constructors so the descriptor and attached-canvas
    // backends share a stable vocabulary.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/backend-vocab-stub");
    let platform_path = fixture_root.join("src/platform.rs");
    fs::create_dir_all(platform_path.parent().expect("platform parent")).expect("fixture dir");
    fs::write(&platform_path, "pub struct PlatformSurface {}\n").expect("platform fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-BACKEND-VOCAB",
        "src/platform.rs",
        &["browser_webgpu_canvas", "browser_webgl2_canvas"],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-BACKEND-VOCAB"
                && finding.message.contains("browser_webgpu_canvas")
        }),
        "doctor must reject platform.rs that drops the browser canvas backend \
         vocabulary: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_webgl2_depth_missing_diagnostic_regression() {
    // ARCH-WEBGL2-DEPTH: capabilities.rs must emit the WebGL2 depth-compatibility
    // diagnostic so users see the reduced near/far precision warning.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/webgl2-depth-stub");
    let capabilities_path = fixture_root.join("src/diagnostics/capabilities.rs");
    fs::create_dir_all(capabilities_path.parent().expect("capabilities parent"))
        .expect("fixture dir");
    fs::write(
        &capabilities_path,
        "pub struct Capabilities { pub backend: Backend }\n",
    )
    .expect("capabilities fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-WEBGL2-DEPTH",
        "src/diagnostics/capabilities.rs",
        &[
            "pub fn diagnostics(self) -> Vec<Diagnostic>",
            "self.backend == Backend::WebGl2",
            "DiagnosticCode::WebGl2DepthCompatibility",
        ],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-WEBGL2-DEPTH"
                && finding.message.contains("WebGl2DepthCompatibility")
        }),
        "doctor must reject capabilities.rs that drops the WebGL2 depth diagnostic: \
         {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_solid_kiss_docs_missing_gate_regression() {
    // ARCH-SOLID-KISS-DOCS: docs/specs/module-boundaries.md must enumerate the
    // SOLID/KISS gate so the design rules stay anchored to a doc the doctor reads.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/solid-kiss-docs-stub");
    let module_path = fixture_root.join("docs/specs/module-boundaries.md");
    fs::create_dir_all(module_path.parent().expect("module boundaries parent"))
        .expect("fixture dir");
    fs::write(
        &module_path,
        "# Module Boundaries\n\nNo SOLID/KISS gate text here.\n",
    )
    .expect("module-boundaries fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-SOLID-KISS-DOCS",
        "docs/specs/module-boundaries.md",
        &[
            "## SOLID/KISS Gate",
            "Every public feature must name exactly one owner module",
            "No catch-all `Manager`, `Engine`, `World`, or broad `Context` type",
        ],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-SOLID-KISS-DOCS" && finding.message.contains("SOLID/KISS Gate")
        }),
        "doctor must reject module-boundaries.md that drops the SOLID/KISS gate \
         contract: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_direct_light_shading_missing_world_transform_iter_regression() {
    // ARCH-DIRECT-LIGHT-SHADING: scene.rs must expose the world-transform light
    // iterator so direct-light shading uses composed world transforms instead of
    // local node transforms.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/direct-light-shading-stub");
    let scene_path = fixture_root.join("src/scene.rs");
    fs::create_dir_all(scene_path.parent().expect("scene parent")).expect("fixture dir");
    fs::write(&scene_path, "pub struct Scene { pub root: NodeKey }\n").expect("scene fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-DIRECT-LIGHT-SHADING",
        "src/scene.rs",
        &[
            "impl Iterator<Item = (NodeKey, LightKey, Light, Transform)>",
            "self.world_transform(node_key)",
        ],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-DIRECT-LIGHT-SHADING"
                && finding.message.contains("world_transform")
        }),
        "doctor must reject scene.rs that drops the world-transform light iteration \
         contract: {findings:?}",
    );
}
