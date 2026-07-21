use crate::app::prelude::*;

const RULE: &str = "FR01-FR04-CONTRACT-DISCOVERY";

pub(crate) fn check_fr01_fr04_contract_discovery(root: &Path, findings: &mut Vec<Finding>) {
    for (relative, needles) in [
        (
            "src/vocabulary.rs",
            &[
                "VOCABULARY_SCHEMA_V1",
                "render_backends",
                "recipe_material_kinds",
                "placement_verbs",
                "texture_color_spaces",
            ][..],
        ),
        (
            "src/scene/recipe/build.rs",
            &[
                "RECIPE_POLICY_SCHEMA_V1",
                "to_schema_report",
                "allowed_uri_schemes",
                "allowed_roots",
                "compiled_default",
            ][..],
        ),
        (
            "src/scene/recipe/field_model.rs",
            &[
                "FIELD_MODEL_SCHEMA_V1",
                "struct SchemaFieldModelV1",
                "struct SchemaFieldV1",
                "ROOT_FIELDS",
                "CAPTURE_FIELDS",
                "PRIMITIVE_KINDS",
                "RENDER_PROFILES",
                "deprecated",
                "examples",
            ][..],
        ),
        (
            "src/scene/recipe/validation/suggestions.rs",
            &[
                "field_model",
                "ROOT_FIELDS",
                "IMPORT_FIELDS",
                "CAPTURE_FIELDS",
            ][..],
        ),
        (
            "src/bin/scena/help.rs",
            &[
                "command_contracts",
                "\"emits\"",
                "scena.asset_doctor.v1",
                "scena.cli_error.v1",
                "scena.recipe_policy.v1",
            ][..],
        ),
        (
            "src/bin/scena.rs",
            &[
                "scena.cli_error.v1",
                "invalid_command",
                "scena_vocab::run_vocab_list_command",
                "scena_policy::run_recipe_policy_command",
                "run_recipe_build_command",
            ][..],
        ),
        (
            "src/scene_host/core.rs",
            &["RendererSlot::ManifestOnly", "for_manifest_build"][..],
        ),
        (
            "src/scene_host/recipe.rs",
            &[
                "build_recipe_manifest_json",
                "RecipeBuildMode::ManifestOnly",
                "recipe_manifest_host(width, height)",
                "RecipeBuildResultV1::manifest_only",
                "validate_scene_setup_for_manifest",
                "fetch_attempts()",
            ][..],
        ),
        (
            "src/scene_host/recipe/manifest.rs",
            &[
                "build_recipe_manifest_json",
                "RecipeBuildMode::ManifestOnly",
                "RecipeBuildResultV1::manifest_only",
                "fetch_attempts()",
            ][..],
        ),
        (
            "src/scene_host/recipe/setup.rs",
            &[
                "validate_scene_setup_for_manifest",
                "validate_environment_source_with_options",
                "apply_renderer",
            ][..],
        ),
        (
            "src/assets.rs",
            &[
                "fetch_attempts: Arc<AtomicU64>",
                "fn tracked_fetcher",
                "pub fn fetch_attempts",
            ][..],
        ),
        (
            "src/assets/fetch.rs",
            &[
                "struct TrackedAssetFetcher",
                "fetch_add(1, Ordering::Relaxed)",
            ][..],
        ),
        (
            "src/assets/environment_loading.rs",
            &[
                "validate_environment_source_with_options",
                "tracked_fetcher().fetch",
            ][..],
        ),
        (
            "src/scene/recipe/types/build_manifest.rs",
            &[
                "RECIPE_BUILD_RESULT_SCHEMA_V1",
                "struct RecipeBuildResultV1",
                "renderer_constructions",
                "capture_constructions",
            ][..],
        ),
        (
            "src/bin/scena/recipe.rs",
            &[
                "run_recipe_build_command",
                "build_recipe_manifest_json",
                "recipe build <recipe.json>",
            ][..],
        ),
        (
            "src/bin/scena/place.rs",
            &[
                "emit_recipe_patch",
                "expected_source_sha256",
                "stale_source",
                "SceneRecipeSemanticChangeV1::transform",
            ][..],
        ),
        (
            "src/scene/placement.rs",
            &[
                "SCENE_RECIPE_PATCH_SCHEMA_V1",
                "struct SceneRecipePatchResultV1",
                "formatting_preserved",
                "updated_recipe",
            ][..],
        ),
        (
            "src/schema_catalog.rs",
            &["scena.recipe_build_result.v1", "FIELD_MODEL_SCHEMA_V1"][..],
        ),
        (
            "src/schema_catalog/entries.rs",
            &[
                "scena.vocab.v1",
                "scena.recipe_policy.v1",
                "scena.cli_error.v1",
                "scena.recipe_patch.v1",
            ][..],
        ),
        (
            "docs/schema-contracts.md",
            &[
                "### `scena.vocab.v1`",
                "### `scena.recipe_policy.v1`",
                "`command_contracts`",
                "internal gate artifacts",
                "### `scena.recipe_patch.v1`",
                "`scena.recipe_build_result.v1`",
                "`scena.field_model.v1`",
            ][..],
        ),
        (
            "tests/scena_cli_schema.rs",
            &[
                "fr01_vocab_and_fr04_policy_are_machine_discoverable",
                "fr04_machine_help_declares_success_and_error_schemas_per_command",
                "fr01_schema_get_emits_authoritative_recipe_field_model",
                "fr01_field_model_fixtures_round_trip_and_fail_for_declared_constraints",
            ][..],
        ),
        (
            "tests/scena_cli_recipe.rs",
            &["fr03_place_apply_emits_persistent_recipe_and_rejects_stale_source"][..],
        ),
        (
            "tests/fr02_recipe_build_cli.rs",
            &[
                "fr02_recipe_build_emits_manifest_policy_and_zero_render_execution",
                "fr02_recipe_build_reports_broken_asset_and_policy_denial_without_rendering",
                "fr02_recipe_build_validates_required_environment_and_counts_real_fetch_attempts",
                "assert_zero_render_execution",
            ][..],
        ),
        (
            "tests/fr04_cli_schema_matrix.rs",
            &[
                "fr04_command_contracts_match_observed_top_level_output_families",
                "fr04_polymorphic_failure_fixtures_emit_declared_top_level_schemas",
                "fr04_each_command_has_a_real_structured_argument_error_fixture",
                "fr04_every_declared_output_schema_has_real_cli_fixture_evidence",
                "EVIDENCE",
            ][..],
        ),
    ] {
        require_contains(root, findings, RULE, relative, needles);
    }
}
