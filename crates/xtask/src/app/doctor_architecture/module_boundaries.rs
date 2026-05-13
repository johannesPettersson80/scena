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
        "src/render/gpu/webgl2.rs",
        &[
            "pub(super) fn render_canvas",
            "pub(super) fn prepare_canvas_vertices",
            "webgl2 resources were not prepared; call Renderer::prepare before render",
            "webgl2 vertex stream was not prepared; call Renderer::prepare after scene changes",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-LIFECYCLE",
        "src/render/gpu.rs",
        &["webgl2::prepare_canvas_vertices(", "material_slots"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-LIFECYCLE",
        "src/render/gpu/lifecycle.rs",
        &[
            "pub(in crate::render) fn clear_prepared_resources_for_context_recovery",
            "self.webgl2_render_cache = None;",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-LIFECYCLE",
        "src/render/gpu/webgl2.rs",
        &[
            "pub(super) struct WebGl2RenderCache",
            "cache: &mut Option<WebGl2RenderCache>",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-LIFECYCLE",
        "src/render/surface.rs",
        &["gpu.clear_prepared_resources_for_context_recovery();"],
    );
}
