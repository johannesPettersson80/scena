use super::external_resources::{ExternalResourceFetchInputs, fetch_scene_external_resources};
use super::fetch::AssetFetcher;
use super::load::{
    self, AssetLoadControl, AssetLoadOptions, AssetLoadProgress, AssetLoadReport,
    AssetLoadTelemetry, AssetReloadError, check_cancelled,
};
use super::{
    AssetPath, Assets, RetainPolicy, SceneAsset, TextureCacheUpdatePolicy, bundled_scene_bytes,
};
use crate::diagnostics::AssetError;
use std::borrow::Cow;

#[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
fn asset_now_ms() -> f64 {
    js_sys::Date::now()
}

#[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
fn log_asset_step(label: &str, start_ms: f64) -> f64 {
    let now = asset_now_ms();
    if crate::diagnostics::browser_timing_enabled() {
        web_sys::console::log_1(
            &format!("[scena-demo] asset {label}: {:.1}ms", now - start_ms).into(),
        );
    }
    now
}

impl<F: AssetFetcher> Assets<F> {
    pub async fn load_scene_from_bytes(
        &self,
        path: impl Into<AssetPath>,
        bytes: &[u8],
    ) -> Result<SceneAsset, AssetError> {
        let path = path.into();
        let options = AssetLoadOptions::default();
        let scene = {
            let mut storage = self.storage();
            let mut scene = SceneAsset::from_gltf_bytes(path.clone(), bytes, &mut storage)?;
            if self.retain_policy == RetainPolicy::Always {
                scene = scene.with_retained_source_bytes(bytes);
            }
            storage.cache_scene(
                path,
                options,
                scene.clone(),
                AssetLoadTelemetry {
                    fetched_bytes: bytes.len(),
                    ..AssetLoadTelemetry::default()
                },
            );
            scene
        };
        #[cfg(target_arch = "wasm32")]
        {
            self.decode_browser_texture_images().await?;
        }
        Ok(scene)
    }

    pub async fn load_scene(&self, path: impl Into<AssetPath>) -> Result<SceneAsset, AssetError> {
        Ok(self.load_scene_with_report(path).await?.into_asset())
    }

    pub async fn load_scene_with_options(
        &self,
        path: impl Into<AssetPath>,
        options: AssetLoadOptions,
    ) -> Result<SceneAsset, AssetError> {
        Ok(self
            .load_scene_report_inner(path.into(), None, None, options)
            .await?
            .into_asset())
    }

    pub async fn load_scene_with_report(
        &self,
        path: impl Into<AssetPath>,
    ) -> Result<AssetLoadReport<SceneAsset>, AssetError> {
        self.load_scene_report_inner(path.into(), None, None, AssetLoadOptions::default())
            .await
    }

    pub async fn load_scene_with_report_options(
        &self,
        path: impl Into<AssetPath>,
        options: AssetLoadOptions,
    ) -> Result<AssetLoadReport<SceneAsset>, AssetError> {
        self.load_scene_report_inner(path.into(), None, None, options)
            .await
    }

    pub async fn load_scene_with_progress<P>(
        &self,
        path: impl Into<AssetPath>,
        mut progress: P,
    ) -> Result<AssetLoadReport<SceneAsset>, AssetError>
    where
        P: FnMut(AssetLoadProgress),
    {
        self.load_scene_report_inner(
            path.into(),
            None,
            Some(&mut progress),
            AssetLoadOptions::default(),
        )
        .await
    }

    pub async fn load_scene_controlled(
        &self,
        path: impl Into<AssetPath>,
        control: &AssetLoadControl,
    ) -> Result<SceneAsset, AssetError> {
        Ok(self
            .load_scene_report_inner(
                path.into(),
                Some(control),
                None,
                AssetLoadOptions::default(),
            )
            .await?
            .into_asset())
    }

    pub async fn reload_scene(&self, scene: &SceneAsset) -> Result<SceneAsset, AssetError> {
        Ok(self.reload_scene_report_inner(scene).await?.into_asset())
    }

    pub async fn reload_scene_with_report(
        &self,
        scene: &SceneAsset,
    ) -> Result<AssetLoadReport<SceneAsset>, AssetReloadError> {
        let path = scene.path().clone();
        self.reload_scene_report_inner(scene)
            .await
            .map_err(|error| AssetReloadError {
                path,
                error,
                previous_asset_preserved: true,
            })
    }

    async fn reload_scene_report_inner(
        &self,
        scene: &SceneAsset,
    ) -> Result<AssetLoadReport<SceneAsset>, AssetError> {
        let path = scene.path().clone();
        if self.retain_policy != RetainPolicy::Always {
            return Err(AssetError::ReloadRequiresRetain {
                path: path.as_str().to_string(),
                help: "set RetainPolicy::Always before reloading scene assets",
            });
        }

        let reload_options = AssetLoadOptions::default()
            .with_strict_textures(true)
            .with_strict_external_resources(true);
        let mut progress_events = vec![AssetLoadProgress::LoadStarted { path: path.clone() }];
        let mut progress = None;
        let (reloaded, telemetry) = match self
            .parse_scene_uncached_with_texture_policy(
                path.clone(),
                None,
                &mut progress_events,
                &mut progress,
                reload_options,
                TextureCacheUpdatePolicy::ReplaceChangedSource,
            )
            .await
        {
            Ok(result) => result,
            Err(error @ AssetError::NotFound { .. } | error @ AssetError::Io { .. })
                if asset_error_path(&error) == Some(path.as_str()) =>
            {
                let Some(bytes) = scene.retained_source_bytes() else {
                    return Err(AssetError::ReloadRequiresRetain {
                        path: path.as_str().to_string(),
                        help: "retained source bytes are unavailable; reload needs the original source to be fetchable",
                    });
                };
                let mut storage = self.storage();
                let previous_policy = storage.texture_cache_update_policy;
                storage.texture_cache_update_policy =
                    TextureCacheUpdatePolicy::ReplaceChangedSource;
                let result = SceneAsset::from_gltf_bytes(path.clone(), bytes, &mut storage)
                    .map(|scene| scene.with_retained_source_bytes(bytes));
                storage.texture_cache_update_policy = previous_policy;
                (result?, storage.telemetry_for_path(&path))
            }
            Err(error) => return Err(error),
        };
        load::emit_progress(
            &mut progress_events,
            &mut progress,
            AssetLoadProgress::Parsed {
                path: path.clone(),
                nodes: reloaded.node_count(),
                meshes: reloaded.mesh_count(),
            },
        );
        {
            let mut storage = self.storage();
            storage.replace_cached_scene(
                path.clone(),
                reload_options,
                reloaded.clone(),
                telemetry.clone(),
            );
        }
        load::emit_progress(
            &mut progress_events,
            &mut progress,
            AssetLoadProgress::Cached { path: path.clone() },
        );
        Ok(AssetLoadReport {
            asset: reloaded,
            path,
            cache_hit: false,
            requested_options: reload_options,
            cache_entry_options: reload_options,
            fetched_bytes: telemetry.fetched_bytes,
            external_buffers: telemetry.external_buffers,
            external_images: telemetry.external_images,
            external_resources: telemetry.external_resources,
            warnings: telemetry.warnings,
            progress_events,
        })
    }

    async fn load_scene_report_inner(
        &self,
        path: AssetPath,
        control: Option<&AssetLoadControl>,
        mut progress: Option<&mut dyn FnMut(AssetLoadProgress)>,
        options: AssetLoadOptions,
    ) -> Result<AssetLoadReport<SceneAsset>, AssetError> {
        let mut progress_events = Vec::new();
        load::emit_progress(
            &mut progress_events,
            &mut progress,
            AssetLoadProgress::LoadStarted { path: path.clone() },
        );
        check_cancelled(&path, control)?;
        if let Some((scene, telemetry, cache_entry_options)) = {
            let storage = self.storage();
            storage.cached_scene(&path, options)
        } {
            load::emit_progress(
                &mut progress_events,
                &mut progress,
                AssetLoadProgress::CacheHit { path: path.clone() },
            );
            return Ok(AssetLoadReport {
                asset: scene,
                path,
                cache_hit: true,
                requested_options: options,
                cache_entry_options,
                fetched_bytes: 0,
                external_buffers: telemetry.external_buffers,
                external_images: telemetry.external_images,
                external_resources: telemetry.external_resources,
                warnings: telemetry.warnings,
                progress_events,
            });
        }

        let (scene, telemetry) = self
            .parse_scene_uncached(
                path.clone(),
                control,
                &mut progress_events,
                &mut progress,
                options,
            )
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
        {
            let mut storage = self.storage();
            storage.cache_scene(path.clone(), options, scene.clone(), telemetry.clone());
        }
        load::emit_progress(
            &mut progress_events,
            &mut progress,
            AssetLoadProgress::Cached { path: path.clone() },
        );
        Ok(AssetLoadReport {
            asset: scene,
            path,
            cache_hit: false,
            requested_options: options,
            cache_entry_options: options,
            fetched_bytes: telemetry.fetched_bytes,
            external_buffers: telemetry.external_buffers,
            external_images: telemetry.external_images,
            external_resources: telemetry.external_resources,
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
        options: AssetLoadOptions,
    ) -> Result<(SceneAsset, AssetLoadTelemetry), AssetError> {
        self.parse_scene_uncached_with_texture_policy(
            path,
            control,
            progress_events,
            progress,
            options,
            TextureCacheUpdatePolicy::Immutable,
        )
        .await
    }

    async fn parse_scene_uncached_with_texture_policy(
        &self,
        path: AssetPath,
        control: Option<&AssetLoadControl>,
        progress_events: &mut Vec<AssetLoadProgress>,
        progress: &mut Option<&mut dyn FnMut(AssetLoadProgress)>,
        options: AssetLoadOptions,
        texture_cache_update_policy: TextureCacheUpdatePolicy,
    ) -> Result<(SceneAsset, AssetLoadTelemetry), AssetError> {
        #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
        let total_start = asset_now_ms();
        #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
        let mut step_start = total_start;

        check_cancelled(&path, control)?;
        check_fetch_byte_limit_before_fetch(&path, options.fetch_byte_limit())?;
        let fetcher = self.tracked_fetcher();
        let bytes = match bundled_scene_bytes(&path) {
            Some(bytes) => Cow::Borrowed(bytes),
            None => Cow::Owned(fetcher.fetch(&path).await?),
        };
        check_fetch_byte_limit_after_fetch(&path, bytes.len(), options.fetch_byte_limit())?;
        #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
        {
            step_start = log_asset_step("fetch scene bytes", step_start);
        }
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
        #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
        {
            step_start = log_asset_step("external URI discovery", step_start);
        }
        let external_resources = fetch_scene_external_resources(
            ExternalResourceFetchInputs {
                fetcher: &fetcher,
                scene_path: &path,
                scene_bytes: bytes.len(),
                external_paths,
                external_image_paths,
                control,
                options,
            },
            progress_events,
            progress,
        )
        .await?;
        #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
        {
            step_start = log_asset_step("external resource fetches", step_start);
        }
        check_cancelled(&path, control)?;
        let scene = {
            let mut storage = self.storage();
            let previous_policy = storage.texture_cache_update_policy;
            storage.texture_cache_update_policy = texture_cache_update_policy;
            let scene_result =
                if external_resources.buffers.is_empty() && external_resources.images.is_empty() {
                    SceneAsset::from_gltf_bytes(path.clone(), &bytes, &mut storage)
                } else {
                    SceneAsset::from_gltf_bytes_with_external_resources(
                        path.clone(),
                        &bytes,
                        &external_resources.buffers,
                        &external_resources.images,
                        &mut storage,
                    )
                };
            storage.texture_cache_update_policy = previous_policy;
            let mut scene = scene_result?;
            if self.retain_policy == RetainPolicy::Always {
                scene = scene.with_retained_source_bytes(&bytes);
            }
            scene
        };
        #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
        {
            step_start = log_asset_step("SceneAsset::from_gltf_bytes", step_start);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.decode_browser_texture_images().await?;
        }
        #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
        {
            log_asset_step("browser image decode", step_start);
        }
        #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
        {
            log_asset_step("parse_scene_uncached total", total_start);
        }
        let mut telemetry = external_resources.telemetry;
        telemetry.warnings.extend_from_slice(scene.load_warnings());
        Ok((scene, telemetry))
    }

    #[cfg(target_arch = "wasm32")]
    async fn decode_browser_texture_images(&self) -> Result<(), AssetError> {
        let requests = {
            let storage = self.storage();
            storage
                .textures
                .iter()
                .filter_map(|(handle, texture)| {
                    texture
                        .browser_decode_source()
                        .map(|bytes| (handle, texture.path().clone(), bytes))
                })
                .collect::<Vec<_>>()
        };

        for (handle, path, bytes) in requests {
            let image = super::texture::decode_browser_image_bitmap(&path, bytes).await?;
            if let Some(texture) = self.storage().textures.get_mut(handle) {
                std::sync::Arc::make_mut(texture).set_browser_image(image);
            }
        }
        Ok(())
    }
}

fn asset_error_path(error: &AssetError) -> Option<&str> {
    match error {
        AssetError::NotFound { path } | AssetError::Io { path, .. } => Some(path),
        _ => None,
    }
}

pub(super) fn check_fetch_byte_limit_before_fetch(
    path: &AssetPath,
    limit: Option<usize>,
) -> Result<(), AssetError> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (path, limit);
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let Some(limit) = limit else {
            return Ok(());
        };
        if let Ok(metadata) = std::fs::metadata(path.as_str())
            && metadata.is_file()
        {
            let source_bytes = usize::try_from(metadata.len()).unwrap_or(usize::MAX);
            if source_bytes > limit {
                return Err(AssetError::PolicyViolation {
                    path: path.as_str().to_string(),
                    reason: format!(
                        "source is {source_bytes} bytes, exceeding fetch_byte_limit {limit}"
                    ),
                    help: "use a smaller asset or raise the operator-owned fetch_byte_limit policy",
                });
            }
        }
        Ok(())
    }
}

pub(super) fn check_fetch_byte_limit_after_fetch(
    path: &AssetPath,
    bytes: usize,
    limit: Option<usize>,
) -> Result<(), AssetError> {
    let Some(limit) = limit else {
        return Ok(());
    };
    if bytes > limit {
        return Err(AssetError::PolicyViolation {
            path: path.as_str().to_string(),
            reason: format!("source is {bytes} bytes, exceeding fetch_byte_limit {limit}"),
            help: "use a smaller asset or raise the operator-owned fetch_byte_limit policy",
        });
    }
    Ok(())
}
