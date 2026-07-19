use crate::app::prelude::*;

pub(crate) fn check_c13_strict_gpu_construction_contracts(
    root: &Path,
    findings: &mut Vec<Finding>,
) {
    const RULE: &str = "SCENE-C13";
    let required: &[(&str, &[&str])] = &[
        (
            "src/scene_host/construction.rs",
            &[
                "pub const fn backend_selection_report",
                "pub fn headless_gpu_with_fetcher(",
                "headless_gpu_with_fetcher_using(fetcher, width, height, Renderer::headless_gpu)",
                "let renderer = build_gpu(width, height)?;",
                "pub fn headless_prefer_gpu_with_fetcher(",
                "HeadlessBackendSelectionReport::cpu_fallback(gpu_error)",
            ],
        ),
        (
            "src/render/backend_selection.rs",
            &[
                "pub struct HeadlessBackendSelectionReport",
                "requested_backend: Backend",
                "selected_backend: Backend",
                "gpu_error: Option<BuildError>",
                "pub const fn fallback_used",
            ],
        ),
        (
            "src/viewer.rs",
            &[
                "enum HeadlessBackendPolicy",
                "StrictGpu",
                "PreferGpu",
                "pub const fn with_headless_gpu",
                "pub const fn with_headless_prefer_gpu",
                "pub const fn backend_selection_report",
            ],
        ),
        (
            "src/viewer/load_progress.rs",
            &[
                "HeadlessBackendPolicy::StrictGpu",
                "Renderer::headless_gpu_with_options(",
                "HeadlessBackendPolicy::PreferGpu",
                "HeadlessBackendSelectionReport::cpu_fallback(",
            ],
        ),
        (
            "src/scene_host/recipe/host.rs",
            &[
                "pub(super) enum RecipeBackendPolicy",
                "StrictGpu",
                "PreferGpu",
                "HeadlessBackendSelectionReport::cpu_fallback(",
            ],
        ),
        (
            "src/scene_host/recipe.rs",
            &[
                "pub const fn backend_selection_report(",
                "self.host.backend_selection_report()",
            ],
        ),
        (
            "src/scene_host/recipe/backend.rs",
            &[
                "pub async fn build_recipe_json_gpu(",
                "RecipeBackendPolicy::StrictGpu",
                "pub async fn build_recipe_json_prefer_gpu(",
            ],
        ),
        (
            "src/bin/scena/input.rs",
            &["build_recipe_json_gpu(", "builder.with_headless_gpu()"],
        ),
        ("src/bin/scena/recipe.rs", &["build_recipe_json_gpu("]),
        (
            "src/bin/scena/recipe/quality/depth_of_field.rs",
            &["build_recipe_json_gpu("],
        ),
        (
            "src/scene_host/core_tests.rs",
            &[
                "c13_strict_headless_gpu_with_fetcher_propagates_injected_gpu_failure",
                "c13_prefer_gpu_reports_injected_cpu_fallback",
                "c13_public_strict_constructor_never_returns_cpu_backend",
            ],
        ),
        (
            "examples/scene_host_contracts.rs",
            &[
                "SceneHostCore::headless_gpu(128, 128)?",
                "SceneHostCore::headless_prefer_gpu(128, 128)?",
                "selection.fallback_used()",
            ],
        ),
        (
            "docs/capabilities.md",
            &[
                "## Backend selection is separate evidence",
                "HeadlessBackendSelectionReport",
                "Release and visual-proof lanes must use strict construction",
            ],
        ),
        (
            ".github/workflows/ci.yml",
            &["cargo test --lib --features scene-host scene_host::core_tests::c13_"],
        ),
        (
            ".github/workflows/release.yml",
            &["cargo test --lib --features scene-host scene_host::core_tests::c13_"],
        ),
    ];
    for (relative, needles) in required {
        require_contains(root, findings, RULE, relative, needles);
    }

    for relative in [
        "src/scene_host/construction.rs",
        "src/viewer/load_progress.rs",
        "src/scene_host/recipe/host.rs",
    ] {
        forbid_contains(
            root,
            findings,
            RULE,
            relative,
            &["or_else(|_gpu_error|", "or_else(|gpu_error|"],
        );
    }
    forbid_contains(
        root,
        findings,
        RULE,
        "src/scene_host/recipe.rs",
        &["pub backend_selection_report:"],
    );
    for relative in [
        "src/bin/scena/input.rs",
        "src/bin/scena/recipe.rs",
        "src/bin/scena/recipe/quality/depth_of_field.rs",
    ] {
        forbid_contains(
            root,
            findings,
            RULE,
            relative,
            &["build_recipe_json_prefer_gpu("],
        );
    }
}
