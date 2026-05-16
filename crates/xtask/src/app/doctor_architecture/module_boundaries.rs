use crate::app::prelude::*;

pub(crate) fn check_module_boundaries(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-MODULES",
        "docs/specs/module-boundaries.md",
        &[
            "`scene`",
            "`assets`",
            "`geometry`",
            "`material`",
            "`render`",
            "`animation`",
            "`controls`",
            "`picking`",
            "`diagnostics`",
            "`platform`",
            "No hidden asset fetch, shader compile, or first-time GPU upload inside `render()`",
            "Host-owned convenience facade exceptions",
            "`HeadlessGltfViewer` and `InteractiveGltfViewer` are the v1.0 host-owned convenience",
            "Large module allowlist",
            "`src/assets.rs`",
            "`src/viewer.rs`",
        ],
    );

    forbid_contains(
        root,
        findings,
        "ARCH-PLATFORM",
        "src/platform.rs",
        &["wgpu::", "ForwardPass", "ShadowPass", "PostProcessPass"],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-ASSETS",
        "src/assets.rs",
        &["wgpu::", "RenderPass", "Surface"],
    );
    check_render_asset_loading_contracts(root, findings);
    forbid_contains_required_path(
        root,
        findings,
        "ARCH-RENDER-LIFECYCLE",
        Path::new("src/render/gpu/draw.rs"),
        &[
            "create_shader_module",
            "create_render_pipeline",
            "create_buffer",
            "create_texture",
            "create_bind_group",
            "request_adapter",
            "request_device",
            "mapped_at_creation: true",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-LIFECYCLE",
        "src/render/gpu/draw.rs",
        &[
            "pub(in crate::render) fn render_to_surface",
            "GpuResourcesNotPrepared",
            "surface.surface.get_current_texture()",
            "encode_unlit_pass",
            "surface_output.present();",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-LIFECYCLE",
        "src/render/gpu.rs",
        &[
            "self.configure_surface(target);",
            "self.release_prepared_resources();",
            "let vertex_bytes = encode_vertices(primitives);",
            "create_material_resources",
            "material_slots",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-LIFECYCLE",
        "src/render/gpu/lifecycle.rs",
        &["pub(in crate::render) fn clear_prepared_resources_for_context_recovery"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-LIFECYCLE",
        "src/render/surface.rs",
        &["gpu.clear_prepared_resources_for_context_recovery();"],
    );
}
