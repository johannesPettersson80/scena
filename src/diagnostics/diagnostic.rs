#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub help: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCode {
    MissingActiveCamera,
    InvalidCameraProjection,
    ObjectsBehindCamera,
    SceneOutsideCameraFrustum,
    InvisibleScene,
    MissingLightingOrEnvironment,
    LargeScenePrecisionRisk,
    DepthPrecisionRisk,
    WebGl2DepthCompatibility,
    ForwardPbrDegraded,
    DirectionalShadowsDegraded,
    PointShadowsDisabled,
    SpotShadowsDisabled,
    BloomDisabled,
    AmbientOcclusionDisabled,
    GpuCullingDisabled,
    DestructionQueuePressure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

impl Diagnostic {
    pub fn code(&self) -> DiagnosticCode {
        self.code
    }

    pub fn severity(&self) -> DiagnosticSeverity {
        self.severity
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn help(&self) -> Option<&str> {
        self.help.as_deref()
    }

    pub fn suggested_fix(&self) -> Option<&str> {
        self.help()
    }

    pub fn info(code: DiagnosticCode, message: impl Into<String>, help: impl Into<String>) -> Self {
        Self {
            code,
            severity: DiagnosticSeverity::Info,
            message: message.into(),
            help: Some(help.into()),
        }
    }

    pub fn warning(
        code: DiagnosticCode,
        message: impl Into<String>,
        help: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity: DiagnosticSeverity::Warning,
            message: message.into(),
            help: Some(help.into()),
        }
    }

    pub fn error(
        code: DiagnosticCode,
        message: impl Into<String>,
        help: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity: DiagnosticSeverity::Error,
            message: message.into(),
            help: Some(help.into()),
        }
    }
}
