use super::serialization::stable_transform;
use super::{ScenePlacementDiagnosticV1, SceneRecipeSemanticChangeV1, Transform};

impl SceneRecipeSemanticChangeV1 {
    pub fn transform(path: impl Into<String>, before: Option<Transform>, after: Transform) -> Self {
        Self {
            path: path.into(),
            operation: "replace".to_owned(),
            before: serde_json::to_value(before.map(stable_transform))
                .expect("stable transform serialization is infallible"),
            after: serde_json::to_value(stable_transform(after))
                .expect("stable transform serialization is infallible"),
        }
    }
}

impl ScenePlacementDiagnosticV1 {
    pub fn new(
        code: impl Into<String>,
        path: impl Into<String>,
        message: impl Into<String>,
        help: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity: "error".to_owned(),
            path: path.into(),
            message: message.into(),
            help: help.into(),
            suggestion: None,
            candidates: Vec::new(),
            auto_fixable: false,
        }
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    pub fn with_candidates(mut self, candidates: Vec<String>) -> Self {
        self.candidates = candidates;
        self
    }
}
