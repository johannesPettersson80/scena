use std::collections::BTreeMap;

use super::AssetPath;
use super::fetch::AssetFetcher;
use super::load::{
    self, AssetExternalResource, AssetLoadControl, AssetLoadOptions, AssetLoadProgress,
    AssetLoadTelemetry, AssetLoadWarning, check_cancelled,
};
use super::texture::validate_texture_source_format;
use crate::diagnostics::AssetError;

pub(super) struct LoadedExternalResources {
    pub(super) buffers: BTreeMap<usize, Vec<u8>>,
    pub(super) images: BTreeMap<AssetPath, Vec<u8>>,
    pub(super) telemetry: AssetLoadTelemetry,
}

pub(super) struct ExternalResourceFetchInputs<'a, F> {
    pub(super) fetcher: &'a F,
    pub(super) scene_path: &'a AssetPath,
    pub(super) scene_bytes: usize,
    pub(super) external_paths: Vec<(usize, AssetPath, usize)>,
    pub(super) external_image_paths: Vec<AssetPath>,
    pub(super) control: Option<&'a AssetLoadControl>,
    pub(super) options: AssetLoadOptions,
}

struct ExternalResourceLoadPlan<'a, F> {
    fetcher: &'a F,
    scene_path: &'a AssetPath,
    control: Option<&'a AssetLoadControl>,
    options: AssetLoadOptions,
}

pub(super) async fn fetch_scene_external_resources<F: AssetFetcher>(
    inputs: ExternalResourceFetchInputs<'_, F>,
    progress_events: &mut Vec<AssetLoadProgress>,
    progress: &mut Option<&mut dyn FnMut(AssetLoadProgress)>,
) -> Result<LoadedExternalResources, AssetError> {
    let plan = ExternalResourceLoadPlan {
        fetcher: inputs.fetcher,
        scene_path: inputs.scene_path,
        control: inputs.control,
        options: inputs.options,
    };
    let mut output = LoadedExternalResources {
        buffers: BTreeMap::new(),
        images: BTreeMap::new(),
        telemetry: AssetLoadTelemetry {
            fetched_bytes: inputs.scene_bytes,
            external_buffers: 0,
            external_images: 0,
            external_resources: Vec::new(),
            warnings: Vec::new(),
        },
    };

    for (index, external_path, byte_length) in inputs.external_paths {
        fetch_external_buffer(
            &plan,
            &mut output,
            progress_events,
            progress,
            index,
            external_path,
            byte_length,
        )
        .await?;
    }

    for external_path in inputs.external_image_paths {
        fetch_external_image(&plan, &mut output, progress_events, progress, external_path).await?;
    }

    Ok(output)
}

async fn fetch_external_buffer<F: AssetFetcher>(
    plan: &ExternalResourceLoadPlan<'_, F>,
    output: &mut LoadedExternalResources,
    progress_events: &mut Vec<AssetLoadProgress>,
    progress: &mut Option<&mut dyn FnMut(AssetLoadProgress)>,
    index: usize,
    external_path: AssetPath,
    byte_length: usize,
) -> Result<(), AssetError> {
    check_cancelled(plan.scene_path, plan.control)?;
    check_fetch_budget_before_fetch(plan, output, &external_path, Some(byte_length))?;
    let bytes = match plan.fetcher.fetch(&external_path).await {
        Ok(bytes) => bytes,
        Err(error @ AssetError::NotFound { .. }) => {
            if plan.options.strict_external_resources() {
                return Err(error);
            }
            record_missing_external_buffer(
                &mut output.telemetry,
                &external_path,
                index,
                "not found".to_string(),
            );
            if byte_length == 0 {
                output.buffers.insert(index, Vec::new());
                return Ok(());
            }
            return Err(error);
        }
        Err(error @ AssetError::Io { .. }) => {
            let reason = match &error {
                AssetError::Io { reason, .. } => reason.clone(),
                _ => unreachable!("matched Io error above"),
            };
            if plan.options.strict_external_resources() {
                return Err(error);
            }
            record_missing_external_buffer(&mut output.telemetry, &external_path, index, reason);
            if byte_length == 0 {
                output.buffers.insert(index, Vec::new());
                return Ok(());
            }
            return Err(error);
        }
        Err(error) => return Err(error),
    };
    check_fetch_budget_after_fetch(plan, output, &external_path, bytes.len())?;

    load::emit_progress(
        progress_events,
        progress,
        AssetLoadProgress::ExternalBufferFetched {
            path: external_path.clone(),
            index,
            bytes: bytes.len(),
        },
    );
    output.telemetry.fetched_bytes = output.telemetry.fetched_bytes.saturating_add(bytes.len());
    output.telemetry.external_buffers = output.telemetry.external_buffers.saturating_add(1);
    output
        .telemetry
        .external_resources
        .push(AssetExternalResource::fetched_buffer(
            external_path.clone(),
            index,
            bytes.len(),
        ));
    output.buffers.insert(index, bytes);
    Ok(())
}

async fn fetch_external_image<F: AssetFetcher>(
    plan: &ExternalResourceLoadPlan<'_, F>,
    output: &mut LoadedExternalResources,
    progress_events: &mut Vec<AssetLoadProgress>,
    progress: &mut Option<&mut dyn FnMut(AssetLoadProgress)>,
    external_path: AssetPath,
) -> Result<(), AssetError> {
    if output.images.contains_key(&external_path) {
        return Ok(());
    }
    if let Err(error) = validate_texture_source_format(&external_path) {
        output
            .telemetry
            .external_resources
            .push(AssetExternalResource::skipped_unsupported_image(
                external_path,
                error.to_string(),
            ));
        return Ok(());
    }

    check_cancelled(plan.scene_path, plan.control)?;
    check_fetch_budget_before_fetch(plan, output, &external_path, None)?;
    let bytes = match plan.fetcher.fetch(&external_path).await {
        Ok(bytes) => bytes,
        Err(error @ AssetError::NotFound { .. }) => {
            if plan.options.strict_textures() {
                return Err(error);
            }
            record_missing_external_image(
                &mut output.telemetry,
                external_path,
                "not found".to_string(),
            );
            return Ok(());
        }
        Err(error @ AssetError::Io { .. }) => {
            let reason = match &error {
                AssetError::Io { reason, .. } => reason.clone(),
                _ => unreachable!("matched Io error above"),
            };
            if plan.options.strict_textures() {
                return Err(error);
            }
            record_missing_external_image(&mut output.telemetry, external_path, reason);
            return Ok(());
        }
        Err(error) => return Err(error),
    };
    check_fetch_budget_after_fetch(plan, output, &external_path, bytes.len())?;

    load::emit_progress(
        progress_events,
        progress,
        AssetLoadProgress::ExternalImageFetched {
            path: external_path.clone(),
            bytes: bytes.len(),
        },
    );
    output.telemetry.fetched_bytes = output.telemetry.fetched_bytes.saturating_add(bytes.len());
    output
        .telemetry
        .external_resources
        .push(AssetExternalResource::fetched_image(
            external_path.clone(),
            bytes.len(),
        ));
    output.images.insert(external_path, bytes);
    output.telemetry.external_images = output.telemetry.external_images.saturating_add(1);
    Ok(())
}

fn check_fetch_budget_before_fetch<F: AssetFetcher>(
    plan: &ExternalResourceLoadPlan<'_, F>,
    output: &LoadedExternalResources,
    path: &AssetPath,
    declared_bytes: Option<usize>,
) -> Result<(), AssetError> {
    let Some(limit) = plan.options.fetch_byte_limit() else {
        return Ok(());
    };
    if let Some(declared_bytes) = declared_bytes {
        check_fetch_budget_total(path, output.telemetry.fetched_bytes, declared_bytes, limit)?;
    }
    #[cfg(not(target_arch = "wasm32"))]
    if let Ok(metadata) = std::fs::metadata(path.as_str())
        && metadata.is_file()
    {
        let source_bytes = usize::try_from(metadata.len()).unwrap_or(usize::MAX);
        check_fetch_budget_total(path, output.telemetry.fetched_bytes, source_bytes, limit)?;
    }
    Ok(())
}

fn check_fetch_budget_after_fetch<F: AssetFetcher>(
    plan: &ExternalResourceLoadPlan<'_, F>,
    output: &LoadedExternalResources,
    path: &AssetPath,
    bytes: usize,
) -> Result<(), AssetError> {
    let Some(limit) = plan.options.fetch_byte_limit() else {
        return Ok(());
    };
    check_fetch_budget_total(path, output.telemetry.fetched_bytes, bytes, limit)
}

fn check_fetch_budget_total(
    path: &AssetPath,
    already_fetched: usize,
    requested_bytes: usize,
    limit: usize,
) -> Result<(), AssetError> {
    let total = already_fetched.saturating_add(requested_bytes);
    if total > limit {
        return Err(AssetError::PolicyViolation {
            path: path.as_str().to_string(),
            reason: format!("fetch would reach {total} bytes, exceeding fetch_byte_limit {limit}"),
            help: "use smaller external resources or raise the operator-owned fetch_byte_limit policy",
        });
    }
    Ok(())
}

fn record_missing_external_buffer(
    telemetry: &mut AssetLoadTelemetry,
    path: &AssetPath,
    index: usize,
    reason: String,
) {
    warn_external_buffer_missing(path, &reason);
    telemetry
        .warnings
        .push(AssetLoadWarning::ExternalBufferMissing {
            path: path.clone(),
            index,
            reason: reason.clone(),
        });
    telemetry
        .external_resources
        .push(AssetExternalResource::missing_buffer(
            path.clone(),
            index,
            reason,
        ));
}

fn record_missing_external_image(
    telemetry: &mut AssetLoadTelemetry,
    path: AssetPath,
    reason: String,
) {
    warn_external_image_missing(&path, &reason);
    telemetry
        .warnings
        .push(AssetLoadWarning::ExternalImageMissing {
            path: path.clone(),
            reason: reason.clone(),
        });
    telemetry
        .external_resources
        .push(AssetExternalResource::missing_image(path, reason));
}

fn warn_external_image_missing(path: &AssetPath, reason: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::console::warn_1(&wasm_bindgen::JsValue::from_str(&format!(
            "scena asset warning: external glTF image '{}' could not be fetched: {}",
            path.as_str(),
            reason
        )));
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (path, reason);
    }
}

fn warn_external_buffer_missing(path: &AssetPath, reason: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::console::warn_1(&wasm_bindgen::JsValue::from_str(&format!(
            "scena asset warning: external glTF buffer '{}' could not be fetched: {}",
            path.as_str(),
            reason
        )));
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (path, reason);
    }
}
