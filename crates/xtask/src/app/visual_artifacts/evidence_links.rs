use crate::app::prelude::*;
pub(crate) fn evidence_links_for_category(category: &str) -> Vec<&'static str> {
    match category {
        "public-api" => vec![
            "docs/api.md",
            "tests/m0_foundation.rs",
            "tests/m5_release.rs",
        ],
        "assets-gltf" => vec![
            "docs/assets.md",
            "tests/m3a_app_features.rs",
            "tests/m3b_gltf_animation.rs",
            "tests/m8_assets_materials_ecosystem.rs",
        ],
        "browser-platform" => vec![
            "docs/browser.md",
            "tests/m6_browser_renderer_parity.rs",
            "tests/browser/m6_rust_wasm_renderer_probe.js",
        ],
        "native-platform" => vec![
            "docs/platforms.md",
            ".github/workflows/ci.yml",
            ".github/workflows/release.yml",
        ],
        "visual-proof" => vec![
            "docs/headless-rendering.md",
            "tests/m1_visual_proof.rs",
            "tests/m2_visual_proof.rs",
            "tests/m3a_visual_proof.rs",
            "tests/m3b_visual_proof.rs",
        ],
        "performance" => vec![
            "docs/release-notes/v1.1.0.md",
            "tests/m4_performance_platform.rs",
            "tests/m5_release.rs",
        ],
        "doctor" => vec!["docs/README.md", "crates/xtask/src/main.rs"],
        "scope-non-goal" => vec!["README.md", "AGENTS.md"],
        "render-lifecycle" => vec!["docs/lifecycle.md", "tests/m0_foundation.rs"],
        _ => Vec::new(),
    }
}
