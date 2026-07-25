use crate::app::prelude::*;

pub(crate) fn check_a07_name_candidates_and_remedies(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "A07-NAME-CANDIDATES-REMEDIES";

    for (path, needles) in [
        (
            "src/diagnostics/name_candidates.rs",
            &[
                "pub fn nearest_name_candidates",
                ".flat_map(char::to_lowercase)",
                ".filter(|character| character.is_alphanumeric())",
                "ranked.sort();",
                ".take(limit)",
            ][..],
        ),
        (
            "src/scene/import/lookups.rs",
            &[
                ".filter_map(|record| record.name.as_deref())",
                "self.clips.iter().filter_map(|clip| clip.name())",
                "self.anchors.iter().map(|anchor| anchor.name.as_str())",
                ".map(|connector| connector.name.as_str())",
            ][..],
        ),
        (
            "src/scene/import/variants.rs",
            &["nearest_name_candidates(name, &self.material_variants, 3)"][..],
        ),
        (
            "src/scene/mixers.rs",
            &["AnimationError::ClipNotFound { name, candidates }"][..],
        ),
        (
            "src/diagnostics/display/lookup.rs",
            &["write_missing_with_candidates", "nearest candidates:"][..],
        ),
        (
            "src/diagnostics/display_animation.rs",
            &[
                "Self::ClipNotFound { name, candidates }",
                "nearest candidates:",
            ][..],
        ),
        (
            "src/diagnostics/animation_error.rs",
            &[
                "pub enum AnimationError",
                "ClipNotFound",
                "candidates: Vec<String>",
            ][..],
        ),
        (
            "src/scene/recipe/types/build_manifest.rs",
            &[
                "pub candidates: Vec<String>",
                "fn with_candidates",
                "self.suggestion = candidates.first().cloned()",
            ][..],
        ),
        (
            "src/scene/recipe/validation/authoring/targets/common.rs",
            &[".with_candidates(nearest_name_candidates(value, ids, 3))"][..],
        ),
        (
            "src/scene/recipe/validation/authoring/targets/import_refs.rs",
            &[
                ".with_candidates(nearest_name_candidates(target, node_ids, 3))",
                ".with_candidates(nearest_name_candidates(value, node_ids, 3))",
                "nearest_name_candidates(import, import_ids, 3)",
            ][..],
        ),
        (
            "src/scene/recipe/validation/setup/scene.rs",
            &[
                ".with_candidates(nearest_name_candidates(",
                "EnvironmentPreset::ALL",
            ][..],
        ),
        (
            "src/schema_catalog/fixtures.rs",
            &["let candidate = nearest_name_candidates("][..],
        ),
        (
            "src/bin/scena.rs",
            &[
                // X01 moved the dispatch site from `CliError::classify` to
                // `CliError::from_failure`, which classifies on the typed error
                // kind and falls back to `classify` only for `Unclassified`.
                // The pin follows the call, it is not dropped: structured
                // candidates must still reach the emitted error either way.
                "CliError::from_failure(",
                "cli_error_candidates(&args)",
                "fn cli_error_candidates",
                "fn examples_agent_error_candidates",
            ][..],
        ),
        (
            "src/bin/scena/examples_agent/catalog.rs",
            &["fn template_name_candidates", "spec.canonical"][..],
        ),
        (
            "src/scene_host/error.rs",
            &[
                "pub fn candidates(&self) -> &[String]",
                "pub fn with_candidates",
                "RenderError::NoActiveCamera =>",
            ][..],
        ),
        (
            "src/scene_host/animation.rs",
            &[
                "AnimationError::ClipNotFound { name, candidates }",
                ".with_candidates(candidates)",
            ][..],
        ),
        (
            "src/scene_host/recipe/diagnostic.rs",
            &[
                "let candidates = error.candidates().to_vec()",
                ".with_candidates(candidates)",
            ][..],
        ),
        (
            "src/diagnostics/display.rs",
            &["Scene::add_default_camera", "Scene::set_active_camera"][..],
        ),
        (
            "README.md",
            &[
                "Misspelled node/mesh-resource",
                "deterministically ranked `candidates`",
            ][..],
        ),
        (
            "docs/errors.md",
            &[
                "SceneHostError::candidates()",
                "AnimationError::ClipNotFound",
            ][..],
        ),
        (
            "docs/api.md",
            &[
                "deterministically ranked `candidates`",
                "RenderError::NoActiveCamera",
            ][..],
        ),
        (
            "docs/schema-contracts.md",
            &[
                "optional structured `candidates`",
                "`scena.cli_error.v1` includes a structured `candidates` array",
            ][..],
        ),
        (
            "docs/troubleshooting.md",
            &["## A name was not found", "Scene::add_default_camera"][..],
        ),
        (
            "docs/guides/llm-app-builder.md",
            &[
                "`scena.cli_error.v1.candidates`",
                "deterministic candidate ranking",
            ][..],
        ),
        (
            ".codex/skills/scena-app-builder/references/debugging.md",
            &[
                "read the structured `candidates` array",
                "Scene::set_active_camera",
            ][..],
        ),
        (
            "CHANGELOG.md",
            &["one shared normalized nearest-name algorithm"][..],
        ),
        (
            "docs/release-notes/v1.8.0.md",
            &["one deterministic, capped", "candidates` list"][..],
        ),
        (
            "tests/a07_name_candidates.rs",
            &["feature = \"scene-host\""][..],
        ),
    ] {
        require_contains(root, findings, RULE, path, needles);
    }

    require_candidate_variants(
        root,
        findings,
        RULE,
        "src/diagnostics.rs",
        &[
            "NodeNameNotFound",
            "AnchorNotFound",
            "ConnectorNotFound",
            "ClipNotFound",
            "VariantNotFound",
        ],
    );

    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/a07_name_candidates.rs",
        &[
            "normalized_name_candidates_are_deterministic_ranked_and_capped",
            "import_lookup_errors_carry_node_anchor_connector_clip_and_variant_candidates",
            "recipe_lookup_diagnostics_carry_node_geometry_material_and_preset_candidates",
            "cli_schema_and_template_lookup_errors_expose_structured_candidates",
            "no_active_camera_display_and_host_conversion_keep_the_remedy",
        ],
    );
}

fn require_candidate_variants(
    root: &Path,
    findings: &mut Vec<Finding>,
    rule: &'static str,
    relative: &str,
    variants: &[&str],
) {
    let Ok(source) = fs::read_to_string(root.join(relative)) else {
        findings.push(Finding::new(rule, format!("could not read {relative}")));
        return;
    };
    for variant in variants {
        let marker = format!("{variant} {{");
        let Some(start) = source.find(&marker) else {
            findings.push(Finding::new(
                rule,
                format!("{relative} is missing {variant}"),
            ));
            continue;
        };
        let block = &source[start..];
        let end = block.find("\n    },").unwrap_or(block.len());
        if !block[..end].contains("candidates: Vec<String>") {
            findings.push(Finding::new(
                rule,
                format!("{relative} {variant} must carry candidates: Vec<String>"),
            ));
        }
    }
}
