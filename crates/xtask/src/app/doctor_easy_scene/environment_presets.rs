use crate::app::prelude::*;

pub(super) fn check_environment_presets(root: &Path, findings: &mut Vec<Finding>) {
    check_c02_portable_agent_asset_contracts(root, findings);
    require_contains(
        root,
        findings,
        "ENVIRONMENT-PRESETS",
        "src/assets.rs",
        &[
            "mod environment_preset;",
            "EnvironmentPreset",
            "EnvironmentPresetMetadata",
        ],
    );
    require_contains(
        root,
        findings,
        "ENVIRONMENT-PRESETS",
        "src/assets/environment_preset.rs",
        &[
            "pub enum EnvironmentPreset",
            "NeutralStudio",
            "Studio",
            "PACKAGE_SIZE_BUDGET_BYTES",
            "load_environment_preset",
            "source_sha256",
            "source_url",
            "license",
        ],
    );
    require_contains(
        root,
        findings,
        "ENVIRONMENT-PRESETS",
        "src/lib.rs",
        &["EnvironmentPreset", "EnvironmentPresetMetadata"],
    );
    require_contains(
        root,
        findings,
        "ENVIRONMENT-PRESETS",
        "tests/round_c_environment_presets.rs",
        &[
            "environment_preset_catalog_exposes_metadata_and_package_budget",
            "environment_presets_load_without_user_supplied_paths",
            "environment_presets_render_reference_contact_sheet",
            "environment-preset-reference-docs-image.ppm",
        ],
    );
    require_contains(
        root,
        findings,
        "ENVIRONMENT-PRESETS",
        "docs/guides/easy-scene-setup.md",
        &[
            "EnvironmentPreset::Studio",
            "load_environment_preset",
            "EnvironmentPreset::ALL",
            "KTX2 cubemap presets are still future work",
        ],
    );
}

pub(crate) fn check_c02_portable_agent_asset_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "C02-PORTABLE-AGENT-ASSETS",
        "Cargo.toml",
        &[
            "/tests/assets/environment/PRESET-LICENSES.md",
            "/tests/assets/environment/polyhaven/**",
            "/tests/assets/gltf/AGENT-TEMPLATE-ASSETS-LICENSE.md",
            "/tests/assets/gltf/material_variants_scene.gltf",
            "/tests/assets/gltf/animated_triangle_scene.glb",
            "/tests/assets/gltf/cad_plate_drawing_scene.gltf",
        ],
    );
    require_contains(
        root,
        findings,
        "C02-PORTABLE-AGENT-ASSETS",
        "src/assets/builtin.rs",
        &[
            "scena://bundled/agent-template/material_variants_scene.gltf",
            "scena://bundled/agent-template/animated_triangle_scene.glb",
            "scena://bundled/agent-template/cad_plate_drawing_scene.gltf",
            "include_bytes!",
            "bundled_scene_bytes",
        ],
    );
    require_contains(
        root,
        findings,
        "C02-PORTABLE-AGENT-ASSETS",
        "src/assets/environment_preset.rs",
        &[
            "scena://bundled/environment/studio_small_03_128x64.hdr",
            "tests/assets/environment/generated/studio_small_03_128x64.hdr",
            "include_bytes!",
            "runtime_uri",
            "source_size_bytes",
            "bundled_environment_bytes",
        ],
    );
    require_contains(
        root,
        findings,
        "C02-PORTABLE-AGENT-ASSETS",
        "src/assets/environment_loading.rs",
        &["bundled_environment_bytes", "is_bundled_environment_uri"],
    );
    require_contains(
        root,
        findings,
        "C02-PORTABLE-AGENT-ASSETS",
        "src/scene_host/recipe/setup.rs",
        &[
            "optional_environment_skipped",
            "\"warning\"",
            "environment_load_failed",
        ],
    );
    require_contains(
        root,
        findings,
        "C02-PORTABLE-AGENT-ASSETS",
        "src/bin/scena/examples_agent.rs",
        &[
            "scena://bundled/agent-template/",
            "\"environment\": { \"preset\": \"studio\" }",
        ],
    );
    require_contains(
        root,
        findings,
        "C02-PORTABLE-AGENT-ASSETS",
        "src/bin/scena/examples_agent/starter.rs",
        &[".entry(\"environment\")", "\"preset\": \"studio\""],
    );
    require_contains(
        root,
        findings,
        "C02-PORTABLE-AGENT-ASSETS",
        "tests/scena_cli_agent_templates.rs",
        &[
            "scena_examples_agent_primitive_flow_runs_from_an_unrelated_working_directory",
            "scena_examples_agent_defaults_preserve_an_explicit_environment",
            "scena_examples_agent_every_template_runs_end_to_end_outside_a_checkout",
            "recipe\", \"build",
            "portable-frame.png",
        ],
    );
    require_contains(
        root,
        findings,
        "C02-PORTABLE-AGENT-ASSETS",
        "tests/assets/environment/PRESET-LICENSES.md",
        &[
            "CC0-1.0",
            "polyhaven.com/a/studio_small_08",
            "f6a989f89432eb4eee3191364a9c1ceed195c4ec3544173a3c04fd96cb91d0ba",
            "studio_small_03_128x64.hdr",
            "0d1acad0",
        ],
    );
    require_contains(
        root,
        findings,
        "C02-PORTABLE-AGENT-ASSETS",
        "tests/assets/gltf/AGENT-TEMPLATE-ASSETS-LICENSE.md",
        &[
            "MIT OR Apache-2.0",
            "material_variants_scene.gltf",
            "animated_triangle_scene.glb",
            "cad_plate_drawing_scene.gltf",
        ],
    );
    for (path, needles) in [
        (
            "README.md",
            &["outside a repository checkout", "scene-host,inspection"][..],
        ),
        (
            "docs/getting-started.md",
            &["portable from any working directory", "recipe build"][..],
        ),
        (
            "docs/examples.md",
            &["package-embedded glTF fixtures", "takes precedence"][..],
        ),
        (
            "docs/assets.md",
            &["scena://bundled/", "PRESET-LICENSES.md"][..],
        ),
        (
            "docs/troubleshooting.md",
            &["regenerate", "work outside a checkout"][..],
        ),
        (
            "docs/guides/llm-app-builder.md",
            &[
                "template catalog is self-contained",
                "any working directory",
            ][..],
        ),
        (
            ".codex/skills/scena-app-builder/SKILL.md",
            &["Installed templates are portable", "package-embedded"][..],
        ),
        (
            "CHANGELOG.md",
            &[
                "every installed agent template portable",
                "package-embedded",
            ][..],
        ),
    ] {
        require_contains(root, findings, "C02-PORTABLE-AGENT-ASSETS", path, needles);
    }
}
