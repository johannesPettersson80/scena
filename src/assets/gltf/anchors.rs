use std::collections::BTreeSet;

use serde_json::Value as JsonValue;

use crate::scene::{SourceUnits, Transform};

use super::parse_node_transform;

#[derive(Debug, Clone, PartialEq)]
pub struct SceneAssetAnchor {
    name: String,
    tags: BTreeSet<String>,
    label: Option<String>,
    source_units: Option<SourceUnits>,
    transform: Transform,
    invalid_reason: Option<String>,
}

impl SceneAssetAnchor {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn tags(&self) -> &BTreeSet<String> {
        &self.tags
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub const fn source_units(&self) -> Option<SourceUnits> {
        self.source_units
    }

    pub fn transform(&self) -> Transform {
        self.transform
    }

    pub(crate) fn invalid_reason(&self) -> Option<&str> {
        self.invalid_reason.as_deref()
    }
}

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
                    tags: parse_tags(anchor),
                    label: anchor
                        .get("label")
                        .and_then(JsonValue::as_str)
                        .map(str::to_string),
                    source_units: parse_source_units(anchor),
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
    if let Some(reason) = validate_tags(anchor) {
        return Some(reason);
    }
    if anchor
        .get("label")
        .is_some_and(|label| !matches!(label.as_str(), Some(text) if !text.trim().is_empty()))
    {
        return Some("anchor label must be a non-empty string when present".to_string());
    }
    if anchor.get("units").is_some() && parse_source_units(anchor).is_none() {
        return Some(
            "anchor units must be meters, centimeters, millimeters, inches, or feet".to_string(),
        );
    }

    None
}

fn parse_source_units(anchor: &JsonValue) -> Option<SourceUnits> {
    match anchor.get("units").and_then(JsonValue::as_str)? {
        "meter" | "meters" | "m" => Some(SourceUnits::Meters),
        "centimeter" | "centimeters" | "cm" => Some(SourceUnits::Centimeters),
        "millimeter" | "millimeters" | "mm" => Some(SourceUnits::Millimeters),
        "inch" | "inches" | "in" => Some(SourceUnits::Inches),
        "foot" | "feet" | "ft" => Some(SourceUnits::Feet),
        _ => None,
    }
}

fn parse_tags(anchor: &JsonValue) -> BTreeSet<String> {
    anchor
        .get("tags")
        .and_then(JsonValue::as_array)
        .map(|tags| {
            tags.iter()
                .filter_map(JsonValue::as_str)
                .filter(|tag| !tag.trim().is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn validate_tags(anchor: &JsonValue) -> Option<String> {
    let tags = anchor.get("tags")?;
    let Some(tags) = tags.as_array() else {
        return Some("anchor tags must be an array of non-empty strings".to_string());
    };
    if tags
        .iter()
        .any(|tag| !matches!(tag.as_str(), Some(text) if !text.trim().is_empty()))
    {
        return Some("anchor tags must be an array of non-empty strings".to_string());
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
