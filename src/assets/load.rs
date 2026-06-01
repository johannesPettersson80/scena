use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};

use crate::diagnostics::AssetError;

use super::{AssetPath, AssetProvenance, SceneAsset, SceneAssetGeometrySummary};

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
    pub(super) fetched_bytes: usize,
    pub(super) external_buffers: usize,
    pub(super) external_images: usize,
    pub(super) warnings: Vec<AssetLoadWarning>,
    pub(super) progress_events: Vec<AssetLoadProgress>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AssetLoadOptions {
    strict_textures: bool,
    strict_external_resources: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AssetLoadWarning {
    ExternalBufferMissing {
        path: AssetPath,
        index: usize,
        reason: String,
    },
    ExternalImageMissing {
        path: AssetPath,
        reason: String,
    },
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
    pub(super) warnings: Vec<AssetLoadWarning>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetLoadReportV1 {
    pub schema: String,
    pub path: String,
    pub cache_hit: bool,
    pub fetched_bytes: usize,
    pub external_buffers: usize,
    pub external_images: usize,
    pub provenance: AssetProvenance,
    pub geometry: SceneAssetGeometrySummary,
    pub warnings: Vec<AssetLoadWarningV1>,
    pub progress_events: Vec<AssetLoadProgressV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AssetLoadWarningV1 {
    ExternalBufferMissing {
        path: String,
        index: usize,
        reason: String,
    },
    ExternalImageMissing {
        path: String,
        reason: String,
    },
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
    Parsed {
        path: String,
        nodes: usize,
        meshes: usize,
    },
    Cached {
        path: String,
    },
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

    pub const fn fetched_bytes(&self) -> usize {
        self.fetched_bytes
    }

    pub const fn external_buffers(&self) -> usize {
        self.external_buffers
    }

    pub const fn external_images(&self) -> usize {
        self.external_images
    }

    pub fn warnings(&self) -> &[AssetLoadWarning] {
        &self.warnings
    }

    pub fn progress_events(&self) -> &[AssetLoadProgress] {
        &self.progress_events
    }
}

impl AssetLoadOptions {
    pub const fn new() -> Self {
        Self {
            strict_textures: false,
            strict_external_resources: false,
        }
    }

    pub const fn with_strict_textures(mut self, strict_textures: bool) -> Self {
        self.strict_textures = strict_textures;
        self
    }

    pub const fn strict_textures(&self) -> bool {
        self.strict_textures
    }

    pub const fn with_strict_external_resources(mut self, strict_external_resources: bool) -> Self {
        self.strict_external_resources = strict_external_resources;
        self
    }

    pub const fn strict_external_resources(&self) -> bool {
        self.strict_external_resources
    }
}

impl AssetLoadReport<SceneAsset> {
    pub fn to_schema_report(&self) -> AssetLoadReportV1 {
        AssetLoadReportV1 {
            schema: ASSET_LOAD_REPORT_SCHEMA_V1.to_owned(),
            path: self.path.as_str().to_owned(),
            cache_hit: self.cache_hit,
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
        }
    }

    pub fn to_schema_json(&self) -> serde_json::Value {
        serde_json::to_value(self.to_schema_report())
            .expect("asset load report schema contains only serializable fields")
    }
}

impl From<&AssetLoadWarning> for AssetLoadWarningV1 {
    fn from(warning: &AssetLoadWarning) -> Self {
        match warning {
            AssetLoadWarning::ExternalBufferMissing {
                path,
                index,
                reason,
            } => Self::ExternalBufferMissing {
                path: path.as_str().to_owned(),
                index: *index,
                reason: reason.clone(),
            },
            AssetLoadWarning::ExternalImageMissing { path, reason } => Self::ExternalImageMissing {
                path: path.as_str().to_owned(),
                reason: reason.clone(),
            },
        }
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
