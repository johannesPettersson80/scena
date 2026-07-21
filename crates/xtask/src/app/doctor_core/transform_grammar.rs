use crate::app::prelude::*;

pub(crate) fn check_a08_transform_grammar(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "A08-CANONICAL-TRANSFORM-GRAMMAR";

    for (path, needles) in [
        (
            "src/scene/recipe/types/authoring/transform.rs",
            &[
                "#[serde(tag = \"kind\"",
                "pub enum SceneRecipeTransformV1",
                "impl TryFrom<&SceneRecipeTransformV1> for Transform",
                ".rotate_x_deg(rotation_degrees.x)",
                ".rotate_y_deg(rotation_degrees.y)",
                ".rotate_z_deg(rotation_degrees.z)",
                "rotation: rotation.normalize()",
                "PlacementRequiresScene",
            ][..],
        ),
        (
            "src/scene/recipe/types/authoring/imports.rs",
            &[
                "pub transform: Option<SceneRecipeTransformV1>",
                "enum ImportTransformCompatibilityV1",
                "Canonical(SceneRecipeTransformV1)",
                "Legacy(LegacyImportTransformV1)",
                "#[serde(deny_unknown_fields)]",
                "deserialize_with = \"deserialize_import_transform\"",
            ][..],
        ),
        (
            "src/scene/recipe/validation/imports.rs",
            &[
                "TransformUse::Import",
                "legacy_fields_are_exact",
                "legacy_transform_shape",
                r#"add kind:\"raw\" now"#,
                "SceneRecipeTransformV1::from(legacy)",
            ][..],
        ),
        (
            "src/scene/recipe/validation/authoring/targets/common.rs",
            &[
                "enum TransformUse",
                "Import,",
                "import transforms accept only canonical raw or trs local transforms",
                "raw transform rotation must be a finite non-zero",
            ][..],
        ),
        (
            "src/scene/recipe/field_model.rs",
            &[
                "IMPORT_TRANSFORM_KINDS",
                "$.imports[].transform.kind",
                ".with_enum_strings(IMPORT_TRANSFORM_KINDS)",
                "$.nodes[].transform.kind",
            ][..],
        ),
        (
            "src/scene_host/recipe.rs",
            &["apply_import_transform(", "import.transform.as_ref()"][..],
        ),
        (
            "src/scene_host/recipe/authoring/transform.rs",
            &[
                "fn apply_import_transform",
                "Transform::try_from(transform)",
                "invalid_import_transform",
                "import_transform_failed",
            ][..],
        ),
        (
            "src/bin/scena/place.rs",
            &[
                ".map(scena::Transform::try_from)",
                "SceneRecipeTransformV1::from(transform)",
            ][..],
        ),
        (
            "src/scene/placement.rs",
            &[
                "enum StableTransformCompatibilityV1",
                "Canonical(SceneRecipeTransformV1)",
                "Legacy(LegacyStableTransformV1)",
                "fn stable_transform(transform: Transform) -> SceneRecipeTransformV1",
                "SceneRecipeTransformV1::Raw",
                "round3(transform.rotation.w)",
                "Transform::try_from(&transform)",
            ][..],
        ),
        (
            "tests/assets/stable-contracts/placement_result.v1.json",
            &["\"kind\": \"raw\""][..],
        ),
        (
            "tests/assets/stable-contracts/recipe_patch.v1.json",
            &["\"kind\": \"raw\""][..],
        ),
        (
            "tests/assets/cli-golden/place_center_stdout.json",
            &["\"kind\": \"raw\""][..],
        ),
        (
            "README.md",
            &[
                "same tagged local-transform grammar",
                "legacy_transform_shape",
            ][..],
        ),
        (
            "docs/api.md",
            &[
                "SceneRecipeTransformConversionError",
                "Transform::try_from(&SceneRecipeTransformV1)",
            ][..],
        ),
        (
            "docs/schema-contracts.md",
            &[
                "Import and node local transforms share the tagged",
                "exact compatibility alias",
                "Placement and recipe-patch transforms use the same canonical raw discriminator",
            ][..],
        ),
        (
            "docs/guides/llm-app-builder.md",
            &[
                "both `imports[].transform` and `nodes[].transform`",
                "legacy_transform_shape",
            ][..],
        ),
        (
            ".codex/skills/scena-app-builder/references/recipe-loop.md",
            &["give every import/node transform an explicit `kind`"][..],
        ),
        (
            "CHANGELOG.md",
            &["Unify recipe import and authored-node transforms"][..],
        ),
        (
            "docs/release-notes/v1.8.0.md",
            &["tagged transforms for authored nodes", "narrow v1 readers"][..],
        ),
    ] {
        require_contains(root, findings, RULE, path, needles);
    }

    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/a08_transform_grammar.rs",
        &[
            "import_and_node_trs_use_one_tagged_shape_and_intrinsic_xyz_composition",
            "legacy_import_raw_shape_is_an_explicit_warning_alias_and_serializes_canonically",
            "explicit_kind_wins_and_invalid_canonical_fields_do_not_fall_back_to_legacy",
            "canonical_import_raw_transform_rejects_a_zero_quaternion_before_build",
            "placement_results_emit_canonical_raw_and_migrate_the_legacy_v1_shape",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/stable_contracts.rs",
        &["placement_and_recipe_patch_goldens_match_live_schema_serialization"],
    );
}
