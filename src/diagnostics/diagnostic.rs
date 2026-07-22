use serde::{Deserialize, Serialize};

use crate::scene::NodeKey;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub help: Option<String>,
    /// Viewer or renderer setting affected by this diagnostic, when one
    /// setting is the actionable remediation target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setting: Option<String>,
    /// Whether the high-level API already applied a safe fallback while
    /// preserving the warning for the caller.
    #[serde(default, skip_serializing_if = "is_false")]
    pub fallback_applied: bool,
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
    MultisampleFallback,
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

    pub fn setting(&self) -> Option<&str> {
        self.setting.as_deref()
    }

    pub const fn fallback_applied(&self) -> bool {
        self.fallback_applied
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
            setting: None,
            fallback_applied: false,
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
            setting: None,
            fallback_applied: false,
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
            setting: None,
            fallback_applied: false,
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

    pub(crate) fn with_applied_fallback(mut self, setting: impl Into<String>) -> Self {
        self.setting = Some(setting.into());
        self.fallback_applied = true;
        self
    }
}

const fn is_false(value: &bool) -> bool {
    !*value
}

impl DiagnosticContext {
    pub const fn node(self) -> Option<NodeKey> {
        self.node
    }
}
