use crate::app::prelude::*;

pub(crate) fn check_viewer_headless_png(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "VIEWER-HEADLESS-PNG",
        "src/viewer.rs",
        &["mod capture;", "ViewerPngError"],
    );
    require_contains(
        root,
        findings,
        "VIEWER-HEADLESS-PNG",
        "src/viewer/capture.rs",
        &[
            "pub enum ViewerPngError",
            "pub async fn render_png_bytes(",
            "pub async fn render_png(",
            "CPU headless renderer",
            "does not request a GPU adapter",
        ],
    );
    require_contains(
        root,
        findings,
        "VIEWER-HEADLESS-PNG",
        "src/lib.rs",
        &["ViewerPngError"],
    );
    require_contains(
        root,
        findings,
        "VIEWER-HEADLESS-PNG",
        "tests/round_d_viewer_capture_png.rs",
        &[
            "headless_viewer_builder_renders_gltf_to_png_bytes_without_gpu_setup",
            "headless_viewer_builder_renders_gltf_to_png_file_without_gpu_setup",
            ".render_png_bytes()",
            ".render_png(",
            "visible CPU-rendered pixels",
        ],
    );
    require_contains(
        root,
        findings,
        "VIEWER-HEADLESS-PNG",
        "docs/guides/easy-scene-setup.md",
        &[
            "render_png_bytes()",
            "CPU headless renderer",
            "without requesting a GPU adapter",
        ],
    );
    require_contains(
        root,
        findings,
        "VIEWER-HEADLESS-PNG",
        "docs/checklists/next-release-easy-use-and-state-of-the-art.md",
        &[
            "CPU rasterizer fallback for no-GPU screenshots",
            "Status: **[shipped]**",
            "render_png_bytes()",
        ],
    );
}
