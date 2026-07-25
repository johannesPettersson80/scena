use super::{
    AnimationError, AssetError, BuildError, Error, ErrorDiagnostic, ImportError, InstantiateError,
    LookupError, PrepareError, RenderError,
};

impl BuildError {
    pub fn help(&self) -> &'static str {
        match self {
            Self::InvalidTargetSize { .. } => "use non-zero renderer target dimensions",
            Self::AsyncSurfaceRequired { .. } => {
                "construct the attached-surface renderer through the async builder for this target"
            }
            Self::CreateSurface { .. } => {
                "verify the window/display handles remain valid while creating the surface"
            }
            Self::NoAdapter { .. } => {
                "install or enable a compatible graphics adapter, or select the deterministic headless CPU backend"
            }
            Self::RequestDevice { .. } => {
                "inspect adapter limits/features and request a supported renderer quality profile"
            }
            Self::SurfaceUnsupported { .. } => {
                "choose a surface format/present mode supported by the active adapter and window"
            }
            Self::UnsupportedBackend { .. } => {
                "select a backend compiled and supported for the current native or browser target"
            }
        }
    }
}

impl ImportError {
    pub fn help(&self) -> &'static str {
        match self {
            Self::Asset(error) => error.help(),
            Self::Instantiate(error) => error.help(),
        }
    }
}

impl InstantiateError {
    pub fn help(&self) -> &'static str {
        match self {
            Self::InvalidChildIndex { .. }
            | Self::CyclicNodeGraph { .. }
            | Self::MultipleNodeParents { .. } => {
                "repair the glTF node hierarchy so child indices exist and each node has at most one acyclic parent"
            }
            Self::InvalidSkinIndex { .. } | Self::InvalidSkinJointIndex { .. } => {
                "repair the glTF skin reference and ensure every joint names an existing node"
            }
            Self::InvalidAnimationClip { .. } => {
                "repair finite keyframe times/values and channel widths before instantiating the clip"
            }
            Self::InvalidAnchorExtras { .. } | Self::InvalidConnectorExtras { .. } => {
                "repair the named extras transform using finite translation/scale and a valid orientation"
            }
            Self::StaleReplacementImport => {
                "resolve the current live SceneImport before attempting replacement"
            }
            Self::ForeignReplacementImport => {
                "replace the import through the same Scene that instantiated it"
            }
            Self::MissingReplacementRoot { .. } => {
                "preserve live import roots until atomic replacement completes"
            }
            Self::UnsupportedCoordinateSystem { .. } => {
                "choose a supported source coordinate system or convert the asset before import"
            }
        }
    }
}

impl AnimationError {
    pub fn help(&self) -> &'static str {
        match self {
            Self::ClipNotFound { .. } => {
                "inspect SceneImport::clips and use one of the returned candidate names"
            }
            Self::InvalidClip { .. } => {
                "validate finite ordered keyframes and output widths before creating the mixer"
            }
            Self::MixerNotFound(_) => {
                "use an AnimationMixerKey created by this Scene and not yet removed"
            }
            Self::StaleMixer(_) => "create a new mixer after import replacement or mixer removal",
        }
    }
}

impl Error {
    pub fn help(&self) -> &'static str {
        match self {
            Self::Build(error) => error.help(),
            Self::Asset(error) => error.help(),
            Self::Import(error) => error.help(),
            Self::Instantiate(error) => error.help(),
            Self::Prepare(error) => error.help(),
            Self::Render(error) => error.help(),
            Self::Lookup(error) => error.help(),
            Self::Animation(error) => error.help(),
        }
    }

    pub fn diagnostic(&self) -> ErrorDiagnostic {
        let mut diagnostic = match self {
            Self::Build(error) => error.diagnostic(),
            Self::Asset(error) => error.diagnostic(),
            Self::Import(error) => error.diagnostic(),
            Self::Instantiate(error) => error.diagnostic(),
            Self::Prepare(error) => error.diagnostic(),
            Self::Render(error) => error.diagnostic(),
            Self::Lookup(error) => error.diagnostic(),
            Self::Animation(error) => error.diagnostic(),
        };
        diagnostic
            .context
            .insert("wrapper".to_owned(), "scena::Error".to_owned());
        diagnostic
    }
}

fn structured_error(
    code: &str,
    family: &str,
    message: String,
    help: &'static str,
) -> ErrorDiagnostic {
    ErrorDiagnostic {
        code: code.to_owned(),
        message,
        help: help.to_owned(),
        context: [("family".to_owned(), family.to_owned())].into(),
    }
}

macro_rules! structured_diagnostic {
    ($error:ty, $code:literal, $family:literal) => {
        impl $error {
            pub fn diagnostic(&self) -> ErrorDiagnostic {
                structured_error($code, $family, self.to_string(), self.help())
            }
        }
    };
}

structured_diagnostic!(BuildError, "build_error", "build");
structured_diagnostic!(AssetError, "asset_error", "asset");
structured_diagnostic!(ImportError, "import_error", "import");
structured_diagnostic!(InstantiateError, "instantiate_error", "instantiate");
structured_diagnostic!(PrepareError, "prepare_error", "prepare");
structured_diagnostic!(RenderError, "render_error", "render");
structured_diagnostic!(LookupError, "lookup_error", "lookup");
structured_diagnostic!(AnimationError, "animation_error", "animation");

impl AssetError {
    pub fn help(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "check the asset path and the configured AssetFetcher",
            Self::Io { .. } => "check filesystem or network access in the host application",
            Self::PolicyViolation { help, .. } => help,
            Self::Parse { .. } => "validate the asset with the source tool or glTF validator",
            Self::InvalidTextureIdentity { .. } => {
                "use a non-empty stable application-owned identity without control characters"
            }
            Self::InvalidTextureData { .. } => {
                "provide exactly width x height pixels in the constructor's declared format and use only finite float channels"
            }
            Self::TextureSizeLimit { .. } => {
                "resize the texture before loading or raise the explicit application policy on a capable backend"
            }
            Self::TextureIdentityCollision { .. } => {
                "keep an identity bound to immutable pixels/options, or mint a new identity when generated content changes"
            }
            Self::TextureColorSpaceMismatch { .. } => {
                "use the slot-typed constructor/loader so color slots use sRGB and data slots use linear sampling"
            }
            Self::MorphWeightWidthMismatch { .. } => {
                "export exactly one animation weight per morph target for every targeted primitive"
            }
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
            Self::Ktx2ColorSpaceMismatch { help, .. } => help,
            Self::Cancelled { .. } => {
                "retry the load with a fresh AssetLoadControl when the host still needs the asset"
            }
            Self::UnsupportedEnvironmentFormat { .. } => {
                "use an equirectangular .hdr environment or the bundled default environment"
            }
            Self::ReloadRequiresRetain { .. } => {
                "set RetainPolicy::Always before loading assets that need hot reload"
            }
            Self::GeometryHandleNotFound { .. }
            | Self::MaterialHandleNotFound { .. }
            | Self::TextureHandleNotFound { .. }
            | Self::EnvironmentHandleNotFound { .. } => {
                "verify the handle came from the same Assets collection: \
                 call assets.contains_<kind>(handle) on the store you queried; \
                 compare assets.store_id() against the store that minted the \
                 handle to distinguish 'wrong store' from 'stale handle freed \
                 by Assets::release_unreferenced'"
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
            | Self::TextureNotFound { .. }
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
            Self::GpuResourceUpload { .. } => {
                "call Renderer::prepare again after fixing the browser/GPU resource state; render must not hide upload failures"
            }
            Self::GpuDeviceRebuildRequired { .. } => {
                "recreate the Renderer and prepare retained scene/assets again; on native attached surfaces recover_surface with a fresh PlatformSurface re-requests the device"
            }
            Self::UnsupportedSampleCount { .. } => {
                "choose an anti_aliasing sample count supported by the active GPU adapter, such as msaa4, then prepare again"
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
            Self::SurfaceOutdated { .. } => {
                "the renderer already reconfigured and retried once; wait for the next resize event or replace the surface"
            }
            Self::SurfaceConfigurationChanged { .. } => {
                "call Renderer::prepare again so pipelines match the refreshed surface format, then render"
            }
            Self::GpuValidation { .. } => {
                "inspect the wgpu validation diagnostic and fix the renderer or surface configuration; do not retry as transient churn"
            }
            Self::GpuOutOfMemory { .. } => {
                "release GPU resources or reduce render-target and asset memory, then rebuild the renderer if the device was lost"
            }
            Self::ContextLost { .. } => {
                "call recover_context with retained assets, then prepare again"
            }
            Self::GpuDeviceLost { .. } => {
                "recreate the Renderer and prepare retained scene/assets again; a lost wgpu Device/Queue cannot be reused"
            }
            Self::GpuResourcesNotPrepared { .. } => "call Renderer::prepare before rendering",
            Self::UnsupportedSampleCount { .. } => {
                "choose an anti_aliasing sample count supported by the active GPU adapter, such as msaa4"
            }
            Self::UnsupportedSupersampleFactor { .. } => {
                "lower render.supersample or reduce capture width/height; full-frame supersampling costs N^2 pixels"
            }
            Self::GpuReadback { .. } => {
                "retry after device polling or choose a supported readback path"
            }
        }
    }
}

impl LookupError {
    pub fn help(&self) -> &'static str {
        match self {
            Self::NoActiveCamera => "call Scene::add_default_camera or Scene::set_active_camera",
            Self::NodeNotFound(_) => "use a NodeKey created by this Scene",
            Self::CannotRemoveRootNode(_) => {
                "remove child nodes instead; the root is the permanent scene anchor"
            }
            Self::ImportFromDifferentScene => {
                "use the Scene that originally instantiated this SceneImport"
            }
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
            Self::ConnectorNotFound { .. } => {
                "call connectors_named or diagnostic overlays to inspect connectors"
            }
            Self::AmbiguousConnectorName { .. } => {
                "call connectors_named or resolve by host node before connecting"
            }
            Self::ClipNotFound { .. } => "call clips_named to inspect available animation clips",
            Self::AmbiguousClipName { .. } => "call clips_named to choose a specific clip",
            Self::VariantNotFound { .. } => {
                "call SceneImport::material_variants to inspect declared KHR_materials_variants names"
            }
            Self::AmbiguousVariantName { .. } => {
                "rename duplicate KHR_materials_variants entries or address the asset authoring issue before selecting a variant"
            }
            Self::PathNotFound { .. } => {
                "use SceneImport::path_segments when names contain slashes"
            }
            Self::InvalidViewport { .. } => "use non-zero physical viewport dimensions",
            Self::InvalidBounds { .. } => {
                "use finite non-empty bounds whose min components are less than or equal to max"
            }
            Self::InvalidFramingOption { .. } => {
                "use finite bounds, a non-zero view direction, 0 < fill <= 1, and margins smaller than the viewport"
            }
            Self::UnsupportedCameraType { .. } => {
                "use a perspective camera for this helper or call a camera-type-specific framing method"
            }
            Self::ImportHasNoBounds => {
                "frame a node, add renderable geometry, or choose a manual camera pose"
            }
            Self::StaleImport => {
                "re-resolve nodes, anchors, and clips from the replacement SceneImport"
            }
            Self::NodeIsNotMesh { .. } => "check NodeKind before using mesh-only helpers",
            Self::NonInvertibleParentTransform { .. } => {
                "use a finite non-zero parent scale before applying world-space placement helpers"
            }
            Self::InvalidMorphWeights { .. } => "use finite morph weight values",
            Self::MorphWeightWidthMismatch { .. } => {
                "supply exactly one weight per morph target declared by the geometry"
            }
            Self::InvalidTransform { .. } => {
                "use finite translation, rotation, and scale components"
            }
            Self::InvalidCameraProjection { .. } => {
                "use a finite field of view between 0 and 180 degrees, a non-negative finite aspect (zero selects the target aspect), finite ordered extents, and a valid near/far range"
            }
            Self::GeometryNotFound { .. } => {
                "call asset-aware helpers with the same Assets store that created or loaded the geometry"
            }
            Self::InvalidSkinBinding { .. } => {
                "provide exactly one inverse bind matrix for each joint in the skin binding"
            }
            Self::CameraNotFound(_) => "use a CameraKey created by this Scene",
            Self::ClippingPlaneNotFound(_) => "use a ClippingPlaneKey created by this Scene",
            Self::InstanceSetNotFound(_) => "use an InstanceSetKey created by this Scene",
            Self::ParticleSetNotFound(_) => "use a ParticleSetKey created by this Scene",
            Self::InstanceNotFound { .. } => {
                "use an InstanceId that is still present in the requested InstanceSet"
            }
            Self::InvalidInstanceTint { .. } => {
                "use finite opaque per-instance tints; transparent instance tinting requires a transparent instancing path"
            }
            Self::LabelNotFound(_) => "use a LabelKey created by this Scene",
            Self::UnsupportedLabelText { .. } => {
                "use basic Latin text for TrueType labels or render complex-script text in the host"
            }
            Self::InvalidLabelStyle { .. } => {
                "use opaque label colors or omit the optional background/halo until transparent labels are implemented"
            }
        }
    }
}
