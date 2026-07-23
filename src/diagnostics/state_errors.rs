/// Uniform, serializable recovery guidance for public scena errors.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ErrorDiagnostic {
    pub code: String,
    pub message: String,
    pub help: String,
    pub context: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotPreparedReason {
    NeverPrepared,
    DifferentScene,
    SceneChanged {
        prepared_revision: u64,
        current_revision: u64,
        change: ChangeKind,
    },
    EnvironmentChanged {
        prepared_revision: u64,
        current_revision: u64,
        change: ChangeKind,
    },
    TargetChanged {
        prepared_revision: u64,
        current_revision: u64,
        change: ChangeKind,
    },
    OutputSettingsChanged {
        prepared_revision: u64,
        current_revision: u64,
        change: ChangeKind,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    SceneStructure,
    Transform,
    Camera,
    Appearance,
    Visibility,
    Environment,
    RenderTarget,
    OutputSettings,
}
