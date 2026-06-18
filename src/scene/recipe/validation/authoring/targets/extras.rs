use std::collections::BTreeSet;

use serde_json::Value;

use crate::Color;
use crate::scene::recipe::types::SceneRecipeDiagnosticV1;

use super::super::{finite_vec3, validate_known_fields, validate_required_id};
use super::common::{TransformUse, validate_ref, validate_transform, validate_vec3};
use super::import_refs::validate_node_ref;
use crate::scene::recipe::validation::diagnostic;

const INSTANCE_SET_FIELDS: &[&str] = &[
    "id",
    "geometry",
    "material",
    "parent",
    "transform",
    "instances",
];
const INSTANCE_FIELDS: &[&str] = &["id", "transform", "tint", "visible"];
const LABEL_FIELDS: &[&str] = &[
    "id",
    "text",
    "font",
    "parent",
    "transform",
    "color",
    "background",
    "halo",
    "size_px",
];
const CLIPPING_PLANE_FIELDS: &[&str] = &["id", "normal", "distance", "active"];

pub(in crate::scene::recipe::validation::authoring) fn has_authored_instance_sets(
    value: Option<&Value>,
) -> bool {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|entry| {
            entry.as_object().is_some_and(|object| {
                object.get("geometry").is_some_and(Value::is_string)
                    && object.get("material").is_some_and(Value::is_string)
                    && object
                        .get("instances")
                        .and_then(Value::as_array)
                        .is_some_and(|instances| !instances.is_empty())
            })
        })
}

pub(in crate::scene::recipe::validation::authoring) fn has_authored_labels(
    value: Option<&Value>,
) -> bool {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|entry| {
            entry.as_object().is_some_and(|object| {
                object
                    .get("text")
                    .and_then(Value::as_str)
                    .is_some_and(|text| !text.trim().is_empty())
            })
        })
}

pub(in crate::scene::recipe::validation::authoring) fn validate_instance_sets(
    value: Option<&Value>,
    geometries: &BTreeSet<String>,
    materials: &BTreeSet<String>,
    colors: &BTreeSet<String>,
    node_ids: &BTreeSet<String>,
    import_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(instance_sets) = value.as_array() else {
        diagnostics.push(diagnostic(
            "invalid_instance_sets",
            "error",
            "$.instance_sets",
            "instance_sets must be an array",
            "emit instance_sets:[{id,geometry,material,instances}]",
            None,
            false,
        ));
        return;
    };
    for (index, entry) in instance_sets.iter().enumerate() {
        let path = format!("$.instance_sets[{index}]");
        let Some(object) = entry.as_object() else {
            diagnostics.push(diagnostic(
                "invalid_instance_set",
                "error",
                &path,
                "instance set entry must be an object",
                "emit {id, geometry, material, instances}",
                None,
                false,
            ));
            continue;
        };
        validate_known_fields(&path, object, INSTANCE_SET_FIELDS, diagnostics);
        validate_required_id(&path, object.get("id"), diagnostics);
        validate_ref(
            &format!("{path}.geometry"),
            object.get("geometry"),
            geometries,
            "geometry",
            diagnostics,
        );
        validate_ref(
            &format!("{path}.material"),
            object.get("material"),
            materials,
            "material",
            diagnostics,
        );
        if let Some(parent) = object.get("parent") {
            validate_node_ref(
                &format!("{path}.parent"),
                Some(parent),
                node_ids,
                import_ids,
                diagnostics,
            );
        }
        if let Some(transform) = object.get("transform") {
            validate_transform(
                &format!("{path}.transform"),
                transform,
                TransformUse::Node,
                node_ids,
                import_ids,
                diagnostics,
            );
        }
        validate_instances(
            &path,
            object.get("instances"),
            colors,
            node_ids,
            import_ids,
            diagnostics,
        );
    }
}

fn validate_instances(
    path: &str,
    value: Option<&Value>,
    colors: &BTreeSet<String>,
    node_ids: &BTreeSet<String>,
    import_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        diagnostics.push(diagnostic(
            "missing_instances",
            "error",
            format!("{path}.instances"),
            "instance set must include at least one instance",
            "emit instances:[{id,transform?}]",
            None,
            false,
        ));
        return;
    };
    let Some(instances) = value.as_array().filter(|instances| !instances.is_empty()) else {
        diagnostics.push(diagnostic(
            "invalid_instances",
            "error",
            format!("{path}.instances"),
            "instances must be a non-empty array",
            "emit at least one instance entry",
            None,
            false,
        ));
        return;
    };
    let mut ids = BTreeSet::new();
    for (index, instance) in instances.iter().enumerate() {
        let entry_path = format!("{path}.instances[{index}]");
        let Some(object) = instance.as_object() else {
            diagnostics.push(diagnostic(
                "invalid_instance",
                "error",
                &entry_path,
                "instance entry must be an object",
                "emit {id, transform?, tint?, visible?}",
                None,
                false,
            ));
            continue;
        };
        validate_known_fields(&entry_path, object, INSTANCE_FIELDS, diagnostics);
        match object.get("id").and_then(Value::as_str) {
            Some(id) if !id.trim().is_empty() && ids.insert(id.to_owned()) => {}
            Some(id) if id.trim().is_empty() => diagnostics.push(diagnostic(
                "invalid_id",
                "error",
                format!("{entry_path}.id"),
                "instance id must not be empty",
                "use a stable per-instance id",
                None,
                false,
            )),
            Some(id) => diagnostics.push(diagnostic(
                "duplicate_id",
                "error",
                format!("{entry_path}.id"),
                format!("instance id '{id}' is used more than once in this set"),
                "make per-instance ids unique within the set",
                None,
                false,
            )),
            None => diagnostics.push(diagnostic(
                "missing_id",
                "error",
                format!("{entry_path}.id"),
                "instance entry must include an id string",
                "add a stable per-instance id",
                None,
                false,
            )),
        }
        if let Some(transform) = object.get("transform") {
            validate_transform(
                &format!("{entry_path}.transform"),
                transform,
                TransformUse::Node,
                node_ids,
                import_ids,
                diagnostics,
            );
        }
        if let Some(tint) = object.get("tint") {
            validate_color_value(
                &format!("{entry_path}.tint"),
                Some(tint),
                colors,
                diagnostics,
            );
        }
        if object
            .get("visible")
            .is_some_and(|value| !value.is_boolean())
        {
            diagnostics.push(diagnostic(
                "invalid_visible",
                "error",
                format!("{entry_path}.visible"),
                "visible must be a boolean",
                "use true or false",
                None,
                false,
            ));
        }
    }
}

pub(in crate::scene::recipe::validation::authoring) fn validate_labels(
    value: Option<&Value>,
    colors: &BTreeSet<String>,
    fonts: &BTreeSet<String>,
    target_ids: &BTreeSet<String>,
    import_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(labels) = value.as_array() else {
        diagnostics.push(diagnostic(
            "invalid_labels",
            "error",
            "$.labels",
            "labels must be an array",
            "emit labels:[{id,text,transform?}]",
            None,
            false,
        ));
        return;
    };
    for (index, label) in labels.iter().enumerate() {
        let path = format!("$.labels[{index}]");
        let Some(object) = label.as_object() else {
            diagnostics.push(diagnostic(
                "invalid_label",
                "error",
                &path,
                "label entry must be an object",
                "emit {id, text, transform?}",
                None,
                false,
            ));
            continue;
        };
        validate_known_fields(&path, object, LABEL_FIELDS, diagnostics);
        validate_required_id(&path, object.get("id"), diagnostics);
        let text = object.get("text").and_then(Value::as_str);
        match text {
            Some(text) if !text.trim().is_empty() => {}
            _ => diagnostics.push(diagnostic(
                "invalid_label",
                "error",
                format!("{path}.text"),
                "label text must be a non-empty string",
                "set text to the visible label content",
                None,
                false,
            )),
        }
        if let Some(font) = object.get("font") {
            validate_ref(
                &format!("{path}.font"),
                Some(font),
                fonts,
                "font",
                diagnostics,
            );
            if let Some(text) = text
                && let Some(character) = first_non_basic_latin(text)
            {
                diagnostics.push(diagnostic(
                    "unsupported_feature",
                    "error",
                    format!("{path}.text"),
                    format!(
                        "font-backed labels support basic Latin text only; found U+{:04X}",
                        character as u32
                    ),
                    "render complex-script text in the host or use basic Latin for recipe-authored labels",
                    None,
                    false,
                ));
            }
        }
        if let Some(parent) = object.get("parent") {
            validate_node_ref(
                &format!("{path}.parent"),
                Some(parent),
                target_ids,
                import_ids,
                diagnostics,
            );
        }
        if let Some(transform) = object.get("transform") {
            validate_transform(
                &format!("{path}.transform"),
                transform,
                TransformUse::Node,
                target_ids,
                import_ids,
                diagnostics,
            );
        }
        for field in ["color", "background", "halo"] {
            if let Some(color) = object.get(field) {
                validate_color_value(&format!("{path}.{field}"), Some(color), colors, diagnostics);
            }
        }
        match object.get("size_px").and_then(Value::as_f64) {
            Some(size) if size.is_finite() && size > 0.0 => {}
            Some(_) => diagnostics.push(diagnostic(
                "invalid_label",
                "error",
                format!("{path}.size_px"),
                "label size_px must be finite and positive",
                "use a readable pixel size",
                None,
                false,
            )),
            None if object.contains_key("size_px") => diagnostics.push(diagnostic(
                "invalid_label",
                "error",
                format!("{path}.size_px"),
                "label size_px must be a number",
                "use a readable pixel size",
                None,
                false,
            )),
            None => {}
        }
    }
}

fn first_non_basic_latin(text: &str) -> Option<char> {
    text.chars()
        .find(|character| !matches!(character, '\u{20}'..='\u{7e}'))
}

pub(in crate::scene::recipe::validation::authoring::targets) fn validate_color_value(
    path: &str,
    value: Option<&Value>,
    colors: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    match value.and_then(Value::as_str) {
        Some(value) if colors.contains(value) || Color::from_hex(value).is_ok() => {}
        Some(value) => diagnostics.push(diagnostic(
            "unknown_color_ref",
            "error",
            path,
            format!("color reference '{value}' does not name a declared color"),
            "reference a key from colors or use a direct #RRGGBB value",
            None,
            false,
        )),
        None => diagnostics.push(diagnostic(
            "missing_color_ref",
            "error",
            path,
            "field must be a color id or #RRGGBB string",
            "reference a key from colors or use a direct #RRGGBB value",
            None,
            false,
        )),
    }
}

pub(in crate::scene::recipe::validation::authoring) fn validate_clipping_planes(
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(planes) = value.as_array() else {
        diagnostics.push(diagnostic(
            "invalid_clipping_planes",
            "error",
            "$.clipping_planes",
            "clipping_planes must be an array",
            "emit clipping_planes:[{id,normal,distance}]",
            None,
            false,
        ));
        return;
    };
    for (index, plane) in planes.iter().enumerate() {
        let path = format!("$.clipping_planes[{index}]");
        let Some(object) = plane.as_object() else {
            diagnostics.push(diagnostic(
                "invalid_clipping_plane",
                "error",
                &path,
                "clipping plane entry must be an object",
                "emit {id, normal, distance, active?}",
                None,
                false,
            ));
            continue;
        };
        validate_known_fields(&path, object, CLIPPING_PLANE_FIELDS, diagnostics);
        validate_required_id(&path, object.get("id"), diagnostics);
        validate_vec3(&format!("{path}.normal"), object.get("normal"), diagnostics);
        if let Some(normal) = object.get("normal").and_then(finite_vec3)
            && normal
                .iter()
                .map(|component| component * component)
                .sum::<f32>()
                <= f32::EPSILON
        {
            diagnostics.push(diagnostic(
                "invalid_clipping_plane",
                "error",
                format!("{path}.normal"),
                "clipping plane normal must be non-zero",
                "use a finite non-zero normal vector",
                None,
                false,
            ));
        }
        match object.get("distance").and_then(Value::as_f64) {
            Some(distance) if distance.is_finite() => {}
            _ => diagnostics.push(diagnostic(
                "invalid_clipping_plane",
                "error",
                format!("{path}.distance"),
                "clipping plane distance must be finite",
                "use the plane constant in meters",
                None,
                false,
            )),
        }
        if object
            .get("active")
            .is_some_and(|value| !value.is_boolean())
        {
            diagnostics.push(diagnostic(
                "invalid_clipping_plane",
                "error",
                format!("{path}.active"),
                "clipping plane active must be a boolean",
                "use true or false",
                None,
                false,
            ));
        }
    }
}
