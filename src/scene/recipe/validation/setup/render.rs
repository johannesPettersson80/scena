use serde_json::Value;

use super::{
    SceneRecipeDiagnosticV1, diagnostic, validate_finite_number_optional, validate_known_fields,
    validate_non_negative_number_required, validate_u8, validate_u8_max,
    validate_unit_number_optional, validate_unit_number_required,
};

const RENDER_FIELDS: &[&str] = &[
    "profile",
    "quality",
    "anti_aliasing",
    "supersample",
    "reconstruction",
    "bloom",
    "ssao",
    "screen_space_reflections",
    "depth_of_field",
    "exposure_ev",
    "auto_exposure",
    "tonemapper",
];
const AUTO_EXPOSURE_FIELDS: &[&str] = &[
    "preset",
    "min_ev",
    "max_ev",
    "highlight_percentile",
    "highlight_target_luminance",
];
const BLOOM_FIELDS: &[&str] = &["threshold_srgb", "intensity", "radius_px"];
const SSAO_FIELDS: &[&str] = &["radius_px", "intensity", "depth_threshold"];
const SCREEN_SPACE_REFLECTION_FIELDS: &[&str] =
    &["strength", "roughness", "horizon_fraction", "fade"];
const DEPTH_OF_FIELD_FIELDS: &[&str] = &["focus_distance", "aperture_f_stop", "radius_px"];

pub(in crate::scene::recipe::validation) fn validate_render_setup(
    render: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(render) = render else {
        return;
    };
    let Some(object) = render.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_render_setting",
            "error",
            "$.render",
            "render must be an object",
            "emit render:{profile?,quality?,anti_aliasing?,supersample?,reconstruction?,bloom?,ssao?,screen_space_reflections?,depth_of_field?,exposure_ev?,auto_exposure?,tonemapper?}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields("$.render", object, RENDER_FIELDS, diagnostics);
    super::validate_enum(
        "$.render.profile",
        object.get("profile"),
        &["auto", "quality", "balanced", "compatibility", "industrial"],
        "invalid_render_setting",
        diagnostics,
    );
    super::validate_enum(
        "$.render.quality",
        object.get("quality"),
        &["low", "medium", "high"],
        "invalid_render_setting",
        diagnostics,
    );
    super::validate_enum(
        "$.render.anti_aliasing",
        object.get("anti_aliasing"),
        &["none", "fxaa", "msaa4", "msaa8"],
        "invalid_render_setting",
        diagnostics,
    );
    validate_supersample(object.get("supersample"), diagnostics);
    super::validate_enum(
        "$.render.reconstruction",
        object.get("reconstruction"),
        &["box", "tent", "gaussian"],
        "invalid_render_setting",
        diagnostics,
    );
    super::validate_enum(
        "$.render.tonemapper",
        object.get("tonemapper"),
        &["standard", "aces", "pbr_neutral"],
        "invalid_render_setting",
        diagnostics,
    );
    validate_finite_number_optional(
        "$.render.exposure_ev",
        object.get("exposure_ev"),
        diagnostics,
    );
    validate_auto_exposure(
        object.get("auto_exposure"),
        object.contains_key("exposure_ev"),
        diagnostics,
    );
    validate_bloom(object.get("bloom"), diagnostics);
    validate_ssao(object.get("ssao"), diagnostics);
    validate_screen_space_reflections(object.get("screen_space_reflections"), diagnostics);
    validate_depth_of_field(object.get("depth_of_field"), diagnostics);
}

fn validate_auto_exposure(
    value: Option<&Value>,
    has_static_exposure: bool,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    if has_static_exposure {
        diagnostics.push(diagnostic(
            "conflicting_exposure_settings",
            "error",
            "$.render.auto_exposure",
            "auto_exposure and exposure_ev are mutually exclusive in scene_recipe.v1",
            "remove exposure_ev when using an auto exposure preset, or remove auto_exposure for a fixed exposure",
            None,
            false,
        ));
    }
    if let Some(preset) = value.as_str() {
        validate_auto_exposure_preset("$.render.auto_exposure", preset, diagnostics);
        return;
    }
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_render_setting",
            "error",
            "$.render.auto_exposure",
            "auto_exposure must be a preset string or {preset,...} object",
            "use auto_exposure:\"product_studio\" or auto_exposure:{\"preset\":\"product_studio\"}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields(
        "$.render.auto_exposure",
        object,
        AUTO_EXPOSURE_FIELDS,
        diagnostics,
    );
    match object.get("preset").and_then(Value::as_str) {
        Some(preset) => {
            validate_auto_exposure_preset("$.render.auto_exposure.preset", preset, diagnostics)
        }
        None => diagnostics.push(diagnostic(
            "invalid_render_setting",
            "error",
            "$.render.auto_exposure.preset",
            "auto_exposure object requires a preset string",
            "use product_studio, indoor, outdoor, or mixed",
            None,
            false,
        )),
    }
    validate_finite_number_optional(
        "$.render.auto_exposure.min_ev",
        object.get("min_ev"),
        diagnostics,
    );
    validate_finite_number_optional(
        "$.render.auto_exposure.max_ev",
        object.get("max_ev"),
        diagnostics,
    );
    validate_unit_number_optional(
        "$.render.auto_exposure.highlight_percentile",
        object.get("highlight_percentile"),
        diagnostics,
    );
    validate_unit_number_optional(
        "$.render.auto_exposure.highlight_target_luminance",
        object.get("highlight_target_luminance"),
        diagnostics,
    );
}

fn validate_auto_exposure_preset(
    path: &str,
    preset: &str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if crate::AutoExposureConfig::from_preset_name(preset).is_none() {
        diagnostics.push(diagnostic(
            "invalid_render_setting",
            "error",
            path,
            format!("auto exposure preset '{preset}' is not supported"),
            format!(
                "use one of: {}",
                crate::AutoExposureConfig::PRESET_NAMES.join(", ")
            ),
            None,
            false,
        ));
    }
}

fn validate_supersample(value: Option<&Value>, diagnostics: &mut Vec<SceneRecipeDiagnosticV1>) {
    let Some(value) = value else {
        return;
    };
    let Some(factor) = value.as_u64() else {
        diagnostics.push(diagnostic(
            "invalid_render_setting",
            "error",
            "$.render.supersample",
            "supersample must be an integer factor 1, 2, 3, 4, or 8",
            "emit supersample:2, supersample:3, supersample:4, or supersample:8 for hero-shot quality; cost grows with N^2 and 8 requires small captures",
            None,
            false,
        ));
        return;
    };
    if !matches!(factor, 1 | 2 | 3 | 4 | 8) {
        diagnostics.push(diagnostic(
            "invalid_render_setting",
            "error",
            "$.render.supersample",
            "supersample must be 1, 2, 3, 4, or 8",
            "use 1 to disable full-frame supersampling; use 2-4 for hero-shot quality; use 8 only for small captures because cost grows with N^2",
            None,
            false,
        ));
    }
}

fn validate_bloom(value: Option<&Value>, diagnostics: &mut Vec<SceneRecipeDiagnosticV1>) {
    let Some(value) = value else {
        return;
    };
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_render_setting",
            "error",
            "$.render.bloom",
            "bloom must be an object",
            "emit bloom:{threshold_srgb,intensity,radius_px}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields("$.render.bloom", object, BLOOM_FIELDS, diagnostics);
    validate_u8(
        "$.render.bloom.threshold_srgb",
        object.get("threshold_srgb"),
        diagnostics,
    );
    validate_unit_number_required(
        "$.render.bloom.intensity",
        object.get("intensity"),
        diagnostics,
    );
    validate_u8_max(
        "$.render.bloom.radius_px",
        object.get("radius_px"),
        12,
        diagnostics,
    );
}

fn validate_ssao(value: Option<&Value>, diagnostics: &mut Vec<SceneRecipeDiagnosticV1>) {
    let Some(value) = value else {
        return;
    };
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_render_setting",
            "error",
            "$.render.ssao",
            "ssao must be an object",
            "emit ssao:{radius_px,intensity,depth_threshold}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields("$.render.ssao", object, SSAO_FIELDS, diagnostics);
    validate_u8_max(
        "$.render.ssao.radius_px",
        object.get("radius_px"),
        12,
        diagnostics,
    );
    validate_unit_number_required(
        "$.render.ssao.intensity",
        object.get("intensity"),
        diagnostics,
    );
    validate_non_negative_number_required(
        "$.render.ssao.depth_threshold",
        object.get("depth_threshold"),
        diagnostics,
    );
}

fn validate_screen_space_reflections(
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_render_setting",
            "error",
            "$.render.screen_space_reflections",
            "screen_space_reflections must be an object",
            "emit screen_space_reflections:{strength,roughness,horizon_fraction,fade}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields(
        "$.render.screen_space_reflections",
        object,
        SCREEN_SPACE_REFLECTION_FIELDS,
        diagnostics,
    );
    for field in SCREEN_SPACE_REFLECTION_FIELDS {
        validate_unit_number_required(
            &format!("$.render.screen_space_reflections.{field}"),
            object.get(*field),
            diagnostics,
        );
    }
}

fn validate_depth_of_field(value: Option<&Value>, diagnostics: &mut Vec<SceneRecipeDiagnosticV1>) {
    let Some(value) = value else {
        return;
    };
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_render_setting",
            "error",
            "$.render.depth_of_field",
            "depth_of_field must be an object",
            "emit depth_of_field:{focus_distance,aperture_f_stop,radius_px}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields(
        "$.render.depth_of_field",
        object,
        DEPTH_OF_FIELD_FIELDS,
        diagnostics,
    );
    validate_positive_number_at_least_required(
        "$.render.depth_of_field.focus_distance",
        object.get("focus_distance"),
        0.001,
        "focus_distance must be finite and >= 0.001",
        "use a positive camera-space focus distance",
        diagnostics,
    );
    validate_positive_number_at_least_required(
        "$.render.depth_of_field.aperture_f_stop",
        object.get("aperture_f_stop"),
        0.7,
        "aperture_f_stop must be finite and >= 0.7",
        "use a realistic positive f-stop such as 1.4, 2.8, or 8.0",
        diagnostics,
    );
    validate_u8_range(
        "$.render.depth_of_field.radius_px",
        object.get("radius_px"),
        1,
        16,
        diagnostics,
    );
}

fn validate_positive_number_at_least_required(
    path: &str,
    value: Option<&Value>,
    minimum: f64,
    message: &str,
    help: &str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let valid = value
        .and_then(Value::as_f64)
        .is_some_and(|number| number.is_finite() && number >= minimum);
    if !valid {
        diagnostics.push(diagnostic(
            "invalid_render_setting",
            "error",
            path,
            message,
            help,
            None,
            false,
        ));
    }
}

fn validate_u8_range(
    path: &str,
    value: Option<&Value>,
    minimum: u8,
    maximum: u8,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let valid = value
        .and_then(Value::as_u64)
        .is_some_and(|number| (u64::from(minimum)..=u64::from(maximum)).contains(&number));
    if !valid {
        diagnostics.push(diagnostic(
            "invalid_render_setting",
            "error",
            path,
            format!("field must be an integer in [{minimum}, {maximum}]"),
            "use a positive blur radius; larger values cost more and are clamped by the renderer",
            None,
            false,
        ));
    }
}
