//! Structured errors, debug overlays, capability reports, and renderer stats.

use crate::animation::{AnimationClipKey, AnimationMixerKey};
use crate::assets::{EnvironmentHandle, GeometryHandle, MaterialHandle, TextureHandle};
use crate::geometry::{Aabb, GeometryTopology};
use crate::material::{AlphaMode, MaterialKind};
use crate::scene::{
    CameraKey, ClippingPlaneKey, InstanceSetKey, LabelKey, NodeKey, SourceCoordinateSystem,
    SourceUnits, Transform,
};

mod capabilities;
mod diagnostic;
mod display;
mod help;
pub use capabilities::{
    AdapterLimitsReport, AlphaPipelineStatus, Backend, Capabilities, CapabilityReport,
    CapabilityStatus, GpuAdapterReport, HardwareTier, OutputStageStatus,
};
pub use diagnostic::{Diagnostic, DiagnosticCode, DiagnosticSeverity};

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    Build(BuildError),
    Asset(AssetError),
    Import(ImportError),
    Instantiate(InstantiateError),
    Prepare(PrepareError),
    Render(RenderError),
    Lookup(LookupError),
    Animation(AnimationError),
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
    MissingTexture {
        path: String,
        material_slot: String,
        texture_index: usize,
        help: &'static str,
    },
    UnsupportedTextureFormat {
        path: String,
        help: &'static str,
    },
    Cancelled {
        path: String,
        help: &'static str,
    },
    UnsupportedEnvironmentFormat {
        path: String,
        help: &'static str,
    },
    ReloadRequiresRetain {
        path: String,
        help: &'static str,
    },
    GeometryHandleNotFound {
        geometry: GeometryHandle,
    },
    MaterialHandleNotFound {
        material: MaterialHandle,
    },
    TextureHandleNotFound {
        texture: TextureHandle,
    },
    EnvironmentHandleNotFound {
        environment: EnvironmentHandle,
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
    TextureNotFound {
        node: NodeKey,
        material: MaterialHandle,
        texture: TextureHandle,
        slot: &'static str,
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
    InvalidSkinGeometry {
        node: NodeKey,
        reason: String,
    },
    BackendCapabilityMismatch {
        feature: &'static str,
        backend: Backend,
        help: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    NotPrepared { reason: NotPreparedReason },
    NoActiveCamera,
    CameraNotFound(CameraKey),
    InvalidSurfaceSize { width: u32, height: u32 },
    SurfaceLost { recoverable: bool },
    ContextLost { recoverable: bool },
    GpuDeviceLost { recoverable: bool },
    GpuResourcesNotPrepared { backend: Backend },
    GpuReadback { backend: Backend },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstantiateError {
    InvalidChildIndex {
        parent: usize,
        child: usize,
    },
    InvalidSkinIndex {
        node: usize,
        skin: usize,
    },
    InvalidSkinJointIndex {
        skin: usize,
        joint: usize,
    },
    InvalidAnchorExtras {
        node: String,
        reason: String,
    },
    UnsupportedCoordinateSystem {
        coordinate_system: SourceCoordinateSystem,
        reason: String,
    },
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
    RendererChanged {
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
    DebugOverlay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum DebugOverlay {
    #[default]
    None,
    Wireframe,
    Normals,
    BoundingBoxes,
    ShadowMap,
    LightCount,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LookupError {
    NodeNotFound(NodeKey),
    NodeNameNotFound {
        name: String,
    },
    AmbiguousNodeName {
        name: String,
        matches: Vec<NodeKey>,
    },
    AnchorNotFound {
        name: String,
    },
    AmbiguousAnchorName {
        name: String,
        hosts: Vec<NodeKey>,
    },
    ConnectorNotFound {
        name: String,
    },
    AmbiguousConnectorName {
        name: String,
        hosts: Vec<NodeKey>,
    },
    ClipNotFound {
        name: String,
    },
    AmbiguousClipName {
        name: String,
        matches: Vec<AnimationClipKey>,
    },
    /// Phase 2B step 3: a variant name passed to
    /// `Scene::set_active_variant` does not appear in the
    /// `SceneImport::material_variants` list. Returned instead of
    /// silently no-oping so callers know the asset doesn't carry
    /// that KHR_materials_variants name.
    VariantNotFound {
        name: String,
    },
    PathNotFound {
        path: String,
    },
    InvalidViewport {
        width: u32,
        height: u32,
    },
    ImportHasNoBounds,
    StaleImport,
    NodeIsNotMesh {
        node: NodeKey,
    },
    NonInvertibleParentTransform {
        node: NodeKey,
        parent: NodeKey,
    },
    GeometryNotFound {
        node: NodeKey,
        geometry: GeometryHandle,
    },
    CameraNotFound(CameraKey),
    ClippingPlaneNotFound(ClippingPlaneKey),
    InstanceSetNotFound(InstanceSetKey),
    LabelNotFound(LabelKey),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnimationError {
    ClipNotFound { name: String },
    MixerNotFound(AnimationMixerKey),
    StaleMixer(AnimationMixerKey),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportDiagnosticOverlay {
    kind: ImportDiagnosticOverlayKind,
    node: NodeKey,
    transform: Transform,
    bounds: Option<Aabb>,
    label: Option<String>,
    source_units: SourceUnits,
    source_coordinate_system: SourceCoordinateSystem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportDiagnosticOverlayKind {
    Origin,
    Axes,
    Bounds,
    Anchor,
    Connector,
    Pivot,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RendererStats {
    pub buffers: u64,
    pub textures: u64,
    pub materials: u64,
    pub material_bindings: u64,
    pub material_texture_bindings: u64,
    pub material_sampler_bindings: u64,
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
    pub gpu_culling_dispatches: u64,
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

impl ImportDiagnosticOverlay {
    pub fn new(
        kind: ImportDiagnosticOverlayKind,
        node: NodeKey,
        transform: Transform,
        bounds: Option<Aabb>,
        label: Option<String>,
    ) -> Self {
        Self {
            kind,
            node,
            transform,
            bounds,
            label,
            source_units: SourceUnits::Meters,
            source_coordinate_system: SourceCoordinateSystem::GltfYUpRightHanded,
        }
    }

    pub const fn with_source_metadata(
        mut self,
        units: SourceUnits,
        coordinate_system: SourceCoordinateSystem,
    ) -> Self {
        self.source_units = units;
        self.source_coordinate_system = coordinate_system;
        self
    }

    pub const fn kind(&self) -> ImportDiagnosticOverlayKind {
        self.kind
    }

    pub const fn node(&self) -> NodeKey {
        self.node
    }

    pub const fn transform(&self) -> Transform {
        self.transform
    }

    pub const fn bounds(&self) -> Option<Aabb> {
        self.bounds
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub const fn source_units(&self) -> SourceUnits {
        self.source_units
    }

    pub const fn source_coordinate_system(&self) -> SourceCoordinateSystem {
        self.source_coordinate_system
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
    pub skipped: bool,
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

impl From<AnimationError> for Error {
    fn from(error: AnimationError) -> Self {
        Self::Animation(error)
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
