use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use notify_debouncer_full::notify::{self, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};

use super::{AssetPath, Assets, SceneAsset};

type NativeDebouncer = Debouncer<notify::RecommendedWatcher, RecommendedCache>;

#[derive(Debug)]
pub struct AssetHotReloadWatcher {
    debouncer: NativeDebouncer,
    receiver: mpsc::Receiver<DebounceEventResult>,
    watched_scenes: Vec<WatchedScene>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WatchedScene {
    asset_path: AssetPath,
    file_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetHotReloadError {
    Watch { path: String, reason: String },
    Notify { reasons: Vec<String> },
    ChannelClosed,
}

impl<F> Assets<F> {
    /// Watches a retained scene asset's source file and emits one debounced
    /// reload decision per changed asset path.
    ///
    /// This native-only helper deliberately stops at the asset boundary: the
    /// host still calls [`Assets::reload_scene`] and [`crate::Scene::replace_import`],
    /// then prepares/renders explicitly.
    pub fn watch_scene_for_hot_reload(
        &self,
        scene: &SceneAsset,
        debounce: Duration,
    ) -> Result<AssetHotReloadWatcher, AssetHotReloadError> {
        AssetHotReloadWatcher::watch_scene(scene.path().clone(), debounce)
    }
}

impl AssetHotReloadWatcher {
    pub fn watch_scene(
        asset_path: AssetPath,
        debounce: Duration,
    ) -> Result<Self, AssetHotReloadError> {
        let file_path = PathBuf::from(asset_path.as_str());
        let watched = WatchedScene {
            asset_path,
            file_path: normalize_path(&file_path),
        };
        let (sender, receiver) = mpsc::channel();
        let mut debouncer = new_debouncer(debounce, None, move |result: DebounceEventResult| {
            let _ = sender.send(result);
        })
        .map_err(|error| AssetHotReloadError::Watch {
            path: file_path.display().to_string(),
            reason: error.to_string(),
        })?;
        debouncer
            .watch(&file_path, RecursiveMode::NonRecursive)
            .map_err(|error| AssetHotReloadError::Watch {
                path: file_path.display().to_string(),
                reason: error.to_string(),
            })?;
        Ok(Self {
            debouncer,
            receiver,
            watched_scenes: vec![watched],
        })
    }

    pub fn drain_changed_scenes(&mut self) -> Result<Vec<AssetPath>, AssetHotReloadError> {
        let mut changed = BTreeSet::new();
        loop {
            match self.receiver.try_recv() {
                Ok(Ok(events)) => {
                    for event in events {
                        for path in &event.paths {
                            self.collect_changed_path(path, &mut changed);
                        }
                    }
                }
                Ok(Err(errors)) => {
                    return Err(AssetHotReloadError::Notify {
                        reasons: errors.into_iter().map(|error| error.to_string()).collect(),
                    });
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err(AssetHotReloadError::ChannelClosed);
                }
            }
        }
        Ok(changed.into_iter().collect())
    }

    pub fn watched_scenes(&self) -> impl Iterator<Item = &AssetPath> {
        self.watched_scenes
            .iter()
            .map(|watched| &watched.asset_path)
    }

    pub fn stop(self) {
        self.debouncer.stop();
    }

    fn collect_changed_path(&self, path: &Path, changed: &mut BTreeSet<AssetPath>) {
        let path = normalize_path(path);
        for watched in &self.watched_scenes {
            if path == watched.file_path {
                changed.insert(watched.asset_path.clone());
            }
        }
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

impl fmt::Display for AssetHotReloadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Watch { path, reason } => {
                write!(formatter, "failed to watch asset {path}: {reason}")
            }
            Self::Notify { reasons } => write!(
                formatter,
                "asset hot-reload watcher reported notify errors: {}",
                reasons.join("; ")
            ),
            Self::ChannelClosed => write!(formatter, "asset hot-reload watcher channel closed"),
        }
    }
}

impl std::error::Error for AssetHotReloadError {}
