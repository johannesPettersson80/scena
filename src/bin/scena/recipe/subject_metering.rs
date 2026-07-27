use std::collections::BTreeSet;

use super::super::scena_cli_error::{CliErrorKind, CliFailure};

pub(crate) fn resolve_and_apply_subject_metering(
    host: &mut scena::SceneHostCore<scena::DefaultAssetFetcher>,
    manifest: &scena::SceneRecipeBuildV1,
    recipe: &scena::SceneRecipeV1,
    gpu: bool,
) -> Result<(), CliFailure> {
    let Some(metering) = recipe
        .render
        .as_ref()
        .and_then(|render| render.metering.as_ref())
    else {
        host.renderer_mut().clear_auto_exposure_subject_metering();
        return Ok(());
    };
    if metering.mode != "subject" {
        host.renderer_mut().clear_auto_exposure_subject_metering();
        return Ok(());
    }
    let target = metering.target.as_ref().ok_or_else(|| {
        CliFailure::new(
            CliErrorKind::InvalidInput,
            "render.metering mode=subject requires target",
        )
    })?;
    let handles = resolve_subject_handles(manifest, target)?;
    if gpu {
        host.set_semantic_aov_capture_enabled(true);
    }
    host.prepare().map_err(|error| {
        CliFailure::new(
            CliErrorKind::Runtime,
            format!("failed to prepare recipe scene for subject metering measurement: {error}"),
        )
    })?;
    host.render().map_err(|error| {
        CliFailure::new(
            CliErrorKind::Runtime,
            format!("failed to render recipe scene for subject metering measurement: {error}"),
        )
    })?;
    #[cfg(not(target_arch = "wasm32"))]
    let aov = if gpu {
        host.capture_semantic_aovs_gpu().map_err(|error| {
            CliFailure::new(
                CliErrorKind::Runtime,
                format!("failed to capture GPU semantic AOVs for subject metering: {error}"),
            )
        })?
    } else {
        host.capture_semantic_aovs().map_err(|error| {
            CliFailure::new(
                CliErrorKind::Runtime,
                format!("failed to capture semantic AOVs for subject metering: {error}"),
            )
        })?
    };
    #[cfg(target_arch = "wasm32")]
    let aov = {
        if gpu {
            return Err(CliFailure::new(
                CliErrorKind::Unsupported,
                "synchronous recipe subject-metering GPU capture is unavailable on wasm32; use the browser SceneHost async capture API",
            ));
        }
        host.capture_semantic_aovs().map_err(|error| {
            CliFailure::new(
                CliErrorKind::Runtime,
                format!("failed to capture semantic AOVs for subject metering: {error}"),
            )
        })?
    };
    match visible_subject_metering_rect(&aov, &handles) {
        Ok(subject_rect) => host.renderer_mut().set_auto_exposure_subject_metering(
            subject_rect,
            metering.surround_weight.unwrap_or(0.1) as f32,
        ),
        Err(message) if subject_metering_can_degrade_to_verification(&message) => {
            host.renderer_mut().clear_auto_exposure_subject_metering();
        }
        Err(message) => {
            return Err(CliFailure::new(
                CliErrorKind::Runtime,
                format!("failed to resolve subject metering: {message}"),
            ));
        }
    }
    Ok(())
}

fn resolve_subject_handles(
    manifest: &scena::SceneRecipeBuildV1,
    target: &scena::SceneRecipeTargetV1,
) -> Result<BTreeSet<u64>, CliFailure> {
    let handles = scena::resolve_scene_recipe_target_handles(
        manifest,
        target,
        scena::SceneRecipeTargetResolutionMode::SubjectIncludingHidden,
    )
    .map_err(|error| {
        let kind = match error.kind {
            scena::SceneRecipeTargetResolutionErrorKind::Unresolved
            | scena::SceneRecipeTargetResolutionErrorKind::Unsupported => {
                CliErrorKind::InvalidInput
            }
            scena::SceneRecipeTargetResolutionErrorKind::Hidden
            | scena::SceneRecipeTargetResolutionErrorKind::Empty => CliErrorKind::Runtime,
            _ => CliErrorKind::Runtime,
        };
        let message = if error.candidates.is_empty() {
            format!(
                "failed to resolve subject metering target: {}",
                error.message
            )
        } else {
            format!(
                "failed to resolve subject metering target: {}; nearest candidates: {}",
                error.message,
                error.candidates.join(", ")
            )
        };
        CliFailure::new(kind, message)
    })?;
    Ok(handles.into_iter().collect())
}

fn visible_subject_metering_rect(
    aov: &scena::SceneHostSemanticAovCaptureV1,
    target_handles: &BTreeSet<u64>,
) -> Result<scena::AutoExposureSubjectRect, String> {
    if target_handles.is_empty() {
        return Err("subject metering target handle set is empty".to_owned());
    }
    if aov.width == 0 || aov.height == 0 {
        return Err("semantic AOV has an empty viewport".to_owned());
    }
    if aov.id_indices.len() != aov.depth_meters.len()
        || aov.id_indices.len() != aov.width as usize * aov.height as usize
    {
        return Err(format!(
            "semantic AOV shape mismatch: ids={} depths={} viewport={}x{}",
            aov.id_indices.len(),
            aov.depth_meters.len(),
            aov.width,
            aov.height
        ));
    }
    let target_palette_indices = aov
        .legend
        .iter()
        .filter(|entry| {
            target_handles.contains(&entry.node_handle)
                || entry
                    .instance_handle
                    .is_some_and(|handle| target_handles.contains(&handle))
        })
        .map(|entry| entry.palette_index)
        .collect::<BTreeSet<_>>();
    if target_palette_indices.is_empty() {
        return Err("subject metering target has no semantic AOV palette entry".to_owned());
    }

    let mut min_x = aov.width;
    let mut min_y = aov.height;
    let mut max_x = 0_u32;
    let mut max_y = 0_u32;
    for (index, (palette_index, depth)) in aov
        .id_indices
        .iter()
        .zip(aov.depth_meters.iter())
        .enumerate()
    {
        if !target_palette_indices.contains(palette_index) || !depth.is_finite() || *depth <= 0.0 {
            continue;
        }
        let x = index as u32 % aov.width;
        let y = index as u32 / aov.width;
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x.saturating_add(1));
        max_y = max_y.max(y.saturating_add(1));
    }
    if min_x >= max_x || min_y >= max_y {
        return Err("subject metering target has no visible semantic pixels".to_owned());
    }
    Ok(scena::AutoExposureSubjectRect::new(
        min_x,
        min_y,
        max_x - min_x,
        max_y - min_y,
    ))
}

fn subject_metering_can_degrade_to_verification(message: &str) -> bool {
    message == "subject metering target has no semantic AOV palette entry"
        || message == "subject metering target has no visible semantic pixels"
}
