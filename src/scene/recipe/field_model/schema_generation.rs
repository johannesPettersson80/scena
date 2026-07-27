use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};

use super::SchemaFieldV1;

pub(super) fn field_owner() -> String {
    "scene_recipe_v1".to_owned()
}

pub(super) fn collect_fields(
    root: &Value,
    schema: &Value,
    path: &str,
    fields: &mut BTreeMap<String, SchemaFieldV1>,
) {
    let schema = resolve_schema(root, schema);
    for combinator in ["allOf", "anyOf", "oneOf"] {
        if let Some(branches) = schema.get(combinator).and_then(Value::as_array) {
            for branch in branches {
                collect_fields(root, branch, path, fields);
            }
        }
    }

    if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
        let required = schema
            .get("required")
            .and_then(Value::as_array)
            .map(|names| {
                names
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default();
        for (name, child) in properties {
            let child_path = format!("{path}.{name}");
            insert_schema_field(
                root,
                child,
                &child_path,
                required.contains(name.as_str()),
                fields,
            );
            collect_fields(root, child, &child_path, fields);
        }
    }

    if let Some(items) = schema.get("items") {
        collect_fields(root, items, &format!("{path}[]"), fields);
    }
    if let Some(additional) = schema.get("additionalProperties")
        && !additional.is_boolean()
    {
        collect_fields(root, additional, &format!("{path}.*"), fields);
    }
}

fn insert_schema_field(
    root: &Value,
    schema: &Value,
    path: &str,
    required: bool,
    fields: &mut BTreeMap<String, SchemaFieldV1>,
) {
    let resolved = resolve_schema(root, schema);
    let candidate = SchemaFieldV1 {
        path: path.to_owned(),
        value_type: schema_value_type(resolved),
        required,
        enum_values: schema_enum_values(resolved),
        minimum: resolved.get("minimum").and_then(Value::as_f64),
        maximum: resolved.get("maximum").and_then(Value::as_f64),
        default: resolved.get("default").cloned(),
        deprecated: resolved
            .get("deprecated")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        examples: schema_examples(resolved),
        owner: "scene_recipe_v1".to_owned(),
        feature_requirements: Vec::new(),
        constraints: Vec::new(),
    };
    fields
        .entry(path.to_owned())
        .and_modify(|current| merge_schema_field(current, &candidate))
        .or_insert(candidate);
}

fn resolve_schema<'a>(root: &'a Value, schema: &'a Value) -> &'a Value {
    let Some(reference) = schema.get("$ref").and_then(Value::as_str) else {
        return schema;
    };
    reference
        .strip_prefix('#')
        .and_then(|pointer| root.pointer(pointer))
        .unwrap_or(schema)
}

fn schema_value_type(schema: &Value) -> String {
    if let Some(value_type) = schema.get("type").and_then(Value::as_str) {
        return value_type.to_owned();
    }
    if schema.get("properties").is_some() {
        return "object".to_owned();
    }
    if schema.get("items").is_some() {
        return "array".to_owned();
    }
    if let Some(branches) = schema
        .get("oneOf")
        .or_else(|| schema.get("anyOf"))
        .and_then(Value::as_array)
    {
        let types = branches
            .iter()
            .map(schema_value_type)
            .filter(|value| value != "unknown" && value != "null")
            .collect::<BTreeSet<_>>();
        if types.len() == 1 {
            return types.into_iter().next().expect("one inferred type");
        }
        if !types.is_empty() {
            return types.into_iter().collect::<Vec<_>>().join("|");
        }
    }
    "unknown".to_owned()
}

fn schema_enum_values(schema: &Value) -> Vec<Value> {
    let mut values = schema
        .get("enum")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if let Some(value) = schema.get("const") {
        values.push(value.clone());
    }
    for combinator in ["allOf", "anyOf", "oneOf"] {
        if let Some(branches) = schema.get(combinator).and_then(Value::as_array) {
            for branch in branches {
                values.extend(schema_enum_values(branch));
            }
        }
    }
    values.sort_by_key(Value::to_string);
    values.dedup();
    values
}

fn schema_examples(schema: &Value) -> Vec<Value> {
    if let Some(examples) = schema.get("examples").and_then(Value::as_array)
        && !examples.is_empty()
    {
        return examples.clone();
    }
    if let Some(default) = schema.get("default") {
        return vec![default.clone()];
    }
    if let Some(value) = schema_enum_values(schema).into_iter().next() {
        return vec![value];
    }
    vec![match schema_value_type(schema).as_str() {
        "string" => json!(""),
        "integer" => json!(0),
        "number" => json!(0.0),
        "boolean" => json!(false),
        "array" => json!([]),
        "object" => json!({}),
        _ => Value::Null,
    }]
}

fn merge_schema_field(current: &mut SchemaFieldV1, candidate: &SchemaFieldV1) {
    current.required |= candidate.required;
    if current.value_type == "unknown" {
        current.value_type.clone_from(&candidate.value_type);
    } else if current.value_type != candidate.value_type && candidate.value_type != "unknown" {
        let mut types = current
            .value_type
            .split('|')
            .chain(candidate.value_type.split('|'))
            .collect::<BTreeSet<_>>();
        types.remove("null");
        current.value_type = types.into_iter().collect::<Vec<_>>().join("|");
    }
    current.enum_values.extend(candidate.enum_values.clone());
    current.enum_values.sort_by_key(Value::to_string);
    current.enum_values.dedup();
    current.minimum = current.minimum.or(candidate.minimum);
    current.maximum = current.maximum.or(candidate.maximum);
    current.default = current
        .default
        .clone()
        .or_else(|| candidate.default.clone());
    current.deprecated |= candidate.deprecated;
}

pub(super) fn apply_cross_field_metadata(fields: &mut BTreeMap<String, SchemaFieldV1>) {
    for field in fields.values_mut() {
        if field.path.starts_with("$.render.") {
            field.feature_requirements.push(
                "backend capability must report support or an explicit degradation".to_owned(),
            );
        }
        if field.path.contains("texture") && field.path.ends_with(".color_space") {
            field.constraints.push(
                "color space must match the material slot semantics; color slots are sRGB and data slots are linear"
                    .to_owned(),
            );
        }
    }
    for (path, constraint) in [
        (
            "$.geometries[].primitive",
            "exactly one of primitive or mesh is required per geometry",
        ),
        (
            "$.geometries[].mesh",
            "exactly one of primitive or mesh is required per geometry",
        ),
        (
            "$.cameras[].fov_degrees",
            "perspective cameras require a valid lens or fov; conflicting values are rejected",
        ),
        (
            "$.scene.environment.uri",
            "required when environment.kind is uri and resolved by recipe policy",
        ),
        (
            "$.render.auto_exposure",
            "manual exposure_ev and automatic exposure are mutually planned by validation",
        ),
        (
            "$.render.exposure_compensation_ev",
            "valid only with auto_exposure; use exposure_ev for full manual exposure",
        ),
        (
            "$.render.metering.mode",
            "valid only with auto_exposure; subject mode requires target and spot mode requires rect",
        ),
        (
            "$.render.metering.fallback",
            "subject metering defaults to error; average_metering_with_warning allows an explicit degraded fallback",
        ),
        (
            "$.photo.subject.fallback",
            "photo subject fallback defaults to error; average_metering_with_warning allows an explicit degraded fallback",
        ),
        (
            "$.photo.composition",
            "camera_behavior composition fields are policy constraints, not hidden final camera constants",
        ),
        (
            "$.photo.exposure",
            "camera_behavior exposure fields are acceptance bands; fixed exposure_ev remains rejected in strict easy mode",
        ),
        (
            "$.photo.focus",
            "camera_behavior focus fields request subject focus policy; manual focus_distance remains rejected in strict easy mode",
        ),
        (
            "$.photo.staging",
            "camera_behavior staging owns background/grid/ground defaults; manual grid and literal background colors are rejected in strict easy mode",
        ),
    ] {
        if let Some(field) = fields.get_mut(path) {
            field.constraints.push(constraint.to_owned());
        }
    }
}
