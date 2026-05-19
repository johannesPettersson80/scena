use crate::app::prelude::*;

pub(super) fn check_viewer_load_progress(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "VIEWER-LOAD-PROGRESS",
        "src/viewer.rs",
        &[
            "mod load_progress;",
            "load_progress_events: Vec<AssetLoadProgress>",
        ],
    );
    require_contains(
        root,
        findings,
        "VIEWER-LOAD-PROGRESS",
        "src/viewer/load_progress.rs",
        &[
            "pub async fn build_with_progress<",
            "pub async fn render_with_progress<",
            "pub fn build_with_progress<",
            "pub async fn build_async_with_progress<",
            "pub fn load_progress_events(&self) -> &[AssetLoadProgress]",
        ],
    );
    require_contains(
        root,
        findings,
        "VIEWER-LOAD-PROGRESS",
        "src/lib.rs",
        &["AssetLoadProgress"],
    );
    require_contains(
        root,
        findings,
        "VIEWER-LOAD-PROGRESS",
        "tests/first_render_api.rs",
        &[
            "headless_gltf_viewer_surfaces_asset_load_progress",
            ".build_with_progress(|event| observed.push(event))",
            "viewer.load_progress_events()",
            "AssetLoadProgress::LoadStarted",
            "AssetLoadProgress::Parsed",
            "AssetLoadProgress::Cached",
        ],
    );
    require_contains(
        root,
        findings,
        "VIEWER-LOAD-PROGRESS",
        "tests/m7_interactive_viewer.rs",
        &[
            "interactive_gltf_viewer_surfaces_asset_load_progress",
            ".build_with_progress(|event| observed.push(event))",
            "viewer.load_progress_events()",
            "AssetLoadProgress::LoadStarted",
            "AssetLoadProgress::Parsed",
        ],
    );
    require_contains(
        root,
        findings,
        "VIEWER-LOAD-PROGRESS",
        "docs/guides/easy-scene-setup.md",
        &[
            "AssetLoadProgress",
            "build_with_progress",
            "load_progress_events",
        ],
    );
}
