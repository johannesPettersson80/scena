//! Structured errors, debug overlays, capability reports, and renderer stats.

use std::error;
use std::fmt;

use crate::scene::{CameraKey, NodeKey};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Build(BuildError),
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
pub enum PrepareError {
    InvalidTargetSize { width: u32, height: u32 },
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
    TargetChanged {
        prepared_revision: u64,
        current_revision: u64,
        change: ChangeKind,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    SceneStructure,
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
    NativeSurface,
    WebGpu,
    WebGl2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities {
    pub backend: Backend,
    pub color_target_format: &'static str,
    pub gpu_device: bool,
    pub surface_attached: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RendererStats {
    pub frames_rendered: u64,
    pub draw_calls: u64,
    pub primitives: u64,
    pub gpu_submissions: u64,
    pub pending_destructions: u64,
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

impl fmt::Display for PrepareError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTargetSize { width, height } => {
                write!(formatter, "invalid render target size {width}x{height}")
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
            } => write!(
                formatter,
                "scene changed after prepare ({prepared_revision} -> {current_revision}, {change:?})"
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
        }
    }

    pub const fn for_gpu_backend(backend: Backend) -> Self {
        Self {
            backend,
            color_target_format: "Rgba8UnormSrgb",
            gpu_device: true,
            surface_attached: false,
        }
    }

    pub const fn for_attached_gpu_backend(backend: Backend) -> Self {
        Self {
            backend,
            color_target_format: "Rgba8UnormSrgb",
            gpu_device: true,
            surface_attached: true,
        }
    }
}
