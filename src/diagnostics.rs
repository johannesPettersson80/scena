//! Structured errors, debug overlays, capability reports, and renderer stats.

use crate::assets::{EnvironmentHandle, GeometryHandle, MaterialHandle};
use crate::geometry::GeometryTopology;
use crate::material::{AlphaMode, MaterialKind};
use crate::scene::{CameraKey, ClippingPlaneKey, InstanceSetKey, NodeKey};

mod capabilities;
mod display;
pub use capabilities::{
    AlphaPipelineStatus, Backend, Capabilities, CapabilityStatus, OutputStageStatus,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    Build(BuildError),
    Asset(AssetError),
    Import(ImportError),
    Instantiate(InstantiateError),
    Prepare(PrepareError),
    Render(RenderError),
    Lookup(LookupError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    InvalidTargetSize { width: u32, height: u32 },
    AsyncSurfaceRequired { backend: Backend },
    CreateSurface { backend: Backend },
    NoAdapter { backend: Backend },
    RequestDevice { backend: Backend },
    SurfaceUnsupported { backend: Backend },
    UnsupportedBackend { backend: Backend },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetError {
    NotFound {
        path: String,
    },
    Io {
        path: String,
        reason: String,
    },
    Parse {
        path: String,
        reason: String,
    },
    UnsupportedRequiredExtension {
        path: String,
        extension: String,
    },
    UnsupportedOptionalExtensionUsed {
        path: String,
        extension: String,
        help: String,
    },
    UnsupportedEnvironmentFormat {
        path: String,
        help: &'static str,
    },
    ReloadRequiresRetain {
        path: String,
        help: &'static str,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportError {
    Asset(AssetError),
    Instantiate(InstantiateError),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PrepareError {
    InvalidTargetSize {
        width: u32,
        height: u32,
    },
    AssetsRequired {
        node: NodeKey,
    },
    GeometryNotFound {
        node: NodeKey,
        geometry: GeometryHandle,
    },
    MaterialNotFound {
        node: NodeKey,
        material: MaterialHandle,
    },
    EnvironmentAssetsRequired {
        environment: EnvironmentHandle,
    },
    EnvironmentNotFound {
        environment: EnvironmentHandle,
    },
    UnsupportedGeometryTopology {
        node: NodeKey,
        topology: GeometryTopology,
    },
    UnsupportedMaterialKind {
        node: NodeKey,
        kind: MaterialKind,
    },
    UnsupportedAlphaMode {
        node: NodeKey,
        alpha_mode: AlphaMode,
    },
    UnsupportedModelNode {
        node: NodeKey,
    },
    MultipleShadowedDirectionalLights {
        first: NodeKey,
        second: NodeKey,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    NotPrepared { reason: NotPreparedReason },
    NoActiveCamera,
    CameraNotFound(CameraKey),
    InvalidSurfaceSize { width: u32, height: u32 },
    GpuResourcesNotPrepared { backend: Backend },
    GpuReadback { backend: Backend },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstantiateError {
    InvalidChildIndex { parent: usize, child: usize },
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    SceneStructure,
    Environment,
    RenderTarget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LookupError {
    NodeNotFound(NodeKey),
    NodeNameNotFound { name: String },
    AmbiguousNodeName { name: String, matches: Vec<NodeKey> },
    PathNotFound { path: String },
    StaleImport,
    CameraNotFound(CameraKey),
    ClippingPlaneNotFound(ClippingPlaneKey),
    InstanceSetNotFound(InstanceSetKey),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub help: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCode {
    LargeScenePrecisionRisk,
    DepthPrecisionRisk,
    WebGl2DepthCompatibility,
    DestructionQueuePressure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RendererStats {
    pub buffers: u64,
    pub textures: u64,
    pub materials: u64,
    pub render_targets: u64,
    pub pipelines: u64,
    pub bind_groups: u64,
    pub shader_modules: u64,
    pub environments: u64,
    pub environment_cubemaps: u64,
    pub environment_prefilter_passes: u64,
    pub environment_brdf_luts: u64,
    pub scene_imports: u64,
    pub shadow_maps: u64,
    pub depth_prepass_passes: u64,
    pub depth_prepass_draws: u64,
    pub fxaa_passes: u64,
    pub live_logical_handles: u64,
    pub pending_destructions: u64,
    pub frames_rendered: u64,
    pub draw_calls: u64,
    pub triangles: u64,
    pub culled_objects: u64,
    pub skipped_frames: u64,
    pub gpu_submissions: u64,
    pub approximate_gpu_memory_bytes: Option<u64>,
    pub cpu_frame_ms: f32,
    pub gpu_frame_ms: Option<f32>,
    pub primitives: u64,
    pub target_width: u32,
    pub target_height: u32,
    pub directional_shadow_map_resolution: Option<u32>,
    pub directional_shadow_pcf_kernel: Option<u8>,
}

impl Diagnostic {
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DevicePoll {
    pub pending_destructions_before: u64,
    pub pending_destructions_after: u64,
    pub destroyed_resources: u64,
    pub gpu_polled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderOutcome {
    pub width: u32,
    pub height: u32,
    pub draw_calls: u64,
    pub primitives: u64,
}

impl From<BuildError> for Error {
    fn from(error: BuildError) -> Self {
        Self::Build(error)
    }
}

impl From<AssetError> for Error {
    fn from(error: AssetError) -> Self {
        Self::Asset(error)
    }
}

impl From<ImportError> for Error {
    fn from(error: ImportError) -> Self {
        Self::Import(error)
    }
}

impl From<InstantiateError> for Error {
    fn from(error: InstantiateError) -> Self {
        Self::Instantiate(error)
    }
}

impl From<PrepareError> for Error {
    fn from(error: PrepareError) -> Self {
        Self::Prepare(error)
    }
}

impl From<RenderError> for Error {
    fn from(error: RenderError) -> Self {
        Self::Render(error)
    }
}

impl From<LookupError> for Error {
    fn from(error: LookupError) -> Self {
        Self::Lookup(error)
    }
}
