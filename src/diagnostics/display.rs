use std::error;
use std::fmt;

use super::{
    AnimationError, AssetError, BuildError, Error, ImportError, InstantiateError, LookupError,
    NotPreparedReason, PrepareError, RenderError,
};

#[path = "display/asset.rs"]
mod asset;
#[path = "display/lookup.rs"]
mod lookup;

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Build(error) => error.fmt(formatter),
            Self::Asset(error) => error.fmt(formatter),
            Self::Import(error) => error.fmt(formatter),
            Self::Instantiate(error) => error.fmt(formatter),
            Self::Prepare(error) => error.fmt(formatter),
            Self::Render(error) => error.fmt(formatter),
            Self::Lookup(error) => error.fmt(formatter),
            Self::Animation(error) => error.fmt(formatter),
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

impl fmt::Display for ImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Asset(error) => error.fmt(formatter),
            Self::Instantiate(error) => error.fmt(formatter),
        }
    }
}

impl From<AssetError> for ImportError {
    fn from(error: AssetError) -> Self {
        Self::Asset(error)
    }
}

impl From<InstantiateError> for ImportError {
    fn from(error: InstantiateError) -> Self {
        Self::Instantiate(error)
    }
}

impl fmt::Display for InstantiateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidChildIndex { parent, child } => write!(
                formatter,
                "glTF node {parent} references invalid child node index {child}"
            ),
            Self::InvalidSkinIndex { node, skin } => {
                write!(
                    formatter,
                    "glTF node {node} references invalid skin index {skin}"
                )
            }
            Self::InvalidSkinJointIndex { skin, joint } => write!(
                formatter,
                "glTF skin {skin} references invalid joint node index {joint}"
            ),
            Self::InvalidAnimationClip { name, reason } => write!(
                formatter,
                "glTF animation clip '{}' is invalid after scene binding: {reason}",
                name.as_deref().unwrap_or("<unnamed>"),
            ),
            Self::InvalidAnchorExtras { node, reason } => {
                write!(
                    formatter,
                    "glTF node {node} has invalid anchor extras: {reason}"
                )
            }
            Self::InvalidConnectorExtras { node, reason } => {
                write!(
                    formatter,
                    "glTF node {node} has invalid connector extras: {reason}"
                )
            }
            Self::CyclicNodeGraph { node } => {
                write!(formatter, "glTF node graph contains a cycle at node {node}")
            }
            Self::MultipleNodeParents {
                node,
                first_parent,
                second_parent,
            } => write!(
                formatter,
                "glTF node {node} has multiple parents {first_parent} and {second_parent}"
            ),
            Self::StaleReplacementImport => {
                write!(formatter, "cannot replace an invalidated scene import")
            }
            Self::ForeignReplacementImport => {
                write!(
                    formatter,
                    "cannot replace an import owned by a different scene"
                )
            }
            Self::MissingReplacementRoot { root } => {
                write!(
                    formatter,
                    "cannot replace import because root {root:?} is missing"
                )
            }
            Self::UnsupportedCoordinateSystem {
                coordinate_system,
                reason,
            } => write!(
                formatter,
                "source coordinate system {coordinate_system:?} is not supported for this import: {reason}"
            ),
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
            Self::TextureNotFound {
                node,
                material,
                texture,
                slot,
            } => {
                write!(
                    formatter,
                    "node {node:?} material {material:?} references missing texture handle {texture:?} in slot {slot}"
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
            Self::MultipleShadowedDirectionalLights { first, second } => write!(
                formatter,
                "only one shadowed directional light is supported; nodes {first:?} and {second:?} both cast shadows"
            ),
            Self::InvalidSkinGeometry { node, reason } => {
                write!(
                    formatter,
                    "node {node:?} has invalid skin geometry: {reason}"
                )
            }
            Self::BackendCapabilityMismatch {
                feature,
                backend,
                help,
            } => {
                write!(
                    formatter,
                    "backend {backend:?} cannot provide required feature {feature}: {help}"
                )
            }
            Self::GpuResourceUpload { backend, reason } => {
                write!(
                    formatter,
                    "backend {backend:?} failed during explicit GPU resource upload: {reason}"
                )
            }
            Self::GpuDeviceRebuildRequired {
                backend,
                recoverable,
            } => write!(
                formatter,
                "GPU device for {backend:?} was lost and cannot be reused; renderer rebuild required (host-recoverable={recoverable})"
            ),
            Self::UnsupportedSampleCount {
                backend,
                requested,
                maximum,
            } => write!(
                formatter,
                "backend {backend:?} supports at most {maximum} samples, but explicit prepare requested {requested}"
            ),
        }
    }
}

impl fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotPrepared { reason } => write!(formatter, "renderer is not prepared: {reason}"),
            Self::NoActiveCamera => write!(
                formatter,
                "scene has no active camera; call Scene::add_default_camera or Scene::set_active_camera"
            ),
            Self::CameraNotFound(_) => write!(formatter, "camera key does not exist in the scene"),
            Self::InvalidSurfaceSize { width, height } => {
                write!(formatter, "invalid surface size {width}x{height}")
            }
            Self::SurfaceLost { recoverable } => {
                write!(
                    formatter,
                    "render surface was lost; recoverable={recoverable}"
                )
            }
            Self::SurfaceOutdated {
                backend,
                retry_attempted,
            } => write!(
                formatter,
                "{backend:?} render surface remained outdated after retry={retry_attempted}"
            ),
            Self::SurfaceConfigurationChanged { backend } => write!(
                formatter,
                "{backend:?} surface format or present mode changed during recovery"
            ),
            Self::GpuValidation { backend } => {
                write!(formatter, "{backend:?} reported a GPU validation error")
            }
            Self::GpuOutOfMemory { backend } => {
                write!(formatter, "{backend:?} GPU reported out of memory")
            }
            Self::ContextLost { recoverable } => {
                write!(
                    formatter,
                    "render context was lost; recoverable={recoverable}"
                )
            }
            Self::GpuDeviceLost { recoverable } => {
                write!(formatter, "GPU device was lost; recoverable={recoverable}")
            }
            Self::GpuResourcesNotPrepared { backend } => {
                write!(formatter, "GPU resources for {backend:?} were not prepared")
            }
            Self::UnsupportedSampleCount {
                backend,
                requested,
                maximum,
            } => {
                write!(
                    formatter,
                    "backend {backend:?} does not support MSAA sample count {requested}; maximum supported sample count is {maximum}"
                )
            }
            Self::UnsupportedSupersampleFactor {
                factor,
                width,
                height,
                scaled_width,
                scaled_height,
                maximum_dimension,
                maximum_pixels,
            } => {
                write!(
                    formatter,
                    "supersample factor {factor} for {width}x{height} would render {scaled_width}x{scaled_height}, exceeding the maximum internal target {maximum_dimension}px per axis or {maximum_pixels} pixels"
                )
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
            Self::OutputSettingsChanged {
                prepared_revision,
                current_revision,
                change,
            } => write!(
                formatter,
                "output settings changed after prepare ({prepared_revision} -> {current_revision}, {change:?})"
            ),
        }
    }
}

impl error::Error for Error {}
impl error::Error for BuildError {}
impl error::Error for AssetError {}
impl error::Error for ImportError {}
impl error::Error for InstantiateError {}
impl error::Error for PrepareError {}
impl error::Error for RenderError {}
impl error::Error for LookupError {}
impl error::Error for AnimationError {}
