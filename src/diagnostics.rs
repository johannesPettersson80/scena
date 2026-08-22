//! Structured errors, capability reports, and renderer stats.

use crate::animation::AnimationClipKey;
use crate::assets::{EnvironmentHandle, GeometryHandle, MaterialHandle, TextureHandle};
use crate::geometry::GeometryTopology;
use crate::material::{AlphaMode, MaterialKind};
use crate::scene::{
    CameraKey, ClippingPlaneKey, InstanceSetKey, LabelKey, NodeKey, ParticleSetKey,
    SourceCoordinateSystem,
};

mod animation_error;
#[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
mod browser_timing;
mod capabilities;
mod capability_status;
mod conversions;
mod diagnostic;
mod display;
mod display_animation;
mod frame;
mod help;
mod import_overlay;
mod name_candidates;
mod post_processing;
mod state_errors;
mod stats;
pub use animation_error::AnimationError;
#[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
pub(crate) use browser_timing::browser_timing_enabled;
pub use capabilities::{
    AdapterLimitsReport, AlphaPipelineStatus, Backend, CAPABILITY_REPORT_SCHEMA_V1, Capabilities,
    CapabilityConstraintProbeV1, CapabilityConstraintStatusV1, CapabilityProbeModeV1,
    CapabilityProbeStatusV1, CapabilityProbeUnavailableV1, CapabilityProbeV1, CapabilityReport,
    CapabilityReportV1, CapabilityStatus, CapabilityTargetProbeV1, GpuAdapterReport,
    GpuDeviceReport, HardwareTier, OutputColorSpace, OutputStageStatus,
};
pub use diagnostic::{Diagnostic, DiagnosticCode, DiagnosticContext, DiagnosticSeverity};
pub use frame::{DevicePoll, DevicePollStatus, RenderOutcome};
pub use import_overlay::{ImportDiagnosticOverlay, ImportDiagnosticOverlayKind};
pub use name_candidates::nearest_name_candidates;
pub use post_processing::{
    PostProcessingDepthSourceV1, PostProcessingPassV1, PostProcessingReportV1,
};
pub use state_errors::{ChangeKind, ErrorDiagnostic, NotPreparedReason};
pub use stats::RendererStats;

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

/// KTX2 Data Format Descriptor values reported for a material-role mismatch.
///
/// The detail payload is boxed by [`AssetError::Ktx2ColorSpaceMismatch`] so a
/// rich texture diagnostic does not inflate every result that returns
/// [`AssetError`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ktx2ColorSpaceDfd {
    pub expected_primaries: &'static str,
    pub expected_transfer: &'static str,
    pub actual_primaries: &'static str,
    pub actual_transfer: &'static str,
}

/// Material and source details for an unresolved glTF texture reference.
///
/// This payload is boxed by [`AssetError::MissingTexture`] so actionable
/// diagnostics do not inflate every result that returns [`AssetError`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingTextureDetails {
    pub material_index: Option<usize>,
    pub material_name: Option<String>,
    pub image_source: Option<String>,
    pub reason: String,
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
    PolicyViolation {
        path: String,
        reason: String,
        help: &'static str,
    },
    Parse {
        path: String,
        reason: String,
    },
    InvalidTextureIdentity {
        identity: String,
        reason: String,
    },
    InvalidTextureData {
        identity: String,
        width: u32,
        height: u32,
        expected_elements: usize,
        actual_elements: usize,
        reason: String,
    },
    TextureSizeLimit {
        path: String,
        width: u32,
        height: u32,
        maximum_dimension: u32,
        required_bytes: u64,
        maximum_bytes: u64,
    },
    TextureIdentityCollision {
        identity: String,
    },
    TextureColorSpaceMismatch {
        identity: String,
        slot: String,
        expected: String,
        actual: String,
    },
    MorphWeightWidthMismatch {
        path: String,
        clip_index: usize,
        channel_index: usize,
        node_index: usize,
        primitive_index: usize,
        expected: usize,
        actual: usize,
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
        context: Box<MissingTextureDetails>,
        help: &'static str,
    },
    UnsupportedTextureFormat {
        path: String,
        help: &'static str,
    },
    Ktx2ColorSpaceMismatch {
        path: String,
        material_slot: String,
        dfd: Box<Ktx2ColorSpaceDfd>,
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
    GpuResourceUpload {
        backend: Backend,
        reason: String,
    },
    GpuDeviceRebuildRequired {
        backend: Backend,
        recoverable: bool,
    },
    UnsupportedSampleCount {
        backend: Backend,
        requested: u32,
        maximum: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    NotPrepared {
        reason: NotPreparedReason,
    },
    NoActiveCamera,
    CameraNotFound(CameraKey),
    InvalidSurfaceSize {
        width: u32,
        height: u32,
    },
    SurfaceLost {
        recoverable: bool,
    },
    SurfaceOutdated {
        backend: Backend,
        retry_attempted: bool,
    },
    SurfaceConfigurationChanged {
        backend: Backend,
    },
    GpuValidation {
        backend: Backend,
        detail: Option<String>,
    },
    GpuOutOfMemory {
        backend: Backend,
    },
    ContextLost {
        recoverable: bool,
    },
    GpuDeviceLost {
        recoverable: bool,
    },
    GpuResourcesNotPrepared {
        backend: Backend,
    },
    UnsupportedSampleCount {
        backend: Backend,
        requested: u32,
        maximum: u32,
    },
    UnsupportedSupersampleFactor {
        factor: u32,
        width: u32,
        height: u32,
        scaled_width: u32,
        scaled_height: u32,
        maximum_dimension: u32,
        maximum_pixels: u64,
    },
    GpuReadback {
        backend: Backend,
    },
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
    InvalidAnimationClip {
        name: Option<String>,
        reason: String,
    },
    InvalidAnchorExtras {
        node: String,
        reason: String,
    },
    InvalidConnectorExtras {
        node: String,
        reason: String,
    },
    CyclicNodeGraph {
        node: usize,
    },
    MultipleNodeParents {
        node: usize,
        first_parent: usize,
        second_parent: usize,
    },
    StaleReplacementImport,
    ForeignReplacementImport,
    MissingReplacementRoot {
        root: NodeKey,
    },
    UnsupportedCoordinateSystem {
        coordinate_system: SourceCoordinateSystem,
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LookupError {
    NoActiveCamera,
    NodeNotFound(NodeKey),
    CannotRemoveRootNode(NodeKey),
    ImportFromDifferentScene,
    NodeNameNotFound {
        name: String,
        candidates: Vec<String>,
    },
    AmbiguousNodeName {
        name: String,
        matches: Vec<NodeKey>,
    },
    AnchorNotFound {
        name: String,
        candidates: Vec<String>,
    },
    AmbiguousAnchorName {
        name: String,
        hosts: Vec<NodeKey>,
    },
    ConnectorNotFound {
        name: String,
        candidates: Vec<String>,
    },
    AmbiguousConnectorName {
        name: String,
        hosts: Vec<NodeKey>,
    },
    ClipNotFound {
        name: String,
        candidates: Vec<String>,
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
        candidates: Vec<String>,
    },
    /// A material variant name appears more than once in the source
    /// `KHR_materials_variants` declaration. Returned instead of
    /// picking one by enumeration position so hosts can repair the
    /// asset metadata rather than applying a surprising material.
    AmbiguousVariantName {
        name: String,
        matches: Vec<u32>,
    },
    PathNotFound {
        path: String,
    },
    /// A viewport width or height was zero where projection/framing needs pixels.
    InvalidViewport {
        width: u32,
        height: u32,
    },
    /// Bounds were empty, non-finite, or otherwise unsuitable for framing.
    InvalidBounds {
        reason: &'static str,
    },
    /// A named framing option failed validation before camera state was changed.
    InvalidFramingOption {
        field: &'static str,
        reason: &'static str,
    },
    /// The requested operation does not support the camera type yet.
    UnsupportedCameraType {
        camera: CameraKey,
        operation: &'static str,
        supported: &'static str,
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
    /// A morph weight vector contained a non-finite value.
    InvalidMorphWeights {
        node: NodeKey,
        reason: &'static str,
    },
    /// A morph weight vector's width does not match the width already
    /// established for the node. Returned instead of silently zipping, which
    /// would apply only the leading targets and report success.
    MorphWeightWidthMismatch {
        node: NodeKey,
        expected: usize,
        supplied: usize,
    },
    InvalidTransform {
        reason: &'static str,
    },
    InvalidCameraProjection {
        reason: &'static str,
    },
    GeometryNotFound {
        node: NodeKey,
        geometry: GeometryHandle,
    },
    InvalidSkinBinding {
        joint_count: usize,
        inverse_bind_count: usize,
    },
    CameraNotFound(CameraKey),
    ClippingPlaneNotFound(ClippingPlaneKey),
    InstanceSetNotFound(InstanceSetKey),
    ParticleSetNotFound(ParticleSetKey),
    InstanceNotFound {
        instance_set: InstanceSetKey,
        instance: crate::scene::InstanceId,
    },
    InvalidInstanceTint {
        instance_set: InstanceSetKey,
        instance: crate::scene::InstanceId,
        reason: &'static str,
    },
    LabelNotFound(LabelKey),
    UnsupportedLabelText {
        label: LabelKey,
        reason: String,
    },
    InvalidLabelStyle {
        field: &'static str,
        reason: &'static str,
    },
}
