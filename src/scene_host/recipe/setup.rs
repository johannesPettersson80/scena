use std::collections::BTreeMap;

use super::authoring::{DiagnosticPathExt, authored_color};
use super::policy::RecipeTextureBudget;
use super::{error_diagnostic, scene_host_error_diagnostic};
use crate::assets::DefaultAssetFetcher;
use crate::scene::recipe::{
    RecipeBuildPolicy, SceneRecipeBloomV1, SceneRecipeColorV1, SceneRecipeDiagnosticV1,
    SceneRecipeEnvironmentV1, SceneRecipeGridV1, SceneRecipeRenderV1, SceneRecipeSceneV1,
    SceneRecipeSsaoV1,
};
use crate::scene_host::SceneHostCore;
use crate::{
    AntiAliasing, AssetPath, Background, GridFloorOptions, PostBloomConfig, Profile, Quality,
    ReconstructionFilter, RendererOptions, ScreenSpaceAmbientOcclusionConfig, Tonemapper,
};

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
    if let Some(exposure_ev) = render.exposure_ev {
        host.renderer.set_exposure_ev(exposure_ev as f32);
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
    match environment.kind.as_str() {
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
                .load_environment(AssetPath::from(resolved.as_str()))
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

fn apply_grid(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    colors: &BTreeMap<String, SceneRecipeColorV1>,
    grid: &SceneRecipeGridV1,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let mut options = GridFloorOptions::new();
    if let Ok(Some(bounds)) = host
        .scene
        .node_world_bounds(host.scene.root(), &host.assets)
    {
        options = options.under_bounds(bounds);
    }
    if let Some(floor_y) = grid.floor_y {
        options = options.floor_y(floor_y as f32);
    }
    if let Some(padding) = grid.padding {
        options = options.padding(padding as f32);
    }
    if let Some(line_spacing) = grid.line_spacing {
        options = options.line_spacing(line_spacing as f32);
    }
    if let Some(color) = grid.color.as_deref() {
        match authored_color(colors, color) {
            Ok(color) => options = options.color(color),
            Err(diagnostic) => {
                diagnostics.push((*diagnostic).with_path("$.scene.grid.color".to_owned()));
                return;
            }
        }
    }
    if let Some(color) = grid.line_color.as_deref() {
        match authored_color(colors, color) {
            Ok(color) => options = options.line_color(color),
            Err(diagnostic) => {
                diagnostics.push((*diagnostic).with_path("$.scene.grid.line_color".to_owned()));
                return;
            }
        }
    }
    if let Some(roughness) = grid.roughness {
        options = options.roughness(roughness as f32);
    }
    match host.scene.add_grid_floor(&host.assets, options) {
        Ok(handles) => {
            host.register_node(handles.slab);
            host.register_node(handles.grid);
        }
        Err(error) => diagnostics.push(scene_host_error_diagnostic(
            "$.scene.grid",
            "grid_create_failed",
            error.into(),
        )),
    }
}
