//! Why a path-traced render could not be produced.

use std::fmt;

use crate::assets::EnvironmentHandle;
use crate::scene::{CameraKey, NodeKey};

/// Error returned by [`Renderer::path_trace`](crate::Renderer::path_trace).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum PathTraceError {
    /// The renderer's target has no pixels.
    InvalidTargetSize { width: u32, height: u32 },
    /// The camera is not part of the scene.
    CameraNotFound(CameraKey),
    /// The camera cannot form an image.
    InvalidCamera {
        camera: CameraKey,
        reason: &'static str,
    },
    /// A node's geometry is not in the given assets.
    GeometryNotFound { node: NodeKey },
    /// A node's material is not in the given assets.
    MaterialNotFound { node: NodeKey },
    /// A node's geometry could not be posed or indexed.
    InvalidGeometry { node: NodeKey, reason: String },
    /// Model nodes are not rendered; instantiate the import into the scene.
    UnsupportedModelNode { node: NodeKey },
    /// The scene has more triangles than the tracer accepts.
    SceneTooLarge { triangles: u64, limit: u64 },
    /// The renderer's environment is not in the given assets.
    EnvironmentNotFound { environment: EnvironmentHandle },
}

impl fmt::Display for PathTraceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTargetSize { width, height } => {
                write!(
                    formatter,
                    "path tracing needs a target with pixels, got {width}x{height}"
                )
            }
            Self::CameraNotFound(camera) => {
                write!(formatter, "camera {camera:?} is not in the scene")
            }
            Self::InvalidCamera { camera, reason } => {
                write!(
                    formatter,
                    "camera {camera:?} cannot form an image: {reason}"
                )
            }
            Self::GeometryNotFound { node } => write!(
                formatter,
                "the geometry of node {node:?} is not in the assets passed to path_trace"
            ),
            Self::MaterialNotFound { node } => write!(
                formatter,
                "the material of node {node:?} is not in the assets passed to path_trace"
            ),
            Self::InvalidGeometry { node, reason } => {
                write!(
                    formatter,
                    "the geometry of node {node:?} cannot be traced: {reason}"
                )
            }
            Self::UnsupportedModelNode { node } => write!(
                formatter,
                "model node {node:?} is not rendered; instantiate the imported scene instead"
            ),
            Self::SceneTooLarge { triangles, limit } => write!(
                formatter,
                "the scene has {triangles} triangles and the path tracer accepts {limit}; \
                 instanced geometry counts once per instance"
            ),
            Self::EnvironmentNotFound { environment } => write!(
                formatter,
                "the renderer's environment {environment:?} is not in the assets passed to path_trace"
            ),
        }
    }
}

impl std::error::Error for PathTraceError {}
