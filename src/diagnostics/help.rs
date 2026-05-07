use super::{AssetError, LookupError, PrepareError, RenderError};

impl AssetError {
    pub fn help(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "check the asset path and the configured AssetFetcher",
            Self::Io { .. } => "check filesystem or network access in the host application",
            Self::Parse { .. } => "validate the asset with the source tool or glTF validator",
            Self::UnsupportedRequiredExtension { .. } => {
                "remove the required extension, export with a supported profile, or enable a decoder feature when one exists"
            }
            Self::UnsupportedOptionalExtensionUsed { .. } => {
                "use extension_diagnostics to inspect the degradation policy before import"
            }
            Self::MissingTexture { .. } => {
                "fix the glTF material slot texture index or export the referenced image"
            }
            Self::UnsupportedTextureFormat { .. } => {
                "use a supported texture format such as PNG, JPEG, or WebP, or enable a decoder feature when one exists"
            }
            Self::Cancelled { .. } => {
                "retry the load with a fresh AssetLoadControl when the host still needs the asset"
            }
            Self::UnsupportedEnvironmentFormat { .. } => {
                "use an equirectangular .hdr environment or the bundled default environment"
            }
            Self::ReloadRequiresRetain { .. } => {
                "set RetainPolicy::Always before loading assets that need hot reload"
            }
        }
    }
}

impl PrepareError {
    pub fn help(&self) -> &'static str {
        match self {
            Self::InvalidTargetSize { .. } => "construct Renderer with non-zero target dimensions",
            Self::AssetsRequired { .. } => {
                "call Renderer::prepare_with_assets when the scene contains asset handles"
            }
            Self::GeometryNotFound { .. }
            | Self::MaterialNotFound { .. }
            | Self::EnvironmentNotFound { .. } => {
                "keep the Assets collection that created the handle alive and pass it to prepare"
            }
            Self::EnvironmentAssetsRequired { .. } => {
                "call Renderer::prepare_with_assets when an environment handle is active"
            }
            Self::UnsupportedGeometryTopology { .. } => {
                "convert the geometry to triangles or lines before prepare"
            }
            Self::UnsupportedMaterialKind { .. }
            | Self::UnsupportedAlphaMode { .. }
            | Self::UnsupportedModelNode { .. } => {
                "choose a supported renderer path or import through Scene::instantiate"
            }
            Self::MultipleShadowedDirectionalLights { .. } => {
                "keep one shadowed directional light enabled for v1.0"
            }
            Self::InvalidSkinGeometry { .. } => "verify joint and weight arrays match vertex count",
            Self::BackendCapabilityMismatch { .. } => {
                "query renderer.capabilities and choose a compatible quality/profile path"
            }
        }
    }
}

impl RenderError {
    pub fn help(&self) -> &'static str {
        match self {
            Self::NotPrepared { .. } => {
                "call Renderer::prepare after scene, target, or renderer changes"
            }
            Self::NoActiveCamera => "call Scene::add_default_camera or Scene::set_active_camera",
            Self::CameraNotFound(_) => "use a CameraKey created by this Scene",
            Self::InvalidSurfaceSize { .. } => {
                "ignore zero-sized host surface events until the surface is visible"
            }
            Self::SurfaceLost { .. } => "call recover_surface, then prepare again",
            Self::ContextLost { .. } | Self::GpuDeviceLost { .. } => {
                "call recover_context with retained assets, then prepare again"
            }
            Self::GpuResourcesNotPrepared { .. } => "call Renderer::prepare before rendering",
            Self::GpuReadback { .. } => {
                "retry after device polling or choose a supported readback path"
            }
        }
    }
}

impl LookupError {
    pub fn help(&self) -> &'static str {
        match self {
            Self::NodeNotFound(_) => "use a NodeKey created by this Scene",
            Self::NodeNameNotFound { .. } => "call nodes_named to inspect available import names",
            Self::AmbiguousNodeName { .. } => {
                "call nodes_named or path_segments for explicit lookup"
            }
            Self::AnchorNotFound { .. } => {
                "call anchors_named or anchor_debug_metadata to inspect anchors"
            }
            Self::AmbiguousAnchorName { .. } => {
                "call anchors_named or anchors_for to choose a host node"
            }
            Self::ClipNotFound { .. } => "call clips_named to inspect available animation clips",
            Self::AmbiguousClipName { .. } => "call clips_named to choose a specific clip",
            Self::PathNotFound { .. } => {
                "use SceneImport::path_segments when names contain slashes"
            }
            Self::InvalidViewport { .. } => "use non-zero physical viewport dimensions",
            Self::ImportHasNoBounds => {
                "frame a node, add renderable geometry, or choose a manual camera pose"
            }
            Self::StaleImport => {
                "re-resolve nodes, anchors, and clips from the replacement SceneImport"
            }
            Self::NodeIsNotMesh { .. } => "check NodeKind before using mesh-only helpers",
            Self::CameraNotFound(_) => "use a CameraKey created by this Scene",
            Self::ClippingPlaneNotFound(_) => "use a ClippingPlaneKey created by this Scene",
            Self::InstanceSetNotFound(_) => "use an InstanceSetKey created by this Scene",
            Self::LabelNotFound(_) => "use a LabelKey created by this Scene",
        }
    }
}
