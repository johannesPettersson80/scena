use std::collections::BTreeMap;

use super::fetch::AssetFetcher;
use super::load::{
    self, AssetLoadControl, AssetLoadProgress, AssetLoadReport, AssetLoadTelemetry,
    AssetLoadWarning, check_cancelled,
};
use super::texture::validate_texture_source_format;
use super::{AssetPath, Assets, RetainPolicy, SceneAsset};
use crate::diagnostics::AssetError;

impl<F: AssetFetcher> Assets<F> {
    pub async fn load_scene(&self, path: impl Into<AssetPath>) -> Result<SceneAsset, AssetError> {
        Ok(self.load_scene_with_report(path).await?.into_asset())
    }

    pub async fn load_scene_with_report(
        &self,
        path: impl Into<AssetPath>,
    ) -> Result<AssetLoadReport<SceneAsset>, AssetError> {
        self.load_scene_report_inner(path.into(), None, None).await
    }

    pub async fn load_scene_with_progress<P>(
        &self,
        path: impl Into<AssetPath>,
        mut progress: P,
    ) -> Result<AssetLoadReport<SceneAsset>, AssetError>
    where
        P: FnMut(AssetLoadProgress),
    {
        self.load_scene_report_inner(path.into(), None, Some(&mut progress))
            .await
    }

    pub async fn load_scene_controlled(
        &self,
        path: impl Into<AssetPath>,
        control: &AssetLoadControl,
    ) -> Result<SceneAsset, AssetError> {
        Ok(self
            .load_scene_report_inner(path.into(), Some(control), None)
            .await?
            .into_asset())
    }

    pub async fn reload_scene(&self, scene: &SceneAsset) -> Result<SceneAsset, AssetError> {
        let path = scene.path().clone();
        if self.retain_policy != RetainPolicy::Always {
            return Err(AssetError::ReloadRequiresRetain {
                path: path.as_str().to_string(),
                help: "set RetainPolicy::Always before reloading scene assets",
            });
        }

        let mut progress_events = Vec::new();
        let mut progress = None;
        let reloaded = match self
            .parse_scene_uncached(path.clone(), None, &mut progress_events, &mut progress)
            .await
        {
            Ok((scene, _telemetry)) => scene,
            Err(AssetError::NotFound { .. } | AssetError::Io { .. }) => {
                let Some(bytes) = scene.retained_source_bytes() else {
                    return Err(AssetError::ReloadRequiresRetain {
                        path: path.as_str().to_string(),
                        help: "retained source bytes are unavailable; reload needs the original source to be fetchable",
                    });
                };
                let mut storage = self.storage();
                SceneAsset::from_gltf_bytes(path.clone(), bytes, &mut storage)?
                    .with_retained_source_bytes(bytes)
            }
            Err(error) => return Err(error),
        };
        self.storage().scene_lookup.insert(path, reloaded.clone());
        Ok(reloaded)
    }

    async fn load_scene_report_inner(
        &self,
        path: AssetPath,
        control: Option<&AssetLoadControl>,
        mut progress: Option<&mut dyn FnMut(AssetLoadProgress)>,
    ) -> Result<AssetLoadReport<SceneAsset>, AssetError> {
        let mut progress_events = Vec::new();
        load::emit_progress(
            &mut progress_events,
            &mut progress,
            AssetLoadProgress::LoadStarted { path: path.clone() },
        );
        check_cancelled(&path, control)?;
        if let Some(scene) = self.storage().scene_lookup.get(&path).cloned() {
            load::emit_progress(
                &mut progress_events,
                &mut progress,
                AssetLoadProgress::CacheHit { path: path.clone() },
            );
            return Ok(AssetLoadReport {
                asset: scene,
                path,
                cache_hit: true,
                fetched_bytes: 0,
                external_buffers: 0,
                warnings: Vec::new(),
                progress_events,
            });
        }

        let (scene, telemetry) = self
            .parse_scene_uncached(path.clone(), control, &mut progress_events, &mut progress)
            .await?;
        load::emit_progress(
            &mut progress_events,
            &mut progress,
            AssetLoadProgress::Parsed {
                path: path.clone(),
                nodes: scene.node_count(),
                meshes: scene.mesh_count(),
            },
        );
        check_cancelled(&path, control)?;
        self.storage()
            .scene_lookup
            .insert(path.clone(), scene.clone());
        load::emit_progress(
            &mut progress_events,
            &mut progress,
            AssetLoadProgress::Cached { path: path.clone() },
        );
        Ok(AssetLoadReport {
            asset: scene,
            path,
            cache_hit: false,
            fetched_bytes: telemetry.fetched_bytes,
            external_buffers: telemetry.external_buffers,
            warnings: telemetry.warnings,
            progress_events,
        })
    }

    async fn parse_scene_uncached(
        &self,
        path: AssetPath,
        control: Option<&AssetLoadControl>,
        progress_events: &mut Vec<AssetLoadProgress>,
        progress: &mut Option<&mut dyn FnMut(AssetLoadProgress)>,
    ) -> Result<(SceneAsset, AssetLoadTelemetry), AssetError> {
        check_cancelled(&path, control)?;
        let bytes = self.fetcher.fetch(&path).await?;
        load::emit_progress(
            progress_events,
            progress,
            AssetLoadProgress::AssetFetched {
                path: path.clone(),
                bytes: bytes.len(),
            },
        );
        check_cancelled(&path, control)?;
        let external_paths = SceneAsset::external_buffer_paths(&path, &bytes)?;
        let external_image_paths = SceneAsset::external_image_paths(&path, &bytes)?;
        let mut external_buffers = BTreeMap::new();
        let mut external_images = BTreeMap::new();
        let mut telemetry = AssetLoadTelemetry {
            fetched_bytes: bytes.len(),
            external_buffers: 0,
            warnings: Vec::new(),
        };
        for (index, external_path) in external_paths {
            check_cancelled(&path, control)?;
            let bytes = self.fetcher.fetch(&external_path).await?;
            load::emit_progress(
                progress_events,
                progress,
                AssetLoadProgress::ExternalBufferFetched {
                    path: external_path.clone(),
                    index,
                    bytes: bytes.len(),
                },
            );
            telemetry.fetched_bytes = telemetry.fetched_bytes.saturating_add(bytes.len());
            telemetry.external_buffers = telemetry.external_buffers.saturating_add(1);
            external_buffers.insert(index, bytes);
        }
        for external_path in external_image_paths {
            if external_images.contains_key(&external_path) {
                continue;
            }
            if validate_texture_source_format(&external_path).is_err() {
                continue;
            }
            check_cancelled(&path, control)?;
            let bytes = match self.fetcher.fetch(&external_path).await {
                Ok(bytes) => bytes,
                Err(AssetError::NotFound { .. }) => {
                    telemetry
                        .warnings
                        .push(AssetLoadWarning::ExternalImageMissing {
                            path: external_path,
                            reason: "not found".to_string(),
                        });
                    continue;
                }
                Err(AssetError::Io { reason, .. }) => {
                    telemetry
                        .warnings
                        .push(AssetLoadWarning::ExternalImageMissing {
                            path: external_path,
                            reason,
                        });
                    continue;
                }
                Err(error) => return Err(error),
            };
            telemetry.fetched_bytes = telemetry.fetched_bytes.saturating_add(bytes.len());
            external_images.insert(external_path, bytes);
        }
        check_cancelled(&path, control)?;
        let mut storage = self.storage();
        let mut scene = if external_buffers.is_empty() && external_images.is_empty() {
            SceneAsset::from_gltf_bytes(path.clone(), &bytes, &mut storage)?
        } else {
            SceneAsset::from_gltf_bytes_with_external_resources(
                path.clone(),
                &bytes,
                &external_buffers,
                &external_images,
                &mut storage,
            )?
        };
        if self.retain_policy == RetainPolicy::Always {
            scene = scene.with_retained_source_bytes(&bytes);
        }
        Ok((scene, telemetry))
    }
}
