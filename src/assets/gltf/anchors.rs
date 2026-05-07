use serde_json::Value as JsonValue;

use super::{SceneAssetAnchor, parse_node_transform};

pub(super) fn parse_node_anchors(node: &JsonValue) -> Vec<SceneAssetAnchor> {
    node.get("extras")
        .and_then(|extras| extras.get("scena"))
        .and_then(|scena| scena.get("anchors"))
        .and_then(JsonValue::as_array)
        .map(|anchors| {
            anchors
                .iter()
                .map(|anchor| SceneAssetAnchor {
                    name: anchor
                        .get("name")
                        .and_then(JsonValue::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    transform: parse_node_transform(anchor),
                    invalid_reason: validate_anchor_extras(anchor),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn validate_anchor_extras(anchor: &JsonValue) -> Option<String> {
    match anchor.get("name").and_then(JsonValue::as_str) {
        Some(name) if !name.trim().is_empty() => {}
        Some(_) => return Some("anchor name must not be empty".to_string()),
        None => return Some("anchor name must be a string".to_string()),
    }

    for field in ["translation", "scale"] {
        let values = match validate_number_array(anchor, field, 3) {
            Ok(values) => values,
            Err(reason) => return Some(reason),
        };
        if field == "scale"
            && values
                .as_deref()
                .is_some_and(|values| values.contains(&0.0))
        {
            return Some("anchor scale components must not be zero".to_string());
        }
    }

    let rotation = match validate_number_array(anchor, "rotation", 4) {
        Ok(rotation) => rotation,
        Err(reason) => return Some(reason),
    };
    if let Some(rotation) = rotation {
        let length_squared = rotation.iter().map(|value| value * value).sum::<f32>();
        if (length_squared.sqrt() - 1.0).abs() > 1e-3 {
            return Some("anchor rotation quaternion must be normalized".to_string());
        }
    }

    None
}

fn validate_number_array(
    anchor: &JsonValue,
    field: &str,
    expected_len: usize,
) -> Result<Option<Vec<f32>>, String> {
    let Some(value) = anchor.get(field) else {
        return Ok(None);
    };
    let values = value
        .as_array()
        .ok_or_else(|| format!("anchor {field} must be an array"))?;
    if values.len() != expected_len {
        return Err(format!(
            "anchor {field} must contain {expected_len} numeric components"
        ));
    }
    values
        .iter()
        .map(|value| {
            value
                .as_f64()
                .map(|value| value as f32)
                .filter(|value| value.is_finite())
                .ok_or_else(|| format!("anchor {field} components must be finite numbers"))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}
