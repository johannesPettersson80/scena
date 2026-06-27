use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::visual_patch::{VISUAL_PATCH_SCHEMA_V1, VisualPatchResultV1, VisualPatchV1};
use super::{SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::AssetFetcher;

pub const SCENE_HOST_VISUAL_STATE_SCHEMA_V1: &str = "scena.scene_host_visual_state.v1";
pub const SCENE_HOST_VISUAL_STATES_SCHEMA_V1: &str = "scena.scene_host_visual_states.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneHostVisualStateV1 {
    pub schema: String,
    pub name: String,
    pub patch: VisualPatchV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneHostVisualStatesReportV1 {
    pub schema: String,
    pub states: Vec<SceneHostVisualStateSummaryV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneHostVisualStateSummaryV1 {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

impl SceneHostVisualStateV1 {
    pub fn new(name: impl Into<String>, patch: VisualPatchV1) -> Self {
        Self {
            schema: SCENE_HOST_VISUAL_STATE_SCHEMA_V1.to_owned(),
            name: name.into(),
            patch,
            metadata: None,
        }
    }

    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn store_visual_state(
        &mut self,
        state: SceneHostVisualStateV1,
    ) -> Result<SceneHostVisualStateV1, SceneHostError> {
        validate_visual_state(&state)?;
        self.visual_states.insert(state.name.clone(), state.clone());
        Ok(state)
    }

    pub fn store_visual_state_json(&mut self, json: &str) -> Result<String, SceneHostError> {
        let state: SceneHostVisualStateV1 = serde_json::from_str(json).map_err(|error| {
            SceneHostError::new(
                SceneHostErrorCode::InvalidInput,
                format!("invalid visual state JSON: {error}"),
            )
        })?;
        let stored = self.store_visual_state(state)?;
        serde_json::to_string(&stored).map_err(serialization_error)
    }

    pub fn visual_state(&self, name: &str) -> Result<&SceneHostVisualStateV1, SceneHostError> {
        validate_visual_state_name(name)?;
        self.visual_states.get(name).ok_or_else(|| {
            SceneHostError::new(
                SceneHostErrorCode::InvalidInput,
                format!("visual state '{name}' is not stored"),
            )
        })
    }

    pub fn visual_state_json(&self, name: &str) -> Result<String, SceneHostError> {
        serde_json::to_string(self.visual_state(name)?).map_err(serialization_error)
    }

    pub fn visual_states(&self) -> SceneHostVisualStatesReportV1 {
        visual_states_report(&self.visual_states)
    }

    pub fn visual_states_json(&self) -> Result<String, SceneHostError> {
        serde_json::to_string(&self.visual_states()).map_err(serialization_error)
    }

    pub fn apply_visual_state(
        &mut self,
        name: &str,
    ) -> Result<VisualPatchResultV1, SceneHostError> {
        let patch = self.visual_state(name)?.patch.clone();
        self.apply_patch(&patch)
    }

    pub fn apply_visual_state_json(&mut self, name: &str) -> Result<String, SceneHostError> {
        let result = self.apply_visual_state(name)?;
        serde_json::to_string(&result).map_err(serialization_error)
    }
}

fn validate_visual_state(state: &SceneHostVisualStateV1) -> Result<(), SceneHostError> {
    if state.schema != SCENE_HOST_VISUAL_STATE_SCHEMA_V1 {
        return Err(SceneHostError::new(
            SceneHostErrorCode::InvalidInput,
            format!(
                "unsupported visual state schema {}; expected {}",
                state.schema, SCENE_HOST_VISUAL_STATE_SCHEMA_V1
            ),
        ));
    }
    validate_visual_state_name(&state.name)?;
    if state.patch.schema != VISUAL_PATCH_SCHEMA_V1 {
        return Err(SceneHostError::new(
            SceneHostErrorCode::InvalidInput,
            format!(
                "visual state patch schema must be {}; got {}",
                VISUAL_PATCH_SCHEMA_V1, state.patch.schema
            ),
        ));
    }
    Ok(())
}

fn validate_visual_state_name(name: &str) -> Result<(), SceneHostError> {
    if !name.is_empty() && !name.chars().any(char::is_control) {
        Ok(())
    } else {
        Err(SceneHostError::new(
            SceneHostErrorCode::InvalidInput,
            "visual state name must be non-empty and contain no control characters",
        ))
    }
}

fn visual_states_report(
    states: &BTreeMap<String, SceneHostVisualStateV1>,
) -> SceneHostVisualStatesReportV1 {
    SceneHostVisualStatesReportV1 {
        schema: SCENE_HOST_VISUAL_STATES_SCHEMA_V1.to_owned(),
        states: states
            .values()
            .map(|state| SceneHostVisualStateSummaryV1 {
                name: state.name.clone(),
                metadata: state.metadata.clone(),
            })
            .collect(),
    }
}

fn serialization_error(error: serde_json::Error) -> SceneHostError {
    SceneHostError::new(
        SceneHostErrorCode::Inspect,
        format!("visual state serialization failed: {error}"),
    )
}
