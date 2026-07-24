use crate::app::prelude::*;

mod command_routing;
mod operator_roots;
mod resource_resolution;
pub(crate) use command_routing::check_c03_canonical_recipe_command_routing;
pub(crate) use operator_roots::check_a02_operator_recipe_roots;
pub(crate) use resource_resolution::check_a01_recipe_resource_resolution;

/// `RECIPE-BUILD-POLICY-BOUNDARY`: recipe JSON is untrusted input, so
/// policy limits must be enforced before allocation/fetch/decode seams and
/// pinned by known-bad tests.
pub(crate) fn check_recipe_build_policy_boundary(root: &Path, findings: &mut Vec<Finding>) {
    check_c03_canonical_recipe_command_routing(root, findings);
    check_a01_recipe_resource_resolution(root, findings);
    check_a02_operator_recipe_roots(root, findings);
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
            "scene_recipe_rotation_degrees_uses_non_commuting_xyz_call_order",
            "scene_recipe_validation_accepts_ergonomic_backbone_fields",
            "scene_recipe_validation_rejects_unknown_ergonomic_presets_at_exact_paths",
            "scene_recipe_build_routes_ergonomic_fields_through_rust_helpers",
            "scene_recipe_slice4_render_settings_change_pixels_through_recipe",
            "scene_recipe_slice4_grid_emits_visible_line_pixels",
            "scene_recipe_light_presets_fail_closed",
            "invalid_color_count",
            "\"$.geometries[0].mesh.indices[1]\"",
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
        &root.join("src/bin/scena/verify.rs"),
        &[
            "input.is_recipe()",
            "run_verify_recipe_appearance",
            "scene_host_build_from_resolved_recipe",
            "introspect_appearance",
        ],
    );

    require_markers(
        root,
        findings,
        "RECIPE-ERGONOMIC-BACKBONE",
        &root.join("src/material/presets.rs"),
        &[
            "pub const PRESET_NAMES",
            "pub fn from_preset_name",
            "Self::chrome()",
            "Self::metal(",
            "Self::rough_metal(",
            "Self::brushed_steel()",
            "Self::plastic(",
            "Self::clearcoat_plastic(",
            "Self::satin(",
            "Self::leather(",
            "Self::rubber()",
            "Self::matte(",
            "Self::clear_glass(",
            "Self::frosted_glass(",
        ],
    );

    require_markers(
        root,
        findings,
        "RECIPE-ERGONOMIC-BACKBONE",
        &root.join("src/render/exposure.rs"),
        &[
            "pub const PRESET_NAMES",
            "pub fn from_preset_name",
            "Self::product_studio()",
            "Self::indoor()",
            "Self::outdoor()",
            "Self::mixed()",
        ],
    );

    require_markers(
        root,
        findings,
        "RECIPE-ERGONOMIC-BACKBONE",
        &root.join("src/scene/camera.rs"),
        &[
            "pub const LENS_PRESET_NAMES",
            "pub fn from_lens_preset_name",
            "Self::wide_angle()",
            "Self::standard()",
            "Self::portrait()",
            "Self::telephoto()",
        ],
    );

    require_markers(
        root,
        findings,
        "RECIPE-ERGONOMIC-BACKBONE",
        &root.join("src/scene/framing.rs"),
        &[
            "pub const PRESET_NAMES",
            "pub fn from_preset_name",
            "Self::new().front()",
            "Self::new().isometric()",
            "Self::new().three_quarter_front_right()",
            "Self::new().three_quarter_back_left()",
            "add_perspective_camera_default_for",
            "self.frame_bounds(camera, bounds, options)",
        ],
    );

    require_markers(
        root,
        findings,
        "RECIPE-ERGONOMIC-BACKBONE",
        &root.join("src/material/color.rs"),
        &["pub const NAMED_CONSTANTS", "pub fn from_named_constant"],
    );

    require_markers(
        root,
        findings,
        "RECIPE-ERGONOMIC-BACKBONE",
        &root.join("src/assets/environment_preset.rs"),
        &[
            "pub const ALL",
            "pub const fn recipe_name",
            "pub fn from_recipe_name",
        ],
    );

    require_markers(
        root,
        findings,
        "RECIPE-ERGONOMIC-BACKBONE",
        &root.join("src/scene_host/product.rs"),
        &[
            "pub enum SceneSetupPreset",
            "pub fn from_recipe_name",
            "pub const fn auto_exposure",
            "AutoExposureConfig::product_studio()",
            "AutoExposureConfig::mixed()",
            "AutoExposureConfig::indoor()",
            "pub fn apply_scene_setup_preset_renderer",
        ],
    );

    require_markers(
        root,
        findings,
        "RECIPE-ERGONOMIC-BACKBONE",
        &root.join("src/scene_host/recipe/authoring/materials.rs"),
        &[
            "MaterialDesc::from_preset_name",
            "with_metallic_factor",
            "with_roughness_factor",
        ],
    );

    require_markers(
        root,
        findings,
        "RECIPE-ERGONOMIC-BACKBONE",
        &root.join("src/scene_host/recipe/authoring/cameras.rs"),
        &[
            "PerspectiveCamera::from_lens_preset_name",
            "FramingOptions::from_preset_name",
            ".add_perspective_camera_default_for(",
            ".frame_bounds(",
        ],
    );

    require_markers(
        root,
        findings,
        "RECIPE-ERGONOMIC-BACKBONE",
        &root.join("src/scene_host/recipe/authoring/lights.rs"),
        &[".add_studio_lighting()"],
    );

    require_markers(
        root,
        findings,
        "RECIPE-ERGONOMIC-BACKBONE",
        &root.join("src/scene_host/recipe/setup.rs"),
        &[
            "AutoExposureConfig::from_preset_name",
            "host.renderer.set_auto_exposure(config)",
            "SceneSetupPreset::from_recipe_name",
            "host.apply_scene_setup_preset_renderer(preset)",
            "texture_budget.reserve_environment_uri",
            ".load_environment_preset_with_options(",
            "grid_options_under_scene_bounds(host, preset.grid_options())",
        ],
    );

    require_markers(
        root,
        findings,
        "RECIPE-ERGONOMIC-BACKBONE",
        &root.join("src/scene_host/recipe/setup/grid.rs"),
        &[
            "if grid.under_bounds",
            "options = options.under_bounds(bounds)",
        ],
    );

    require_markers(
        root,
        findings,
        "RECIPE-BUILD-POLICY-BOUNDARY",
        &root.join("tests/scena_cli_recipe.rs"),
        &[
            "scena_recipe_render_grid_floor_lines_are_antialiased_and_stable_on_cpu_and_gpu",
            "scena_recipe_render_verify_passes_screen_space_reflection_quality_on_cpu_and_gpu",
            "grid-floor-line-quality",
            "grid-floor-line-detail-quality",
        ],
    );

    require_markers(
        root,
        findings,
        "RECIPE-BUILD-POLICY-BOUNDARY",
        &root.join("src/scene/recipe/types/setup.rs"),
        &[
            "pub line_width_px: Option<f64>",
            "pub screen_space_reflections: Option<SceneRecipeScreenSpaceReflectionsV1>",
            "pub struct SceneRecipeScreenSpaceReflectionsV1",
        ],
    );

    require_markers(
        root,
        findings,
        "RECIPE-BUILD-POLICY-BOUNDARY",
        &root.join("src/scene/recipe/validation/setup/scene.rs"),
        &["\"line_width_px\"", "$.scene.grid.line_width_px"],
    );

    require_markers(
        root,
        findings,
        "RECIPE-BUILD-POLICY-BOUNDARY",
        &root.join("src/scene/recipe/validation/setup/render.rs"),
        &[
            "\"screen_space_reflections\"",
            "$.render.screen_space_reflections",
            "SCREEN_SPACE_REFLECTION_FIELDS",
        ],
    );

    require_markers(
        root,
        findings,
        "RECIPE-BUILD-POLICY-BOUNDARY",
        &root.join("src/scene_host/recipe/setup.rs"),
        &["set_screen_space_reflections", "ssr_from_recipe"],
    );

    require_markers(
        root,
        findings,
        "RECIPE-BUILD-POLICY-BOUNDARY",
        &root.join("src/scene_host/recipe/setup/grid.rs"),
        &["options.line_width_px(line_width_px as f32)"],
    );

    require_markers(
        root,
        findings,
        "RECIPE-BUILD-POLICY-BOUNDARY",
        &root.join("src/scene/framing/grid.rs"),
        &["grid_material(options)", "grid_transform(layout)"],
    );

    require_markers(
        root,
        findings,
        "RECIPE-BUILD-POLICY-BOUNDARY",
        &root.join("src/scene/framing/grid.rs"),
        &["options.resolved_line_width_px()", "grid_lift"],
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
        &[
            "mesa-vulkan-drivers",
            "VK_ICD_FILENAMES",
            "cargo test --lib --features scene-host,inspection",
            "cargo test --features scene-host,inspection --test scena_cli_agent_templates",
            "cargo test --features scene-host,inspection --test scena_cli_recipe --test scena_cli_agent",
            "cargo test --features ktx2 --test m8_assets_materials_ecosystem m8_ktx2_basisu_feature_decodes_basisu_ktx2_rgba_pixels -- --exact",
        ],
    );

    require_markers(
        root,
        findings,
        "RECIPE-BUILD-POLICY-BOUNDARY",
        &root.join("src/scene/recipe/validation/authoring/targets/lights.rs"),
        &[
            "validate_light_preset",
            "DIRECTIONAL_LIGHT_PRESETS",
            "POINT_LIGHT_PRESETS",
        ],
    );

    require_markers(
        root,
        findings,
        "RECIPE-BUILD-POLICY-BOUNDARY",
        &root.join("src/bin/scena/examples_agent.rs"),
        &[
            "TEMPLATE_MATERIAL_VARIANTS_ASSET",
            "scena://bundled/agent-template/",
            "\"environment\": { \"preset\": \"studio\" }",
            "TEMPLATE_CAPTURE_MIN_WIDTH: u32 = 640",
            "\"preset\": \"key\"",
            "\"preset\": \"fill\"",
            "\"preset\": \"rim\"",
        ],
    );

    require_markers(
        root,
        findings,
        "RECIPE-BUILD-POLICY-BOUNDARY",
        &root.join("src/bin/scena/examples_agent/data_visualization.rs"),
        &[
            "TemplateBuilder::ready(\"data-visualization\", &[\"inspection\", \"scene-host\"])",
            "data-mark-blue",
            "authored data-color render and appearance proof",
        ],
    );

    require_markers(
        root,
        findings,
        "RECIPE-BUILD-POLICY-BOUNDARY",
        &root.join("src/bin/scena/examples_agent/starter.rs"),
        &[
            "apply_presentation_defaults",
            "TEMPLATE_CAPTURE_MIN_WIDTH",
            "\"preset\": \"key\"",
            ".entry(\"environment\")",
            "\"preset\": \"studio\"",
        ],
    );

    require_markers(
        root,
        findings,
        "RECIPE-BUILD-POLICY-BOUNDARY",
        &root.join("tests/scena_cli_agent_templates.rs"),
        &[
            "assert_template_recipe_has_beauty_defaults",
            "assert_data_visualization_template_targets_authored_blue_mark",
        ],
    );

    require_markers(
        root,
        findings,
        "RECIPE-BUILD-POLICY-BOUNDARY",
        &root.join("tests/assets/stable-contracts/scene_recipe.v1.json"),
        &[
            "\"preset\": \"key\"",
            "\"preset\": \"fill\"",
            "\"preset\": \"rim\"",
            "studio_small_03_1k.hdr",
            "\"screen_space_reflections\"",
            "\"width\": 640",
            "\"height\": 480",
        ],
    );

    require_markers(
        root,
        findings,
        "RECIPE-BUILD-POLICY-BOUNDARY",
        &root.join(".codex/skills/scena-app-builder/SKILL.md"),
        &[
            "Make the output presentable",
            "packaged `studio` preset",
            "crossed by leader/dimension lines",
            "expect_grounded",
            "ground_contact_missing",
            "expect_helper_occluded",
            "helper_layer_overdraws_subject",
            "expect_backend",
            "backend_expectation_mismatch",
            "expect_clipping",
            "clipping_plane_count_mismatch",
            "section_box_missing",
            "expect_state",
            "material_variant_state_mismatch",
        ],
    );

    require_markers(
        root,
        findings,
        "RECIPE-BUILD-POLICY-BOUNDARY",
        &root.join("docs/guides/llm-app-builder.md"),
        &[
            "Make It Look Good",
            "packaged `studio` preset",
            "overlay_label_intersects_line",
            "expect_grounded",
            "ground_contact_missing",
            "expect_helper_occluded",
            "helper_layer_overdraws_subject",
            "expect_backend",
            "backend_expectation_mismatch",
            "expect_clipping",
            "clipping_plane_count_mismatch",
            "section_box_missing",
            "expect_state",
            "material_variant_state_mismatch",
        ],
    );

    require_markers(
        root,
        findings,
        "RECIPE-BUILD-POLICY-BOUNDARY",
        &root.join("src/scene_host/recipe.rs"),
        &["recipe_environment_changes_lit_pbr_pixels_on_headless_gpu"],
    );

    require_markers(
        root,
        findings,
        "RECIPE-BUILD-POLICY-BOUNDARY",
        &root.join("docs/checklists/scene-authoring-recipe.md"),
        &["[90, 45, 0]", "[0, 45, 90]", "Renderer::headless_gpu"],
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
    let Ok(mut text) = fs::read_to_string(path) else {
        findings.push(Finding::new(rule, format!("{rel} must exist")));
        return;
    };
    let relative_path = path.strip_prefix(root).unwrap_or(path);
    if relative_path.extension().and_then(OsStr::to_str) == Some("rs")
        && relative_path.file_name().and_then(OsStr::to_str) != Some("mod.rs")
    {
        let module_dir = relative_path.with_extension("");
        for child in source_files(root)
            .into_iter()
            .filter(|candidate| candidate.starts_with(&module_dir))
        {
            if let Ok(child_text) = fs::read_to_string(root.join(child)) {
                text.push('\n');
                text.push_str(&child_text);
            }
        }
    }
    for marker in markers {
        if !text.contains(marker) {
            findings.push(Finding::new(
                rule,
                format!("{rel} missing required policy-boundary marker `{marker}`"),
            ));
        }
    }
}
