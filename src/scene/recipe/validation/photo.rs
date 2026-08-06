use std::collections::BTreeSet;

use serde_json::{Map, Value};

use super::diagnostic;
use crate::scene::recipe::SceneRecipeDiagnosticV1;

const PHOTO_FIELDS: &[&str] = &[
    "intent",
    "quality",
    "subject",
    "composition",
    "exposure",
    "focus",
    "staging",
];
const PHOTO_SUBJECT_DIRECT_FIELDS: &[&str] = &["kind", "id"];
const PHOTO_SUBJECT_SPEC_FIELDS: &[&str] = &["target", "fallback"];
const SUBJECT_FALLBACK_POLICIES: &[&str] = &["error", "average_metering_with_warning"];
const PHOTO_COMPOSITION_FIELDS: &[&str] = &["view", "fill_fraction", "max_center_offset_fraction"];
const PHOTO_RANGE_FIELDS: &[&str] = &["min", "max"];
const PHOTO_EXPOSURE_FIELDS: &[&str] = &[
    "metering",
    "mean_luminance_srgb8",
    "max_low_clip_fraction",
    "max_high_clip_fraction",
];
const PHOTO_FOCUS_FIELDS: &[&str] = &["mode", "coverage", "strength"];
const PHOTO_STAGING_FIELDS: &[&str] = &["environment", "background", "ground", "grid"];
const CAMERA_BEHAVIOR_VIEWS: &[&str] = &["three_quarter_front_right"];
const CAMERA_BEHAVIOR_METERING: &[&str] = &["subject"];
const CAMERA_BEHAVIOR_FOCUS_MODES: &[&str] = &["subject"];
const CAMERA_BEHAVIOR_FOCUS_COVERAGES: &[&str] = &["all"];
const CAMERA_BEHAVIOR_FOCUS_STRENGTHS: &[&str] = &["subtle"];
const CAMERA_BEHAVIOR_ENVIRONMENTS: &[&str] = &["studio", "bright_product_studio"];
const CAMERA_BEHAVIOR_BACKGROUNDS: &[&str] = &["dark_studio"];
const CAMERA_BEHAVIOR_GROUNDS: &[&str] = &["matte", "reflective"];
const FINAL_PHOTO_MIN_PIXELS: u64 = 8_000_000;
const FINAL_PHOTO_MIN_SUPERSAMPLE: u64 = 2;

pub(super) fn validate_photo(
    root: &Map<String, Value>,
    import_ids: &BTreeSet<String>,
    node_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let photo = root.get("photo");
    let Some(photo) = photo else {
        return;
    };
    let Some(object) = photo.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_photo",
            "error",
            "$.photo",
            "photo must be an object",
            "emit photo:{intent:\"camera_behavior\",subject:{kind:\"import\",id:\"subject\"}}",
            None,
            false,
        ));
        return;
    };
    for key in object.keys() {
        if !PHOTO_FIELDS.contains(&key.as_str()) {
            diagnostics.push(diagnostic(
                "unknown_field",
                "error",
                format!("$.photo.{key}"),
                format!("photo field '{key}' is not part of scena.scene_recipe.v1"),
                "remove the field or move caller-owned data to metadata",
                None,
                false,
            ));
        }
    }
    match object.get("intent").and_then(Value::as_str) {
        Some("camera_behavior" | "camera-behavior" | "product_hero" | "product-hero") => {}
        Some(intent) => diagnostics.push(diagnostic(
            "invalid_photo_intent",
            "error",
            "$.photo.intent",
            format!("photo intent '{intent}' is not supported"),
            "use intent:\"camera_behavior\" for automatic camera composition, metering, and focus",
            None,
            false,
        )),
        None => diagnostics.push(diagnostic(
            "missing_photo_intent",
            "error",
            "$.photo.intent",
            "photo requires an intent string",
            "use intent:\"camera_behavior\"",
            None,
            false,
        )),
    }
    match object.get("quality") {
        None => {}
        Some(Value::String(quality)) => match quality.as_str() {
            "preview" => {}
            "final" => {
                validate_final_photo_contract(root.get("capture"), root.get("render"), diagnostics)
            }
            _ => diagnostics.push(diagnostic(
                "invalid_photo_quality",
                "error",
                "$.photo.quality",
                format!("photo quality '{quality}' is not supported"),
                "use quality:\"preview\" or quality:\"final\"",
                None,
                false,
            )),
        },
        Some(_) => diagnostics.push(diagnostic(
            "invalid_photo_quality",
            "error",
            "$.photo.quality",
            "photo quality must be a string",
            "use quality:\"preview\" or quality:\"final\"",
            None,
            false,
        )),
    }
    validate_photo_subject(object.get("subject"), import_ids, node_ids, diagnostics);
    validate_photo_composition(object.get("composition"), diagnostics);
    validate_photo_exposure(object.get("exposure"), diagnostics);
    validate_photo_focus(object.get("focus"), diagnostics);
    validate_photo_staging(object.get("staging"), diagnostics);
    if matches!(
        object.get("intent").and_then(Value::as_str),
        Some("camera_behavior" | "camera-behavior" | "product_hero" | "product-hero")
    ) {
        validate_camera_behavior_conflicts(
            root.get("scene"),
            root.get("render"),
            root.get("cameras"),
            diagnostics,
        );
    }
}

fn validate_final_photo_contract(
    capture: Option<&Value>,
    render: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if let Some(capture) = capture.and_then(Value::as_object) {
        let pixels = capture
            .get("width")
            .and_then(Value::as_u64)
            .zip(capture.get("height").and_then(Value::as_u64))
            .and_then(|(width, height)| width.checked_mul(height));
        if pixels.is_some_and(|pixels| pixels < FINAL_PHOTO_MIN_PIXELS) {
            diagnostics.push(diagnostic(
                "final_photo_capture_below_min",
                "error",
                "$.capture",
                format!(
                    "final photo capture must contain at least {FINAL_PHOTO_MIN_PIXELS} pixels"
                ),
                "omit capture for the 3840x2520 final default or request at least 8 megapixels",
                None,
                false,
            ));
        }
    }

    let Some(render) = render.and_then(Value::as_object) else {
        return;
    };
    if render
        .get("supersample")
        .and_then(Value::as_u64)
        .is_some_and(|factor| factor < FINAL_PHOTO_MIN_SUPERSAMPLE)
    {
        diagnostics.push(diagnostic(
            "final_photo_supersample_below_min",
            "error",
            "$.render.supersample",
            "final photo rendering requires supersample factor 2 or greater",
            "omit render.supersample for the SSAA2 final default or request 2, 3, 4, or 8",
            None,
            false,
        ));
    }
    if render
        .get("anti_aliasing")
        .and_then(Value::as_str)
        .is_some_and(|anti_aliasing| anti_aliasing != "none")
    {
        diagnostics.push(diagnostic(
            "final_photo_redundant_msaa",
            "error",
            "$.render.anti_aliasing",
            "final photo rendering uses full-frame supersampling and does not combine it with edge-only anti-aliasing",
            "omit render.anti_aliasing or set it to \"none\"",
            None,
            false,
        ));
    }
    if render
        .get("reconstruction")
        .and_then(Value::as_str)
        .is_some_and(|reconstruction| reconstruction != "tent")
    {
        diagnostics.push(diagnostic(
            "final_photo_reconstruction_unsupported",
            "error",
            "$.render.reconstruction",
            "final photo rendering requires tent reconstruction",
            "omit render.reconstruction for the tent final default or set it to \"tent\"",
            None,
            false,
        ));
    }
}

fn validate_photo_subject(
    subject: Option<&Value>,
    import_ids: &BTreeSet<String>,
    node_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(subject) = subject else {
        return;
    };
    let Some(object) = subject.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_photo_subject",
            "error",
            "$.photo.subject",
            "photo subject must be an object",
            "emit subject:{kind:\"import\",id:\"subject\"}",
            None,
            false,
        ));
        return;
    };
    if object.contains_key("target") || object.contains_key("fallback") {
        validate_photo_subject_spec(object, import_ids, node_ids, diagnostics);
    } else {
        validate_photo_subject_direct(object, "$.photo.subject", import_ids, node_ids, diagnostics);
    }
}

fn validate_photo_subject_spec(
    object: &serde_json::Map<String, Value>,
    import_ids: &BTreeSet<String>,
    node_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    for key in object.keys() {
        if !PHOTO_SUBJECT_SPEC_FIELDS.contains(&key.as_str()) {
            diagnostics.push(diagnostic(
                "unknown_field",
                "error",
                format!("$.photo.subject.{key}"),
                format!("photo subject field '{key}' is not part of scena.scene_recipe.v1"),
                "remove the field or move caller-owned data to metadata",
                None,
                false,
            ));
        }
    }
    validate_subject_fallback(
        object.get("fallback"),
        "$.photo.subject.fallback",
        diagnostics,
    );
    let Some(target) = object.get("target") else {
        diagnostics.push(diagnostic(
            "invalid_photo_subject",
            "error",
            "$.photo.subject.target",
            "photo subject spec requires a target",
            "emit subject:{target:{kind:\"import\",id:\"subject\"},fallback:\"error\"}",
            None,
            false,
        ));
        return;
    };
    let Some(target_object) = target.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_photo_subject",
            "error",
            "$.photo.subject.target",
            "photo subject target must be an object",
            "emit target:{kind:\"import\",id:\"subject\"}",
            None,
            false,
        ));
        return;
    };
    validate_photo_subject_direct(
        target_object,
        "$.photo.subject.target",
        import_ids,
        node_ids,
        diagnostics,
    );
}

fn validate_photo_subject_direct(
    object: &serde_json::Map<String, Value>,
    path: &str,
    import_ids: &BTreeSet<String>,
    node_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    for key in object.keys() {
        if !PHOTO_SUBJECT_DIRECT_FIELDS.contains(&key.as_str()) {
            diagnostics.push(diagnostic(
                "unknown_field",
                "error",
                format!("{path}.{key}"),
                format!("photo subject field '{key}' is not part of scena.scene_recipe.v1"),
                "remove the field or use subject:{target:{...},fallback:\"error\"} for subject fallback policy",
                None,
                false,
            ));
        }
    }
    let kind = object.get("kind").and_then(Value::as_str);
    match kind {
        Some("import" | "node") => {}
        Some(kind) => diagnostics.push(diagnostic(
            "invalid_photo_subject",
            "error",
            format!("{path}.kind"),
            format!("photo subject kind '{kind}' is not supported"),
            "use subject:{kind:\"import\",id:\"subject\"} or subject:{kind:\"node\",id:\"node_id\"}",
            None,
            false,
        )),
        None => diagnostics.push(diagnostic(
            "invalid_photo_subject",
            "error",
            format!("{path}.kind"),
            "photo subject requires kind:\"import\" or kind:\"node\"",
            "use subject:{kind:\"import\",id:\"subject\"} or subject:{kind:\"node\",id:\"node_id\"}",
            None,
            false,
        )),
    }
    match object.get("id").and_then(Value::as_str) {
        Some(id) if id.trim().is_empty() => diagnostics.push(diagnostic(
            "invalid_photo_subject",
            "error",
            format!("{path}.id"),
            "photo subject id must not be empty",
            "reference a declared import id or authored node id",
            None,
            false,
        )),
        Some(id) if kind == Some("import") && !import_ids.contains(id) => {
            diagnostics.push(diagnostic(
                "unknown_photo_subject",
                "error",
                format!("{path}.id"),
                format!("photo subject import '{id}' does not match a declared import"),
                "set subject.id to one of the ids in imports[]",
                None,
                false,
            ))
        }
        Some(id) if kind == Some("node") && !node_ids.contains(id) && import_ids.is_empty() => {
            diagnostics.push(diagnostic(
                "unknown_photo_subject",
                "error",
                format!("{path}.id"),
                format!("photo subject node '{id}' does not match an authored node"),
                "set subject.id to one of the ids in nodes[]",
                None,
                false,
            ))
        }
        Some(_) => {}
        None => diagnostics.push(diagnostic(
            "invalid_photo_subject",
            "error",
            format!("{path}.id"),
            "photo subject requires an id",
            "reference a declared import id or authored node id",
            None,
            false,
        )),
    }
}

pub(super) fn validate_subject_fallback(
    fallback: Option<&Value>,
    path: &str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(fallback) = fallback else {
        return;
    };
    match fallback.as_str() {
        Some(value) if SUBJECT_FALLBACK_POLICIES.contains(&value) => {}
        Some(value) => diagnostics.push(diagnostic(
            "invalid_subject_fallback",
            "error",
            path,
            format!("subject fallback '{value}' is not supported"),
            "use fallback:\"error\" or fallback:\"average_metering_with_warning\"",
            None,
            false,
        )),
        None => diagnostics.push(diagnostic(
            "invalid_subject_fallback",
            "error",
            path,
            "subject fallback must be a string",
            "use fallback:\"error\" or fallback:\"average_metering_with_warning\"",
            None,
            false,
        )),
    }
}

fn validate_photo_composition(
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(object) = validate_photo_object(
        value,
        "$.photo.composition",
        "photo composition must be an object",
        "emit composition:{view?,fill_fraction?,max_center_offset_fraction?}",
        diagnostics,
    ) else {
        return;
    };
    validate_photo_fields(
        "$.photo.composition",
        object,
        PHOTO_COMPOSITION_FIELDS,
        diagnostics,
    );
    validate_photo_enum(
        "$.photo.composition.view",
        object.get("view"),
        CAMERA_BEHAVIOR_VIEWS,
        "invalid_photo_composition",
        diagnostics,
    );
    validate_photo_range(
        "$.photo.composition.fill_fraction",
        object.get("fill_fraction"),
        0.0,
        1.0,
        diagnostics,
    );
    validate_unit_number(
        "$.photo.composition.max_center_offset_fraction",
        object.get("max_center_offset_fraction"),
        diagnostics,
    );
}

fn validate_photo_exposure(value: Option<&Value>, diagnostics: &mut Vec<SceneRecipeDiagnosticV1>) {
    let Some(object) = validate_photo_object(
        value,
        "$.photo.exposure",
        "photo exposure must be an object",
        "emit exposure:{metering?,mean_luminance_srgb8?,max_low_clip_fraction?,max_high_clip_fraction?}",
        diagnostics,
    ) else {
        return;
    };
    validate_photo_fields(
        "$.photo.exposure",
        object,
        PHOTO_EXPOSURE_FIELDS,
        diagnostics,
    );
    validate_photo_enum(
        "$.photo.exposure.metering",
        object.get("metering"),
        CAMERA_BEHAVIOR_METERING,
        "invalid_photo_exposure",
        diagnostics,
    );
    validate_photo_range(
        "$.photo.exposure.mean_luminance_srgb8",
        object.get("mean_luminance_srgb8"),
        0.0,
        255.0,
        diagnostics,
    );
    validate_unit_number(
        "$.photo.exposure.max_low_clip_fraction",
        object.get("max_low_clip_fraction"),
        diagnostics,
    );
    validate_unit_number(
        "$.photo.exposure.max_high_clip_fraction",
        object.get("max_high_clip_fraction"),
        diagnostics,
    );
}

fn validate_photo_focus(value: Option<&Value>, diagnostics: &mut Vec<SceneRecipeDiagnosticV1>) {
    let Some(object) = validate_photo_object(
        value,
        "$.photo.focus",
        "photo focus must be an object",
        "emit focus:{mode:\"subject\",coverage?:\"all\",strength?:\"subtle\"}",
        diagnostics,
    ) else {
        return;
    };
    validate_photo_fields("$.photo.focus", object, PHOTO_FOCUS_FIELDS, diagnostics);
    validate_photo_enum(
        "$.photo.focus.mode",
        object.get("mode"),
        CAMERA_BEHAVIOR_FOCUS_MODES,
        "invalid_photo_focus",
        diagnostics,
    );
    validate_photo_enum(
        "$.photo.focus.coverage",
        object.get("coverage"),
        CAMERA_BEHAVIOR_FOCUS_COVERAGES,
        "invalid_photo_focus",
        diagnostics,
    );
    validate_photo_enum(
        "$.photo.focus.strength",
        object.get("strength"),
        CAMERA_BEHAVIOR_FOCUS_STRENGTHS,
        "invalid_photo_focus",
        diagnostics,
    );
}

fn validate_photo_staging(value: Option<&Value>, diagnostics: &mut Vec<SceneRecipeDiagnosticV1>) {
    let Some(object) = validate_photo_object(
        value,
        "$.photo.staging",
        "photo staging must be an object",
        "emit staging:{environment?,background?,ground?,grid?}",
        diagnostics,
    ) else {
        return;
    };
    validate_photo_fields("$.photo.staging", object, PHOTO_STAGING_FIELDS, diagnostics);
    validate_photo_enum(
        "$.photo.staging.environment",
        object.get("environment"),
        CAMERA_BEHAVIOR_ENVIRONMENTS,
        "invalid_photo_staging",
        diagnostics,
    );
    validate_photo_enum(
        "$.photo.staging.background",
        object.get("background"),
        CAMERA_BEHAVIOR_BACKGROUNDS,
        "invalid_photo_staging",
        diagnostics,
    );
    validate_photo_enum(
        "$.photo.staging.ground",
        object.get("ground"),
        CAMERA_BEHAVIOR_GROUNDS,
        "invalid_photo_staging",
        diagnostics,
    );
    if let Some(grid) = object.get("grid") {
        match grid.as_bool() {
            Some(false) => {}
            Some(true) => diagnostics.push(diagnostic(
                "invalid_photo_staging",
                "error",
                "$.photo.staging.grid",
                "camera_behavior staging requires grid:false",
                "remove the field or set grid:false; use ordinary recipe rendering for CAD grids",
                None,
                false,
            )),
            None => diagnostics.push(diagnostic(
                "invalid_photo_staging",
                "error",
                "$.photo.staging.grid",
                "photo staging grid must be a boolean",
                "use grid:false for camera_behavior",
                None,
                false,
            )),
        }
    }
}

fn validate_photo_object<'a>(
    value: Option<&'a Value>,
    path: &str,
    message: &str,
    help: &str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> Option<&'a Map<String, Value>> {
    let value = value?;
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_photo",
            "error",
            path,
            message,
            help,
            None,
            false,
        ));
        return None;
    };
    Some(object)
}

fn validate_photo_fields(
    path: &str,
    object: &Map<String, Value>,
    allowed: &[&str],
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            diagnostics.push(diagnostic(
                "unknown_field",
                "error",
                format!("{path}.{key}"),
                format!("photo field '{key}' is not part of scena.scene_recipe.v1"),
                "remove the field or move caller-owned data to metadata",
                None,
                false,
            ));
        }
    }
}

fn validate_photo_enum(
    path: &str,
    value: Option<&Value>,
    allowed: &[&str],
    code: &str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    match value.as_str() {
        Some(value) if allowed.contains(&value) => {}
        Some(value) => diagnostics.push(diagnostic(
            code,
            "error",
            path,
            format!("unsupported value '{value}'"),
            format!("use one of: {}", allowed.join(", ")),
            None,
            false,
        )),
        None => diagnostics.push(diagnostic(
            code,
            "error",
            path,
            "field must be a string",
            format!("use one of: {}", allowed.join(", ")),
            None,
            false,
        )),
    }
}

fn validate_photo_range(
    path: &str,
    value: Option<&Value>,
    min_allowed: f64,
    max_allowed: f64,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_photo_range",
            "error",
            path,
            "photo range must be an object with optional min/max numbers",
            "emit {min:...,max:...} within the documented policy band",
            None,
            false,
        ));
        return;
    };
    validate_photo_fields(path, object, PHOTO_RANGE_FIELDS, diagnostics);
    let min = object.get("min").and_then(Value::as_f64);
    let max = object.get("max").and_then(Value::as_f64);
    for (name, number) in [("min", min), ("max", max)] {
        if object.contains_key(name)
            && !number.is_some_and(|value| {
                value.is_finite() && value >= min_allowed && value <= max_allowed
            })
        {
            diagnostics.push(diagnostic(
                "invalid_photo_range",
                "error",
                format!("{path}.{name}"),
                format!("range {name} must be finite and in [{min_allowed}, {max_allowed}]"),
                "use the documented camera_behavior quality policy range",
                None,
                false,
            ));
        }
    }
    if let (Some(min), Some(max)) = (min, max)
        && min > max
    {
        diagnostics.push(diagnostic(
            "invalid_photo_range",
            "error",
            format!("{path}.max"),
            "range max must be greater than or equal to min",
            "swap the range endpoints or choose a wider camera_behavior policy band",
            None,
            false,
        ));
    }
}

fn validate_unit_number(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if let Some(value) = value
        && !value
            .as_f64()
            .is_some_and(|value| value.is_finite() && (0.0..=1.0).contains(&value))
    {
        diagnostics.push(diagnostic(
            "invalid_photo_range",
            "error",
            path,
            "field must be finite and in [0, 1]",
            "use a normalized fraction",
            None,
            false,
        ));
    }
}

fn validate_camera_behavior_conflicts(
    scene: Option<&Value>,
    render: Option<&Value>,
    _cameras: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if let Some(scene_object) = scene.and_then(Value::as_object) {
        if scene_object
            .get("grid")
            .and_then(Value::as_object)
            .and_then(|grid| grid.get("enabled"))
            .and_then(Value::as_bool)
            == Some(true)
        {
            diagnostics.push(photo_conflict(
                "$.scene.grid.enabled",
                "photo.intent:\"camera_behavior\" owns staging and cannot also enable the CAD grid",
                "remove scene.grid or set enabled:false; use ordinary recipe rendering for grid/CAD inspection views",
            ));
        }
        if scene_object
            .get("background")
            .and_then(Value::as_object)
            .is_some_and(|background| {
                background.contains_key("color")
                    || matches!(
                        background.get("kind").and_then(Value::as_str),
                        Some("custom" | "color")
                    )
            })
        {
            diagnostics.push(photo_conflict(
                "$.scene.background",
                "photo.intent:\"camera_behavior\" owns background staging and cannot also use a manual background color",
                "remove scene.background and use photo.staging.background, or use ordinary recipe rendering for manual backgrounds",
            ));
        }
    }

    if let Some(render_object) = render.and_then(Value::as_object) {
        if render_object.contains_key("exposure_ev") {
            diagnostics.push(photo_conflict(
                "$.render.exposure_ev",
                "photo.intent:\"camera_behavior\" owns exposure and cannot also use fixed exposure_ev",
                "remove exposure_ev; use render.exposure_compensation_ev with auto exposure for small corrections outside photo.intent",
            ));
        }
        if render_object
            .get("depth_of_field")
            .and_then(Value::as_object)
            .is_some_and(|depth_of_field| depth_of_field.contains_key("focus_distance"))
        {
            diagnostics.push(photo_conflict(
                "$.render.depth_of_field.focus_distance",
                "photo.intent:\"camera_behavior\" owns subject focus and cannot also use manual focus_distance",
                "remove focus_distance and let the camera behavior loop focus the subject; use ordinary recipe rendering for manual depth of field",
            ));
        }
    }
}

fn photo_conflict(
    path: impl Into<String>,
    message: impl Into<String>,
    suggestion: impl Into<String>,
) -> SceneRecipeDiagnosticV1 {
    diagnostic(
        "conflicting_photo_intent_setting",
        "error",
        path,
        message,
        suggestion,
        None,
        false,
    )
}
