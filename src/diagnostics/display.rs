use std::error;
use std::fmt;

use super::{
    AssetError, BuildError, Error, ImportError, InstantiateError, LookupError, NotPreparedReason,
    PrepareError, RenderError,
};

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
            Self::UnsupportedEnvironmentFormat { path, help } => {
                write!(
                    formatter,
                    "environment {path} uses an unsupported format: {help}"
                )
            }
            Self::ReloadRequiresRetain { path, help } => {
                write!(formatter, "asset {path} cannot be reloaded: {help}")
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
            Self::MultipleShadowedDirectionalLights { first, second } => write!(
                formatter,
                "only one shadowed directional light is supported; nodes {first:?} and {second:?} both cast shadows"
            ),
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
            Self::NodeNameNotFound { name } => {
                write!(formatter, "imported scene has no node named '{name}'")
            }
            Self::AmbiguousNodeName { name, matches } => write!(
                formatter,
                "imported scene node name '{name}' is ambiguous across {} nodes",
                matches.len()
            ),
            Self::PathNotFound { path } => {
                write!(formatter, "imported scene path '{path}' was not found")
            }
            Self::StaleImport => write!(formatter, "scene import has been invalidated"),
            Self::CameraNotFound(_) => write!(formatter, "camera key does not exist in the scene"),
            Self::ClippingPlaneNotFound(_) => {
                write!(formatter, "clipping plane key does not exist in the scene")
            }
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
