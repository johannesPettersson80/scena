use serde::{Deserialize, Serialize};

pub const ASSET_CONVERSION_SCHEMA_V1: &str = "scena.asset_conversion.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetConversionStatusV1 {
    Planned,
    Converted,
    InvalidRequest,
    ToolUnavailable,
    ConversionFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetConversionDiagnosticStreamV1 {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetConversionDiagnosticSeverityV1 {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetConversionDiagnosticV1 {
    pub stream: AssetConversionDiagnosticStreamV1,
    pub severity: AssetConversionDiagnosticSeverityV1,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetConversionReportV1 {
    pub schema: String,
    pub ok: bool,
    pub status: AssetConversionStatusV1,
    pub workflow: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<AssetConversionDiagnosticV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_exit_code: Option<i32>,
    pub message: String,
}
