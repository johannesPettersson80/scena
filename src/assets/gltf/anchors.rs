//! Stage C2: anchor extras parsing now reads from
//! `gltf::Node::extras()`, deserialized into `serde_json::Value` (the
//! gltf crate exposes extras as `Option<Box<RawValue>>`).

use std::collections::BTreeSet;

use ::gltf::Node;
use serde_json::Value as JsonValue;

use crate::assets::AssetPath;
use crate::diagnostics::AssetError;
use crate::scene::{SourceUnits, Transform};

use super::transform::parse_marker_transform;

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

    #[cfg(test)]
    pub(crate) fn invalid_for_transaction_test(reason: &str) -> Self {
        Self {
            name: "invalid-anchor".to_owned(),
            tags: BTreeSet::new(),
            label: None,
            source_units: None,
            transform: Transform::IDENTITY,
            invalid_reason: Some(reason.to_owned()),
        }
    }
}

pub(super) fn parse_node_anchors(
    path: &AssetPath,
    node: &Node,
) -> Result<Vec<SceneAssetAnchor>, AssetError> {
    let Some(raw_extras) = node.extras().as_ref() else {
        return Ok(Vec::new());
    };
    let extras =
        serde_json::from_str::<JsonValue>(raw_extras.get()).map_err(|error| AssetError::Parse {
            path: path.as_str().to_owned(),
            reason: format!("nodes[{}].extras is invalid JSON: {error}", node.index()),
        })?;
    let Some(anchors) = extras.get("scena").and_then(|scena| scena.get("anchors")) else {
        return Ok(Vec::new());
    };
    let anchors = anchors.as_array().ok_or_else(|| AssetError::Parse {
        path: path.as_str().to_owned(),
        reason: format!(
            "nodes[{}].extras.scena.anchors must be an array",
            node.index()
        ),
    })?;
    anchors
        .iter()
        .enumerate()
        .map(|(index, anchor)| {
            let marker_path = format!("nodes[{}].extras.scena.anchors[{index}]", node.index());
            if let Some(reason) = validate_anchor_extras(anchor) {
                return Err(AssetError::Parse {
                    path: path.as_str().to_owned(),
                    reason: format!("{marker_path}: {reason}"),
                });
            }
            let transform = parse_marker_transform(anchor, &marker_path).map_err(|reason| {
                AssetError::Parse {
                    path: path.as_str().to_owned(),
                    reason,
                }
            })?;
            Ok(SceneAssetAnchor {
                name: anchor
                    .get("name")
                    .and_then(JsonValue::as_str)
                    .expect("validated anchor name")
                    .to_string(),
                tags: parse_tags(anchor),
                label: anchor
                    .get("label")
                    .and_then(JsonValue::as_str)
                    .map(str::to_string),
                source_units: parse_source_units(anchor),
                transform,
                invalid_reason: None,
            })
        })
        .collect()
}

fn validate_anchor_extras(anchor: &JsonValue) -> Option<String> {
    match anchor.get("name").and_then(JsonValue::as_str) {
        Some(name) if !name.trim().is_empty() => {}
        Some(_) => return Some("anchor name must not be empty".to_string()),
        None => return Some("anchor name must be a string".to_string()),
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
