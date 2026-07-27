use std::collections::BTreeSet;

use super::super::scena_cli_error::{CliErrorKind, CliFailure};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SubjectFocusObservation {
    pub(crate) visible_pixel_count: usize,
    pub(crate) focus_distance_m: f32,
    pub(crate) near_depth_m: f32,
    pub(crate) far_depth_m: f32,
    pub(crate) confidence: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SubjectFocusResolution {
    request: SubjectFocusRequest,
    observation: Option<SubjectFocusObservation>,
    unresolved_reason: Option<String>,
    handles: BTreeSet<u64>,
}

#[derive(Debug, Clone, PartialEq)]
struct SubjectFocusRequest {
    target: scena::SceneRecipeTargetV1,
    coverage: String,
    strength: String,
}

pub(crate) fn resolve_and_apply_subject_focus(
    host: &mut scena::SceneHostCore<scena::DefaultAssetFetcher>,
    manifest: &scena::SceneRecipeBuildV1,
    recipe: &scena::SceneRecipeV1,
    gpu: bool,
) -> Result<Option<SubjectFocusResolution>, CliFailure> {
    let Some(request) = subject_focus_request(recipe)? else {
        return Ok(None);
    };
    let handles = resolve_subject_handles(manifest, &request.target)?;
    if gpu {
        host.set_semantic_aov_capture_enabled(true);
    }
    host.prepare().map_err(|error| {
        CliFailure::new(
            CliErrorKind::Runtime,
            format!("failed to prepare recipe scene for subject focus measurement: {error}"),
        )
    })?;
    host.render().map_err(|error| {
        CliFailure::new(
            CliErrorKind::Runtime,
            format!("failed to render recipe scene for subject focus measurement: {error}"),
        )
    })?;
    #[cfg(not(target_arch = "wasm32"))]
    let aov = if gpu {
        host.capture_semantic_aovs_gpu().map_err(|error| {
            CliFailure::new(
                CliErrorKind::Runtime,
                format!("failed to capture GPU semantic AOVs for subject focus: {error}"),
            )
        })?
    } else {
        host.capture_semantic_aovs().map_err(|error| {
            CliFailure::new(
                CliErrorKind::Runtime,
                format!("failed to capture semantic AOVs for subject focus: {error}"),
            )
        })?
    };
    #[cfg(target_arch = "wasm32")]
    let aov = {
        if gpu {
            return Err(CliFailure::new(
                CliErrorKind::Unsupported,
                "synchronous recipe subject-focus GPU capture is unavailable on wasm32; use the browser SceneHost async capture API",
            ));
        }
        host.capture_semantic_aovs().map_err(|error| {
            CliFailure::new(
                CliErrorKind::Runtime,
                format!("failed to capture semantic AOVs for subject focus: {error}"),
            )
        })?
    };
    let observation = match visible_subject_focus_observation(&aov, &handles) {
        Ok(observation) => observation,
        Err(message) if subject_focus_can_degrade_to_report(&message) => {
            let reason = subject_focus_unresolved_reason(host, &handles, &message)?;
            return Ok(Some(SubjectFocusResolution {
                request,
                observation: None,
                unresolved_reason: Some(reason),
                handles,
            }));
        }
        Err(message) => {
            return Err(CliFailure::new(
                CliErrorKind::Runtime,
                format!("failed to resolve subject focus: {message}"),
            ));
        }
    };
    let config = depth_of_field_config_for_subject_focus(&request, &observation)?;
    host.renderer_mut().set_depth_of_field(Some(config));
    Ok(Some(SubjectFocusResolution {
        request,
        observation: Some(observation),
        unresolved_reason: None,
        handles,
    }))
}

impl SubjectFocusResolution {
    pub(crate) fn to_focus_report(&self, capture: &scena::CaptureRgba8) -> scena::FocusReportV1 {
        let mut handles = self.handles.iter().copied().collect::<Vec<_>>();
        handles.sort_unstable();
        let target = focus_report_target(&self.request.target, handles);
        if let Some(observation) = &self.observation {
            return scena::FocusReportV1::resolved(
                "subject",
                target,
                Some(self.request.coverage.clone()),
                Some(self.request.strength.clone()),
                scena::FocusReportResolvedV1 {
                    focus_distance_m: observation.focus_distance_m,
                    near_depth_m: observation.near_depth_m,
                    far_depth_m: observation.far_depth_m,
                    visible_pixel_count: observation.visible_pixel_count as u64,
                    confidence: observation.confidence,
                },
                &capture.descriptor,
            );
        }
        scena::FocusReportV1::unresolved(
            "subject",
            target,
            Some(self.request.coverage.clone()),
            Some(self.request.strength.clone()),
            self.unresolved_reason
                .as_deref()
                .unwrap_or("subject_focus_unresolved"),
            &capture.descriptor,
        )
    }
}

fn subject_focus_request(
    recipe: &scena::SceneRecipeV1,
) -> Result<Option<SubjectFocusRequest>, CliFailure> {
    let Some(depth_of_field) = recipe
        .render
        .as_ref()
        .and_then(|render| render.depth_of_field.as_ref())
    else {
        return Ok(None);
    };
    let Some(focus) = depth_of_field.focus.as_ref() else {
        return Ok(None);
    };
    if focus.mode != "subject" {
        return Err(CliFailure::new(
            CliErrorKind::InvalidInput,
            format!(
                "unsupported depth_of_field focus mode '{}'; use subject",
                focus.mode
            ),
        ));
    }
    let coverage = depth_of_field
        .coverage
        .clone()
        .unwrap_or_else(|| "all".to_owned());
    let strength = depth_of_field
        .strength
        .clone()
        .unwrap_or_else(|| "subtle".to_owned());
    Ok(Some(SubjectFocusRequest {
        target: focus.target.clone(),
        coverage,
        strength,
    }))
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
            format!("failed to resolve subject focus target: {}", error.message)
        } else {
            format!(
                "failed to resolve subject focus target: {}; nearest candidates: {}",
                error.message,
                error.candidates.join(", ")
            )
        };
        CliFailure::new(kind, message)
    })?;
    Ok(handles.into_iter().collect())
}

fn focus_report_target(
    target: &scena::SceneRecipeTargetV1,
    handles: impl IntoIterator<Item = u64>,
) -> scena::FocusReportTargetV1 {
    match target {
        scena::SceneRecipeTargetV1::Import { id } => {
            scena::FocusReportTargetV1::new("import", id.clone(), handles)
        }
        scena::SceneRecipeTargetV1::Node { id } => {
            scena::FocusReportTargetV1::new("node", id.clone(), handles)
        }
        scena::SceneRecipeTargetV1::World { .. } => {
            scena::FocusReportTargetV1::new("world", "world", handles)
        }
    }
}

fn depth_of_field_config_for_subject_focus(
    request: &SubjectFocusRequest,
    observation: &SubjectFocusObservation,
) -> Result<scena::DepthOfFieldConfig, CliFailure> {
    if request.coverage != "all" {
        return Err(CliFailure::new(
            CliErrorKind::InvalidInput,
            format!(
                "unsupported depth_of_field coverage '{}'; use all",
                request.coverage
            ),
        ));
    }
    let (aperture_f_stop, radius_px) = match request.strength.as_str() {
        "subtle" => (8.0, 4),
        other => {
            return Err(CliFailure::new(
                CliErrorKind::InvalidInput,
                format!("unsupported depth_of_field strength '{other}'; use subtle"),
            ));
        }
    };
    Ok(scena::DepthOfFieldConfig::new(
        observation.focus_distance_m,
        aperture_f_stop,
        radius_px,
    ))
}

pub(crate) fn visible_subject_focus_observation(
    aov: &scena::SceneHostSemanticAovCaptureV1,
    target_handles: &BTreeSet<u64>,
) -> Result<SubjectFocusObservation, String> {
    if target_handles.is_empty() {
        return Err("subject focus target handle set is empty".to_owned());
    }
    if aov.id_indices.len() != aov.depth_meters.len() {
        return Err(format!(
            "semantic AOV id/depth lengths differ: ids={} depths={}",
            aov.id_indices.len(),
            aov.depth_meters.len()
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
        return Err("subject focus target has no semantic AOV palette entry".to_owned());
    }

    let mut depths = aov
        .id_indices
        .iter()
        .zip(aov.depth_meters.iter())
        .filter_map(|(palette_index, depth)| {
            (target_palette_indices.contains(palette_index) && depth.is_finite() && *depth > 0.0)
                .then_some(*depth)
        })
        .collect::<Vec<_>>();
    if depths.is_empty() {
        return Err("subject focus target has no finite visible depth samples".to_owned());
    }
    depths.sort_by(|left, right| {
        left.partial_cmp(right)
            .expect("finite depth samples compare")
    });
    let visible_pixel_count = depths.len();
    let focus_distance_m = percentile_sorted(&depths, 0.5);
    let near_depth_m = percentile_sorted(&depths, 0.05);
    let far_depth_m = percentile_sorted(&depths, 0.95);
    let target_pixels = aov
        .id_indices
        .iter()
        .filter(|palette_index| target_palette_indices.contains(palette_index))
        .count()
        .max(1);
    let confidence = (visible_pixel_count as f32 / target_pixels as f32).clamp(0.0, 1.0);
    Ok(SubjectFocusObservation {
        visible_pixel_count,
        focus_distance_m,
        near_depth_m,
        far_depth_m,
        confidence,
    })
}

fn subject_focus_can_degrade_to_report(message: &str) -> bool {
    message == "subject focus target has no semantic AOV palette entry"
        || message == "subject focus target has no finite visible depth samples"
}

fn subject_focus_unresolved_reason(
    host: &scena::SceneHostCore<scena::DefaultAssetFetcher>,
    handles: &BTreeSet<u64>,
    fallback: &str,
) -> Result<String, CliFailure> {
    let inspection_json = host.inspect_json().map_err(|error| {
        CliFailure::new(
            CliErrorKind::Runtime,
            format!("failed to inspect recipe scene for subject focus diagnosis: {error}"),
        )
    })?;
    let inspection: scena::SceneInspectionReportV1 = serde_json::from_str(&inspection_json)
        .map_err(|error| {
            CliFailure::new(
                CliErrorKind::Internal,
                format!("failed to decode subject focus inspection report: {error}"),
            )
        })?;
    if handles
        .iter()
        .any(|handle| node_or_ancestor_hidden(&inspection, *handle))
    {
        return Ok("subject_hidden".to_owned());
    }
    Ok(
        if fallback == "subject focus target has no semantic AOV palette entry" {
            "subject_visible_mask_unavailable".to_owned()
        } else {
            "subject_focus_unresolved".to_owned()
        },
    )
}

fn node_or_ancestor_hidden(inspection: &scena::SceneInspectionReportV1, handle: u64) -> bool {
    let Some(node) = inspection.node_by_handle(handle) else {
        return false;
    };
    if !node.visible {
        return true;
    }
    let mut parent = node.parent;
    while let Some(handle) = parent {
        let Some(parent_node) = inspection.node_by_handle(handle) else {
            return false;
        };
        if !parent_node.visible {
            return true;
        }
        parent = parent_node.parent;
    }
    false
}

fn percentile_sorted(values: &[f32], percentile: f32) -> f32 {
    debug_assert!(!values.is_empty());
    let index = ((values.len() - 1) as f32 * percentile.clamp(0.0, 1.0)).round() as usize;
    values[index]
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::visible_subject_focus_observation;
    use scena::{
        SCENE_HOST_SEMANTIC_AOV_SCHEMA_V1, SceneHostSemanticAovCaptureV1,
        SceneHostSemanticAovExclusionsV1, SceneHostSemanticAovLegendEntryV1,
    };

    #[test]
    fn visible_subject_focus_uses_target_depth_median_not_bounds_center() {
        let aov = SceneHostSemanticAovCaptureV1 {
            schema: SCENE_HOST_SEMANTIC_AOV_SCHEMA_V1.to_owned(),
            width: 6,
            height: 1,
            identity_scope: "runtime_scoped".to_owned(),
            sample_pattern: "single_center_sample".to_owned(),
            depth_convention: "linear_camera_distance_scene_meters".to_owned(),
            normal_space: "world".to_owned(),
            near: 0.1,
            far: 20.0,
            id_indices: vec![1, 2, 1, 1, 1, 0],
            depth_meters: vec![1.0, 0.4, 1.1, 1.2, 7.0, f32::INFINITY],
            world_normals: vec![[0.0, 0.0, 1.0]; 6],
            legend: vec![
                SceneHostSemanticAovLegendEntryV1 {
                    palette_index: 1,
                    rgba8: [1, 0, 0, 255],
                    node_handle: 10,
                    instance_handle: None,
                    instance_id: None,
                },
                SceneHostSemanticAovLegendEntryV1 {
                    palette_index: 2,
                    rgba8: [2, 0, 0, 255],
                    node_handle: 99,
                    instance_handle: None,
                    instance_id: None,
                },
            ],
            exclusions: SceneHostSemanticAovExclusionsV1::default(),
        };
        let handles = BTreeSet::from([10]);

        let observation =
            visible_subject_focus_observation(&aov, &handles).expect("target depths resolve");

        assert_eq!(observation.visible_pixel_count, 4);
        assert!(
            (observation.focus_distance_m - 1.2).abs() <= f32::EPSILON,
            "focus must use the median visible target depth, not the 4.0m bounds center fallback: {observation:?}",
        );
        assert!(
            observation.focus_distance_m < 2.0,
            "non-target and background depths must not pull focus behind the visible subject"
        );
    }
}
