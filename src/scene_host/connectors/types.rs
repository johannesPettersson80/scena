use serde::{Deserialize, Serialize};

use crate::{SourceCoordinateSystem, SourceUnits};

pub const CONNECTOR_BROWSER_SCHEMA_V1: &str = "scena.connector_browser.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectorBrowserReportV1 {
    pub schema: String,
    pub scope: ConnectorBrowserScopeV1,
    pub summary: ConnectorBrowserSummaryV1,
    pub connectors: Vec<ConnectorBrowserConnectorV1>,
    pub target_connectors: Vec<ConnectorBrowserConnectorV1>,
    pub candidates: Vec<ConnectorBrowserCandidateV1>,
    pub visual_cues: Vec<ConnectorBrowserVisualCueV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectorBrowserScopeV1 {
    pub kind: String,
    pub import: Option<u64>,
    pub root: Option<u64>,
    pub selection: Vec<u64>,
    pub target_imports: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectorBrowserSummaryV1 {
    pub connector_count: usize,
    pub target_connector_count: usize,
    pub candidate_count: usize,
    pub compatible_count: usize,
    pub snap_ready_count: usize,
    pub invalid_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectorBrowserConnectorV1 {
    pub id: String,
    pub name: String,
    pub node: u64,
    pub placement_node: u64,
    pub import: Option<u64>,
    pub kind: Option<String>,
    pub allowed_mates: Vec<String>,
    pub tags: Vec<String>,
    pub snap_tolerance: Option<f64>,
    pub clearance_hint: Option<f64>,
    pub roll_policy: String,
    pub polarity: Option<String>,
    pub source_units: SourceUnits,
    pub source_coordinate_system: SourceCoordinateSystem,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectorBrowserCandidateV1 {
    pub source_id: String,
    pub source_name: String,
    pub target_id: String,
    pub target_name: String,
    pub compatible: bool,
    pub snap_ready: bool,
    pub distance: Option<f64>,
    pub tolerance: Option<f64>,
    pub visual_cue: Option<String>,
    pub ghost_transform: Option<ConnectorTransformV1>,
    pub connection_line: Option<ConnectorLineV1>,
    pub invalid_reasons: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectorTransformV1 {
    pub translation: [f64; 3],
    pub rotation: [f64; 4],
    pub scale: [f64; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ConnectorLineV1 {
    pub start: [f64; 3],
    pub end: [f64; 3],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectorBrowserVisualCueV1 {
    pub candidate: String,
    pub kind: String,
    pub style: String,
}
