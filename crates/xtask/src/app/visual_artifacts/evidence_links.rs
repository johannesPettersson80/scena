use crate::app::prelude::*;
pub(crate) fn evidence_links_for_category(category: &str) -> Vec<&'static str> {
    match category {
        "public-api" => vec![
            "docs/specs/public-api.md",
            "tests/m0_foundation.rs",
            "tests/m5_release.rs",
        ],
        "assets-gltf" => vec![
            "docs/specs/asset-gltf-contract.md",
            "tests/m3a_app_features.rs",
            "tests/m3b_gltf_animation.rs",
            "tests/m8_assets_materials_ecosystem.rs",
        ],
        "browser-platform" => vec![
            "docs/checklists/m6-browser-renderer-parity.md",
            "tests/m6_browser_renderer_parity.rs",
            "tests/browser/m6_rust_wasm_renderer_probe.js",
        ],
        "native-platform" => vec![
            "docs/specs/platform-capabilities.md",
            "docs/decisions/ADR-0005-local-release-candidate-deferrals.md",
            ".github/workflows/ci.yml",
            ".github/workflows/release.yml",
        ],
        "visual-proof" => vec![
            "docs/specs/visual-quality-contract.md",
            "tests/m1_visual_proof.rs",
            "tests/m2_visual_proof.rs",
            "tests/m3a_visual_proof.rs",
            "tests/m3b_visual_proof.rs",
        ],
        "performance" => vec![
            "docs/specs/release-gates.md",
            "tests/m4_performance_platform.rs",
            "tests/m5_release.rs",
        ],
        "doctor" => vec!["docs/specs/doctor-contract.md", "crates/xtask/src/main.rs"],
        "scope-non-goal" => vec![
            "docs/decisions/ADR-0001-renderer-not-engine.md",
            "docs/specs/module-boundaries.md",
            "AGENTS.md",
        ],
        "render-lifecycle" => vec![
            "docs/specs/render-lifecycle.md",
            "docs/decisions/ADR-0002-explicit-prepare-lifecycle.md",
            "tests/m0_foundation.rs",
        ],
        _ => Vec::new(),
    }
}
