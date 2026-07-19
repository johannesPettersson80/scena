use crate::app::prelude::*;

const PUBLIC_DEPENDENCY_DOCS: &[&str] = &[
    "README.md",
    "docs/getting-started.md",
    "docs/feature-flags.md",
];

pub(crate) fn check_c11_onboarding_contracts(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "DOCS-C11-ONBOARDING";
    let workspace_version = package_version(root).unwrap_or_else(|| "unknown".to_owned());
    for relative in PUBLIC_DEPENDENCY_DOCS {
        let Ok(markdown) = fs::read_to_string(root.join(relative)) else {
            findings.push(Finding::new(RULE, format!("could not read {relative}")));
            continue;
        };
        if !markdown.contains("cargo add scena") {
            findings.push(Finding::new(
                RULE,
                format!(
                    "{relative} must use version-agnostic `cargo add scena` under workspace package policy {workspace_version}"
                ),
            ));
        }
        for (index, line) in markdown.lines().enumerate() {
            if numeric_scena_dependency(line) {
                findings.push(Finding::new(
                    RULE,
                    format!(
                        "{relative}:{} pins a numeric scena dependency while workspace version is {workspace_version}; use `cargo add scena`",
                        index + 1
                    ),
                ));
            }
        }
    }

    for relative in ["README.md", "docs/getting-started.md"] {
        let Ok(markdown) = fs::read_to_string(root.join(relative)) else {
            continue;
        };
        let mut rust_blocks = 0;
        for (index, line) in markdown.lines().enumerate() {
            if !line.trim_start().starts_with("```rust") {
                continue;
            }
            rust_blocks += 1;
            if line.trim() != "```rust,no_run" {
                findings.push(Finding::new(
                    RULE,
                    format!(
                        "{relative}:{} Rust onboarding block is not an explicitly compiled no-run doctest",
                        index + 1
                    ),
                ));
            }
        }
        if rust_blocks == 0 {
            findings.push(Finding::new(
                RULE,
                format!("{relative} must retain at least one compiled Rust onboarding block"),
            ));
        }
    }

    let required: &[(&str, &[&str])] = &[
        (
            "src/lib.rs",
            &[
                "#[cfg(doctest)]",
                "include_str!(\"../README.md\")",
                "include_str!(\"../docs/getting-started.md\")",
            ],
        ),
        (".github/workflows/ci.yml", &["cargo test --doc"]),
        (
            "docs/getting-started.md",
            &[
                "Scene::with_default_camera()?",
                "scene.frame_all_with_assets(camera, &assets)?",
                "pollster::block_on(assets.load_scene(path.as_str()))",
                "scene.frame_import(camera, &import)",
                "renderer.prepare_with_assets(&mut scene, &assets)?",
                "renderer.render_active(&scene)?",
                "capture.write_png(\"first-scene.png\")?",
                "capture.write_png(\"model.png\")?",
                "std::io::Error::other",
            ],
        ),
        (
            "docs/checklists/next-release-easy-use-and-state-of-the-art.md",
            &[
                "SSR, LTC area lights, and clustered/tiled culling subsequently shipped",
                "That history is now superseded",
            ],
        ),
    ];
    for (relative, needles) in required {
        require_contains(root, findings, RULE, relative, needles);
    }
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/c11_onboarding_docs.rs",
        &[
            "every_onboarding_rust_block_is_explicitly_compile_gated",
            "public_dependency_examples_follow_version_agnostic_policy",
            "getting_started_snippets_pin_visible_framed_capture_lifecycles",
            "onboarding_first_scene_renders_deterministic_nonblank_output",
            "onboarding_glb_scene_renders_deterministic_nonblank_output",
            "shipped_renderer_features_have_no_reverse_status_drift",
        ],
    );
}

fn package_version(root: &Path) -> Option<String> {
    let manifest = fs::read_to_string(root.join("Cargo.toml")).ok()?;
    let mut in_package = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
            continue;
        }
        if in_package && let Some(version) = line.strip_prefix("version = ") {
            return Some(version.trim_matches('"').to_owned());
        }
    }
    None
}

fn numeric_scena_dependency(line: &str) -> bool {
    let line = line.trim();
    line.starts_with("scena =")
        && line.contains('"')
        && line.chars().any(|character| character.is_ascii_digit())
        && !line.contains("path =")
}
