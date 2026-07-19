use base64::Engine;

use super::environment::{DEFAULT_ENVIRONMENT_SOURCE_PATH, is_equirectangular_hdr_path};
use super::environment_sidecar::{EnvironmentPrefilterSidecar, sidecar_path_for_environment};
use super::load::AssetLoadOptions;
use super::scene_loading::{
    check_fetch_byte_limit_after_fetch, check_fetch_byte_limit_before_fetch,
};
use super::{AssetFetcher, AssetPath, Assets, EnvironmentDesc, EnvironmentHandle};
use crate::diagnostics::AssetError;

impl<F> Assets<F> {
    /// Verifies that an explicit recipe environment source is fetchable under
    /// the supplied byte policy without decoding or inserting GPU-facing
    /// environment state.
    #[cfg(feature = "scene-host")]
    pub(crate) async fn validate_environment_source_with_options(
        &self,
        path: impl Into<AssetPath>,
        options: AssetLoadOptions,
    ) -> Result<(), AssetError>
    where
        F: AssetFetcher,
    {
        let path = path.into();
        if !is_equirectangular_hdr_path(&path) {
            return Err(AssetError::UnsupportedEnvironmentFormat {
                path: path.as_str().to_owned(),
                help: "use Radiance .hdr equirectangular input for the M2 environment path",
            });
        }
        if let Some(source_bytes) = embedded_environment_bytes(&path)? {
            check_fetch_byte_limit_after_fetch(
                &path,
                source_bytes.len(),
                options.fetch_byte_limit(),
            )?;
            return Ok(());
        }
        check_fetch_byte_limit_before_fetch(&path, options.fetch_byte_limit())?;
        let source_bytes = self.tracked_fetcher().fetch(&path).await?;
        check_fetch_byte_limit_after_fetch(&path, source_bytes.len(), options.fetch_byte_limit())
    }

    pub async fn load_environment(
        &self,
        path: impl Into<AssetPath>,
    ) -> Result<EnvironmentHandle, AssetError>
    where
        F: AssetFetcher,
    {
        self.load_environment_with_options(path, AssetLoadOptions::default())
            .await
    }

    pub async fn load_environment_with_options(
        &self,
        path: impl Into<AssetPath>,
        options: AssetLoadOptions,
    ) -> Result<EnvironmentHandle, AssetError>
    where
        F: AssetFetcher,
    {
        let path = path.into();
        if let Some(handle) = self.storage().environment_lookup.get(&path).copied() {
            return Ok(handle);
        }
        let sidecar = if is_equirectangular_hdr_path(&path) {
            self.try_load_environment_sidecar_with_options(&path, options)
                .await?
        } else {
            None
        };
        let environment = if path.as_str() == DEFAULT_ENVIRONMENT_SOURCE_PATH {
            EnvironmentDesc::neutral_studio()
        } else if is_equirectangular_hdr_path(&path) {
            if let Some(source_bytes) = embedded_environment_bytes(&path)? {
                check_fetch_byte_limit_after_fetch(
                    &path,
                    source_bytes.len(),
                    options.fetch_byte_limit(),
                )?;
                environment_from_hdr_bytes(path.clone(), &source_bytes, sidecar)?
            } else {
                check_fetch_byte_limit_before_fetch(&path, options.fetch_byte_limit())?;
                match self.tracked_fetcher().fetch(&path).await {
                    Ok(source_bytes) => {
                        check_fetch_byte_limit_after_fetch(
                            &path,
                            source_bytes.len(),
                            options.fetch_byte_limit(),
                        )?;
                        environment_from_hdr_bytes(path.clone(), &source_bytes, sidecar)?
                    }
                    Err(AssetError::NotFound { .. } | AssetError::Io { .. }) => {
                        EnvironmentDesc::from_equirectangular_hdr_path(path.clone())
                    }
                    Err(error) => return Err(error),
                }
            }
        } else {
            return Err(AssetError::UnsupportedEnvironmentFormat {
                path: path.as_str().to_string(),
                help: "use Radiance .hdr equirectangular input for the M2 environment path",
            });
        };
        Ok(self.insert_environment(environment))
    }

    pub fn environment(&self, handle: EnvironmentHandle) -> Option<EnvironmentDesc> {
        self.environment_snapshot(handle)
            .map(|snapshot| snapshot.as_ref().clone())
    }

    /// Returns an immutable shared snapshot of an environment descriptor.
    pub fn environment_snapshot(
        &self,
        handle: EnvironmentHandle,
    ) -> Option<std::sync::Arc<EnvironmentDesc>> {
        self.storage().environments.get(handle).cloned()
    }

    pub fn try_environment(
        &self,
        handle: EnvironmentHandle,
    ) -> Result<EnvironmentDesc, AssetError> {
        self.environment(handle)
            .ok_or(AssetError::EnvironmentHandleNotFound {
                environment: handle,
            })
    }

    pub(super) fn insert_environment(&self, environment: EnvironmentDesc) -> EnvironmentHandle {
        let cache_key = environment.source_path().clone();
        let mut storage = self.storage();
        if let Some(handle) = storage.environment_lookup.get(&cache_key) {
            return *handle;
        }
        let handle = storage
            .environments
            .insert(std::sync::Arc::new(environment));
        storage.environment_lookup.insert(cache_key, handle);
        handle
    }

    async fn try_load_environment_sidecar_with_options(
        &self,
        environment_path: &AssetPath,
        options: AssetLoadOptions,
    ) -> Result<Option<EnvironmentPrefilterSidecar>, AssetError>
    where
        F: AssetFetcher,
    {
        if environment_path.as_str().starts_with("data:") {
            return Ok(None);
        }
        let sidecar_path = sidecar_path_for_environment(environment_path);
        check_fetch_byte_limit_before_fetch(&sidecar_path, options.fetch_byte_limit())?;
        match self.tracked_fetcher().fetch(&sidecar_path).await {
            Ok(bytes) => {
                check_fetch_byte_limit_after_fetch(
                    &sidecar_path,
                    bytes.len(),
                    options.fetch_byte_limit(),
                )?;
                match EnvironmentPrefilterSidecar::parse(sidecar_path.clone(), &bytes) {
                    Ok(sidecar) => Ok(Some(sidecar)),
                    Err(error) => {
                        warn_optional_environment_sidecar_failed(
                            &sidecar_path,
                            &format!("{error:?}"),
                        );
                        Ok(None)
                    }
                }
            }
            Err(AssetError::NotFound { .. } | AssetError::Io { .. }) => Ok(None),
            Err(error) => Err(error),
        }
    }
}

fn environment_from_hdr_bytes(
    path: AssetPath,
    source_bytes: &[u8],
    sidecar: Option<EnvironmentPrefilterSidecar>,
) -> Result<EnvironmentDesc, AssetError> {
    if let Some(sidecar) = sidecar
        && let Some(environment) = EnvironmentDesc::from_equirectangular_hdr_sidecar_bytes(
            path.clone(),
            source_bytes,
            sidecar,
        )?
    {
        return Ok(environment);
    }
    EnvironmentDesc::from_equirectangular_hdr_bytes(path, source_bytes)
}

fn warn_optional_environment_sidecar_failed(path: &AssetPath, reason: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::console::warn_1(&wasm_bindgen::JsValue::from_str(&format!(
            "scena asset warning: optional environment sidecar failed for '{}': {}",
            path.as_str(),
            reason
        )));
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (path, reason);
    }
}

fn embedded_environment_bytes(path: &AssetPath) -> Result<Option<Vec<u8>>, AssetError> {
    if !path.as_str().starts_with("data:") {
        return Ok(None);
    }
    let Some((_, encoded)) = path.as_str().split_once(";base64,") else {
        return Err(AssetError::Parse {
            path: path.as_str().to_string(),
            reason:
                "only base64 Radiance HDR data URIs are supported for embedded environment decoding"
                    .to_string(),
        });
    };
    let encoded = encoded
        .split_once('#')
        .map_or(encoded, |(payload, _fragment)| payload);
    let encoded = encoded
        .split_once('?')
        .map_or(encoded, |(payload, _query)| payload);
    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map(Some)
        .map_err(|error| AssetError::Parse {
            path: path.as_str().to_string(),
            reason: format!("invalid embedded environment base64: {error}"),
        })
}
