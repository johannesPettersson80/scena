use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::SceneRecipeV1;

pub const SCENE_RECIPE_DIFF_SCHEMA_V1: &str = "scena.scene_recipe_diff.v1";

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SceneRecipeDiffOptions {
    pub numeric_tolerance: f64,
}

impl SceneRecipeDiffOptions {
    pub fn new(numeric_tolerance: f64) -> Self {
        Self {
            numeric_tolerance: numeric_tolerance.max(0.0),
        }
    }
}

impl Default for SceneRecipeDiffOptions {
    fn default() -> Self {
        Self::new(1.0e-6)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneRecipeDiffScopeV1 {
    Recipe,
    Material,
    Node,
    Camera,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneRecipeDiffChangeKindV1 {
    Added,
    Removed,
    Modified,
    Reordered,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneRecipeDiffChangeV1 {
    pub scope: SceneRecipeDiffScopeV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub kind: SceneRecipeDiffChangeKindV1,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub order_before: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub order_after: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneRecipeDiffReportV1 {
    pub schema: String,
    pub equal: bool,
    pub numeric_tolerance: f64,
    pub changes: Vec<SceneRecipeDiffChangeV1>,
}

pub fn diff_scene_recipes(
    before: &SceneRecipeV1,
    after: &SceneRecipeV1,
    options: SceneRecipeDiffOptions,
) -> SceneRecipeDiffReportV1 {
    let tolerance = if options.numeric_tolerance.is_finite() {
        options.numeric_tolerance
    } else {
        0.0
    };
    let mut changes = Vec::new();
    diff_items(
        SceneRecipeDiffScopeV1::Material,
        &before.materials,
        &after.materials,
        |value| value.id.as_str(),
        tolerance,
        &mut changes,
    );
    diff_items(
        SceneRecipeDiffScopeV1::Node,
        &before.nodes,
        &after.nodes,
        |value| value.id.as_str(),
        tolerance,
        &mut changes,
    );
    diff_items(
        SceneRecipeDiffScopeV1::Camera,
        &before.cameras,
        &after.cameras,
        |value| value.id.as_str(),
        tolerance,
        &mut changes,
    );
    for (id, left, right) in [
        (
            "scene",
            serde_json::to_value(&before.scene).unwrap_or(Value::Null),
            serde_json::to_value(&after.scene).unwrap_or(Value::Null),
        ),
        (
            "render",
            serde_json::to_value(&before.render).unwrap_or(Value::Null),
            serde_json::to_value(&after.render).unwrap_or(Value::Null),
        ),
        (
            "capture",
            serde_json::to_value(&before.capture).unwrap_or(Value::Null),
            serde_json::to_value(&after.capture).unwrap_or(Value::Null),
        ),
    ] {
        let mut fields = Vec::new();
        diff_value("", &left, &right, tolerance, &mut fields);
        if !fields.is_empty() {
            changes.push(change(
                SceneRecipeDiffScopeV1::Recipe,
                Some(id),
                SceneRecipeDiffChangeKindV1::Modified,
                fields,
            ));
        }
    }
    SceneRecipeDiffReportV1 {
        schema: SCENE_RECIPE_DIFF_SCHEMA_V1.to_owned(),
        equal: changes.is_empty(),
        numeric_tolerance: tolerance,
        changes,
    }
}

fn diff_items<T: Serialize, F: Fn(&T) -> &str>(
    scope: SceneRecipeDiffScopeV1,
    before: &[T],
    after: &[T],
    id: F,
    tolerance: f64,
    changes: &mut Vec<SceneRecipeDiffChangeV1>,
) {
    let before_order = before
        .iter()
        .map(|value| id(value).to_owned())
        .collect::<Vec<_>>();
    let after_order = after
        .iter()
        .map(|value| id(value).to_owned())
        .collect::<Vec<_>>();
    let before_map = before
        .iter()
        .map(|value| (id(value), value))
        .collect::<BTreeMap<_, _>>();
    let after_map = after
        .iter()
        .map(|value| (id(value), value))
        .collect::<BTreeMap<_, _>>();
    for item_id in before_map
        .keys()
        .chain(after_map.keys())
        .copied()
        .collect::<BTreeSet<_>>()
    {
        match (before_map.get(item_id), after_map.get(item_id)) {
            (None, Some(_)) => changes.push(change(
                scope,
                Some(item_id),
                SceneRecipeDiffChangeKindV1::Added,
                Vec::new(),
            )),
            (Some(_), None) => changes.push(change(
                scope,
                Some(item_id),
                SceneRecipeDiffChangeKindV1::Removed,
                Vec::new(),
            )),
            (Some(left), Some(right)) => {
                let mut fields = Vec::new();
                diff_value(
                    "",
                    &serde_json::to_value(left).unwrap_or(Value::Null),
                    &serde_json::to_value(right).unwrap_or(Value::Null),
                    tolerance,
                    &mut fields,
                );
                if !fields.is_empty() {
                    changes.push(change(
                        scope,
                        Some(item_id),
                        SceneRecipeDiffChangeKindV1::Modified,
                        fields,
                    ));
                }
            }
            (None, None) => unreachable!(),
        }
    }
    let shared = before_map
        .keys()
        .filter(|item_id| after_map.contains_key(**item_id))
        .copied()
        .collect::<BTreeSet<_>>();
    let left_common = before_order
        .iter()
        .filter(|id| shared.contains(id.as_str()))
        .collect::<Vec<_>>();
    let right_common = after_order
        .iter()
        .filter(|id| shared.contains(id.as_str()))
        .collect::<Vec<_>>();
    if left_common != right_common {
        let mut reordered = change(
            scope,
            None,
            SceneRecipeDiffChangeKindV1::Reordered,
            Vec::new(),
        );
        reordered.order_before = before_order;
        reordered.order_after = after_order;
        changes.push(reordered);
    }
}

fn change(
    scope: SceneRecipeDiffScopeV1,
    id: Option<&str>,
    kind: SceneRecipeDiffChangeKindV1,
    fields: Vec<String>,
) -> SceneRecipeDiffChangeV1 {
    SceneRecipeDiffChangeV1 {
        scope,
        id: id.map(str::to_owned),
        kind,
        fields,
        order_before: Vec::new(),
        order_after: Vec::new(),
    }
}

fn diff_value(path: &str, before: &Value, after: &Value, tolerance: f64, fields: &mut Vec<String>) {
    match (before, after) {
        (Value::Number(left), Value::Number(right)) => {
            if !left
                .as_f64()
                .zip(right.as_f64())
                .is_some_and(|(left, right)| (left - right).abs() <= tolerance)
            {
                fields.push(path.to_owned());
            }
        }
        (Value::Object(left), Value::Object(right)) => {
            for key in left.keys().chain(right.keys()).collect::<BTreeSet<_>>() {
                if key.as_str() == "id" {
                    continue;
                }
                let child = if path.is_empty() {
                    key.to_string()
                } else {
                    format!("{path}.{key}")
                };
                diff_value(
                    &child,
                    left.get(key).unwrap_or(&Value::Null),
                    right.get(key).unwrap_or(&Value::Null),
                    tolerance,
                    fields,
                );
            }
        }
        (Value::Array(left), Value::Array(right)) if left.len() == right.len() => {
            for (index, (left, right)) in left.iter().zip(right).enumerate() {
                diff_value(&format!("{path}[{index}]"), left, right, tolerance, fields);
            }
        }
        _ if before != after => fields.push(path.to_owned()),
        _ => {}
    }
}
