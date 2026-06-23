use self::grid::{add_grid_floor_with_options, apply_grid, grid_options_under_scene_bounds};
use std::collections::BTreeMap;

use super::authoring::{DiagnosticPathExt, authored_color};
use super::error_diagnostic;
use super::policy::RecipeTextureBudget;
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
        )
        .await;
    }
    if let Some(background) = &scene.background {
        match background_from_recipe(colors, background) {
            Ok(background) => host.renderer.set_background(background),
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
        )
        .await;
    }
    if let Some(grid) = &scene.grid
        && grid.enabled
    {
        apply_grid(host, colors, grid, diagnostics);
    }
}

async fn apply_scene_preset(
    policy: &RecipeBuildPolicy,
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    recipe_path: &str,
    scene: &SceneRecipeSceneV1,
    preset: SceneSetupPreset,
    texture_budget: &mut RecipeTextureBudget,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    host.apply_scene_setup_preset_renderer(preset);
    if scene.environment.is_none() {
        apply_environment_preset(
            policy,
            host,
            recipe_path,
            preset.environment(),
            texture_budget,
            diagnostics,
        )
        .await;
    }
    if scene.grid.is_none() {
        let options = grid_options_under_scene_bounds(host, preset.grid_options());
        add_grid_floor_with_options(host, options, preset_grid_reflection(preset), diagnostics);
    }
}

fn profile_from_recipe(value: &str) -> Profile {
    match value {
        "quality" => Profile::Quality,
        "balanced" => Profile::Balanced,
        "compatibility" => Profile::Compatibility,
        "industrial" => Profile::Industrial,
        _ => Profile::Auto,
    }
}

fn quality_from_recipe(value: &str) -> Quality {
    match value {
        "low" => Quality::Low,
        "high" => Quality::High,
        _ => Quality::Medium,
    }
}

fn anti_aliasing_from_recipe(value: &str) -> AntiAliasing {
    match value {
        "none" => AntiAliasing::None,
        "msaa4" => AntiAliasing::Msaa4,
        "msaa8" => AntiAliasing::Msaa8,
        _ => AntiAliasing::Fxaa,
    }
}

fn reconstruction_from_recipe(value: &str) -> ReconstructionFilter {
    match value {
        "tent" => ReconstructionFilter::Tent,
        "gaussian" => ReconstructionFilter::Gaussian,
        _ => ReconstructionFilter::Box,
    }
}

fn tonemapper_from_recipe(value: &str) -> Tonemapper {
    match value {
        "standard" => Tonemapper::Standard,
        "aces" => Tonemapper::Aces,
        _ => Tonemapper::PbrNeutral,
    }
}

fn bloom_from_recipe(value: SceneRecipeBloomV1) -> PostBloomConfig {
    PostBloomConfig::new(
        value.threshold_srgb,
        value.intensity as f32,
        value.radius_px,
    )
}

fn ssao_from_recipe(value: SceneRecipeSsaoV1) -> ScreenSpaceAmbientOcclusionConfig {
    ScreenSpaceAmbientOcclusionConfig::new(
        value.radius_px,
        value.intensity as f32,
        value.depth_threshold as f32,
    )
}

fn ssr_from_recipe(value: SceneRecipeScreenSpaceReflectionsV1) -> ScreenSpaceReflectionConfig {
    ScreenSpaceReflectionConfig::new(
        value.strength as f32,
        value.roughness as f32,
        value.horizon_fraction as f32,
        value.fade as f32,
    )
}

fn dof_from_recipe(value: SceneRecipeDepthOfFieldV1) -> DepthOfFieldConfig {
    DepthOfFieldConfig::new(
        value.focus_distance as f32,
        value.aperture_f_stop as f32,
        value.radius_px,
    )
}

fn auto_exposure_from_recipe(
    value: &SceneRecipeAutoExposureV1,
) -> Result<AutoExposureConfig, Box<SceneRecipeDiagnosticV1>> {
    let (preset, min_ev, max_ev, highlight_percentile, highlight_target_luminance) = match value {
        SceneRecipeAutoExposureV1::Preset(preset) => (preset.as_str(), None, None, None, None),
        SceneRecipeAutoExposureV1::Config {
            preset,
            min_ev,
            max_ev,
            highlight_percentile,
            highlight_target_luminance,
        } => (
            preset.as_str(),
            *min_ev,
            *max_ev,
            *highlight_percentile,
            *highlight_target_luminance,
        ),
    };
    let mut config = AutoExposureConfig::from_preset_name(preset).ok_or_else(|| {
        Box::new(error_diagnostic(
            "$.render.auto_exposure",
            "invalid_render_setting",
            format!("auto exposure preset '{preset}' is not supported"),
            format!(
                "use one of: {}",
                AutoExposureConfig::PRESET_NAMES.join(", ")
            ),
        ))
    })?;
    if let (Some(min_ev), Some(max_ev)) = (min_ev, max_ev) {
        config = config.with_ev_range(min_ev as f32, max_ev as f32);
    }
    if let (Some(percentile), Some(target)) = (highlight_percentile, highlight_target_luminance) {
        config = config.with_highlight_guard(percentile as f32, target as f32);
    }
    Ok(config)
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
) {
    if let Some(preset) = environment.preset.as_deref() {
        let Some(preset) = EnvironmentPreset::from_recipe_name(preset) else {
            diagnostics.push(error_diagnostic(
                "$.scene.environment.preset",
                "invalid_environment",
                "unsupported environment preset",
                "use studio or neutral_studio",
            ));
            return;
        };
        apply_environment_preset(
            policy,
            host,
            recipe_path,
            preset,
            texture_budget,
            diagnostics,
        )
        .await;
        return;
    }
    match environment.kind.as_deref().unwrap_or("") {
        "none" => host.renderer.clear_environment(),
        "default" => {
            let handle = host.assets.default_environment();
            host.renderer.set_environment(handle);
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
            match host
                .assets
                .load_environment_with_options(
                    AssetPath::from(resolved.as_str()),
                    AssetLoadOptions::default().with_fetch_byte_limit(policy.fetch_byte_limit()),
                )
                .await
            {
                Ok(handle) => host.renderer.set_environment(handle),
                Err(error) if environment.optional => diagnostics.push(error_diagnostic(
                    "$.scene.environment",
                    "optional_environment_skipped",
                    format!("optional environment '{uri}' could not be loaded: {error}"),
                    "the environment was marked optional, so the build continues without IBL",
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

async fn apply_environment_preset(
    policy: &RecipeBuildPolicy,
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    recipe_path: &str,
    preset: EnvironmentPreset,
    texture_budget: &mut RecipeTextureBudget,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let metadata = preset.metadata();
    let uri = metadata.source_path();
    let _resolved =
        match texture_budget.reserve_environment_uri(policy, recipe_path, uri, "$.scene.preset") {
            Ok(uri) => uri,
            Err(diagnostic) => {
                diagnostics.push(*diagnostic);
                return;
            }
        };
    match host
        .assets
        .load_environment_preset_with_options(
            preset,
            AssetLoadOptions::default().with_fetch_byte_limit(policy.fetch_byte_limit()),
        )
        .await
    {
        Ok(handle) => host.renderer.set_environment(handle),
        Err(error) => diagnostics.push(error_diagnostic(
            "$.scene.preset",
            "environment_load_failed",
            format!(
                "scene preset environment '{}' could not be loaded from '{uri}': {error}",
                metadata.name()
            ),
            "the bundled environment preset must be readable and inside RecipeBuildPolicy allowed_roots",
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
