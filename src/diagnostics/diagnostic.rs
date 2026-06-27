use serde::{Deserialize, Serialize};

use crate::scene::NodeKey;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub help: Option<String>,
    #[serde(skip)]
    context: DiagnosticContext,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DiagnosticContext {
    node: Option<NodeKey>,
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

    pub fn context(&self) -> DiagnosticContext {
        self.context
    }

    pub fn node(&self) -> Option<NodeKey> {
        self.context.node()
    }

    pub fn info(code: DiagnosticCode, message: impl Into<String>, help: impl Into<String>) -> Self {
        Self {
            code,
            severity: DiagnosticSeverity::Info,
            message: message.into(),
            help: Some(help.into()),
            context: DiagnosticContext::default(),
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
            context: DiagnosticContext::default(),
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
            context: DiagnosticContext::default(),
        }
    }

    pub(crate) fn warning_for_node(
        code: DiagnosticCode,
        node: NodeKey,
        message: impl Into<String>,
        help: impl Into<String>,
    ) -> Self {
        Self::warning(code, message, help).with_node(node)
    }

    pub(crate) fn error_for_node(
        code: DiagnosticCode,
        node: NodeKey,
        message: impl Into<String>,
        help: impl Into<String>,
    ) -> Self {
        Self::error(code, message, help).with_node(node)
    }

    fn with_node(mut self, node: NodeKey) -> Self {
        self.context = DiagnosticContext { node: Some(node) };
        self
    }
}

impl DiagnosticContext {
    pub const fn node(self) -> Option<NodeKey> {
        self.node
    }
}
