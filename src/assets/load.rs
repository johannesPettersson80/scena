use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::diagnostics::AssetError;

use super::AssetPath;

#[derive(Debug, Clone)]
pub struct AssetLoadControl {
    cancelled: Arc<AtomicBool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssetLoadReport<T> {
    pub(super) asset: T,
    pub(super) path: AssetPath,
    pub(super) cache_hit: bool,
    pub(super) fetched_bytes: usize,
    pub(super) external_buffers: usize,
    pub(super) warnings: Vec<AssetLoadWarning>,
    pub(super) progress_events: Vec<AssetLoadProgress>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetLoadWarning {
    ExternalImageMissing { path: AssetPath, reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct AssetLoadTelemetry {
    pub(super) fetched_bytes: usize,
    pub(super) external_buffers: usize,
    pub(super) warnings: Vec<AssetLoadWarning>,
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

    pub fn warnings(&self) -> &[AssetLoadWarning] {
        &self.warnings
    }

    pub fn progress_events(&self) -> &[AssetLoadProgress] {
        &self.progress_events
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
