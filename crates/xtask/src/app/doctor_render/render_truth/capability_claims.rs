use crate::app::prelude::*;

pub(crate) fn check_renderer_truth_capability_claim_contracts(
    root: &Path,
    findings: &mut Vec<Finding>,
) {
    forbid_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/diagnostics/capabilities.rs",
        &[
            "forward_pbr_status(_backend: Backend) -> CapabilityStatus {\n    CapabilityStatus::Supported",
            "directional_shadow_status(_backend: Backend) -> CapabilityStatus {\n    CapabilityStatus::Supported",
            "punctual_shadow_status(_backend: Backend) -> CapabilityStatus {\n    CapabilityStatus::Supported",
            "gpu_frustum_culling_status(backend: Backend) -> CapabilityStatus {\n    match backend {\n        Backend::Headless\n        | Backend::HeadlessGpu\n        | Backend::SurfaceDescriptor\n        | Backend::NativeSurface\n        | Backend::WebGpu\n        | Backend::WebGl2 => CapabilityStatus::Supported",
        ],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "tests/browser/m4_platform_smoke.html",
        &["forward_pbr: { state: \"Supported\" }"],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "tests/browser/m4_platform_smoke.html",
        &["directional_shadows: { state: \"Supported\" }"],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "tests/browser/m4_platform_smoke.html",
        &["point_shadows: { state: \"Supported\" }"],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "tests/browser/m4_platform_smoke.html",
        &["spot_shadows: { state: \"Supported\" }"],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "examples/glb_model_viewer.rs",
        &["minimal_scene.gltf"],
    );
    check_renderer_standard_math_contracts(root, findings);
}
