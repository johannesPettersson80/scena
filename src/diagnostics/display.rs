use std::error;
use std::fmt;

use super::{
    AnimationError, AssetError, BuildError, Error, ImportError, InstantiateError, LookupError,
    NotPreparedReason, PrepareError, RenderError,
};

#[path = "display/asset.rs"]
mod asset;

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
            Self::InvalidAnchorExtras { node, reason } => {
                write!(
                    formatter,
                    "glTF node {node} has invalid anchor extras: {reason}"
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
            Self::SurfaceLost { recoverable } => {
                write!(
                    formatter,
                    "render surface was lost; recoverable={recoverable}"
                )
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
            Self::CannotRemoveRootNode(_) => {
                write!(formatter, "the scene root node cannot be removed")
            }
            Self::NodeNameNotFound { name } => {
                write!(formatter, "imported scene has no node named '{name}'")
            }
            Self::AmbiguousNodeName { name, matches } => write!(
                formatter,
                "imported scene node name '{name}' is ambiguous across {} nodes",
                matches.len()
            ),
            Self::AnchorNotFound { name } => {
                write!(formatter, "imported scene has no anchor named '{name}'")
            }
            Self::AmbiguousAnchorName { name, hosts } => write!(
                formatter,
                "imported scene anchor name '{name}' is ambiguous across {} host nodes",
                hosts.len()
            ),
            Self::ConnectorNotFound { name } => {
                write!(formatter, "imported scene has no connector named '{name}'")
            }
            Self::AmbiguousConnectorName { name, hosts } => write!(
                formatter,
                "imported scene connector name '{name}' is ambiguous across {} host nodes",
                hosts.len()
            ),
            Self::ClipNotFound { name } => {
                write!(
                    formatter,
                    "imported scene has no animation clip named '{name}'"
                )
            }
            Self::VariantNotFound { name } => write!(
                formatter,
                "imported scene has no KHR_materials_variants variant named '{name}'"
            ),
            Self::AmbiguousVariantName { name, matches } => write!(
                formatter,
                "imported scene KHR_materials_variants name '{name}' is ambiguous across {} variants",
                matches.len()
            ),
            Self::AmbiguousClipName { name, matches } => write!(
                formatter,
                "imported scene animation clip name '{name}' is ambiguous across {} clips",
                matches.len()
            ),
            Self::PathNotFound { path } => {
                write!(formatter, "imported scene path '{path}' was not found")
            }
            Self::InvalidViewport { width, height } => {
                write!(
                    formatter,
                    "viewport {width}x{height} is invalid; width and height must be non-zero"
                )
            }
            Self::InvalidBounds { reason } => {
                write!(formatter, "bounds are invalid: {reason}")
            }
            Self::InvalidFramingOption { field, reason } => {
                write!(
                    formatter,
                    "camera framing option '{field}' is invalid: {reason}"
                )
            }
            Self::UnsupportedCameraType {
                camera,
                operation,
                supported,
            } => {
                write!(
                    formatter,
                    "{operation} does not support camera {camera:?}; supported camera type: {supported}"
                )
            }
            Self::ImportHasNoBounds => {
                write!(
                    formatter,
                    "imported scene has no renderable bounds to frame"
                )
            }
            Self::StaleImport => write!(formatter, "scene import has been invalidated"),
            Self::NodeIsNotMesh { node } => write!(formatter, "node {node:?} is not a mesh node"),
            Self::NonInvertibleParentTransform { node, parent } => write!(
                formatter,
                "node {node:?} cannot be placed in world space because parent {parent:?} has a non-invertible transform"
            ),
            Self::GeometryNotFound { node, .. } => {
                write!(
                    formatter,
                    "geometry for mesh node {node:?} was not found in Assets"
                )
            }
            Self::InvalidSkinBinding {
                joint_count,
                inverse_bind_count,
            } => write!(
                formatter,
                "skin binding has {joint_count} joints but {inverse_bind_count} inverse bind matrices"
            ),
            Self::CameraNotFound(_) => write!(formatter, "camera key does not exist in the scene"),
            Self::ClippingPlaneNotFound(_) => {
                write!(formatter, "clipping plane key does not exist in the scene")
            }
            Self::InstanceSetNotFound(_) => {
                write!(formatter, "instance set key does not exist in the scene")
            }
            Self::ParticleSetNotFound(_) => {
                write!(formatter, "particle set key does not exist in the scene")
            }
            Self::InstanceNotFound {
                instance_set,
                instance,
            } => write!(
                formatter,
                "instance {:?} does not exist in instance set {:?}",
                instance, instance_set
            ),
            Self::InvalidInstanceTint {
                instance_set,
                instance,
                reason,
            } => write!(
                formatter,
                "instance {:?} in instance set {:?} has invalid tint: {reason}",
                instance, instance_set
            ),
            Self::LabelNotFound(_) => write!(formatter, "label key does not exist in the scene"),
            Self::UnsupportedLabelText { reason, .. } => {
                write!(
                    formatter,
                    "label text is not supported by its font: {reason}"
                )
            }
            Self::InvalidLabelStyle { field, reason } => {
                write!(formatter, "{field} is not supported: {reason}")
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
impl error::Error for AnimationError {}
