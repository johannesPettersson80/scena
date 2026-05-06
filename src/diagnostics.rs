//! Structured errors, debug overlays, capability reports, and renderer stats.

use std::error;
use std::fmt;

use crate::assets::{EnvironmentHandle, GeometryHandle, MaterialHandle};
use crate::geometry::GeometryTopology;
use crate::material::{AlphaMode, MaterialKind};
use crate::scene::{CameraKey, NodeKey};

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    Build(BuildError),
    Asset(AssetError),
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
    ReloadRequiresRetain {
        path: String,
        help: &'static str,
    },
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
    CameraNotFound(CameraKey),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Headless,
    HeadlessGpu,
    SurfaceDescriptor,
    NativeSurface,
    WebGpu,
    WebGl2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum OutputStageStatus {
    AcesSrgb,
    BackendPassthrough,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AlphaPipelineStatus {
    LinearSourceOver,
    BackendPassthrough,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Capabilities {
    pub backend: Backend,
    pub color_target_format: &'static str,
    pub gpu_device: bool,
    pub surface_attached: bool,
    pub output_stage: OutputStageStatus,
    pub alpha_pipeline: AlphaPipelineStatus,
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
    pub scene_imports: u64,
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

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Build(error) => error.fmt(formatter),
            Self::Asset(error) => error.fmt(formatter),
            Self::Prepare(error) => error.fmt(formatter),
            Self::Render(error) => error.fmt(formatter),
            Self::Lookup(error) => error.fmt(formatter),
        }
    }
}

impl fmt::Display for BuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTargetSize { width, height } => {
                write!(formatter, "invalid render target size {width}x{height}")
            }
            Self::AsyncSurfaceRequired { backend } => {
                write!(
                    formatter,
                    "attached surface initialization for {backend:?} requires async construction"
                )
            }
            Self::CreateSurface { backend } => {
                write!(formatter, "failed to create GPU surface for {backend:?}")
            }
            Self::NoAdapter { backend } => {
                write!(formatter, "no compatible GPU adapter found for {backend:?}")
            }
            Self::RequestDevice { backend } => {
                write!(formatter, "failed to request GPU device for {backend:?}")
            }
            Self::SurfaceUnsupported { backend } => {
                write!(
                    formatter,
                    "no compatible surface configuration found for {backend:?}"
                )
            }
            Self::UnsupportedBackend { backend } => {
                write!(
                    formatter,
                    "backend {backend:?} is not supported on this target"
                )
            }
        }
    }
}

impl fmt::Display for AssetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { path } => write!(formatter, "asset was not found: {path}"),
            Self::Io { path, reason } => {
                write!(formatter, "failed to read asset {path}: {reason}")
            }
            Self::Parse { path, reason } => {
                write!(formatter, "failed to parse asset {path}: {reason}")
            }
            Self::UnsupportedRequiredExtension { path, extension } => write!(
                formatter,
                "asset {path} requires unsupported extension {extension}"
            ),
            Self::UnsupportedOptionalExtensionUsed {
                path,
                extension,
                help,
            } => write!(
                formatter,
                "asset {path} uses unsupported optional extension {extension}: {help}"
            ),
            Self::ReloadRequiresRetain { path, help } => {
                write!(formatter, "asset {path} cannot be reloaded: {help}")
            }
        }
    }
}

impl fmt::Display for PrepareError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTargetSize { width, height } => {
                write!(formatter, "invalid render target size {width}x{height}")
            }
            Self::AssetsRequired { node } => {
                write!(
                    formatter,
                    "node {node:?} references asset handles; call prepare_with_assets"
                )
            }
            Self::GeometryNotFound { node, geometry } => {
                write!(
                    formatter,
                    "node {node:?} references missing geometry handle {geometry:?}"
                )
            }
            Self::MaterialNotFound { node, material } => {
                write!(
                    formatter,
                    "node {node:?} references missing material handle {material:?}"
                )
            }
            Self::EnvironmentAssetsRequired { environment } => {
                write!(
                    formatter,
                    "environment handle {environment:?} requires prepare_with_assets"
                )
            }
            Self::EnvironmentNotFound { environment } => {
                write!(
                    formatter,
                    "active environment handle {environment:?} was not found in assets"
                )
            }
            Self::UnsupportedGeometryTopology { node, topology } => {
                write!(
                    formatter,
                    "node {node:?} uses unsupported geometry topology {topology:?}"
                )
            }
            Self::UnsupportedMaterialKind { node, kind } => {
                write!(
                    formatter,
                    "node {node:?} uses unsupported material kind {kind:?}"
                )
            }
            Self::UnsupportedAlphaMode { node, alpha_mode } => {
                write!(
                    formatter,
                    "node {node:?} uses unsupported alpha mode {alpha_mode:?}"
                )
            }
            Self::UnsupportedModelNode { node } => {
                write!(
                    formatter,
                    "node {node:?} is a model node; model preparation is not implemented"
                )
            }
        }
    }
}

impl fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotPrepared { reason } => write!(formatter, "renderer is not prepared: {reason}"),
            Self::NoActiveCamera => write!(formatter, "scene has no active camera"),
            Self::CameraNotFound(_) => write!(formatter, "camera key does not exist in the scene"),
            Self::InvalidSurfaceSize { width, height } => {
                write!(formatter, "invalid surface size {width}x{height}")
            }
            Self::GpuResourcesNotPrepared { backend } => {
                write!(formatter, "GPU resources for {backend:?} were not prepared")
            }
            Self::GpuReadback { backend } => {
                write!(formatter, "failed to read rendered output for {backend:?}")
            }
        }
    }
}

impl fmt::Display for NotPreparedReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NeverPrepared => write!(formatter, "prepare has not been called"),
            Self::DifferentScene => write!(formatter, "prepare was called for a different scene"),
            Self::SceneChanged {
                prepared_revision,
                current_revision,
                change,
            }
            | Self::EnvironmentChanged {
                prepared_revision,
                current_revision,
                change,
            } => write!(
                formatter,
                "prepared state changed after prepare ({prepared_revision} -> {current_revision}, {change:?})"
            ),
            Self::TargetChanged {
                prepared_revision,
                current_revision,
                change,
            } => write!(
                formatter,
                "render target changed after prepare ({prepared_revision} -> {current_revision}, {change:?})"
            ),
        }
    }
}

impl fmt::Display for LookupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NodeNotFound(_) => write!(formatter, "node key does not exist in the scene"),
            Self::CameraNotFound(_) => write!(formatter, "camera key does not exist in the scene"),
        }
    }
}

impl error::Error for Error {}
impl error::Error for BuildError {}
impl error::Error for AssetError {}
impl error::Error for PrepareError {}
impl error::Error for RenderError {}
impl error::Error for LookupError {}

impl Capabilities {
    pub const fn headless() -> Self {
        Self::for_backend(Backend::Headless)
    }

    pub const fn for_backend(backend: Backend) -> Self {
        Self {
            backend,
            color_target_format: "Rgba8UnormSrgb",
            gpu_device: false,
            surface_attached: false,
            output_stage: OutputStageStatus::AcesSrgb,
            alpha_pipeline: AlphaPipelineStatus::LinearSourceOver,
        }
    }

    pub const fn for_gpu_backend(backend: Backend) -> Self {
        Self {
            backend,
            color_target_format: "Rgba8UnormSrgb",
            gpu_device: true,
            surface_attached: false,
            output_stage: OutputStageStatus::AcesSrgb,
            alpha_pipeline: AlphaPipelineStatus::BackendPassthrough,
        }
    }

    pub const fn for_attached_gpu_backend(backend: Backend) -> Self {
        Self {
            backend,
            color_target_format: "Rgba8UnormSrgb",
            gpu_device: true,
            surface_attached: true,
            output_stage: OutputStageStatus::AcesSrgb,
            alpha_pipeline: AlphaPipelineStatus::BackendPassthrough,
        }
    }
}
