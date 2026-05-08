use std::collections::BTreeSet;

use serde_json::Value as JsonValue;

use crate::scene::{ConnectorMetadata, ConnectorPolarity, ConnectorRollPolicy, Transform};

use super::parse_node_transform;

#[derive(Debug, Clone, PartialEq)]
pub struct SceneAssetConnector {
    name: String,
    kind: Option<String>,
    allowed_mates: Vec<String>,
    tags: BTreeSet<String>,
    snap_tolerance: Option<f32>,
    clearance_hint: Option<f32>,
    roll_policy: ConnectorRollPolicy,
    polarity: Option<ConnectorPolarity>,
    metadata: Option<ConnectorMetadata>,
    transform: Transform,
    invalid_reason: Option<String>,
}

impl SceneAssetConnector {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn kind(&self) -> Option<&str> {
        self.kind.as_deref()
    }

    pub fn allowed_mates(&self) -> Vec<&str> {
        self.allowed_mates.iter().map(String::as_str).collect()
    }

    pub fn tags(&self) -> &BTreeSet<String> {
        &self.tags
    }

    pub const fn snap_tolerance(&self) -> Option<f32> {
        self.snap_tolerance
    }

    pub const fn clearance_hint(&self) -> Option<f32> {
        self.clearance_hint
    }

    pub const fn roll_policy(&self) -> ConnectorRollPolicy {
        self.roll_policy
    }

    pub const fn polarity(&self) -> Option<ConnectorPolarity> {
        self.polarity
    }

    pub const fn metadata(&self) -> Option<&ConnectorMetadata> {
        self.metadata.as_ref()
    }

    pub fn transform(&self) -> Transform {
        self.transform
    }

    pub(crate) fn invalid_reason(&self) -> Option<&str> {
        self.invalid_reason.as_deref()
    }
}

pub(super) fn parse_node_connectors(node: &JsonValue) -> Vec<SceneAssetConnector> {
    node.get("extras")
        .and_then(|extras| extras.get("scena"))
        .and_then(|scena| scena.get("connectors"))
        .and_then(JsonValue::as_array)
        .map(|connectors| {
            connectors
                .iter()
                .map(|connector| SceneAssetConnector {
                    name: connector
                        .get("name")
                        .and_then(JsonValue::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    kind: connector
                        .get("kind")
                        .and_then(JsonValue::as_str)
                        .map(str::to_string),
                    allowed_mates: parse_string_array(connector, "allowedMates", "allowed_mates"),
                    tags: parse_string_array(connector, "tags", "tags")
                        .into_iter()
                        .collect(),
                    snap_tolerance: parse_non_negative_f32(
                        connector,
                        "snapTolerance",
                        "snap_tolerance",
                    ),
                    clearance_hint: parse_non_negative_f32(
                        connector,
                        "clearanceHint",
                        "clearance_hint",
                    ),
                    roll_policy: parse_roll_policy(connector)
                        .unwrap_or(ConnectorRollPolicy::Preserve),
                    polarity: parse_polarity(connector),
                    metadata: connector
                        .get("metadata")
                        .filter(|metadata| metadata.is_object())
                        .cloned()
                        .map(ConnectorMetadata::new),
                    transform: parse_node_transform(connector),
                    invalid_reason: validate_connector_extras(connector),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn validate_connector_extras(connector: &JsonValue) -> Option<String> {
    match connector.get("name").and_then(JsonValue::as_str) {
        Some(name) if !name.trim().is_empty() => {}
        Some(_) => return Some("connector name must not be empty".to_string()),
        None => return Some("connector name must be a string".to_string()),
    }
    if connector
        .get("kind")
        .and_then(JsonValue::as_str)
        .is_some_and(|kind| kind.trim().is_empty())
    {
        return Some("connector kind must not be empty when present".to_string());
    }
    if let Some(reason) = validate_string_array(connector, "allowedMates", "allowed_mates") {
        return Some(reason);
    }
    if let Some(reason) = validate_string_array(connector, "tags", "tags") {
        return Some(reason);
    }
    if let Some(reason) = validate_non_negative_f32(connector, "snapTolerance", "snap_tolerance") {
        return Some(reason);
    }
    if let Some(reason) = validate_non_negative_f32(connector, "clearanceHint", "clearance_hint") {
        return Some(reason);
    }
    if connector
        .get("rollPolicy")
        .or_else(|| connector.get("roll_policy"))
        .is_some()
        && parse_roll_policy(connector).is_none()
    {
        return Some(
            "connector rollPolicy must be preserve, chooseNearest, or explicitAngle".to_string(),
        );
    }
    if connector.get("polarity").is_some() && parse_polarity(connector).is_none() {
        return Some("connector polarity must be plug, socket, or neutral".to_string());
    }
    if connector
        .get("metadata")
        .is_some_and(|metadata| !metadata.is_object())
    {
        return Some("connector metadata must be a JSON object when present".to_string());
    }
    None
}

fn parse_string_array(connector: &JsonValue, camel: &str, snake: &str) -> Vec<String> {
    connector
        .get(camel)
        .or_else(|| connector.get(snake))
        .and_then(JsonValue::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(JsonValue::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn validate_string_array(connector: &JsonValue, camel: &str, snake: &str) -> Option<String> {
    let field = connector.get(camel).or_else(|| connector.get(snake))?;
    let Some(values) = field.as_array() else {
        return Some(format!(
            "connector {camel} must be an array of non-empty strings"
        ));
    };
    if values
        .iter()
        .any(|value| !matches!(value.as_str(), Some(text) if !text.trim().is_empty()))
    {
        return Some(format!(
            "connector {camel} must be an array of non-empty strings"
        ));
    }
    None
}

fn parse_non_negative_f32(connector: &JsonValue, camel: &str, snake: &str) -> Option<f32> {
    connector
        .get(camel)
        .or_else(|| connector.get(snake))
        .and_then(JsonValue::as_f64)
        .filter(|value| value.is_finite() && *value >= 0.0)
        .map(|value| value as f32)
}

fn validate_non_negative_f32(connector: &JsonValue, camel: &str, snake: &str) -> Option<String> {
    if connector
        .get(camel)
        .or_else(|| connector.get(snake))
        .is_some()
        && parse_non_negative_f32(connector, camel, snake).is_none()
    {
        return Some(format!(
            "connector {camel} must be a finite non-negative number"
        ));
    }
    None
}

fn parse_roll_policy(connector: &JsonValue) -> Option<ConnectorRollPolicy> {
    match connector
        .get("rollPolicy")
        .or_else(|| connector.get("roll_policy"))
        .and_then(JsonValue::as_str)?
    {
        "preserve" => Some(ConnectorRollPolicy::Preserve),
        "chooseNearest" | "choose_nearest" | "choose-nearest" => {
            Some(ConnectorRollPolicy::ChooseNearest)
        }
        "explicitAngle" | "explicit_angle" | "explicit-angle" => {
            Some(ConnectorRollPolicy::ExplicitAngle)
        }
        _ => None,
    }
}

fn parse_polarity(connector: &JsonValue) -> Option<ConnectorPolarity> {
    match connector.get("polarity").and_then(JsonValue::as_str)? {
        "plug" => Some(ConnectorPolarity::Plug),
        "socket" => Some(ConnectorPolarity::Socket),
        "neutral" => Some(ConnectorPolarity::Neutral),
        _ => None,
    }
}
