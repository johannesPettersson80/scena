use crate::app::prelude::*;

/// `RECIPE-BUILD-POLICY-BOUNDARY`: recipe JSON is untrusted input, so
/// policy limits must be enforced before allocation/fetch/decode seams and
/// pinned by known-bad tests.
pub(crate) fn check_recipe_build_policy_boundary(root: &Path, findings: &mut Vec<Finding>) {
    let tests_path = root.join("tests/scene_recipe_contracts.rs");
    require_markers(
        root,
        findings,
        "RECIPE-BUILD-POLICY-BOUNDARY",
        &tests_path,
        &[
            "scene_recipe_build_policy_rejects_authored_allocation_bypasses",
            "scene_recipe_build_policy_rejects_authored_texture_and_environment_bypasses",
            "scene_recipe_build_policy_rejects_fail_open_path_sandboxes",
            "scene_recipe_build_policy_rejects_arrow_projection_underestimate",
            "scene_recipe_rejects_imported_weight_animation_without_morph_targets",
            "\"$.geometries[0]\"",
            "\"$.geometries\"",
            "\"$.animations[0].channels[0]\"",
            "\"$.animations[0].channels[0].target.id\"",
            "\"$.materials[0].base_color_texture\"",
            "\"$.scene.environment.uri\"",
            "\"$.imports[0].uri\"",
        ],
    );

    require_markers(
        root,
        findings,
        "RECIPE-BUILD-POLICY-BOUNDARY",
        &root.join("src/scene/recipe/build.rs"),
        &[
            "const DEFAULT_MAX_NODES: usize = 10_000;",
            "const DEFAULT_MAX_VERTICES: usize = 2_000_000;",
            "const DEFAULT_MAX_INDICES: usize = 6_000_000;",
            "const DEFAULT_MAX_MATERIALS: usize = 2_000;",
            "const DEFAULT_MAX_TEXTURES: usize = 256;",
            "const DEFAULT_MAX_INSTANCES: usize = 100_000;",
            "const DEFAULT_MAX_PARTICLES: usize = 100_000;",
            "const DEFAULT_MAX_ANIMATIONS: usize = 4_000;",
            "const DEFAULT_MAX_ANIMATION_CHANNELS: usize = 100_000;",
            "const DEFAULT_MAX_ANIMATION_KEYFRAMES: usize = 2_000_000;",
            "const DEFAULT_MAX_RECIPE_BYTES: usize = 8 * 1024 * 1024;",
            "RecipeBuildPolicy has no allowed local roots",
            "file URI authorities are not allowed by RecipeBuildPolicy",
            "validate_source_size",
            "stable_canonical_path",
        ],
    );

    require_markers(
        root,
        findings,
        "RECIPE-BUILD-POLICY-BOUNDARY",
        &root.join("src/scene_host/recipe/policy/budget.rs"),
        &[
            "struct RecipeBuildBudget",
            "struct RecipeTextureBudget",
            "reserve_loaded_textures",
            "reserve_texture_uri",
            "reserve_environment_uri",
            "reserve_animation",
        ],
    );

    require_markers(
        root,
        findings,
        "RECIPE-BUILD-POLICY-BOUNDARY",
        &root.join(".github/workflows/ci.yml"),
        &["cargo test --features inspection --test scena_cli_recipe --test scena_cli_agent"],
    );

    require_markers(
        root,
        findings,
        "RECIPE-BUILD-POLICY-BOUNDARY",
        &root.join("src/assets/load.rs"),
        &["fetch_byte_limit: Option<usize>", "with_fetch_byte_limit"],
    );

    require_markers(
        root,
        findings,
        "RECIPE-BUILD-POLICY-BOUNDARY",
        &root.join("src/assets/scene_loading.rs"),
        &[
            "check_fetch_byte_limit_before_fetch",
            "check_fetch_byte_limit_after_fetch",
            "AssetError::PolicyViolation",
        ],
    );

    require_markers(
        root,
        findings,
        "RECIPE-BUILD-POLICY-BOUNDARY",
        &root.join("src/assets/external_resources.rs"),
        &[
            "check_fetch_budget_before_fetch",
            "check_fetch_budget_after_fetch",
            "check_fetch_budget_total",
        ],
    );

    require_markers(
        root,
        findings,
        "RECIPE-BUILD-POLICY-BOUNDARY",
        &root.join("src/assets/texture_image_decode.rs"),
        &[
            "IMAGE_DECODE_MAX_DIMENSION",
            "reader.limits(limits)",
            "max_image_width",
            "max_alloc",
        ],
    );
    require_markers(
        root,
        findings,
        "RECIPE-BUILD-POLICY-BOUNDARY",
        &root.join("src/assets/texture_limits.rs"),
        &["IMAGE_DECODE_MAX_DIMENSION", "IMAGE_DECODE_MAX_ALLOC_BYTES"],
    );

    require_markers(
        root,
        findings,
        "RECIPE-BUILD-POLICY-BOUNDARY",
        &root.join("src/assets/gltf/meshopt.rs"),
        &[
            "validate_decode_budget",
            "MAX_MESHOPT_ATTRIBUTE_COUNT",
            "MAX_MESHOPT_DECODED_BYTES",
        ],
    );
}

fn require_markers(
    root: &Path,
    findings: &mut Vec<Finding>,
    rule: &'static str,
    path: &Path,
    markers: &[&str],
) {
    let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string();
    let Ok(text) = fs::read_to_string(path) else {
        findings.push(Finding::new(rule, format!("{rel} must exist")));
        return;
    };
    for marker in markers {
        if !text.contains(marker) {
            findings.push(Finding::new(
                rule,
                format!("{rel} missing required policy-boundary marker `{marker}`"),
            ));
        }
    }
}
