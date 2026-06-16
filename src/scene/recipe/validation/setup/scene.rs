use serde_json::Value;

use super::{
    SceneRecipeDiagnosticV1, diagnostic, validate_finite_number_optional, validate_known_fields,
    validate_non_negative_number_optional, validate_optional_string,
    validate_positive_number_optional, validate_unit_number_optional,
};

const SCENE_FIELDS: &[&str] = &["background", "environment", "grid"];
const BACKGROUND_FIELDS: &[&str] = &["kind", "color"];
const ENVIRONMENT_FIELDS: &[&str] = &["kind", "uri", "optional"];
const GRID_FIELDS: &[&str] = &[
    "enabled",
    "floor_y",
    "padding",
    "line_spacing",
    "color",
    "line_color",
    "roughness",
];

pub(in crate::scene::recipe::validation) fn validate_scene_setup(
    scene: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(scene) = scene else {
        return;
    };
    let Some(object) = scene.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_scene_setup",
            "error",
            "$.scene",
            "scene must be an object",
            "emit scene:{background?,environment?,grid?}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields("$.scene", object, SCENE_FIELDS, diagnostics);
    validate_background(object.get("background"), diagnostics);
    validate_environment(object.get("environment"), diagnostics);
    validate_grid(object.get("grid"), diagnostics);
}

fn validate_background(value: Option<&Value>, diagnostics: &mut Vec<SceneRecipeDiagnosticV1>) {
    let Some(value) = value else {
        return;
    };
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_background",
            "error",
            "$.scene.background",
            "background must be an object with a kind",
            "use kind:\"white\", kind:\"studio\", or kind:\"custom\" with color",
            None,
            false,
        ));
        return;
    };
    validate_known_fields("$.scene.background", object, BACKGROUND_FIELDS, diagnostics);
    super::validate_enum(
        "$.scene.background.kind",
        object.get("kind"),
        &[
            "studio",
            "dark_studio",
            "neutral_gray",
            "white",
            "black",
            "sky",
            "transparent",
            "custom",
        ],
        "invalid_background",
        diagnostics,
    );
    if object.get("kind").and_then(Value::as_str) == Some("custom") {
        match object.get("color").and_then(Value::as_str) {
            Some(color) if !color.trim().is_empty() => {}
            _ => diagnostics.push(diagnostic(
                "invalid_background",
                "error",
                "$.scene.background.color",
                "custom background requires a non-empty color string",
                "reference a recipe color id or use a direct #RRGGBB value",
                None,
                false,
            )),
        }
    }
}

fn validate_environment(value: Option<&Value>, diagnostics: &mut Vec<SceneRecipeDiagnosticV1>) {
    let Some(value) = value else {
        return;
    };
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_environment",
            "error",
            "$.scene.environment",
            "environment must be an object with a kind",
            "use kind:\"default\", kind:\"uri\", or kind:\"none\"",
            None,
            false,
        ));
        return;
    };
    validate_known_fields(
        "$.scene.environment",
        object,
        ENVIRONMENT_FIELDS,
        diagnostics,
    );
    super::validate_enum(
        "$.scene.environment.kind",
        object.get("kind"),
        &["default", "uri", "none"],
        "invalid_environment",
        diagnostics,
    );
    if object.get("kind").and_then(Value::as_str) == Some("uri") {
        match object.get("uri").and_then(Value::as_str) {
            Some(uri) if !uri.trim().is_empty() => {}
            _ => diagnostics.push(diagnostic(
                "invalid_environment",
                "error",
                "$.scene.environment.uri",
                "uri environment requires a non-empty uri string",
                "provide an environment asset path allowed by RecipeBuildPolicy",
                None,
                false,
            )),
        }
    }
    if let Some(optional) = object.get("optional")
        && optional.as_bool().is_none()
    {
        diagnostics.push(diagnostic(
            "invalid_environment",
            "error",
            "$.scene.environment.optional",
            "optional must be a boolean",
            "use true only when an environment fallback is acceptable",
            None,
            false,
        ));
    }
}

fn validate_grid(value: Option<&Value>, diagnostics: &mut Vec<SceneRecipeDiagnosticV1>) {
    let Some(value) = value else {
        return;
    };
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_grid",
            "error",
            "$.scene.grid",
            "grid must be an object",
            "emit grid:{enabled?,padding?,line_spacing?,floor_y?,color?,line_color?,roughness?}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields("$.scene.grid", object, GRID_FIELDS, diagnostics);
    if let Some(enabled) = object.get("enabled")
        && enabled.as_bool().is_none()
    {
        diagnostics.push(diagnostic(
            "invalid_grid",
            "error",
            "$.scene.grid.enabled",
            "enabled must be a boolean",
            "use true or false",
            None,
            false,
        ));
    }
    validate_finite_number_optional("$.scene.grid.floor_y", object.get("floor_y"), diagnostics);
    validate_non_negative_number_optional(
        "$.scene.grid.padding",
        object.get("padding"),
        diagnostics,
    );
    validate_positive_number_optional(
        "$.scene.grid.line_spacing",
        object.get("line_spacing"),
        diagnostics,
    );
    validate_unit_number_optional(
        "$.scene.grid.roughness",
        object.get("roughness"),
        diagnostics,
    );
    validate_optional_string("$.scene.grid.color", object.get("color"), diagnostics);
    validate_optional_string(
        "$.scene.grid.line_color",
        object.get("line_color"),
        diagnostics,
    );
}
