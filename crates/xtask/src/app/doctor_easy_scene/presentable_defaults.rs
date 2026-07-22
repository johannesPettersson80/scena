use crate::app::prelude::*;

pub(crate) fn check_c06_presentable_viewer_defaults(root: &Path, findings: &mut Vec<Finding>) {
    let rule = "C06-PRESENTABLE-VIEWER-DEFAULTS";
    require_contains(
        root,
        findings,
        rule,
        "src/viewer.rs",
        &[
            "background: Some(Background::Studio)",
            "fallback_lighting: true",
            "without_default_lighting",
            "combined_viewer_diagnostics",
        ],
    );
    require_contains(
        root,
        findings,
        rule,
        "src/viewer/load_progress.rs",
        &[
            "scene.light_nodes().next().is_none()",
            "with_applied_fallback(\"viewer.lighting\")",
            "automatic_viewer_lighting_preserves_an_authored_light",
        ],
    );
    require_contains(
        root,
        findings,
        rule,
        "src/diagnostics/diagnostic.rs",
        &["pub setting: Option<String>", "pub fallback_applied: bool"],
    );
    require_contains(
        root,
        findings,
        rule,
        "tests/first_render_api.rs",
        &[
            "headless_gltf_viewer_defaults_make_pbr_assets_visible_and_explain_the_fallback",
            "headless_gltf_viewer_allows_an_explicit_diagnostic_lighting_opt_out",
            "DiagnosticCode::MissingLightingOrEnvironment",
            "Background::Black",
            "Pixel darkness is not the opt-out contract",
            "fallback_applied()",
        ],
    );
    require_contains(
        root,
        findings,
        rule,
        "tests/scena_cli_recipe.rs",
        &[
            "scena_render_cli_defaults_produce_visible_pbr_content",
            "cad_terminal_block.gltf",
            "visible_pixel_fraction",
        ],
    );
    require_contains(
        root,
        findings,
        rule,
        "examples/glb_model_viewer.rs",
        &["cad_terminal_block.gltf", "first.diagnostics()"],
    );
    require_contains(
        root,
        findings,
        rule,
        "tests/examples_visual_proof.rs",
        &["representative PBR CAD fixture", "distinct_rgb.len() > 8"],
    );
    for relative in [
        "README.md",
        "docs/getting-started.md",
        "docs/guides/easy-scene-setup.md",
        "docs/rendering.md",
        "docs/examples.md",
        "docs/errors.md",
        "docs/troubleshooting.md",
        "docs/api.md",
    ] {
        require_contains(root, findings, rule, relative, &["fallback", "diagnostic"]);
    }
    require_contains(
        root,
        findings,
        rule,
        "CHANGELOG.md",
        &["visible neutral", "fallback_applied"],
    );
    require_contains(
        root,
        findings,
        rule,
        "docs/release-notes/v1.8.0.md",
        &[
            "Post-release errata",
            "successful black",
            "not part of v1.8.0",
        ],
    );
}
