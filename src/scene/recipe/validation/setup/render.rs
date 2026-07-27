use std::collections::BTreeSet;

use serde_json::Value;

use crate::scene::recipe::field_model::{
    ANTI_ALIASING_MODES, METERING_MODES, RECONSTRUCTION_FILTERS, RENDER_PROFILES, RENDER_QUALITIES,
    TONEMAPPERS,
};

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
    "exposure_compensation_ev",
    "auto_exposure",
    "metering",
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
const DEPTH_OF_FIELD_FIELDS: &[&str] = &[
    "focus_distance",
    "focus",
    "coverage",
    "strength",
    "aperture_f_stop",
    "radius_px",
];
const DEPTH_OF_FIELD_FOCUS_FIELDS: &[&str] = &["mode", "target"];
const DEPTH_OF_FIELD_FOCUS_TARGET_FIELDS: &[&str] = &["kind", "id"];
const DEPTH_OF_FIELD_COVERAGES: &[&str] = &["all"];
const DEPTH_OF_FIELD_STRENGTHS: &[&str] = &["subtle"];
const METERING_FIELDS: &[&str] = &["mode", "target", "fallback", "rect", "surround_weight"];
const METERING_TARGET_FIELDS: &[&str] = &["kind", "id"];
const METERING_RECT_FIELDS: &[&str] = &["x", "y", "width", "height"];

pub(in crate::scene::recipe::validation) fn validate_render_setup(
    render: Option<&Value>,
    import_ids: &BTreeSet<String>,
    node_ids: &BTreeSet<String>,
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
            "emit render:{profile?,quality?,anti_aliasing?,supersample?,reconstruction?,bloom?,ssao?,screen_space_reflections?,depth_of_field?,exposure_ev?,exposure_compensation_ev?,auto_exposure?,metering?,tonemapper?}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields("$.render", object, RENDER_FIELDS, diagnostics);
    super::validate_enum(
        "$.render.profile",
        object.get("profile"),
        RENDER_PROFILES,
        "invalid_render_setting",
        diagnostics,
    );
    super::validate_enum(
        "$.render.quality",
        object.get("quality"),
        RENDER_QUALITIES,
        "invalid_render_setting",
        diagnostics,
    );
    super::validate_enum(
        "$.render.anti_aliasing",
        object.get("anti_aliasing"),
        ANTI_ALIASING_MODES,
        "invalid_render_setting",
        diagnostics,
    );
    validate_supersample(object.get("supersample"), diagnostics);
    super::validate_enum(
        "$.render.reconstruction",
        object.get("reconstruction"),
        RECONSTRUCTION_FILTERS,
        "invalid_render_setting",
        diagnostics,
    );
    super::validate_enum(
        "$.render.tonemapper",
        object.get("tonemapper"),
        TONEMAPPERS,
        "invalid_render_setting",
        diagnostics,
    );
    validate_finite_number_optional(
        "$.render.exposure_ev",
        object.get("exposure_ev"),
        diagnostics,
    );
    validate_exposure_compensation(
        object.get("exposure_compensation_ev"),
        object.contains_key("auto_exposure"),
        diagnostics,
    );
    validate_auto_exposure(
        object.get("auto_exposure"),
        object.contains_key("exposure_ev"),
        diagnostics,
    );
    validate_metering(
        object.get("metering"),
        object.contains_key("auto_exposure"),
        import_ids,
        diagnostics,
    );
    validate_bloom(object.get("bloom"), diagnostics);
    validate_ssao(object.get("ssao"), diagnostics);
    validate_screen_space_reflections(object.get("screen_space_reflections"), diagnostics);
    validate_depth_of_field(
        object.get("depth_of_field"),
        import_ids,
        node_ids,
        diagnostics,
    );
}

fn validate_exposure_compensation(
    value: Option<&Value>,
    has_auto_exposure: bool,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    validate_finite_number_optional(
        "$.render.exposure_compensation_ev",
        Some(value),
        diagnostics,
    );
    if !has_auto_exposure {
        diagnostics.push(diagnostic(
            "conflicting_exposure_settings",
            "error",
            "$.render.exposure_compensation_ev",
            "exposure_compensation_ev requires auto_exposure in scene_recipe.v1",
            "add render.auto_exposure or use exposure_ev for full manual exposure",
            None,
            false,
        ));
    }
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

fn validate_metering(
    value: Option<&Value>,
    has_auto_exposure: bool,
    import_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    if !has_auto_exposure {
        diagnostics.push(diagnostic(
            "invalid_metering",
            "error",
            "$.render.metering",
            "render.metering only has an effect with render.auto_exposure",
            "add render.auto_exposure or remove render.metering",
            None,
            false,
        ));
    }
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_metering",
            "error",
            "$.render.metering",
            "metering must be an object",
            "emit metering:{mode:\"average\"}, metering:{mode:\"subject\",target:{kind:\"import\",id:\"subject\"}}, or metering:{mode:\"spot\",rect:{x,y,width,height}}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields("$.render.metering", object, METERING_FIELDS, diagnostics);
    validate_unit_number_optional(
        "$.render.metering.surround_weight",
        object.get("surround_weight"),
        diagnostics,
    );
    match object.get("mode").and_then(Value::as_str) {
        Some("average" | "center_weighted" | "highlight_weighted") => {
            if object.contains_key("fallback") {
                diagnostics.push(diagnostic(
                    "invalid_metering",
                    "error",
                    "$.render.metering.fallback",
                    "fallback is only valid for subject metering",
                    "remove fallback or use mode:\"subject\"",
                    None,
                    false,
                ));
            }
            if object.contains_key("target") {
                diagnostics.push(diagnostic(
                    "invalid_metering",
                    "error",
                    "$.render.metering.target",
                    "target is only valid for subject metering",
                    "remove target or use mode:\"subject\"",
                    None,
                    false,
                ));
            }
            if object.contains_key("rect") {
                diagnostics.push(diagnostic(
                    "invalid_metering",
                    "error",
                    "$.render.metering.rect",
                    "rect is only valid for spot metering",
                    "remove rect or use mode:\"spot\"",
                    None,
                    false,
                ));
            }
        }
        Some("subject") => {
            super::super::photo::validate_subject_fallback(
                object.get("fallback"),
                "$.render.metering.fallback",
                diagnostics,
            );
            validate_metering_target(object.get("target"), import_ids, diagnostics);
            if object.contains_key("rect") {
                diagnostics.push(diagnostic(
                    "invalid_metering",
                    "error",
                    "$.render.metering.rect",
                    "subject metering uses target, not rect",
                    "remove rect or use mode:\"spot\"",
                    None,
                    false,
                ));
            }
        }
        Some("spot") => {
            validate_metering_rect(object.get("rect"), diagnostics);
            if object.contains_key("fallback") {
                diagnostics.push(diagnostic(
                    "invalid_metering",
                    "error",
                    "$.render.metering.fallback",
                    "fallback is only valid for subject metering",
                    "remove fallback or use mode:\"subject\"",
                    None,
                    false,
                ));
            }
            if object.contains_key("target") {
                diagnostics.push(diagnostic(
                    "invalid_metering",
                    "error",
                    "$.render.metering.target",
                    "spot metering uses rect, not target",
                    "remove target or use mode:\"subject\"",
                    None,
                    false,
                ));
            }
        }
        Some(mode) => diagnostics.push(diagnostic(
            "invalid_metering",
            "error",
            "$.render.metering.mode",
            format!("metering mode '{mode}' is not supported"),
            format!("use one of: {}", METERING_MODES.join(", ")),
            None,
            false,
        )),
        None => diagnostics.push(diagnostic(
            "invalid_metering",
            "error",
            "$.render.metering.mode",
            "metering requires a mode string",
            format!("use one of: {}", METERING_MODES.join(", ")),
            None,
            false,
        )),
    }
}

fn validate_metering_target(
    value: Option<&Value>,
    import_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        diagnostics.push(diagnostic(
            "invalid_metering",
            "error",
            "$.render.metering.target",
            "subject metering requires a target object",
            "emit target:{kind:\"import\",id:\"subject\"}",
            None,
            false,
        ));
        return;
    };
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_metering",
            "error",
            "$.render.metering.target",
            "subject metering target must be an object",
            "emit target:{kind:\"import\",id:\"subject\"}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields(
        "$.render.metering.target",
        object,
        METERING_TARGET_FIELDS,
        diagnostics,
    );
    match object.get("kind").and_then(Value::as_str) {
        Some("import") => match object.get("id").and_then(Value::as_str) {
            Some(id) if id.trim().is_empty() => diagnostics.push(diagnostic(
                "invalid_metering",
                "error",
                "$.render.metering.target.id",
                "subject metering target import id must not be empty",
                "reference a declared import id",
                None,
                false,
            )),
            Some(id) if !import_ids.contains(id) => diagnostics.push(diagnostic(
                "unknown_metering_target",
                "error",
                "$.render.metering.target.id",
                format!("subject metering target import '{id}' does not match a declared import"),
                "set target.id to one of the ids in imports[]",
                None,
                false,
            )),
            Some(_) => {}
            None => diagnostics.push(diagnostic(
                "invalid_metering",
                "error",
                "$.render.metering.target.id",
                "subject metering target requires an import id",
                "reference a declared import id",
                None,
                false,
            )),
        },
        Some("node") => match object.get("id").and_then(Value::as_str) {
            Some(id) if id.trim().is_empty() => diagnostics.push(diagnostic(
                "invalid_metering",
                "error",
                "$.render.metering.target.id",
                "subject metering target node id must not be empty",
                "reference a declared or imported node id",
                None,
                false,
            )),
            Some(_) => {}
            None => diagnostics.push(diagnostic(
                "invalid_metering",
                "error",
                "$.render.metering.target.id",
                "subject metering node target requires an id",
                "reference a declared or imported node id",
                None,
                false,
            )),
        },
        Some(kind) => diagnostics.push(diagnostic(
            "invalid_metering",
            "error",
            "$.render.metering.target.kind",
            format!("subject metering target kind '{kind}' is not supported"),
            "use target:{kind:\"import\",id:\"subject\"} or target:{kind:\"node\",id:\"part\"}",
            None,
            false,
        )),
        None => diagnostics.push(diagnostic(
            "invalid_metering",
            "error",
            "$.render.metering.target.kind",
            "subject metering target requires kind:\"import\" or kind:\"node\"",
            "use target:{kind:\"import\",id:\"subject\"}",
            None,
            false,
        )),
    }
}

fn validate_metering_rect(value: Option<&Value>, diagnostics: &mut Vec<SceneRecipeDiagnosticV1>) {
    let Some(value) = value else {
        diagnostics.push(diagnostic(
            "invalid_metering",
            "error",
            "$.render.metering.rect",
            "spot metering requires a normalized viewport rect",
            "emit rect:{x:0.35,y:0.25,width:0.3,height:0.4}",
            None,
            false,
        ));
        return;
    };
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_metering",
            "error",
            "$.render.metering.rect",
            "spot metering rect must be an object",
            "emit rect:{x:0.35,y:0.25,width:0.3,height:0.4}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields(
        "$.render.metering.rect",
        object,
        METERING_RECT_FIELDS,
        diagnostics,
    );
    let read = |field: &str| object.get(field).and_then(Value::as_f64);
    for field in METERING_RECT_FIELDS {
        validate_unit_number_required(
            &format!("$.render.metering.rect.{field}"),
            object.get(*field),
            diagnostics,
        );
    }
    let Some((x, y, width, height)) = read("x")
        .zip(read("y"))
        .zip(read("width"))
        .zip(read("height"))
        .map(|(((x, y), width), height)| (x, y, width, height))
    else {
        return;
    };
    if width <= 0.0 || height <= 0.0 || x + width > 1.0 || y + height > 1.0 {
        diagnostics.push(diagnostic(
            "invalid_metering",
            "error",
            "$.render.metering.rect",
            "spot metering rect must be non-empty and stay inside the normalized viewport",
            "use 0 <= x,y,width,height <= 1 with x+width <= 1 and y+height <= 1",
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

fn validate_depth_of_field(
    value: Option<&Value>,
    import_ids: &BTreeSet<String>,
    node_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_render_setting",
            "error",
            "$.render.depth_of_field",
            "depth_of_field must be an object",
            "emit depth_of_field:{focus_distance,aperture_f_stop,radius_px} or depth_of_field:{focus:{mode:\"subject\",target:{kind:\"import\",id:\"subject\"}},coverage:\"all\",strength:\"subtle\"}",
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
    let has_manual_focus = object.contains_key("focus_distance");
    let has_subject_focus = object.contains_key("focus");
    if has_manual_focus && has_subject_focus {
        diagnostics.push(diagnostic(
            "ambiguous_depth_of_field_focus",
            "error",
            "$.render.depth_of_field.focus",
            "depth_of_field cannot combine manual focus_distance with subject focus",
            "remove focus_distance for subject focus, or remove focus for manual depth of field",
            None,
            false,
        ));
    }
    if has_subject_focus {
        validate_subject_focus(object.get("focus"), import_ids, node_ids, diagnostics);
        validate_focus_enum(
            "$.render.depth_of_field.coverage",
            object.get("coverage"),
            DEPTH_OF_FIELD_COVERAGES,
            "coverage:\"all\" is the only supported subject-focus coverage policy",
            "use coverage:\"all\" for subject focus",
            diagnostics,
        );
        validate_focus_enum(
            "$.render.depth_of_field.strength",
            object.get("strength"),
            DEPTH_OF_FIELD_STRENGTHS,
            "strength:\"subtle\" is the only supported subject-focus strength policy",
            "use strength:\"subtle\" for subject focus",
            diagnostics,
        );
        return;
    }

    if object.contains_key("coverage") || object.contains_key("strength") {
        diagnostics.push(diagnostic(
            "invalid_depth_of_field_focus",
            "error",
            "$.render.depth_of_field.coverage",
            "coverage and strength require subject focus",
            "add focus:{mode:\"subject\",target:{kind:\"import\",id:\"subject\"}} or remove coverage and strength from manual depth of field",
            None,
            false,
        ));
    }
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

fn validate_subject_focus(
    value: Option<&Value>,
    import_ids: &BTreeSet<String>,
    node_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        diagnostics.push(diagnostic(
            "invalid_depth_of_field_focus",
            "error",
            "$.render.depth_of_field.focus",
            "subject focus requires a focus object",
            "emit focus:{mode:\"subject\",target:{kind:\"import\",id:\"subject\"}} or focus:{mode:\"subject\",target:{kind:\"node\",id:\"part\"}}",
            None,
            false,
        ));
        return;
    };
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_depth_of_field_focus",
            "error",
            "$.render.depth_of_field.focus",
            "depth_of_field.focus must be an object",
            "emit focus:{mode:\"subject\",target:{kind:\"import\",id:\"subject\"}} or focus:{mode:\"subject\",target:{kind:\"node\",id:\"part\"}}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields(
        "$.render.depth_of_field.focus",
        object,
        DEPTH_OF_FIELD_FOCUS_FIELDS,
        diagnostics,
    );
    match object.get("mode").and_then(Value::as_str) {
        Some("subject") => {}
        Some(mode) => diagnostics.push(diagnostic(
            "invalid_depth_of_field_focus",
            "error",
            "$.render.depth_of_field.focus.mode",
            format!("depth_of_field focus mode '{mode}' is not supported"),
            "use mode:\"subject\"",
            None,
            false,
        )),
        None => diagnostics.push(diagnostic(
            "invalid_depth_of_field_focus",
            "error",
            "$.render.depth_of_field.focus.mode",
            "subject focus requires mode:\"subject\"",
            "use mode:\"subject\"",
            None,
            false,
        )),
    }
    validate_subject_focus_target(object.get("target"), import_ids, node_ids, diagnostics);
}

fn validate_subject_focus_target(
    value: Option<&Value>,
    import_ids: &BTreeSet<String>,
    node_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        diagnostics.push(diagnostic(
            "invalid_depth_of_field_focus",
            "error",
            "$.render.depth_of_field.focus.target",
            "subject focus requires a target object",
            "emit target:{kind:\"import\",id:\"subject\"} or target:{kind:\"node\",id:\"part\"}",
            None,
            false,
        ));
        return;
    };
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_depth_of_field_focus",
            "error",
            "$.render.depth_of_field.focus.target",
            "subject focus target must be an object",
            "emit target:{kind:\"import\",id:\"subject\"} or target:{kind:\"node\",id:\"part\"}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields(
        "$.render.depth_of_field.focus.target",
        object,
        DEPTH_OF_FIELD_FOCUS_TARGET_FIELDS,
        diagnostics,
    );
    match object.get("kind").and_then(Value::as_str) {
        Some("import") => {}
        Some("node") => {}
        Some(kind) => diagnostics.push(diagnostic(
            "invalid_depth_of_field_focus",
            "error",
            "$.render.depth_of_field.focus.target.kind",
            format!("subject focus target kind '{kind}' is not supported"),
            "use target:{kind:\"import\",id:\"subject\"} or target:{kind:\"node\",id:\"part\"}",
            None,
            false,
        )),
        None => diagnostics.push(diagnostic(
            "invalid_depth_of_field_focus",
            "error",
            "$.render.depth_of_field.focus.target.kind",
            "subject focus target requires kind:\"import\" or kind:\"node\"",
            "use target:{kind:\"import\",id:\"subject\"} or target:{kind:\"node\",id:\"part\"}",
            None,
            false,
        )),
    }
    let kind = object.get("kind").and_then(Value::as_str);
    match object.get("id").and_then(Value::as_str) {
        Some(id) if id.trim().is_empty() => diagnostics.push(diagnostic(
            "invalid_depth_of_field_focus",
            "error",
            "$.render.depth_of_field.focus.target.id",
            "subject focus target id must not be empty",
            "reference a declared import id or authored node id",
            None,
            false,
        )),
        Some(id) if kind == Some("import") && !import_ids.contains(id) => diagnostics.push(diagnostic(
            "unknown_depth_of_field_focus_target",
            "error",
            "$.render.depth_of_field.focus.target.id",
            format!("subject focus target import '{id}' does not match a declared import"),
            "set target.id to one of the ids in imports[]",
            None,
            false,
        )),
        Some(id) if kind == Some("node") && !node_ids.contains(id) && import_ids.is_empty() => diagnostics.push(diagnostic(
            "unknown_depth_of_field_focus_target",
            "error",
            "$.render.depth_of_field.focus.target.id",
            format!("subject focus target node '{id}' does not match an authored node"),
            "set target.id to one of the ids in nodes[], or use an imported node path when imports are present",
            None,
            false,
        )),
        Some(_) => {}
        None => diagnostics.push(diagnostic(
            "invalid_depth_of_field_focus",
            "error",
            "$.render.depth_of_field.focus.target.id",
            "subject focus target requires an id",
            "reference a declared import id or authored node id",
            None,
            false,
        )),
    }
}

fn validate_focus_enum(
    path: &str,
    value: Option<&Value>,
    allowed: &[&str],
    message: &str,
    help: &str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    match value.and_then(Value::as_str) {
        Some(value) if allowed.contains(&value) => {}
        _ => diagnostics.push(diagnostic(
            "invalid_depth_of_field_focus",
            "error",
            path,
            message,
            help,
            None,
            false,
        )),
    }
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
