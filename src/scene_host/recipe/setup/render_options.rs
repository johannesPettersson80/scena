use super::*;

pub(super) fn profile_from_recipe(value: &str) -> Profile {
    match value {
        "quality" => Profile::Quality,
        "balanced" => Profile::Balanced,
        "compatibility" => Profile::Compatibility,
        "industrial" => Profile::Industrial,
        _ => Profile::Auto,
    }
}

pub(super) fn quality_from_recipe(value: &str) -> Quality {
    match value {
        "low" => Quality::Low,
        "high" => Quality::High,
        _ => Quality::Medium,
    }
}

pub(super) fn anti_aliasing_from_recipe(value: &str) -> AntiAliasing {
    match value {
        "none" => AntiAliasing::None,
        "msaa4" => AntiAliasing::Msaa4,
        "msaa8" => AntiAliasing::Msaa8,
        _ => AntiAliasing::Fxaa,
    }
}

pub(super) fn reconstruction_from_recipe(value: &str) -> ReconstructionFilter {
    match value {
        "tent" => ReconstructionFilter::Tent,
        "gaussian" => ReconstructionFilter::Gaussian,
        _ => ReconstructionFilter::Box,
    }
}

pub(super) fn tonemapper_from_recipe(value: &str) -> Tonemapper {
    match value {
        "standard" => Tonemapper::Standard,
        "aces" => Tonemapper::Aces,
        _ => Tonemapper::PbrNeutral,
    }
}

pub(super) fn bloom_from_recipe(value: SceneRecipeBloomV1) -> PostBloomConfig {
    PostBloomConfig::new(
        value.threshold_srgb,
        value.intensity as f32,
        value.radius_px,
    )
}

pub(super) fn ssao_from_recipe(value: SceneRecipeSsaoV1) -> ScreenSpaceAmbientOcclusionConfig {
    ScreenSpaceAmbientOcclusionConfig::new(
        value.radius_px,
        value.intensity as f32,
        value.depth_threshold as f32,
    )
}

pub(super) fn ssr_from_recipe(
    value: SceneRecipeScreenSpaceReflectionsV1,
) -> ScreenSpaceReflectionConfig {
    ScreenSpaceReflectionConfig::new(
        value.strength as f32,
        value.roughness as f32,
        value.horizon_fraction as f32,
        value.fade as f32,
    )
}

pub(super) fn dof_from_recipe(
    value: &SceneRecipeDepthOfFieldV1,
) -> Result<Option<DepthOfFieldConfig>, Box<SceneRecipeDiagnosticV1>> {
    if value.focus.is_some() {
        return Ok(None);
    }
    let Some(focus_distance) = value.focus_distance else {
        return Err(Box::new(error_diagnostic(
            "$.render.depth_of_field.focus_distance",
            "invalid_render_setting",
            "manual depth of field requires focus_distance",
            "emit focus_distance with aperture_f_stop and radius_px, or use subject focus once the visible-depth solver is enabled",
        )));
    };
    let Some(aperture_f_stop) = value.aperture_f_stop else {
        return Err(Box::new(error_diagnostic(
            "$.render.depth_of_field.aperture_f_stop",
            "invalid_render_setting",
            "manual depth of field requires aperture_f_stop",
            "emit a realistic positive f-stop such as 1.4, 2.8, or 8.0",
        )));
    };
    let Some(radius_px) = value.radius_px else {
        return Err(Box::new(error_diagnostic(
            "$.render.depth_of_field.radius_px",
            "invalid_render_setting",
            "manual depth of field requires radius_px",
            "emit a positive blur radius from 1 to 16",
        )));
    };
    Ok(Some(DepthOfFieldConfig::new(
        focus_distance as f32,
        aperture_f_stop as f32,
        radius_px,
    )))
}

pub(super) fn auto_exposure_from_recipe(
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
