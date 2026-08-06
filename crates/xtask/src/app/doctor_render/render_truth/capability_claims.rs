use crate::app::prelude::*;

pub(crate) fn check_renderer_truth_capability_claim_contracts(
    root: &Path,
    findings: &mut Vec<Finding>,
) {
    forbid_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/diagnostics/capability_status.rs",
        &[
            "forward_pbr_status(_backend: Backend) -> CapabilityStatus {\n    CapabilityStatus::Supported",
            "forward_pbr_status(\n    backend: Backend,\n    gpu_device: bool,\n) -> CapabilityStatus {\n    CapabilityStatus::Supported",
            "physical_glass_transmission_status(\n    backend: Backend,\n    gpu_device: bool,\n) -> CapabilityStatus {\n    CapabilityStatus::Supported",
            "directional_shadow_status(_backend: Backend) -> CapabilityStatus {\n    CapabilityStatus::Supported",
            "directional_shadow_status(\n    backend: Backend,\n) -> CapabilityStatus {\n    CapabilityStatus::Supported",
            "directional_shadow_status(\n    backend: Backend,\n    gpu_device: bool,\n) -> CapabilityStatus {\n    CapabilityStatus::Supported",
            "punctual_shadow_status(_backend: Backend) -> CapabilityStatus {\n    CapabilityStatus::Supported",
            "gpu_frustum_culling_status(backend: Backend) -> CapabilityStatus {\n    match backend {\n        Backend::Headless\n        | Backend::HeadlessGpu\n        | Backend::SurfaceDescriptor\n        | Backend::NativeSurface\n        | Backend::WebGpu\n        | Backend::WebGl2 => CapabilityStatus::Supported",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/diagnostics/capability_status.rs",
        &[
            "forward_pbr_status(\n    backend: Backend,\n    gpu_device: bool,",
            "physical_glass_transmission_status(\n    backend: Backend,\n    gpu_device: bool,",
            "Backend::HeadlessGpu | Backend::NativeSurface | Backend::WebGpu | Backend::WebGl2",
            "false,\n        )\n        | (Backend::Headless | Backend::SurfaceDescriptor, true) => CapabilityStatus::Degraded",
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
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/diagnostics/capabilities/sample_counts.rs",
        &[
            "pub(super) const fn measured_sample_counts(maximum: u32)",
            "if maximum >= 4 { 4 } else { 0 }",
            "if maximum >= 8 { 8 } else { 0 }",
        ],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/diagnostics/capabilities/sample_counts.rs",
        &["Backend::HeadlessGpu | Backend::NativeSurface => [1, 4, 8]"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu.rs",
        &[
            "fn measured_sample_count_maxima",
            "post::scene_color_format()",
            "wgpu::TextureFormat::Depth32Float",
        ],
    );
    for path in ["src/render/build.rs", "src/render/surface.rs"] {
        require_contains(
            root,
            findings,
            "ARCH-RENDER-TRUTH",
            path,
            &["with_measured_sample_count_maxima"],
        );
    }
    check_renderer_standard_math_contracts(root, findings);
}
