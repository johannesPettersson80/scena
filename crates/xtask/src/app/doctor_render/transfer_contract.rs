use crate::app::prelude::*;

pub(crate) fn check_c07_target_transfer_contract(root: &Path, findings: &mut Vec<Finding>) {
    let rule = "C07-TARGET-DRIVEN-COLOR-TRANSFER";
    require_contains(
        root,
        findings,
        rule,
        "src/render/gpu/draw_common.rs",
        &[
            "target_color_management_uniform",
            "shader_encodes_srgb_for_target",
            "TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Bgra8Unorm",
            "no_post_unorm_readback_encodes_known_linear_value_as_srgb8",
            "target_format_selects_exactly_one_srgb_transfer",
        ],
    );
    forbid_contains(
        root,
        findings,
        rule,
        "src/render/gpu/draw_common.rs",
        &["post_color_management_uniform"],
    );
    require_contains(
        root,
        findings,
        rule,
        "src/render/gpu/post/resources.rs",
        &[
            "POST_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb",
            "scene_linear_sampling",
        ],
    );
    require_contains(
        root,
        findings,
        rule,
        "src/render/gpu/browser_readback.rs",
        &[
            "readback_format_for_surface",
            "browser_readback_preserves_surface_transfer_with_rgba_byte_order",
        ],
    );
    require_contains(
        root,
        findings,
        rule,
        "src/render/post_tests.rs",
        &[
            "gpu_post_toggle_preserves_srgb8_transfer",
            "Color::from_linear_rgb(0.18, 0.18, 0.18)",
            "let expected = [118_u8, 118, 118, 255];",
        ],
    );
    require_contains(
        root,
        findings,
        rule,
        "src/browser_probe/workflows.rs",
        &[
            "color-transfer-no-post",
            "color-transfer-post",
            "expected_center_srgb8",
        ],
    );
    require_contains(
        root,
        findings,
        rule,
        "tests/browser/m6_rust_wasm_renderer_probe.js",
        &[
            "assertColorTransferProof",
            "assertPostTogglePreservesColorTransfer",
            "output regressed to linear byte",
        ],
    );
    require_contains(
        root,
        findings,
        rule,
        "src/diagnostics/capabilities.rs",
        &["with_color_target_format"],
    );
    require_contains(
        root,
        findings,
        rule,
        "src/diagnostics/capabilities/color_formats.rs",
        &["Rgba8Unorm", "Bgra8Unorm"],
    );
    for path in [
        "docs/specs/color-contract.md",
        "docs/browser.md",
        "docs/rendering.md",
        "docs/capabilities.md",
        "CHANGELOG.md",
        "docs/release-notes/v1.8.0.md",
    ] {
        require_contains(root, findings, rule, path, &["sRGB"]);
    }
    require_contains(
        root,
        findings,
        rule,
        "docs/capabilities.md",
        &["Post-processing changes neither the reported attachment format"],
    );
}
