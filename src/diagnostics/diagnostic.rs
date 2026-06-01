use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub help: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
    MaterialPresetFallback,
    DirectionalShadowsDegraded,
    PointShadowsDisabled,
    SpotShadowsDisabled,
    BloomDisabled,
    AmbientOcclusionDisabled,
    OrderIndependentTransparencyDisabled,
    PhysicalGlassTransmissionDegraded,
    WideGamutOutputUnavailable,
    GpuCullingDisabled,
    MaterialTextureMissingDecodedPixels,
    DestructionQueuePressure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
