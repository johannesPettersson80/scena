use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::diagnostics::AssetError;

use super::AssetPath;

/// Source-byte provider for asset loading.
pub trait AssetFetcher {
    type Future<'a>: Future<Output = Result<Vec<u8>, AssetError>> + 'a
    where
        Self: 'a;

    fn fetch<'a>(&'a self, path: &'a AssetPath) -> Self::Future<'a>;
}

/// Per-`Assets` wrapper that counts every source-byte request, including
/// failed requests and external-resource requests made during glTF loading.
pub(super) struct TrackedAssetFetcher<'a, F> {
    inner: &'a F,
    attempts: &'a AtomicU64,
}

impl<'a, F> TrackedAssetFetcher<'a, F> {
    pub(super) const fn new(inner: &'a F, attempts: &'a AtomicU64) -> Self {
        Self { inner, attempts }
    }
}

impl<F: AssetFetcher> AssetFetcher for TrackedAssetFetcher<'_, F> {
    type Future<'a>
        = F::Future<'a>
    where
        Self: 'a;

    fn fetch<'a>(&'a self, path: &'a AssetPath) -> Self::Future<'a> {
        self.attempts.fetch_add(1, Ordering::Relaxed);
        self.inner.fetch(path)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub type DefaultAssetFetcher = FileAssetFetcher;

#[cfg(target_arch = "wasm32")]
pub type DefaultAssetFetcher = BrowserAssetFetcher;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Copy, Default)]
pub struct FileAssetFetcher;

#[cfg(not(target_arch = "wasm32"))]
impl AssetFetcher for FileAssetFetcher {
    type Future<'a> = std::future::Ready<Result<Vec<u8>, AssetError>>;

    fn fetch<'a>(&'a self, path: &'a AssetPath) -> Self::Future<'a> {
        std::future::ready(
            std::fs::read(path.as_str()).map_err(|error| AssetError::Io {
                path: path.as_str().to_string(),
                reason: error.to_string(),
            }),
        )
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Copy, Default)]
pub struct BrowserAssetFetcher;

#[cfg(target_arch = "wasm32")]
impl AssetFetcher for BrowserAssetFetcher {
    type Future<'a> = std::pin::Pin<Box<dyn Future<Output = Result<Vec<u8>, AssetError>> + 'a>>;

    fn fetch<'a>(&'a self, path: &'a AssetPath) -> Self::Future<'a> {
        Box::pin(async move { fetch_browser_bytes(path).await })
    }
}

#[cfg(target_arch = "wasm32")]
async fn fetch_browser_bytes(path: &AssetPath) -> Result<Vec<u8>, AssetError> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let window = web_sys::window().ok_or_else(|| AssetError::Io {
        path: path.as_str().to_string(),
        reason: "browser fetch requires a Window".to_string(),
    })?;
    let response_value = JsFuture::from(window.fetch_with_str(path.as_str()))
        .await
        .map_err(|error| js_asset_error(path, "browser fetch failed", error))?;
    let response = response_value
        .dyn_into::<web_sys::Response>()
        .map_err(|error| js_asset_error(path, "browser fetch did not return a Response", error))?;

    if !response.ok() {
        return Err(if response.status() == 404 {
            AssetError::NotFound {
                path: path.as_str().to_string(),
            }
        } else {
            AssetError::Io {
                path: path.as_str().to_string(),
                reason: format!("browser fetch returned HTTP {}", response.status()),
            }
        });
    }

    let buffer_promise = response
        .array_buffer()
        .map_err(|error| js_asset_error(path, "browser response.arrayBuffer failed", error))?;
    let buffer = JsFuture::from(buffer_promise)
        .await
        .map_err(|error| js_asset_error(path, "browser arrayBuffer await failed", error))?;
    let array = js_sys::Uint8Array::new(&buffer);
    let mut bytes = vec![0; array.length() as usize];
    array.copy_to(&mut bytes);
    Ok(bytes)
}

#[cfg(target_arch = "wasm32")]
fn js_asset_error(path: &AssetPath, context: &str, error: wasm_bindgen::JsValue) -> AssetError {
    AssetError::Io {
        path: path.as_str().to_string(),
        reason: error
            .as_string()
            .unwrap_or_else(|| format!("{context}: {error:?}")),
    }
}
