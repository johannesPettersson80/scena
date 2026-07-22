use self::grid::{add_grid_floor_with_options, apply_grid, grid_options_under_scene_bounds};
use std::collections::BTreeMap;

use super::authoring::{DiagnosticPathExt, authored_color};
use super::policy::RecipeTextureBudget;
use super::{build_diagnostic, error_diagnostic};
use crate::assets::{AssetLoadOptions, DefaultAssetFetcher};
use crate::scene::recipe::{
    RecipeBuildPolicy, SceneRecipeAutoExposureV1, SceneRecipeBloomV1, SceneRecipeColorV1,
    SceneRecipeDepthOfFieldV1, SceneRecipeDiagnosticV1, SceneRecipeEnvironmentV1,
    SceneRecipeGridReflectionV1, SceneRecipeRenderV1, SceneRecipeSceneV1,
    SceneRecipeScreenSpaceReflectionsV1, SceneRecipeSsaoV1,
};
use crate::scene_host::SceneHostCore;
use crate::{
    AntiAliasing, AssetPath, AutoExposureConfig, Background, DepthOfFieldConfig, EnvironmentPreset,
    PostBloomConfig, Profile, Quality, ReconstructionFilter, RendererOptions, SceneSetupPreset,
    ScreenSpaceAmbientOcclusionConfig, ScreenSpaceReflectionConfig, Tonemapper,
};

mod grid;
mod render_options;
use render_options::*;

pub(super) fn renderer_options_from_recipe(
    render: Option<&SceneRecipeRenderV1>,
) -> RendererOptions {
    let mut options = RendererOptions::default();
    let Some(render) = render else {
        return options;
    };
    if let Some(profile) = render.profile.as_deref().map(profile_from_recipe) {
        options = options.with_profile(profile);
    }
    if let Some(quality) = render.quality.as_deref().map(quality_from_recipe) {
        options = options.with_quality(quality);
    }
    options
}

pub(super) fn apply_render_setup(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    render: Option<&SceneRecipeRenderV1>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(render) = render else {
        return;
    };
    if let Some(anti_aliasing) = render
        .anti_aliasing
        .as_deref()
        .map(anti_aliasing_from_recipe)
    {
        host.renderer.set_anti_aliasing(anti_aliasing);
    }
    if let Some(supersample) = render.supersample
        && let Err(error) = host.renderer.set_supersample_factor(u32::from(supersample))
    {
        diagnostics.push(error_diagnostic(
            "$.render.supersample",
            "invalid_render_setting",
            error.to_string(),
            error.help(),
        ));
    }
    if let Some(reconstruction) = render
        .reconstruction
        .as_deref()
        .map(reconstruction_from_recipe)
    {
        host.renderer.set_reconstruction_filter(reconstruction);
    }
    if let Some(bloom) = render.bloom.map(bloom_from_recipe) {
        host.renderer.set_bloom(Some(bloom));
    }
    if let Some(ssao) = render.ssao.map(ssao_from_recipe) {
        host.renderer.set_screen_space_ambient_occlusion(Some(ssao));
    }
    if let Some(reflections) = render.screen_space_reflections.map(ssr_from_recipe) {
        host.renderer
            .set_screen_space_reflections(Some(reflections));
    }
    if let Some(depth_of_field) = render.depth_of_field.map(dof_from_recipe) {
        host.renderer.set_depth_of_field(Some(depth_of_field));
    }
    if let Some(exposure_ev) = render.exposure_ev {
        host.renderer.set_exposure_ev(exposure_ev as f32);
    }
    if let Some(auto_exposure) = &render.auto_exposure {
        match auto_exposure_from_recipe(auto_exposure) {
            Ok(config) => host.renderer.set_auto_exposure(config),
            Err(diagnostic) => diagnostics.push(*diagnostic),
        }
    }
    if let Some(tonemapper) = render.tonemapper.as_deref().map(tonemapper_from_recipe) {
        host.renderer.set_tonemapper(tonemapper);
    }
}

pub(super) async fn apply_scene_setup(
    policy: &RecipeBuildPolicy,
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    recipe_path: &str,
    colors: &BTreeMap<String, SceneRecipeColorV1>,
    scene: Option<&SceneRecipeSceneV1>,
    texture_budget: &mut RecipeTextureBudget,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    apply_scene_setup_with_renderer(
        policy,
        host,
        recipe_path,
        colors,
        scene,
        texture_budget,
        diagnostics,
        true,
    )
    .await;
}

/// Resolves and loads scene-level recipe resources for a manifest build while
/// deliberately avoiding every renderer mutation.
pub(super) async fn validate_scene_setup_for_manifest(
    policy: &RecipeBuildPolicy,
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    recipe_path: &str,
    colors: &BTreeMap<String, SceneRecipeColorV1>,
    scene: Option<&SceneRecipeSceneV1>,
    texture_budget: &mut RecipeTextureBudget,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    apply_scene_setup_with_renderer(
        policy,
        host,
        recipe_path,
        colors,
        scene,
        texture_budget,
        diagnostics,
        false,
    )
    .await;
}

// This orchestration boundary keeps recipe source, mutable budget/diagnostic
// accumulators, and renderer policy explicit; bundling them would obscure the
// ownership split between validation-only and rendering builds.
#[allow(clippy::too_many_arguments)]
async fn apply_scene_setup_with_renderer(
    policy: &RecipeBuildPolicy,
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    recipe_path: &str,
    colors: &BTreeMap<String, SceneRecipeColorV1>,
    scene: Option<&SceneRecipeSceneV1>,
    texture_budget: &mut RecipeTextureBudget,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
    apply_renderer: bool,
) {
    let Some(scene) = scene else {
        return;
    };
    if let Some(preset) = scene
        .preset
        .as_deref()
        .and_then(SceneSetupPreset::from_recipe_name)
    {
        apply_scene_preset(
            policy,
            host,
            recipe_path,
            scene,
            preset,
            texture_budget,
            diagnostics,
            apply_renderer,
        )
        .await;
    }
    if let Some(background) = &scene.background {
        match background_from_recipe(colors, background) {
            Ok(background) if apply_renderer => host.renderer.set_background(background),
            Ok(_) => {}
            Err(diagnostic) => diagnostics.push(*diagnostic),
        }
    }
    if let Some(environment) = &scene.environment {
        apply_environment(
            policy,
            host,
            recipe_path,
            environment,
            texture_budget,
            diagnostics,
            apply_renderer,
        )
        .await;
    }
    if apply_renderer
        && let Some(grid) = &scene.grid
        && grid.enabled
    {
        apply_grid(host, colors, grid, diagnostics);
    }
}

#[allow(clippy::too_many_arguments)]
async fn apply_scene_preset(
    policy: &RecipeBuildPolicy,
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    recipe_path: &str,
    scene: &SceneRecipeSceneV1,
    preset: SceneSetupPreset,
    texture_budget: &mut RecipeTextureBudget,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
    apply_renderer: bool,
) {
    if apply_renderer {
        host.apply_scene_setup_preset_renderer(preset);
    }
    if scene.environment.is_none() {
        apply_environment_preset(
            policy,
            host,
            recipe_path,
            "$.scene.preset",
            preset.environment(),
            texture_budget,
            diagnostics,
            apply_renderer,
        )
        .await;
    }
    if apply_renderer && scene.grid.is_none() {
        let options = grid_options_under_scene_bounds(host, preset.grid_options());
        add_grid_floor_with_options(host, options, preset_grid_reflection(preset), diagnostics);
    }
}

fn background_from_recipe(
    colors: &BTreeMap<String, SceneRecipeColorV1>,
    background: &crate::scene::recipe::SceneRecipeBackgroundV1,
) -> Result<Background, Box<SceneRecipeDiagnosticV1>> {
    match background.kind.as_str() {
        "studio" => Ok(Background::Studio),
        "dark_studio" => Ok(Background::DarkStudio),
        "neutral_gray" => Ok(Background::NeutralGray),
        "white" => Ok(Background::White),
        "black" => Ok(Background::Black),
        "sky" => Ok(Background::Sky),
        "transparent" => Ok(Background::Transparent),
        "custom" => {
            let Some(color) = background.color.as_deref() else {
                return Err(Box::new(error_diagnostic(
                    "$.scene.background.color",
                    "invalid_background",
                    "custom background requires a color",
                    "reference a recipe color id or use a direct #RRGGBB value",
                )));
            };
            authored_color(colors, color)
                .map(Background::Custom)
                .map_err(|diagnostic| {
                    Box::new((*diagnostic).with_path("$.scene.background.color".to_owned()))
                })
        }
        _ => Err(Box::new(error_diagnostic(
            "$.scene.background.kind",
            "invalid_background",
            "unsupported background kind",
            "use a documented background kind",
        ))),
    }
}

async fn apply_environment(
    policy: &RecipeBuildPolicy,
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    recipe_path: &str,
    environment: &SceneRecipeEnvironmentV1,
    texture_budget: &mut RecipeTextureBudget,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
    apply_renderer: bool,
) {
    if let Some(preset) = environment.preset.as_deref() {
        let Some(preset) = EnvironmentPreset::from_recipe_name(preset) else {
            let names = EnvironmentPreset::ALL
                .iter()
                .map(|preset| preset.recipe_name())
                .collect::<Vec<_>>()
                .join(", ");
            diagnostics.push(error_diagnostic(
                "$.scene.environment.preset",
                "invalid_environment",
                "unsupported environment preset",
                format!("use one of: {names}"),
            ));
            return;
        };
        apply_environment_preset(
            policy,
            host,
            recipe_path,
            "$.scene.environment.preset",
            preset,
            texture_budget,
            diagnostics,
            apply_renderer,
        )
        .await;
        return;
    }
    match environment.kind.as_deref().unwrap_or("") {
        "none" if apply_renderer => host.renderer.clear_environment(),
        "none" => {}
        "default" => {
            if apply_renderer {
                let handle = host.assets.default_environment();
                host.renderer.set_environment(handle);
            }
        }
        "uri" => {
            let Some(uri) = environment.uri.as_deref() else {
                diagnostics.push(error_diagnostic(
                    "$.scene.environment.uri",
                    "invalid_environment",
                    "uri environment requires a uri",
                    "provide an environment asset path allowed by RecipeBuildPolicy",
                ));
                return;
            };
            let resolved = match texture_budget.reserve_environment_uri(
                policy,
                recipe_path,
                uri,
                "$.scene.environment.uri",
            ) {
                Ok(uri) => uri,
                Err(diagnostic) => {
                    diagnostics.push(*diagnostic);
                    return;
                }
            };
            let options =
                AssetLoadOptions::default().with_fetch_byte_limit(policy.fetch_byte_limit());
            let load = if apply_renderer {
                host.assets
                    .load_environment_with_options(AssetPath::from(resolved.as_str()), options)
                    .await
                    .map(Some)
            } else {
                host.assets
                    .validate_environment_source_with_options(
                        AssetPath::from(resolved.as_str()),
                        options,
                    )
                    .await
                    .map(|()| None)
            };
            match load {
                Ok(Some(handle)) => host.renderer.set_environment(handle),
                Ok(None) => {}
                Err(error) if environment.optional => diagnostics.push(build_diagnostic(
                    "optional_environment_skipped",
                    "warning",
                    "$.scene.environment",
                    format!("optional environment '{uri}' could not be loaded: {error}"),
                    "the environment was marked optional, so the build continues without IBL",
                    None,
                    false,
                )),
                Err(error) => diagnostics.push(error_diagnostic(
                    "$.scene.environment",
                    "environment_load_failed",
                    format!("required environment '{uri}' could not be loaded: {error}"),
                    "fix the uri or mark the environment optional only if no IBL is acceptable",
                )),
            }
        }
        _ => diagnostics.push(error_diagnostic(
            "$.scene.environment.kind",
            "invalid_environment",
            "unsupported environment kind",
            "use default, uri, or none",
        )),
    }
}

#[allow(clippy::too_many_arguments)]
async fn apply_environment_preset(
    policy: &RecipeBuildPolicy,
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    _recipe_path: &str,
    diagnostic_path: &'static str,
    preset: EnvironmentPreset,
    texture_budget: &mut RecipeTextureBudget,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
    apply_renderer: bool,
) {
    let metadata = preset.metadata();
    let uri = metadata.runtime_uri();
    if let Err(diagnostic) = texture_budget.reserve_builtin_environment(
        policy,
        metadata.source_size_bytes(),
        diagnostic_path,
    ) {
        diagnostics.push(*diagnostic);
        return;
    }
    match host
        .assets
        .load_environment_preset_with_options(
            preset,
            AssetLoadOptions::default().with_fetch_byte_limit(policy.fetch_byte_limit()),
        )
        .await
    {
        Ok(handle) if apply_renderer => host.renderer.set_environment(handle),
        Ok(_) => {}
        Err(error) => diagnostics.push(error_diagnostic(
            diagnostic_path,
            "environment_load_failed",
            format!(
                "scene preset environment '{}' could not be loaded from '{uri}': {error}",
                metadata.name()
            ),
            "the bundled environment preset must be readable and fit the operator-owned recipe budgets",
        )),
    }
}

fn preset_grid_reflection(preset: SceneSetupPreset) -> Option<SceneRecipeGridReflectionV1> {
    preset
        .grid_reflection_strength()
        .map(|strength| SceneRecipeGridReflectionV1 {
            enabled: true,
            strength: Some(strength),
        })
}
