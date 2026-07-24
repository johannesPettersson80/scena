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
                "scene_recipe_json_schema_v1",
                "scene_recipe_json_schema_paths_v1",
                "schemars::schema_for!",
                "collect_fields",
                "apply_cross_field_metadata",
                "ROOT_FIELDS",
                "CAPTURE_FIELDS",
                "PRIMITIVE_KINDS",
                "RENDER_PROFILES",
                "deprecated",
                "examples",
            ][..],
        ),
        ("Cargo.toml", &["schemars = \"1\""][..]),
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
                "failure_exit_classes",
                "error_taxonomy",
                "scena.asset_doctor.v1",
                "scena.cli_error.v1",
                "scena.recipe_policy.v1",
                "--compact",
                "--pretty",
            ][..],
        ),
        (
            "src/bin/scena/output.rs",
            &[
                "enum CliJsonStyle",
                "CliJsonStyle::Compact",
                "CliJsonStyle::Pretty",
                "--compact and --pretty are mutually exclusive",
                "serialize_json",
            ][..],
        ),
        (
            "src/bin/scena/cli_error.rs",
            &[
                "struct CliError",
                "enum CliExitClass",
                "unknown_schema",
                "feature_unavailable",
                "runtime_error",
                "policy_violation",
                "interrupted",
                "error_taxonomy_json",
            ][..],
        ),
        (
            "src/diagnostics/help.rs",
            &[
                "impl BuildError",
                "impl ImportError",
                "impl InstantiateError",
                "impl AnimationError",
                "impl Error",
                "pub fn diagnostic(&self) -> ErrorDiagnostic",
                "structured_diagnostic!(RenderError",
            ][..],
        ),
        (
            "src/bin/scena.rs",
            &[
                "CliError::invalid_command",
                "CliError::classify",
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
                "pub fn create_texture(",
                "pub fn create_texture_for_slot(",
                "pub async fn load_texture_for_slot(",
                "pub fn texture_warnings(",
            ][..],
        ),
        (
            "src/assets/texture.rs",
            &[
                "pub struct TextureMemoryId",
                "pub struct TextureMemoryDesc",
                "pub enum TextureMipPolicy",
                "pub enum TexturePixelFormat",
                "pub enum TextureSlot",
                "pub fn rgba8_for_slot(",
                "pub fn linear_rgba32f(",
            ][..],
        ),
        (
            "src/assets/load/warnings.rs",
            &["TextureDownscaled", "original_width", "decoded_width"][..],
        ),
        (
            "src/diagnostics.rs",
            &[
                "InvalidTextureData",
                "TextureSizeLimit",
                "TextureIdentityCollision",
                "TextureColorSpaceMismatch",
            ][..],
        ),
        (
            "src/assets/fetch.rs",
            &[
                "struct TrackedAssetFetcher",
                "fetch_add(1, Ordering::Relaxed)",
                "ErrorKind::NotFound",
                "AssetError::NotFound",
            ][..],
        ),
        (
            "src/prelude.rs",
            &[
                "pub use crate::{",
                "Assets",
                "FramingOptions",
                "Renderer",
                "TextureMemoryDesc",
            ][..],
        ),
        (
            "src/scene/view.rs",
            &[
                "pub fn frame_node_with_options(",
                "pub fn frame_node_with_assets_and_options",
                "visible_asset_backed_node_subtree_bounds_world",
            ][..],
        ),
        (
            "tests/a15_rust_ergonomics.rs",
            &[
                "use scena::prelude::*",
                "native_missing_file_is_curated_not_found",
                "controls_features_remain_documented_metadata_only_aliases",
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
                "PlaceTargetArg::Node",
                "success_for_target",
                "wrong_target_namespace",
            ][..],
        ),
        (
            "src/scene/placement.rs",
            &[
                "SCENE_RECIPE_PATCH_SCHEMA_V1",
                "struct SceneRecipePatchResultV1",
                "formatting_preserved",
                "updated_recipe",
                "enum ScenePlacementTargetV1",
                "candidates: Vec<String>",
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
                "`failure_exit_classes`",
                "`exit_class`",
                "internal gate artifacts",
                "### `scena.recipe_patch.v1`",
                "target:{kind:\"import\"|\"node\",id}",
                "`scena.recipe_build_result.v1`",
                "`scena.field_model.v1`",
                "`--compact` emits one-line JSON",
            ][..],
        ),
        (
            "tests/a01_cli_error_taxonomy.rs",
            &[
                "cli_errors_expose_stable_typed_exit_taxonomy",
                "runtime_and_feature_failures_are_not_invalid_arguments",
                "every_declared_command_has_error_schema_and_exit_class_inventory",
            ][..],
        ),
        (
            "tests/a02_recipe_field_model.rs",
            &[
                "recipe_field_model_covers_authoring_and_rendering_surface",
                "recipe_json_schema_and_field_model_have_bidirectional_path_parity",
                "recipe_field_model_parity_rejects_an_omitted_promoted_field",
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
            "tests/a11_authored_node_placement.rs",
            &[
                "authored_node_bounds_verbs_preview_apply_and_round_trip",
                "authored_node_target_errors_are_namespace_aware_and_import_features_stay_import_only",
                "authored_starter_manifest_teaches_node_place_and_apply",
            ][..],
        ),
        (
            "tests/a12_json_formatting.rs",
            &[
                "compact_and_pretty_are_global_deterministic_and_semantically_identical",
                "formatting_applies_to_domain_failures_and_cli_errors_without_changing_envelopes",
                "conflicting_json_styles_fail_with_typed_usage_error",
            ][..],
        ),
        (
            "tests/a13_error_remedies.rs",
            &[
                "every_build_instantiate_and_animation_variant_has_curated_help",
                "import_and_top_level_errors_delegate_help_and_structured_diagnostics",
            ][..],
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
