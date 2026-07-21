use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneHostErrorCode {
    Asset,
    Build,
    Import,
    Capture,
    Inspect,
    InvalidInput,
    InvalidViewport,
    Lookup,
    AnimationClipNotFound,
    AnimationHandleNotFound,
    StaleAnimationHandle,
    WrongHandleNamespace,
    NodeHandleNotFound,
    StaleNodeHandle,
    ImportHandleNotFound,
    StaleImportHandle,
    NoActiveCamera,
    Prepare,
    Render,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneHostError {
    code: SceneHostErrorCode,
    message: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    candidates: Vec<String>,
}

impl SceneHostError {
    pub fn new(code: SceneHostErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            candidates: Vec::new(),
        }
    }

    pub const fn code(&self) -> SceneHostErrorCode {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn candidates(&self) -> &[String] {
        &self.candidates
    }

    pub fn with_candidates(mut self, candidates: Vec<String>) -> Self {
        self.candidates = candidates;
        self
    }
}

impl fmt::Display for SceneHostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for SceneHostError {}

impl From<crate::diagnostics::AssetError> for SceneHostError {
    fn from(error: crate::diagnostics::AssetError) -> Self {
        Self::new(SceneHostErrorCode::Asset, error.to_string())
    }
}

impl From<crate::diagnostics::BuildError> for SceneHostError {
    fn from(error: crate::diagnostics::BuildError) -> Self {
        Self::new(SceneHostErrorCode::Build, error.to_string())
    }
}

impl From<crate::diagnostics::ImportError> for SceneHostError {
    fn from(error: crate::diagnostics::ImportError) -> Self {
        Self::new(SceneHostErrorCode::Import, error.to_string())
    }
}

impl From<crate::diagnostics::InstantiateError> for SceneHostError {
    fn from(error: crate::diagnostics::InstantiateError) -> Self {
        Self::new(SceneHostErrorCode::Import, error.to_string())
    }
}

impl From<crate::diagnostics::LookupError> for SceneHostError {
    fn from(error: crate::diagnostics::LookupError) -> Self {
        let candidates = match &error {
            crate::diagnostics::LookupError::NodeNameNotFound { candidates, .. }
            | crate::diagnostics::LookupError::AnchorNotFound { candidates, .. }
            | crate::diagnostics::LookupError::ConnectorNotFound { candidates, .. }
            | crate::diagnostics::LookupError::ClipNotFound { candidates, .. }
            | crate::diagnostics::LookupError::VariantNotFound { candidates, .. } => {
                candidates.clone()
            }
            _ => Vec::new(),
        };
        Self::new(SceneHostErrorCode::Lookup, error.to_string()).with_candidates(candidates)
    }
}

impl From<crate::diagnostics::PrepareError> for SceneHostError {
    fn from(error: crate::diagnostics::PrepareError) -> Self {
        Self::new(SceneHostErrorCode::Prepare, error.to_string())
    }
}

impl From<crate::diagnostics::RenderError> for SceneHostError {
    fn from(error: crate::diagnostics::RenderError) -> Self {
        let code = match &error {
            crate::diagnostics::RenderError::NoActiveCamera => SceneHostErrorCode::NoActiveCamera,
            _ => SceneHostErrorCode::Render,
        };
        Self::new(code, error.to_string())
    }
}

impl From<crate::CaptureError> for SceneHostError {
    fn from(error: crate::CaptureError) -> Self {
        Self::new(SceneHostErrorCode::Capture, error.to_string())
    }
}

impl From<crate::diagnostics::Error> for SceneHostError {
    fn from(error: crate::diagnostics::Error) -> Self {
        match error {
            crate::diagnostics::Error::Build(error) => error.into(),
            crate::diagnostics::Error::Asset(error) => error.into(),
            crate::diagnostics::Error::Import(error) => error.into(),
            crate::diagnostics::Error::Instantiate(error) => error.into(),
            crate::diagnostics::Error::Prepare(error) => error.into(),
            crate::diagnostics::Error::Render(error) => error.into(),
            crate::diagnostics::Error::Lookup(error) => error.into(),
            crate::diagnostics::Error::Animation(error) => {
                Self::new(SceneHostErrorCode::InvalidInput, error.to_string())
            }
        }
    }
}
