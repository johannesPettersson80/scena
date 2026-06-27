use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const BROWSER_PROOF_RUN_SCHEMA_V1: &str = "scena.browser_proof_run.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserProofRunV1 {
    pub schema: String,
    pub lane: String,
    pub script: String,
    pub command: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    pub artifact_json_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_png_path: Option<String>,
    pub dry_run: bool,
    pub status: String,
    pub exit_code: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_json_exists: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_png_exists: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout_tail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr_tail: Option<String>,
}

impl BrowserProofRunV1 {
    pub fn dry_run(
        lane: impl Into<String>,
        script: impl Into<String>,
        command: Vec<String>,
        env: BTreeMap<String, String>,
        artifact_json_path: impl Into<String>,
        artifact_png_path: Option<String>,
    ) -> Self {
        Self {
            schema: BROWSER_PROOF_RUN_SCHEMA_V1.to_owned(),
            lane: lane.into(),
            script: script.into(),
            command,
            env,
            artifact_json_path: artifact_json_path.into(),
            artifact_png_path,
            dry_run: true,
            status: "ready".to_owned(),
            exit_code: 0,
            artifact_json_exists: None,
            artifact_png_exists: None,
            stdout_tail: None,
            stderr_tail: None,
        }
    }
}
