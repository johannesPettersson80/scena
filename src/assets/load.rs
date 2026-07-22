use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};

use crate::diagnostics::AssetError;

use super::{AssetPath, AssetProvenance, SceneAsset, SceneAssetGeometrySummary};

mod fallback;
mod options;
mod warnings;
pub use fallback::{AssetMaterialFallback, AssetMaterialFallbackKind, AssetMaterialFallbackV1};
pub use options::AssetLoadOptions;
pub use warnings::{AssetLoadWarning, AssetLoadWarningV1};

pub const ASSET_LOAD_REPORT_SCHEMA_V1: &str = "scena.asset_load_report.v1";

#[derive(Debug, Clone)]
pub struct AssetLoadControl {
    cancelled: Arc<AtomicBool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetLoadReport<T> {
    pub(super) asset: T,
    pub(super) path: AssetPath,
    pub(super) cache_hit: bool,
    #[serde(default)]
    pub(super) requested_options: AssetLoadOptions,
    #[serde(default)]
    pub(super) cache_entry_options: AssetLoadOptions,
    pub(super) fetched_bytes: usize,
    pub(super) external_buffers: usize,
    pub(super) external_images: usize,
    pub(super) external_resources: Vec<AssetExternalResource>,
    pub(super) warnings: Vec<AssetLoadWarning>,
    pub(super) progress_events: Vec<AssetLoadProgress>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetReloadError {
    pub(super) path: AssetPath,
    pub(super) error: AssetError,
    pub(super) previous_asset_preserved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AssetLoadProgress {
    LoadStarted {
        path: AssetPath,
    },
    CacheHit {
        path: AssetPath,
    },
    AssetFetched {
        path: AssetPath,
        bytes: usize,
    },
    ExternalBufferFetched {
        path: AssetPath,
        index: usize,
        bytes: usize,
    },
    ExternalImageFetched {
        path: AssetPath,
        bytes: usize,
    },
    Parsed {
        path: AssetPath,
        nodes: usize,
        meshes: usize,
    },
    Cached {
        path: AssetPath,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub(super) struct AssetLoadTelemetry {
    pub(super) fetched_bytes: usize,
    pub(super) external_buffers: usize,
    pub(super) external_images: usize,
    pub(super) external_resources: Vec<AssetExternalResource>,
    pub(super) warnings: Vec<AssetLoadWarning>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetLoadReportV1 {
    pub schema: String,
    pub path: String,
    pub cache_hit: bool,
    #[serde(default)]
    pub requested_options: AssetLoadOptions,
    #[serde(default)]
    pub cache_entry_options: AssetLoadOptions,
    pub fetched_bytes: usize,
    pub external_buffers: usize,
    pub external_images: usize,
    pub provenance: AssetProvenance,
    pub geometry: SceneAssetGeometrySummary,
    pub warnings: Vec<AssetLoadWarningV1>,
    pub progress_events: Vec<AssetLoadProgressV1>,
    #[serde(default)]
    pub external_resources: Vec<AssetExternalResourceV1>,
    #[serde(default)]
    pub material_fallbacks: Vec<AssetMaterialFallbackV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AssetLoadProgressV1 {
    LoadStarted {
        path: String,
    },
    CacheHit {
        path: String,
    },
    AssetFetched {
        path: String,
        bytes: usize,
    },
    ExternalBufferFetched {
        path: String,
        index: usize,
        bytes: usize,
    },
    ExternalImageFetched {
        path: String,
        bytes: usize,
    },
    Parsed {
        path: String,
        nodes: usize,
        meshes: usize,
    },
    Cached {
        path: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetExternalResource {
    pub kind: AssetExternalResourceKind,
    pub path: AssetPath,
    pub index: Option<usize>,
    pub status: AssetExternalResourceStatus,
    pub bytes: Option<usize>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetExternalResourceKind {
    Buffer,
    Image,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetExternalResourceStatus {
    Fetched,
    Missing,
    SkippedUnsupportedFormat,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetExternalResourceV1 {
    pub kind: AssetExternalResourceKind,
    pub path: String,
    #[serde(default)]
    pub index: Option<usize>,
    pub status: AssetExternalResourceStatus,
    #[serde(default)]
    pub bytes: Option<usize>,
    #[serde(default)]
    pub reason: Option<String>,
}

impl Default for AssetLoadControl {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetLoadControl {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancelled() -> Self {
        let control = Self::new();
        control.cancel();
        control
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

impl<T> AssetLoadReport<T> {
    pub fn asset(&self) -> &T {
        &self.asset
    }

    pub fn into_asset(self) -> T {
        self.asset
    }

    pub fn path(&self) -> &AssetPath {
        &self.path
    }

    pub const fn cache_hit(&self) -> bool {
        self.cache_hit
    }

    /// Semantic policy requested for this load operation.
    pub const fn options(&self) -> AssetLoadOptions {
        self.requested_options
    }

    /// Policy under which a reused cache entry was originally produced.
    ///
    /// This equals [`Self::options`] on cache misses and exact-policy hits. It
    /// may differ on a cache hit when stored evidence proves that a stricter or
    /// otherwise compatible entry satisfies the requested policy.
    pub const fn cache_entry_options(&self) -> AssetLoadOptions {
        self.cache_entry_options
    }

    pub const fn fetched_bytes(&self) -> usize {
        self.fetched_bytes
    }

    pub const fn external_buffers(&self) -> usize {
        self.external_buffers
    }

    pub const fn external_images(&self) -> usize {
        self.external_images
    }

    pub fn external_resources(&self) -> &[AssetExternalResource] {
        &self.external_resources
    }

    pub fn warnings(&self) -> &[AssetLoadWarning] {
        &self.warnings
    }

    pub fn progress_events(&self) -> &[AssetLoadProgress] {
        &self.progress_events
    }
}

impl AssetReloadError {
    pub fn path(&self) -> &AssetPath {
        &self.path
    }

    pub fn error(&self) -> &AssetError {
        &self.error
    }

    pub const fn previous_asset_preserved(&self) -> bool {
        self.previous_asset_preserved
    }

    pub fn into_asset_error(self) -> AssetError {
        self.error
    }
}

impl std::fmt::Display for AssetReloadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "asset reload failed for {} (previous asset preserved={}): {}",
            self.path.as_str(),
            self.previous_asset_preserved,
            self.error
        )
    }
}

impl std::error::Error for AssetReloadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

impl AssetLoadReport<SceneAsset> {
    pub fn to_schema_report(&self) -> AssetLoadReportV1 {
        AssetLoadReportV1 {
            schema: ASSET_LOAD_REPORT_SCHEMA_V1.to_owned(),
            path: self.path.as_str().to_owned(),
            cache_hit: self.cache_hit,
            requested_options: self.requested_options,
            cache_entry_options: self.cache_entry_options,
            fetched_bytes: self.fetched_bytes,
            external_buffers: self.external_buffers,
            external_images: self.external_images,
            provenance: self.asset.provenance().clone(),
            geometry: self.asset.geometry_summary(),
            warnings: self.warnings.iter().map(AssetLoadWarningV1::from).collect(),
            progress_events: self
                .progress_events
                .iter()
                .map(AssetLoadProgressV1::from)
                .collect(),
            external_resources: self
                .external_resources
                .iter()
                .map(AssetExternalResourceV1::from)
                .collect(),
            material_fallbacks: self
                .asset
                .material_fallbacks()
                .iter()
                .map(AssetMaterialFallbackV1::from)
                .collect(),
        }
    }

    pub fn to_schema_json(&self) -> serde_json::Value {
        serde_json::to_value(self.to_schema_report())
            .expect("asset load report schema contains only serializable fields")
    }
}

impl From<&AssetLoadProgress> for AssetLoadProgressV1 {
    fn from(progress: &AssetLoadProgress) -> Self {
        match progress {
            AssetLoadProgress::LoadStarted { path } => Self::LoadStarted {
                path: path.as_str().to_owned(),
            },
            AssetLoadProgress::CacheHit { path } => Self::CacheHit {
                path: path.as_str().to_owned(),
            },
            AssetLoadProgress::AssetFetched { path, bytes } => Self::AssetFetched {
                path: path.as_str().to_owned(),
                bytes: *bytes,
            },
            AssetLoadProgress::ExternalBufferFetched { path, index, bytes } => {
                Self::ExternalBufferFetched {
                    path: path.as_str().to_owned(),
                    index: *index,
                    bytes: *bytes,
                }
            }
            AssetLoadProgress::ExternalImageFetched { path, bytes } => Self::ExternalImageFetched {
                path: path.as_str().to_owned(),
                bytes: *bytes,
            },
            AssetLoadProgress::Parsed {
                path,
                nodes,
                meshes,
            } => Self::Parsed {
                path: path.as_str().to_owned(),
                nodes: *nodes,
                meshes: *meshes,
            },
            AssetLoadProgress::Cached { path } => Self::Cached {
                path: path.as_str().to_owned(),
            },
        }
    }
}

impl AssetExternalResource {
    pub fn fetched_buffer(path: AssetPath, index: usize, bytes: usize) -> Self {
        Self {
            kind: AssetExternalResourceKind::Buffer,
            path,
            index: Some(index),
            status: AssetExternalResourceStatus::Fetched,
            bytes: Some(bytes),
            reason: None,
        }
    }

    pub fn missing_buffer(path: AssetPath, index: usize, reason: impl Into<String>) -> Self {
        Self {
            kind: AssetExternalResourceKind::Buffer,
            path,
            index: Some(index),
            status: AssetExternalResourceStatus::Missing,
            bytes: None,
            reason: Some(reason.into()),
        }
    }

    pub fn fetched_image(path: AssetPath, bytes: usize) -> Self {
        Self {
            kind: AssetExternalResourceKind::Image,
            path,
            index: None,
            status: AssetExternalResourceStatus::Fetched,
            bytes: Some(bytes),
            reason: None,
        }
    }

    pub fn missing_image(path: AssetPath, reason: impl Into<String>) -> Self {
        Self {
            kind: AssetExternalResourceKind::Image,
            path,
            index: None,
            status: AssetExternalResourceStatus::Missing,
            bytes: None,
            reason: Some(reason.into()),
        }
    }

    pub fn skipped_unsupported_image(path: AssetPath, reason: impl Into<String>) -> Self {
        Self {
            kind: AssetExternalResourceKind::Image,
            path,
            index: None,
            status: AssetExternalResourceStatus::SkippedUnsupportedFormat,
            bytes: None,
            reason: Some(reason.into()),
        }
    }
}

impl From<&AssetExternalResource> for AssetExternalResourceV1 {
    fn from(resource: &AssetExternalResource) -> Self {
        Self {
            kind: resource.kind,
            path: resource.path.as_str().to_owned(),
            index: resource.index,
            status: resource.status,
            bytes: resource.bytes,
            reason: resource.reason.clone(),
        }
    }
}

pub(super) fn check_cancelled(
    path: &AssetPath,
    control: Option<&AssetLoadControl>,
) -> Result<(), AssetError> {
    if control.is_some_and(AssetLoadControl::is_cancelled) {
        return Err(AssetError::Cancelled {
            path: path.as_str().to_string(),
            help: "the load was cancelled before parsed asset data was inserted into the cache",
        });
    }
    Ok(())
}

pub(super) fn emit_progress(
    events: &mut Vec<AssetLoadProgress>,
    observer: &mut Option<&mut dyn FnMut(AssetLoadProgress)>,
    event: AssetLoadProgress,
) {
    if let Some(observer) = observer.as_deref_mut() {
        observer(event.clone());
    }
    events.push(event);
}
