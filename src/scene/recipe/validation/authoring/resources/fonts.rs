use serde_json::Value;

use crate::scene::recipe::types::SceneRecipeDiagnosticV1;
use crate::scene::recipe::validation::diagnostic;

use super::super::{validate_known_fields, validate_required_id};

const FONT_FIELDS: &[&str] = &["id", "uri", "optional"];

pub(in crate::scene::recipe::validation::authoring) fn validate_fonts(
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(fonts) = value.as_array() else {
        diagnostics.push(diagnostic(
            "invalid_fonts",
            "error",
            "$.fonts",
            "fonts must be an array",
            "emit fonts:[{id,uri,optional?}]",
            None,
            false,
        ));
        return;
    };
    for (index, font) in fonts.iter().enumerate() {
        let path = format!("$.fonts[{index}]");
        let Some(object) = font.as_object() else {
            diagnostics.push(diagnostic(
                "invalid_font",
                "error",
                &path,
                "font entry must be an object",
                "emit {id, uri, optional?}",
                None,
                false,
            ));
            continue;
        };
        validate_known_fields(&path, object, FONT_FIELDS, diagnostics);
        validate_required_id(&path, object.get("id"), diagnostics);
        match object.get("uri").and_then(Value::as_str) {
            Some(uri) if !uri.trim().is_empty() => {}
            Some(_) => diagnostics.push(diagnostic(
                "invalid_font_uri",
                "error",
                format!("{path}.uri"),
                "font uri must not be empty",
                "point uri at a TrueType/OpenType font asset",
                None,
                false,
            )),
            None => diagnostics.push(diagnostic(
                "missing_font_uri",
                "error",
                format!("{path}.uri"),
                "font entry must include a uri string",
                "point uri at a TrueType/OpenType font asset",
                None,
                false,
            )),
        }
        if object
            .get("optional")
            .is_some_and(|value| !value.is_boolean())
        {
            diagnostics.push(diagnostic(
                "invalid_optional",
                "error",
                format!("{path}.optional"),
                "optional must be a boolean",
                "use optional:true only when the font may be absent",
                None,
                false,
            ));
        }
    }
}
