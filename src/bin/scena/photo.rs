use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::scena_cli_error::{CliErrorKind, CliFailure, CliUsageError};
use super::scena_input::{capture_descriptor_path, ensure_parent_dir, path_for_json};
use super::scena_output::{
    CliBackendSelectionV1, CliOutcome, add_recipe_policy_to_outcome,
    json_outcome_with_backend_selection,
};
use super::scena_policy::{effective_recipe_policy, push_allow_root};

const PHOTO_RENDER_RESULT_SCHEMA_V1: &str = "scena.photo_render_result.v1";
const CAMERA_BEHAVIOR_INTENT: &str = "camera_behavior";
const CAMERA_BEHAVIOR_TARGET_MEAN_LUMA: f64 = 90.0;
const CAMERA_BEHAVIOR_MIN_MEAN_LUMA: f64 = 80.0;
const CAMERA_BEHAVIOR_MAX_MEAN_LUMA: f64 = 100.0;
const CAMERA_BEHAVIOR_MAX_LOW_CLIP: f64 = 0.20;
const CAMERA_BEHAVIOR_MAX_HIGH_CLIP: f64 = 0.05;
const CAMERA_BEHAVIOR_MIN_FILL_WIDTH: f64 = 0.65;
const CAMERA_BEHAVIOR_MAX_FILL_WIDTH: f64 = 0.85;
const CAMERA_BEHAVIOR_TARGET_FILL_WIDTH: f64 = 0.75;
const CAMERA_BEHAVIOR_MAX_FIT_FRACTION: f64 = 0.96;
const CAMERA_BEHAVIOR_MAX_CENTER_OFFSET: f64 = 0.16;
const CAMERA_BEHAVIOR_MIN_LUMA_STDDEV: f64 = 6.0;
const CAMERA_BEHAVIOR_MIN_LUMA_RANGE: f64 = 32.0;
const CAMERA_BEHAVIOR_MIN_EXPOSURE_EV: f32 = -8.0;
const CAMERA_BEHAVIOR_MAX_EXPOSURE_EV: f32 = 8.0;
const CAMERA_BEHAVIOR_MAX_ATTEMPTS: usize = 6;
const CAMERA_BEHAVIOR_COMPOSITION_CANDIDATE_BUDGET: usize = 10;
const CAMERA_BEHAVIOR_SHADED_CANDIDATE_BUDGET: usize = 3;
const CAMERA_BEHAVIOR_SHADED_CANDIDATE_WIDTH: u32 = 160;
const CAMERA_BEHAVIOR_SHADED_CANDIDATE_HEIGHT: u32 = 105;

#[derive(Debug, Clone, PartialEq)]
struct PhotoRenderArgs {
    input: PathBuf,
    intent: String,
    out: PathBuf,
    report: PathBuf,
    emit_recipe: Option<PathBuf>,
    width: u32,
    height: u32,
    gpu: bool,
    max_imports: Option<usize>,
    allow_roots: Vec<PathBuf>,
    subject: Option<scena::SceneRecipeTargetV1>,
}

#[derive(Debug, Clone, PartialEq)]
struct PhotoPlanArgs {
    input: PathBuf,
    intent: String,
    out: PathBuf,
    width: u32,
    height: u32,
    max_imports: Option<usize>,
    allow_roots: Vec<PathBuf>,
    subject: Option<scena::SceneRecipeTargetV1>,
}

#[derive(Debug)]
struct PhotoSource {
    recipe_text: String,
    recipe_path: String,
    source_kind: &'static str,
}

#[derive(Debug, Clone)]
pub(crate) struct SubjectSelection {
    target_kind: String,
    id: String,
    pub(crate) root_handle: u64,
    draw_handles: BTreeSet<u64>,
}

#[derive(Debug, Clone, Copy)]
struct SubjectMetrics {
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
    fill_fraction: f64,
    fill_width_fraction: f64,
    fill_height_fraction: f64,
    mean_luminance_srgb8: f64,
    luminance_stddev_srgb8: f64,
    luminance_range_srgb8: f64,
    background_separation_srgb8: f64,
    background_mean_luminance_srgb8: f64,
    low_clip_fraction: f64,
    high_clip_fraction: f64,
    center_offset_fraction: f64,
    clipped_fraction: f64,
    empty_space_fraction: f64,
    depth_variation: f64,
    normal_variation: f64,
    highlight_fraction: f64,
    highlight_continuity: f64,
    highlight_distribution: f64,
    shadow_presence: f64,
    shadow_softness: f64,
    silhouette_separation: f64,
    mean_saturation: f64,
    color_cast: f64,
    reflection_washout: f64,
    sample_count: u64,
}

#[derive(Debug, Clone, Copy)]
struct CameraBehaviorGateEvidence {
    metrics: SubjectMetrics,
    metering_domain_rejection_code: Option<&'static str>,
    focus_rejection_code: Option<&'static str>,
}

impl CameraBehaviorGateEvidence {
    fn from_metrics(metrics: SubjectMetrics) -> Self {
        Self {
            metrics,
            metering_domain_rejection_code: None,
            focus_rejection_code: None,
        }
    }

    #[cfg(test)]
    fn with_metering_domain_rejection(mut self, code: &'static str) -> Self {
        self.metering_domain_rejection_code = Some(code);
        self
    }

    #[cfg(test)]
    fn with_focus_rejection(mut self, code: &'static str) -> Self {
        self.focus_rejection_code = Some(code);
        self
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PhotoCandidate {
    id: String,
    exposure_ev: f32,
    composition_fill_fraction: f64,
    camera: PhotoCandidateCamera,
    metrics: SubjectMetrics,
    pub(crate) status: &'static str,
    pub(crate) failure_codes: Vec<&'static str>,
    adjustment: Option<&'static str>,
}

#[derive(Debug, Clone)]
struct PhotoCandidateCamera {
    world_transform: Option<scena::Transform>,
    projection: Option<scena::CaptureProjection>,
    vertical_fov_degrees: Option<f64>,
    focus_distance_m: Option<f32>,
}

impl PhotoCandidateCamera {
    fn from_capture(capture: &scena::CaptureRgba8, subject_bounds: Option<scena::Aabb>) -> Self {
        let world_transform = capture.descriptor.camera.world_transform;
        let projection = capture.descriptor.camera.projection;
        let vertical_fov_degrees = match projection {
            Some(scena::CaptureProjection::Perspective {
                vertical_fov_radians,
                ..
            }) => Some(f64::from(vertical_fov_radians).to_degrees()),
            _ => None,
        };
        let focus_distance_m = candidate_focus_distance_m(world_transform, subject_bounds);
        Self {
            world_transform,
            projection,
            vertical_fov_degrees,
            focus_distance_m,
        }
    }
}

#[derive(Debug, Clone)]
struct PhotoArtifactPaths {
    capture_png_path: String,
    capture_descriptor_path: String,
    emitted_recipe_path: Option<String>,
}

struct PhotoReportInput<'a> {
    args: &'a PhotoRenderArgs,
    source: &'a PhotoSource,
    manifest: &'a scena::SceneRecipeBuildV1,
    subject: &'a SubjectSelection,
    planning: &'a scena::PhotoCandidatePlanV1,
    shaded_selection: &'a ShadedCandidateSelection,
    final_work_metrics: PhotoLoopWorkMetrics,
    focus_work_metrics: PhotoLoopWorkMetrics,
    candidates: &'a [PhotoCandidate],
    selected: &'a PhotoCandidate,
    subject_bounds: Option<scena::Aabb>,
    focus_report: scena::FocusReportV1,
    exposure_report: scena::ExposureReportV1,
    subject_observation: scena::SubjectObservationV1,
    artifacts: PhotoArtifactPaths,
}

#[derive(Debug, Clone)]
pub(crate) struct ShadedCandidateSelection {
    selected_candidate_id: String,
    low_resolution: [u32; 2],
    candidate_budget: usize,
    candidates: Vec<ShadedCandidate>,
    scoring: scena::PhotoCandidateScoringReport,
    work_metrics: PhotoLoopWorkMetrics,
    surface_report: scena::PhotographicSurfaceReportV1,
}

#[derive(Debug, Clone)]
struct ShadedCandidate {
    id: String,
    order: usize,
    metrics: SubjectMetrics,
    render_quality: scena::RenderQualityReportV1,
    lighting_adjusted: bool,
    lighting_adjustment: Option<scena::scene_host::PhotographicLightingAdjustmentV1>,
}

#[derive(Debug, Clone, Copy, Default)]
struct PhotoLoopWorkMetrics {
    render_calls: u64,
    prepare_calls: u64,
    capture_calls: u64,
    gpu_readback_copies: u64,
    blocking_polls: u64,
    blocking_waits: u64,
    subject_meter_samples: u64,
}

impl PhotoLoopWorkMetrics {
    fn record_capture(&mut self, render_work: scena::RenderWorkMetrics) {
        self.render_calls = self.render_calls.saturating_add(1);
        self.prepare_calls = self.prepare_calls.saturating_add(1);
        self.capture_calls = self.capture_calls.saturating_add(1);
        self.gpu_readback_copies = self
            .gpu_readback_copies
            .saturating_add(render_work.readback_copies);
        self.blocking_polls = self
            .blocking_polls
            .saturating_add(render_work.blocking_polls);
        self.blocking_waits = self
            .blocking_waits
            .saturating_add(render_work.blocking_waits);
    }

    fn record_subject_samples(&mut self, samples: u64) {
        self.subject_meter_samples = self.subject_meter_samples.saturating_add(samples);
    }
}

pub(crate) fn run_photo_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    match args {
        [subcommand, rest @ ..] if subcommand == "render" => run_photo_render_command(rest),
        [subcommand, rest @ ..] if subcommand == "plan" => run_photo_plan_command(rest),
        _ => Err(CliUsageError::from(photo_usage()).into()),
    }
}

fn run_photo_plan_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    let args = PhotoPlanArgs::parse(args)?;
    let policy = effective_recipe_policy(&args.allow_roots, args.max_imports)?;
    let policy_report = policy.to_schema_report();
    let source = photo_source_for(&args.input, args.width, args.height)?;
    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        &source.recipe_path,
        &source.recipe_text,
        policy,
    ));
    let build = match build {
        Ok(build) => build,
        Err(manifest) => {
            return Err(CliFailure::new(
                CliErrorKind::InvalidInput,
                format!(
                    "photo plan recipe build failed with {} diagnostics; first code: {}",
                    manifest.diagnostics.len(),
                    manifest
                        .diagnostics
                        .first()
                        .map(|diagnostic| diagnostic.code.as_str())
                        .unwrap_or("unknown")
                ),
            ));
        }
    };

    let requested_subject = photo_source_subject_target(&source, args.subject.as_ref())?;
    let subject = select_camera_behavior_subject(&build.manifest, requested_subject.as_ref())?;
    let host = build.host;
    let backend_selection = CliBackendSelectionV1::new(false, Some(host.backend()));
    let planning = camera_behavior_composition_plan(
        &host,
        subject.root_handle,
        !build.manifest.cameras.is_empty(),
    )?;
    let scoring = render_free_photo_plan_scoring(&planning);
    let staging_choices = unique_staging_choices(&planning);
    let plan = scena::PhotoPlanV1 {
        schema: scena::PHOTO_PLAN_SCHEMA_V1.to_owned(),
        intent: args.intent.clone(),
        source: scena::PhotoPlanSourceV1 {
            kind: source.source_kind.to_owned(),
            path: path_for_json(&args.input),
        },
        subject: scena::PhotoPlanSubjectV1 {
            target: scena::PhotoPlanTargetV1 {
                kind: subject.target_kind.clone(),
                id: subject.id.clone(),
            },
            root_handle: Some(subject.root_handle),
            draw_handle_count: Some(subject.draw_handles.len()),
        },
        selected_candidate_id: planning.selected_candidate_id.clone(),
        candidates_evaluated: planning.candidates.len(),
        rejected_candidate_reasons: plan_rejection_reasons(&planning, &scoring),
        staging_choices,
        planning,
        scoring: Some(scoring),
        artifacts: scena::PhotoPlanArtifactsV1 {
            emitted_recipe_path: None,
        },
    };

    ensure_parent_dir(&args.out)?;
    std::fs::write(
        &args.out,
        serde_json::to_string_pretty(&plan)
            .map_err(|error| format!("failed to serialize photo plan: {error}"))?,
    )
    .map_err(|error| {
        CliFailure::new(
            CliErrorKind::Io,
            format!("failed to write plan '{}': {error}", args.out.display()),
        )
    })?;
    let outcome = json_outcome_with_backend_selection(
        &plan,
        0,
        "failed to serialize photo plan result",
        backend_selection,
    )?;
    add_recipe_policy_to_outcome(outcome, &policy_report)
}

fn run_photo_render_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    let args = PhotoRenderArgs::parse(args)?;
    let policy = effective_recipe_policy(&args.allow_roots, args.max_imports)?;
    let policy_report = policy.to_schema_report();
    let source = photo_source(&args)?;
    let build = if args.gpu {
        pollster::block_on(scena::SceneHostCore::build_recipe_json_gpu(
            &source.recipe_path,
            &source.recipe_text,
            policy,
        ))
    } else {
        pollster::block_on(scena::SceneHostCore::build_recipe_json(
            &source.recipe_path,
            &source.recipe_text,
            policy,
        ))
    };
    let build = match build {
        Ok(build) => build,
        Err(manifest) => {
            let outcome = json_outcome_with_backend_selection(
                &json!({
                    "schema": PHOTO_RENDER_RESULT_SCHEMA_V1,
                    "ok": false,
                    "intent": CAMERA_BEHAVIOR_INTENT,
                    "status": "failed",
                    "failure_codes": ["recipe_build_failed"],
                    "build": manifest,
                }),
                1,
                "failed to serialize photo render build failure",
                CliBackendSelectionV1::new(args.gpu, None),
            )?;
            return add_recipe_policy_to_outcome(outcome, &policy_report);
        }
    };

    let requested_subject = photo_source_subject_target(&source, args.subject.as_ref())?;
    let subject = select_camera_behavior_subject(&build.manifest, requested_subject.as_ref())?;
    emit_effective_recipe(&args, &source)?;
    let authored_lights = !build.manifest.lights.is_empty();
    let authored_camera = !build.manifest.cameras.is_empty();
    let mut host = build.host;
    // GPU hosts must own semantic AOV resources before the next prepare, because
    // every camera-behavior measurement below reads them back.
    if args.gpu {
        host.set_semantic_aov_capture_enabled(true);
    }
    let backend_selection = CliBackendSelectionV1::new(args.gpu, Some(host.backend()));
    let planning = camera_behavior_composition_plan(&host, subject.root_handle, authored_camera)?;
    let mut shaded_selection = apply_camera_behavior_setup_with_plan(
        &mut host,
        &subject,
        authored_lights,
        &planning,
        args.gpu,
    )?;
    let selected_composition =
        selected_shaded_composition_candidate(&planning, &shaded_selection)?.clone();
    let mut selected =
        render_camera_behavior_candidates(&mut host, &subject, &selected_composition, args.gpu)?;
    let visible_focus =
        apply_visible_subject_physical_focus(&mut host, &subject, &selected_composition, args.gpu)?;
    // Focus resolution sets depth of field, so the accepted candidate's capture
    // predates it. Re-render once through the same raster path the loop used, so
    // the delivered image, the reported metrics, and the focus report all
    // describe the same frame.
    let mut focus_work = PhotoLoopWorkMetrics::default();
    if visible_focus.is_some() {
        selected.capture = render_capture(&mut host, &mut focus_work)?;
    }
    let inspection_json = host.inspect_json().map_err(runtime_failure)?;
    let inspection: scena::SceneInspectionReportV1 = serde_json::from_str(&inspection_json)
        .map_err(|error| {
            CliFailure::new(
                CliErrorKind::Internal,
                format!("failed to decode final scene inspection report: {error}"),
            )
        })?;
    let final_aov = capture_camera_behavior_semantic_aovs(&mut host, args.gpu)?;
    let final_metrics = match measure_subject(&selected.capture, &inspection, &final_aov, &subject)
    {
        Ok(metrics) => metrics,
        Err(error) if photo_subject_measurement_can_degrade(&error.message) => {
            empty_subject_metrics()
        }
        Err(error) => return Err(error),
    };
    record_texture_resolution_health(
        &mut shaded_selection.surface_report,
        final_metrics,
        args.width,
        args.height,
    );
    selected.final_candidate.metrics = final_metrics;
    selected.final_candidate.failure_codes = camera_behavior_failure_codes(final_metrics);
    selected.final_candidate.status = if selected.final_candidate.failure_codes.is_empty() {
        "passed"
    } else {
        "failed"
    };
    if let Some(last) = selected.candidates.last_mut() {
        *last = selected.final_candidate.clone();
    }
    let capture = selected.capture;

    ensure_parent_dir(&args.out)?;
    capture.write_png(&args.out).map_err(|error| {
        CliFailure::new(
            CliErrorKind::Io,
            format!("failed to write PNG '{}': {error}", args.out.display()),
        )
    })?;
    let descriptor_path = capture_descriptor_path(&args.out);
    ensure_parent_dir(&descriptor_path)?;
    std::fs::write(
        &descriptor_path,
        serde_json::to_string_pretty(&capture.descriptor)
            .map_err(|error| format!("failed to serialize capture descriptor: {error}"))?,
    )
    .map_err(|error| {
        CliFailure::new(
            CliErrorKind::Io,
            format!(
                "failed to write capture descriptor '{}': {error}",
                descriptor_path.display()
            ),
        )
    })?;

    let subject_bounds = host.node_world_bounds(subject.root_handle).ok().flatten();
    let focus_report =
        camera_behavior_focus_report(&subject, subject_bounds, visible_focus.as_ref(), &capture);
    let exposure_report = camera_behavior_exposure_report(&selected.final_candidate, &capture);
    let subject_observation =
        camera_behavior_subject_observation(&subject, &selected.final_candidate, &capture);
    let report = photo_report(PhotoReportInput {
        args: &args,
        source: &source,
        manifest: &build.manifest,
        subject: &subject,
        planning: &planning,
        shaded_selection: &shaded_selection,
        final_work_metrics: selected.work_metrics,
        focus_work_metrics: focus_work,
        candidates: &selected.candidates,
        selected: &selected.final_candidate,
        subject_bounds,
        focus_report,
        exposure_report,
        subject_observation,
        artifacts: PhotoArtifactPaths {
            capture_png_path: path_for_json(&args.out),
            capture_descriptor_path: path_for_json(&descriptor_path),
            emitted_recipe_path: args.emit_recipe.as_deref().map(path_for_json),
        },
    });
    ensure_parent_dir(&args.report)?;
    std::fs::write(
        &args.report,
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("failed to serialize photo report: {error}"))?,
    )
    .map_err(|error| {
        CliFailure::new(
            CliErrorKind::Io,
            format!(
                "failed to write report '{}': {error}",
                args.report.display()
            ),
        )
    })?;

    let ok = selected.final_candidate.status == "passed";
    let outcome = json_outcome_with_backend_selection(
        &json!({
            "schema": PHOTO_RENDER_RESULT_SCHEMA_V1,
            "ok": ok,
            "intent": args.intent.as_str(),
            "status": if ok { "passed" } else { "failed" },
            "artifacts": {
                "capture_png_path": path_for_json(&args.out),
                "capture_descriptor_path": path_for_json(&descriptor_path),
                "report_path": path_for_json(&args.report),
                "emitted_recipe_path": args.emit_recipe.as_deref().map(path_for_json),
            },
            "quality": report["quality"].clone(),
            "selected": report["selected"].clone(),
            "failure_codes": report["failure_codes"].clone(),
        }),
        if ok { 0 } else { 1 },
        "failed to serialize photo render result",
        backend_selection,
    )?;
    add_recipe_policy_to_outcome(outcome, &policy_report)
}

pub(crate) struct SelectedCapture {
    pub(crate) capture: scena::CaptureRgba8,
    candidates: Vec<PhotoCandidate>,
    pub(crate) final_candidate: PhotoCandidate,
    work_metrics: PhotoLoopWorkMetrics,
}

/// Captures the semantic AOVs the camera-behavior loop measures from, using the
/// backend-matched entry point. GPU hosts must have opted into lifecycle-owned
/// AOV resources before `prepare()`.
fn capture_camera_behavior_semantic_aovs(
    host: &mut scena::SceneHostCore,
    gpu: bool,
) -> Result<scena::SceneHostSemanticAovCaptureV1, CliFailure> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if gpu {
            return host.capture_semantic_aovs_gpu().map_err(runtime_failure);
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        if gpu {
            return Err(CliFailure::new(
                CliErrorKind::Unsupported,
                "synchronous camera-behavior GPU semantic AOV capture is unavailable on wasm32; use the browser SceneHost async capture API",
            ));
        }
    }
    host.capture_semantic_aovs().map_err(runtime_failure)
}

pub(crate) fn render_camera_behavior_candidates(
    host: &mut scena::SceneHostCore,
    subject: &SubjectSelection,
    base_candidate: &scena::PhotoCompositionCandidateV1,
    gpu: bool,
) -> Result<SelectedCapture, CliFailure> {
    let mut candidates = Vec::new();
    let mut final_capture = None;
    let mut work_metrics = PhotoLoopWorkMetrics::default();
    let mut composition = base_candidate.clone();
    let mut pending_adjustment = Some("initial_camera_composition");
    let subject_bounds = host.node_world_bounds(subject.root_handle).ok().flatten();
    for attempt in 0..CAMERA_BEHAVIOR_MAX_ATTEMPTS {
        host.frame_node_with_photo_candidate(subject.root_handle, &composition)
            .map_err(runtime_failure)?;
        let capture = render_capture(host, &mut work_metrics)?;
        let semantic_aov = capture_camera_behavior_semantic_aovs(host, gpu)?;
        let camera = PhotoCandidateCamera::from_capture(&capture, subject_bounds);
        let inspection_json = host.inspect_json().map_err(runtime_failure)?;
        let inspection: scena::SceneInspectionReportV1 = serde_json::from_str(&inspection_json)
            .map_err(|error| {
                CliFailure::new(
                    CliErrorKind::Internal,
                    format!("failed to decode scene inspection report: {error}"),
                )
            })?;
        let metrics = match measure_subject(&capture, &inspection, &semantic_aov, subject) {
            Ok(metrics) => metrics,
            Err(error) if photo_subject_measurement_can_degrade(&error.message) => {
                let candidate = PhotoCandidate {
                    id: format!("candidate_{}", attempt + 1),
                    exposure_ev: host.renderer().exposure_ev(),
                    composition_fill_fraction: composition.fill_fraction,
                    camera,
                    metrics: empty_subject_metrics(),
                    status: "failed",
                    failure_codes: vec!["photo_subject_not_visible"],
                    adjustment: pending_adjustment.take(),
                };
                final_capture = Some(capture);
                candidates.push(candidate);
                break;
            }
            Err(error) => return Err(error),
        };
        work_metrics.record_subject_samples(metrics.sample_count);
        let failure_codes = camera_behavior_failure_codes(metrics);
        let status = if failure_codes.is_empty() {
            "passed"
        } else {
            "failed"
        };
        let candidate = PhotoCandidate {
            id: format!("candidate_{}", attempt + 1),
            exposure_ev: host.renderer().exposure_ev(),
            composition_fill_fraction: composition.fill_fraction,
            camera,
            metrics,
            status,
            failure_codes,
            adjustment: pending_adjustment.take(),
        };
        final_capture = Some(capture);
        candidates.push(candidate.clone());
        if candidate.status == "passed" {
            return Ok(SelectedCapture {
                capture: final_capture.expect("candidate capture exists"),
                candidates,
                final_candidate: candidate,
                work_metrics,
            });
        }

        if attempt + 1 >= CAMERA_BEHAVIOR_MAX_ATTEMPTS {
            break;
        }
        if let Some(next_composition) = corrected_composition_candidate(&composition, metrics) {
            composition = next_composition;
            pending_adjustment = Some("camera_composition");
            continue;
        }
        let Some(next_ev) = corrected_exposure_ev(candidate.exposure_ev, metrics) else {
            break;
        };
        host.renderer_mut().clear_auto_exposure();
        host.renderer_mut().set_exposure_ev(next_ev);
        pending_adjustment = Some("exposure_delta");
    }

    let final_candidate = candidates.last().cloned().ok_or_else(|| {
        CliFailure::new(CliErrorKind::Runtime, "photo render produced no candidate")
    })?;
    Ok(SelectedCapture {
        capture: final_capture.expect("at least one candidate capture exists"),
        candidates,
        final_candidate,
        work_metrics,
    })
}

fn corrected_composition_candidate(
    current: &scena::PhotoCompositionCandidateV1,
    metrics: SubjectMetrics,
) -> Option<scena::PhotoCompositionCandidateV1> {
    if metrics.sample_count == 0 || !metrics.fill_width_fraction.is_finite() {
        return None;
    }
    let width_actionable = width_fill_target_is_actionable(metrics);
    let width_out_of_band = width_actionable
        && (metrics.fill_width_fraction < CAMERA_BEHAVIOR_MIN_FILL_WIDTH
            || metrics.fill_width_fraction > CAMERA_BEHAVIOR_MAX_FILL_WIDTH);
    let fit_out_of_band = metrics.fill_fraction < CAMERA_BEHAVIOR_MIN_FILL_WIDTH
        || metrics.fill_fraction > CAMERA_BEHAVIOR_MAX_FIT_FRACTION;
    let off_center = metrics.center_offset_fraction > CAMERA_BEHAVIOR_MAX_CENTER_OFFSET;
    let clipped = metrics.clipped_fraction > 0.01;
    if !width_out_of_band && !fit_out_of_band && !off_center && !clipped {
        return None;
    }
    let target_fit = if width_out_of_band {
        metrics.fill_fraction
            * (CAMERA_BEHAVIOR_TARGET_FILL_WIDTH / metrics.fill_width_fraction.max(0.001))
    } else if metrics.fill_fraction < CAMERA_BEHAVIOR_MIN_FILL_WIDTH {
        CAMERA_BEHAVIOR_TARGET_FILL_WIDTH
    } else if clipped {
        (metrics.fill_fraction * 0.86).min(CAMERA_BEHAVIOR_MAX_FIT_FRACTION * 0.90)
    } else if metrics.fill_fraction > CAMERA_BEHAVIOR_MAX_FIT_FRACTION {
        CAMERA_BEHAVIOR_MAX_FIT_FRACTION * 0.96
    } else {
        metrics.fill_fraction
    };
    let next_fill = if width_out_of_band || fit_out_of_band || clipped {
        (current.fill_fraction * (target_fit / metrics.fill_fraction.max(0.001))).clamp(0.20, 1.0)
    } else {
        current.fill_fraction
    };
    if off_center && (next_fill - current.fill_fraction).abs() < 0.005 {
        let mut next = current.clone();
        let azimuth_step = if current.azimuth_deg >= 0.0 {
            -12.0
        } else {
            12.0
        };
        next.azimuth_deg = (current.azimuth_deg + azimuth_step).clamp(-75.0, 75.0);
        if (next.azimuth_deg - current.azimuth_deg).abs() < 0.005 {
            next.elevation_deg = (current.elevation_deg + 6.0).clamp(-15.0, 60.0);
        }
        next.view = "camera_solver_recenter".to_owned();
        next.lens = "measured".to_owned();
        next.id = format!(
            "camera_solver_recenter_az_{}_el_{}_fill_{}",
            decimal_id(next.azimuth_deg, 1),
            decimal_id(next.elevation_deg, 1),
            decimal_id(next.fill_fraction, 3)
        );
        return Some(next);
    }
    if (next_fill - current.fill_fraction).abs() < 0.005 {
        return None;
    }
    let mut next = current.clone();
    next.fill_fraction = next_fill;
    next.view = "camera_solver".to_owned();
    next.lens = "measured".to_owned();
    next.id = format!("camera_solver_fill_{}", decimal_id(next.fill_fraction, 3));
    Some(next)
}

fn width_fill_target_is_actionable(metrics: SubjectMetrics) -> bool {
    if metrics.fill_fraction <= 0.0 || !metrics.fill_fraction.is_finite() {
        return false;
    }
    let width_at_max_fit =
        metrics.fill_width_fraction / metrics.fill_fraction * CAMERA_BEHAVIOR_MAX_FIT_FRACTION;
    width_at_max_fit >= CAMERA_BEHAVIOR_MIN_FILL_WIDTH
}

fn photo_subject_measurement_can_degrade(message: &str) -> bool {
    message == "photo subject did not project to the rendered frame"
        || message == "photo subject projected to an empty pixel region"
}

fn empty_subject_metrics() -> SubjectMetrics {
    SubjectMetrics {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 0.0,
        max_y: 0.0,
        fill_fraction: 0.0,
        fill_width_fraction: 0.0,
        fill_height_fraction: 0.0,
        mean_luminance_srgb8: 0.0,
        luminance_stddev_srgb8: 0.0,
        luminance_range_srgb8: 0.0,
        background_separation_srgb8: 0.0,
        background_mean_luminance_srgb8: 0.0,
        low_clip_fraction: 1.0,
        high_clip_fraction: 0.0,
        center_offset_fraction: 1.0,
        clipped_fraction: 1.0,
        empty_space_fraction: 1.0,
        depth_variation: 0.0,
        normal_variation: 0.0,
        highlight_fraction: 0.0,
        highlight_continuity: 0.0,
        highlight_distribution: 0.0,
        shadow_presence: 0.0,
        shadow_softness: 0.0,
        silhouette_separation: 0.0,
        mean_saturation: 0.0,
        color_cast: 0.0,
        reflection_washout: 1.0,
        sample_count: 0,
    }
}

fn render_capture(
    host: &mut scena::SceneHostCore,
    work_metrics: &mut PhotoLoopWorkMetrics,
) -> Result<scena::CaptureRgba8, CliFailure> {
    host.prepare().map_err(runtime_failure)?;
    host.render().map_err(runtime_failure)?;
    let render_work = host.renderer().last_render_work_metrics();
    work_metrics.record_capture(render_work);
    host.capture().map_err(runtime_failure)
}

fn corrected_exposure_ev(current_ev: f32, metrics: SubjectMetrics) -> Option<f32> {
    if metrics.sample_count == 0 || !metrics.mean_luminance_srgb8.is_finite() {
        return None;
    }
    let correction =
        (CAMERA_BEHAVIOR_TARGET_MEAN_LUMA / metrics.mean_luminance_srgb8.max(1.0)).log2();
    if correction.abs() <= 0.03 && metrics.low_clip_fraction <= CAMERA_BEHAVIOR_MAX_LOW_CLIP {
        return None;
    }
    Some((current_ev + correction as f32).clamp(
        CAMERA_BEHAVIOR_MIN_EXPOSURE_EV,
        CAMERA_BEHAVIOR_MAX_EXPOSURE_EV,
    ))
}

fn corrected_photographic_lighting(
    metrics: SubjectMetrics,
) -> Option<scena::scene_host::PhotographicLightingAdjustmentV1> {
    if metrics.sample_count == 0 {
        return None;
    }
    let mut adjustment = scena::scene_host::PhotographicLightingAdjustmentV1::default();
    let mut changed = false;
    if metrics.luminance_stddev_srgb8 < CAMERA_BEHAVIOR_MIN_LUMA_STDDEV
        || metrics.luminance_range_srgb8 < CAMERA_BEHAVIOR_MIN_LUMA_RANGE
    {
        adjustment.key_scale = 1.08;
        adjustment.fill_scale = 0.70;
        adjustment.overhead_scale = 0.82;
        adjustment.environment_rotation_offset_degrees = 35.0;
        changed = true;
    }
    if metrics.background_separation_srgb8 < 12.0 {
        adjustment.rim_scale = 1.55;
        changed = true;
    }
    if metrics.highlight_continuity < 0.08 || metrics.highlight_distribution < 0.25 {
        adjustment.key_scale *= 1.12;
        adjustment.environment_rotation_offset_degrees += 28.0;
        changed = true;
    }
    if metrics.reflection_washout > 0.20 {
        adjustment.key_scale *= 0.82;
        adjustment.environment_intensity_scale *= 0.78;
        adjustment.environment_rotation_offset_degrees += 42.0;
        changed = true;
    }
    if metrics.shadow_presence < 0.01 {
        adjustment.key_scale *= 1.06;
        adjustment.fill_scale *= 0.76;
        changed = true;
    } else if metrics.shadow_softness < 0.20 {
        adjustment.key_scale *= 0.90;
        adjustment.overhead_scale *= 1.12;
        changed = true;
    }
    if metrics.silhouette_separation < 0.10 {
        adjustment.rim_scale *= 1.45;
        changed = true;
    }
    if metrics.high_clip_fraction > CAMERA_BEHAVIOR_MAX_HIGH_CLIP {
        adjustment.key_scale *= 0.78;
        adjustment.overhead_scale *= 0.82;
        adjustment.environment_intensity_scale = 0.88;
        changed = true;
    } else if metrics.low_clip_fraction > CAMERA_BEHAVIOR_MAX_LOW_CLIP {
        adjustment.fill_scale *= 1.18;
        changed = true;
    }
    changed.then_some(adjustment)
}

fn camera_behavior_failure_codes(metrics: SubjectMetrics) -> Vec<&'static str> {
    camera_behavior_acceptance_failure_codes(CameraBehaviorGateEvidence::from_metrics(metrics))
}

fn camera_behavior_acceptance_failure_codes(
    evidence: CameraBehaviorGateEvidence,
) -> Vec<&'static str> {
    let metrics = evidence.metrics;
    let mut failures = Vec::new();
    if let Some(code) = evidence.metering_domain_rejection_code {
        failures.push(code);
    }
    if let Some(code) = evidence.focus_rejection_code {
        failures.push(code);
    }
    if metrics.sample_count == 0 {
        failures.push("subject_visible_pixels_missing");
    }
    if metrics.fill_fraction < CAMERA_BEHAVIOR_MIN_FILL_WIDTH
        || (width_fill_target_is_actionable(metrics)
            && metrics.fill_width_fraction < CAMERA_BEHAVIOR_MIN_FILL_WIDTH)
    {
        failures.push("subject_fill_below_min");
    }
    if metrics.fill_fraction > CAMERA_BEHAVIOR_MAX_FIT_FRACTION
        || metrics.fill_width_fraction > CAMERA_BEHAVIOR_MAX_FILL_WIDTH
    {
        failures.push("subject_fill_above_max");
    }
    if metrics.mean_luminance_srgb8 < CAMERA_BEHAVIOR_MIN_MEAN_LUMA {
        failures.push("subject_luminance_below_min");
    }
    if metrics.mean_luminance_srgb8 > CAMERA_BEHAVIOR_MAX_MEAN_LUMA {
        failures.push("subject_luminance_above_max");
    }
    if metrics.low_clip_fraction > CAMERA_BEHAVIOR_MAX_LOW_CLIP {
        failures.push("subject_low_clip_above_max");
    }
    if metrics.high_clip_fraction > CAMERA_BEHAVIOR_MAX_HIGH_CLIP {
        failures.push("subject_high_clip_above_max");
    }
    if metrics.center_offset_fraction > CAMERA_BEHAVIOR_MAX_CENTER_OFFSET {
        failures.push("subject_center_offset_above_max");
    }
    if metrics.clipped_fraction > 0.01 {
        failures.push("subject_clipped_by_frame");
    }
    if metrics.luminance_stddev_srgb8 < CAMERA_BEHAVIOR_MIN_LUMA_STDDEV
        || metrics.luminance_range_srgb8 < CAMERA_BEHAVIOR_MIN_LUMA_RANGE
    {
        failures.push("subject_luminance_structure_below_min");
    }
    failures
}

fn photo_report(input: PhotoReportInput<'_>) -> Value {
    let PhotoReportInput {
        args,
        source,
        manifest,
        subject,
        planning,
        shaded_selection,
        final_work_metrics,
        focus_work_metrics,
        candidates,
        selected,
        subject_bounds,
        focus_report,
        exposure_report,
        subject_observation,
        artifacts,
    } = input;
    let failure_codes = selected
        .failure_codes
        .iter()
        .map(|code| Value::String((*code).to_owned()))
        .collect::<Vec<_>>();
    let subject_region = photo_subject_region(subject_bounds, &focus_report, &subject_observation);
    json!({
        "schema": scena::PHOTO_REPORT_SCHEMA_V1,
        "status": selected.status,
        "ok": selected.status == "passed",
        "intent": args.intent.as_str(),
        "source": {
            "kind": source.source_kind,
            "path": path_for_json(&args.input),
        },
        "subject": {
            "target": {
                "kind": subject.target_kind.clone(),
                "id": subject.id.clone(),
            },
            "root_handle": subject.root_handle,
            "draw_handle_count": subject.draw_handles.len(),
        },
        "planning": serde_json::to_value(planning)
            .expect("photo candidate plan serializes into photo report"),
        "shaded_selection": shaded_selection_json(shaded_selection),
        "selected": candidate_json(selected),
        "candidates": candidates.iter().map(candidate_json).collect::<Vec<_>>(),
        "retry": retry_json(candidates),
        "work_metrics": photo_work_metrics(
            args,
            planning,
            shaded_selection,
            final_work_metrics,
            focus_work_metrics,
        ),
        "acceptance": {
            "subject_fill_width_fraction": {
                "min": CAMERA_BEHAVIOR_MIN_FILL_WIDTH,
                "max": CAMERA_BEHAVIOR_MAX_FILL_WIDTH,
            },
            "subject_mean_luminance_srgb8": {
                "min": CAMERA_BEHAVIOR_MIN_MEAN_LUMA,
                "max": CAMERA_BEHAVIOR_MAX_MEAN_LUMA,
            },
            "subject_low_clip_fraction": {
                "max": CAMERA_BEHAVIOR_MAX_LOW_CLIP,
            },
            "subject_high_clip_fraction": {
                "max": CAMERA_BEHAVIOR_MAX_HIGH_CLIP,
            },
            "subject_center_offset_fraction": {
                "max": CAMERA_BEHAVIOR_MAX_CENTER_OFFSET,
            },
            "subject_luminance_stddev_srgb8": {
                "min": CAMERA_BEHAVIOR_MIN_LUMA_STDDEV,
            },
            "subject_luminance_range_srgb8": {
                "min": CAMERA_BEHAVIOR_MIN_LUMA_RANGE,
            },
        },
        "quality": {
            "subject": metrics_json(selected.metrics),
        },
        "focus_report": serde_json::to_value(focus_report)
            .expect("focus report serializes into photo report"),
        "exposure_report": serde_json::to_value(exposure_report)
            .expect("exposure report serializes into photo report"),
        "subject_observation": serde_json::to_value(subject_observation)
            .expect("subject observation serializes into photo report"),
        "subject_region": serde_json::to_value(subject_region)
            .expect("subject region serializes into photo report"),
        "failure_codes": failure_codes,
        "artifacts": {
            "capture_png_path": artifacts.capture_png_path,
            "capture_descriptor_path": artifacts.capture_descriptor_path,
            "emitted_recipe_path": artifacts.emitted_recipe_path,
        },
        "build": {
            "ok": manifest.ok,
            "import_count": manifest.imports.len(),
            "node_count": manifest.nodes.len(),
        },
    })
}

fn photo_subject_region(
    world_bounds: Option<scena::Aabb>,
    focus_report: &scena::FocusReportV1,
    observation: &scena::SubjectObservationV1,
) -> scena::PhotoSubjectRegionV1 {
    let focus_distance_m = focus_report
        .resolved
        .as_ref()
        .map(|resolved| round2(resolved.focus_distance_m));
    scena::PhotoSubjectRegionV1::from_subject_observation(
        world_bounds,
        focus_distance_m,
        observation,
    )
}

fn photo_work_metrics(
    args: &PhotoRenderArgs,
    planning: &scena::PhotoCandidatePlanV1,
    shaded_selection: &ShadedCandidateSelection,
    final_work: PhotoLoopWorkMetrics,
    focus_work: PhotoLoopWorkMetrics,
) -> Value {
    let shaded_work = shaded_selection.work_metrics;
    let shaded_candidate_pixels = u64::from(shaded_selection.low_resolution[0])
        .saturating_mul(u64::from(shaded_selection.low_resolution[1]));
    let final_candidate_pixels = u64::from(args.width).saturating_mul(u64::from(args.height));
    let total_render_calls = shaded_work
        .render_calls
        .saturating_add(final_work.render_calls)
        .saturating_add(focus_work.render_calls);
    let prepare_calls = shaded_work
        .prepare_calls
        .saturating_add(final_work.prepare_calls)
        .saturating_add(focus_work.prepare_calls);
    let capture_calls = shaded_work
        .capture_calls
        .saturating_add(final_work.capture_calls)
        .saturating_add(focus_work.capture_calls);
    let subject_meter_samples = shaded_work
        .subject_meter_samples
        .saturating_add(final_work.subject_meter_samples)
        .saturating_add(focus_work.subject_meter_samples);
    json!({
        "timing_policy": "report_only",
        "wall_clock_thresholds": "not_used",
        "allocation_policy": "bounded_by_candidate_count_and_frame_pixels",
        "composition_candidate_budget": planning.budget,
        "composition_candidates": planning.candidates.len(),
        "shaded_candidate_budget": shaded_selection.candidate_budget,
        "shaded_candidate_renders": shaded_work.render_calls,
        "shaded_candidate_width": shaded_selection.low_resolution[0],
        "shaded_candidate_height": shaded_selection.low_resolution[1],
        "shaded_candidate_pixels": shaded_candidate_pixels
            .saturating_mul(shaded_work.render_calls),
        "final_candidate_render_budget": CAMERA_BEHAVIOR_MAX_ATTEMPTS,
        "final_candidate_renders": final_work.render_calls,
        "final_candidate_width": args.width,
        "final_candidate_height": args.height,
        "final_candidate_pixels": final_candidate_pixels
            .saturating_mul(final_work.render_calls),
        "focus_delivery_renders": focus_work.render_calls,
        "total_render_calls": total_render_calls,
        "prepare_calls": prepare_calls,
        "extra_prepare_operations": prepare_calls.saturating_sub(1),
        "capture_calls": capture_calls,
        "gpu_readback_copies": shaded_work
            .gpu_readback_copies
            .saturating_add(final_work.gpu_readback_copies),
        "blocking_polls": shaded_work
            .blocking_polls
            .saturating_add(final_work.blocking_polls),
        "blocking_waits": shaded_work
            .blocking_waits
            .saturating_add(final_work.blocking_waits),
        "subject_meter_samples": subject_meter_samples,
        "shaded_subject_meter_samples": shaded_work.subject_meter_samples,
        "final_subject_meter_samples": final_work.subject_meter_samples,
    })
}

fn camera_behavior_exposure_report(
    candidate: &PhotoCandidate,
    capture: &scena::CaptureRgba8,
) -> scena::ExposureReportV1 {
    let metrics = candidate.metrics;
    let suggested_compensation_ev = corrected_exposure_ev(candidate.exposure_ev, metrics)
        .map(|next_ev| next_ev - candidate.exposure_ev)
        .unwrap_or(0.0);
    scena::ExposureReportV1::measured_subject(
        "camera_behavior_retry",
        candidate.exposure_ev,
        scena::ExposureReportSubjectV1::new(
            round2_f64(metrics.mean_luminance_srgb8),
            round4(metrics.low_clip_fraction),
            round4(metrics.high_clip_fraction),
            metrics.sample_count,
        ),
        suggested_compensation_ev,
        &capture.descriptor,
    )
}

fn camera_behavior_focus_report(
    subject: &SubjectSelection,
    subject_bounds: Option<scena::Aabb>,
    visible_focus: Option<&super::scena_recipe::subject_focus::SubjectFocusObservation>,
    capture: &scena::CaptureRgba8,
) -> scena::FocusReportV1 {
    let target = scena::FocusReportTargetV1 {
        kind: subject.target_kind.clone(),
        id: subject.id.clone(),
        handles: subject_handles(subject),
    };
    if let Some(observation) = visible_focus {
        return scena::FocusReportV1::resolved(
            "subject",
            target,
            Some("visible_subject_depth".to_owned()),
            Some("physical_circle_of_confusion".to_owned()),
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
    let Some(bounds) = subject_bounds else {
        return scena::FocusReportV1::unresolved(
            "subject",
            target,
            Some("subject".to_owned()),
            Some("camera_auto".to_owned()),
            "subject world bounds were unavailable",
            &capture.descriptor,
        );
    };
    let Some(camera_transform) = capture.descriptor.camera.world_transform else {
        return scena::FocusReportV1::unresolved(
            "subject",
            target,
            Some("subject".to_owned()),
            Some("camera_auto".to_owned()),
            "capture descriptor did not include a camera transform",
            &capture.descriptor,
        );
    };
    let focus_distance =
        candidate_focus_distance_m(Some(camera_transform), Some(bounds)).unwrap_or(0.001);
    let radius = bounds.bounding_sphere_radius();
    scena::FocusReportV1::resolved(
        "subject",
        target,
        Some("subject".to_owned()),
        Some("camera_auto".to_owned()),
        scena::FocusReportResolvedV1 {
            focus_distance_m: focus_distance,
            near_depth_m: (focus_distance - radius).max(0.001),
            far_depth_m: (focus_distance + radius).max(0.001),
            visible_pixel_count: 1,
            confidence: 0.65,
        },
        &capture.descriptor,
    )
}

fn apply_visible_subject_physical_focus(
    host: &mut scena::SceneHostCore,
    subject: &SubjectSelection,
    candidate: &scena::PhotoCompositionCandidateV1,
    gpu: bool,
) -> Result<Option<super::scena_recipe::subject_focus::SubjectFocusObservation>, CliFailure> {
    let aov = capture_camera_behavior_semantic_aovs(host, gpu)?;
    let handles = subject_handles(subject)
        .into_iter()
        .collect::<BTreeSet<_>>();
    let observation =
        match super::scena_recipe::subject_focus::visible_subject_focus_observation(&aov, &handles)
        {
            Ok(observation) => observation,
            Err(message)
                if message == "subject focus target has no semantic AOV palette entry"
                    || message == "subject focus target has no finite visible depth samples" =>
            {
                return Ok(None);
            }
            Err(message) => {
                return Err(CliFailure::new(
                    CliErrorKind::Runtime,
                    format!("failed to resolve photographic autofocus: {message}"),
                ));
            }
        };
    let physical = candidate.physical_camera;
    let aperture = physical_aperture_for_depth_range(
        physical.focal_length_mm as f32,
        physical.circle_of_confusion_mm as f32,
        observation.focus_distance_m,
        observation.near_depth_m,
        observation.far_depth_m,
    );
    host.renderer_mut()
        .set_depth_of_field(Some(scena::DepthOfFieldConfig::physical(
            observation.focus_distance_m,
            physical.focal_length_mm as f32,
            physical.sensor_height_mm as f32,
            aperture,
            physical.aperture_blades,
            16,
        )));
    Ok(Some(observation))
}

fn physical_aperture_for_depth_range(
    focal_length_mm: f32,
    circle_of_confusion_mm: f32,
    focus_distance_m: f32,
    near_depth_m: f32,
    far_depth_m: f32,
) -> f32 {
    let focal_m = focal_length_mm.clamp(8.0, 600.0) * 0.001;
    let coc_m = circle_of_confusion_mm.clamp(0.005, 0.1) * 0.001;
    let focus = focus_distance_m.max(focal_m + 1.0e-4);
    let image_focus = focal_m * focus / (focus - focal_m);
    [near_depth_m, far_depth_m]
        .into_iter()
        .filter(|depth| depth.is_finite() && *depth > focal_m)
        .map(|depth| {
            let image_depth = focal_m * depth / (depth - focal_m);
            focal_m * (image_depth - image_focus).abs() / (coc_m * image_depth.max(1.0e-6))
        })
        .fold(2.8_f32, f32::max)
        .clamp(2.8, 22.0)
}

fn candidate_focus_distance_m(
    camera_transform: Option<scena::Transform>,
    subject_bounds: Option<scena::Aabb>,
) -> Option<f32> {
    let camera_transform = camera_transform?;
    let bounds = subject_bounds?;
    Some(
        (camera_transform.translation - bounds.center())
            .length()
            .max(0.001),
    )
}

fn camera_behavior_subject_observation(
    subject: &SubjectSelection,
    candidate: &PhotoCandidate,
    capture: &scena::CaptureRgba8,
) -> scena::SubjectObservationV1 {
    let metrics = candidate.metrics;
    let bounds = subject_bounds_from_metrics(metrics);
    scena::SubjectObservationV1::observed(
        "photo.subject",
        scena::SubjectObservationTargetV1::new(
            subject.target_kind.clone(),
            subject.id.clone(),
            subject_handles(subject),
        ),
        &capture.descriptor,
        bounds,
        bounds,
        scena::SubjectObservationMetricsV1 {
            visible_pixel_count: metrics.sample_count,
            projected_area_px: bounds.area_px,
            visible_fill_fraction: metrics.fill_width_fraction as f32,
            visible_fraction_of_projected: 1.0,
            occlusion_estimate: 0.0,
        },
        None,
        scena::SubjectObservationFallbackV1 {
            degraded: false,
            flags: vec!["geometry_derived_semantic_mask".to_owned()],
            reason_codes: Vec::new(),
        },
    )
    .with_pixel_quality(scena::SubjectObservationPixelQualityV1 {
        mean_luminance_srgb8: round2_f64(metrics.mean_luminance_srgb8),
        luminance_stddev_srgb8: round2_f64(metrics.luminance_stddev_srgb8),
        luminance_range_srgb8: round2_f64(metrics.luminance_range_srgb8),
        low_clip_fraction: round4(metrics.low_clip_fraction),
        high_clip_fraction: round4(metrics.high_clip_fraction),
        sample_count: metrics.sample_count,
    })
}

fn subject_handles(subject: &SubjectSelection) -> Vec<u64> {
    let mut handles = Vec::with_capacity(subject.draw_handles.len() + 1);
    handles.push(subject.root_handle);
    handles.extend(subject.draw_handles.iter().copied());
    handles.sort_unstable();
    handles.dedup();
    handles
}

fn subject_bounds_from_metrics(metrics: SubjectMetrics) -> scena::SubjectObservationBoundsV1 {
    let width = (metrics.max_x - metrics.min_x).max(0.0);
    let height = (metrics.max_y - metrics.min_y).max(0.0);
    scena::SubjectObservationBoundsV1 {
        min_x: round2(metrics.min_x) as f32,
        min_y: round2(metrics.min_y) as f32,
        max_x: round2(metrics.max_x) as f32,
        max_y: round2(metrics.max_y) as f32,
        width: round2(width) as f32,
        height: round2(height) as f32,
        area_px: (width * height).round() as u64,
    }
}

fn shaded_selection_json(selection: &ShadedCandidateSelection) -> Value {
    let scores_by_id = selection
        .scoring
        .scores
        .iter()
        .map(|score| (score.candidate_id.as_str(), score))
        .collect::<std::collections::BTreeMap<_, _>>();
    json!({
        "schema": scena::PHOTO_SHADED_CANDIDATE_SELECTION_SCHEMA_V1,
        "status": if selection.candidates.is_empty() { "failed" } else { "passed" },
        "selected_candidate_id": selection.selected_candidate_id,
        "low_resolution": {
            "width": selection.low_resolution[0],
            "height": selection.low_resolution[1],
        },
        "candidate_budget": selection.candidate_budget,
        "evaluated_count": selection.candidates.len(),
        "asset_health": serde_json::to_value(&selection.surface_report)
            .expect("photographic asset health report serializes"),
        "work_metrics": {
            "rendered_candidates": selection.candidates.len(),
            "candidate_width": selection.low_resolution[0],
            "candidate_height": selection.low_resolution[1],
            "candidate_pixel_count": u64::from(selection.low_resolution[0])
                * u64::from(selection.low_resolution[1]),
            "total_candidate_pixels": selection.candidates.len() as u64
                * u64::from(selection.low_resolution[0])
                * u64::from(selection.low_resolution[1]),
        },
        "scoring": {
            "degraded": selection.scoring.degraded,
            "reason_codes": selection.scoring.reason_codes,
        },
        "candidates": selection.candidates.iter().map(|candidate| {
            let score = scores_by_id.get(candidate.id.as_str());
            json!({
                "id": candidate.id,
                "order": candidate.order,
                "score": score.map(|score| round2_f64(score.score)),
                "reason_codes": score.map(|score| score.reason_codes.clone()).unwrap_or_default(),
                "subject": metrics_json(candidate.metrics),
                "render_quality": serde_json::to_value(&candidate.render_quality)
                    .expect("candidate render quality serializes into photo report"),
                "lighting_adjusted": candidate.lighting_adjusted,
                "lighting_adjustment": candidate.lighting_adjustment,
            })
        }).collect::<Vec<_>>(),
    })
}

fn retry_json(candidates: &[PhotoCandidate]) -> Value {
    let budget_exhausted = candidates.len() >= CAMERA_BEHAVIOR_MAX_ATTEMPTS
        && candidates
            .last()
            .is_some_and(|candidate| candidate.status != "passed");
    let first_retryable_failure =
        candidates
            .iter()
            .find(|candidate| candidate.status != "passed")
            .and_then(|candidate| {
                if candidate.failure_codes.iter().any(|code| {
                    matches!(*code, "subject_fill_below_min" | "subject_fill_above_max")
                }) {
                    return Some(json!({
                        "source_candidate_id": candidate.id,
                        "kind": "camera_composition",
                        "target_fill_width_fraction": CAMERA_BEHAVIOR_TARGET_FILL_WIDTH,
                    }));
                }
                corrected_exposure_ev(candidate.exposure_ev, candidate.metrics).map(|next_ev| {
                    json!({
                        "source_candidate_id": candidate.id,
                        "kind": "exposure_compensation_ev",
                        "delta_ev": next_ev - candidate.exposure_ev,
                        "next_exposure_ev": next_ev,
                    })
                })
            });
    let retry_input = if candidates.len() > 1 {
        candidates.get(1).map(|candidate| {
            json!({
                "candidate_id": candidate.id,
                "exposure_ev": candidate.exposure_ev,
            })
        })
    } else {
        None
    };
    json!({
        "policy": {
            "max_attempts": CAMERA_BEHAVIOR_MAX_ATTEMPTS,
            "max_retries": CAMERA_BEHAVIOR_MAX_ATTEMPTS.saturating_sub(1),
            "allowed_adjustments": ["camera_composition", "exposure_compensation_ev"],
            "loop": "bounded",
        },
        "attempts": candidates.len(),
        "retry_used": candidates.len() > 1,
        "budget_exhausted": budget_exhausted,
        "final_attempt_id": candidates.last().map(|candidate| candidate.id.clone()),
        "suggestion": first_retryable_failure,
        "retry_input": retry_input,
    })
}

fn candidate_json(candidate: &PhotoCandidate) -> Value {
    json!({
        "id": candidate.id,
        "status": candidate.status,
        "exposure_ev": candidate.exposure_ev,
        "composition_fill_fraction": round4(candidate.composition_fill_fraction),
        "camera": camera_json(&candidate.camera),
        "adjustment": candidate.adjustment,
        "failure_codes": candidate.failure_codes,
        "subject": metrics_json(candidate.metrics),
    })
}

fn camera_json(camera: &PhotoCandidateCamera) -> Value {
    json!({
        "source": "capture_descriptor",
        "world_transform": camera.world_transform,
        "projection": camera.projection,
        "vertical_fov_degrees": camera.vertical_fov_degrees.map(round2_f64),
        "focus_distance_m": camera.focus_distance_m.map(round2),
    })
}

fn metrics_json(metrics: SubjectMetrics) -> Value {
    json!({
        "rect_css_px": {
            "min_x": round2(metrics.min_x),
            "min_y": round2(metrics.min_y),
            "max_x": round2(metrics.max_x),
            "max_y": round2(metrics.max_y),
            "width": round2(metrics.max_x - metrics.min_x),
            "height": round2(metrics.max_y - metrics.min_y),
        },
        "fill_fraction": round4(metrics.fill_fraction),
        "fill_width_fraction": round4(metrics.fill_width_fraction),
        "fill_height_fraction": round4(metrics.fill_height_fraction),
        "mean_luminance_srgb8": round2_f64(metrics.mean_luminance_srgb8),
        "luminance_stddev_srgb8": round2_f64(metrics.luminance_stddev_srgb8),
        "luminance_range_srgb8": round2_f64(metrics.luminance_range_srgb8),
        "background_separation_srgb8": round2_f64(metrics.background_separation_srgb8),
        "background_mean_luminance_srgb8": round2_f64(metrics.background_mean_luminance_srgb8),
        "low_clip_fraction": round4(metrics.low_clip_fraction),
        "high_clip_fraction": round4(metrics.high_clip_fraction),
        "center_offset_fraction": round4(metrics.center_offset_fraction),
        "clipped_fraction": round4(metrics.clipped_fraction),
        "empty_space_fraction": round4(metrics.empty_space_fraction),
        "depth_variation": round4(metrics.depth_variation),
        "normal_variation": round4(metrics.normal_variation),
        "highlight_fraction": round4(metrics.highlight_fraction),
        "highlight_continuity": round4(metrics.highlight_continuity),
        "highlight_distribution": round4(metrics.highlight_distribution),
        "shadow_presence": round4(metrics.shadow_presence),
        "shadow_softness": round4(metrics.shadow_softness),
        "silhouette_separation": round4(metrics.silhouette_separation),
        "mean_saturation": round4(metrics.mean_saturation),
        "color_cast": round4(metrics.color_cast),
        "reflection_washout": round4(metrics.reflection_washout),
        "sample_count": metrics.sample_count,
    })
}

fn measure_subject(
    capture: &scena::CaptureRgba8,
    inspection: &scena::SceneInspectionReportV1,
    semantic_aov: &scena::SceneHostSemanticAovCaptureV1,
    subject: &SubjectSelection,
) -> Result<SubjectMetrics, CliFailure> {
    if semantic_aov.width != capture.descriptor.width
        || semantic_aov.height != capture.descriptor.height
        || semantic_aov.id_indices.len()
            != (capture.descriptor.width * capture.descriptor.height) as usize
    {
        return Err(CliFailure::new(
            CliErrorKind::Runtime,
            "photo semantic subject mask dimensions do not match the rendered frame",
        ));
    }
    let subject_handles = subject_handles(subject)
        .into_iter()
        .collect::<BTreeSet<_>>();
    let subject_palette = semantic_aov
        .legend
        .iter()
        .filter(|entry| {
            subject_handles.contains(&entry.node_handle)
                || entry
                    .instance_handle
                    .is_some_and(|handle| subject_handles.contains(&handle))
        })
        .map(|entry| entry.palette_index)
        .collect::<BTreeSet<_>>();
    if subject_palette.is_empty() {
        return Err(CliFailure::new(
            CliErrorKind::Runtime,
            "photo subject has no semantic AOV palette entry",
        ));
    }
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for draw in &inspection.draw_list {
        if !subject.draw_handles.contains(&draw.node) {
            continue;
        }
        let Some(rect) =
            scena::project_aabb_from_capture(capture, draw.local_bounds, draw.world_transform)
        else {
            continue;
        };
        min_x = min_x.min(rect.min_x);
        min_y = min_y.min(rect.min_y);
        max_x = max_x.max(rect.max_x);
        max_y = max_y.max(rect.max_y);
    }
    if !min_x.is_finite() || !min_y.is_finite() || !max_x.is_finite() || !max_y.is_finite() {
        return Err(CliFailure::new(
            CliErrorKind::Runtime,
            "photo subject did not project to the rendered frame",
        ));
    }
    let raw_min_x = min_x;
    let raw_min_y = min_y;
    let raw_max_x = max_x;
    let raw_max_y = max_y;
    min_x = min_x.max(0.0).min(capture.descriptor.width as f32);
    min_y = min_y.max(0.0).min(capture.descriptor.height as f32);
    max_x = max_x.max(min_x).min(capture.descriptor.width as f32);
    max_y = max_y.max(min_y).min(capture.descriptor.height as f32);
    let raw_area =
        f64::from((raw_max_x - raw_min_x).max(0.0)) * f64::from((raw_max_y - raw_min_y).max(0.0));
    let clamped_area = f64::from((max_x - min_x).max(0.0)) * f64::from((max_y - min_y).max(0.0));
    let clipped_fraction = if raw_area > 0.0 {
        (1.0 - clamped_area / raw_area).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let projected_min_x = min_x;
    let projected_min_y = min_y;
    let projected_max_x = max_x;
    let projected_max_y = max_y;
    let start_x = min_x.floor() as u32;
    let start_y = min_y.floor() as u32;
    let end_x = max_x.ceil() as u32;
    let end_y = max_y.ceil() as u32;
    let mut visible_min_x = f32::INFINITY;
    let mut visible_min_y = f32::INFINITY;
    let mut visible_max_x = f32::NEG_INFINITY;
    let mut visible_max_y = f32::NEG_INFINITY;
    let mut sum_luma = 0.0_f64;
    let mut sum_luma_sq = 0.0_f64;
    let mut min_luma = f64::INFINITY;
    let mut max_luma = f64::NEG_INFINITY;
    let mut background_delta_sum = 0.0_f64;
    let mut low_clip = 0_u64;
    let mut high_clip = 0_u64;
    let mut sample_count = 0_u64;
    let background = capture_background_rgba8(capture);
    for y in start_y..end_y {
        for x in start_x..end_x {
            let offset = ((y as usize) * capture.descriptor.width as usize + x as usize) * 4;
            let Some(pixel) = capture.rgba8.get(offset..offset + 4) else {
                continue;
            };
            let pixel_index = y as usize * capture.descriptor.width as usize + x as usize;
            if !semantic_aov
                .id_indices
                .get(pixel_index)
                .is_some_and(|index| subject_palette.contains(index))
            {
                continue;
            }
            if pixel[3] == 0 {
                continue;
            }
            let background_delta = max_rgb_delta_rgba8(pixel, background);
            let luma = 0.2126 * f64::from(pixel[0])
                + 0.7152 * f64::from(pixel[1])
                + 0.0722 * f64::from(pixel[2]);
            sum_luma += luma;
            sum_luma_sq += luma * luma;
            background_delta_sum += f64::from(background_delta);
            min_luma = min_luma.min(luma);
            max_luma = max_luma.max(luma);
            visible_min_x = visible_min_x.min(x as f32);
            visible_min_y = visible_min_y.min(y as f32);
            visible_max_x = visible_max_x.max((x + 1) as f32);
            visible_max_y = visible_max_y.max((y + 1) as f32);
            if luma <= 10.0 {
                low_clip = low_clip.saturating_add(1);
            }
            if luma >= 245.0 {
                high_clip = high_clip.saturating_add(1);
            }
            sample_count = sample_count.saturating_add(1);
        }
    }
    if sample_count == 0 {
        return Err(CliFailure::new(
            CliErrorKind::Runtime,
            "photo subject projected to an empty pixel region",
        ));
    }
    min_x = visible_min_x;
    min_y = visible_min_y;
    max_x = visible_max_x;
    max_y = visible_max_y;
    let fill_width_fraction = f64::from((projected_max_x - projected_min_x).max(0.0))
        / f64::from(capture.descriptor.width.max(1));
    let fill_height_fraction = f64::from((projected_max_y - projected_min_y).max(0.0))
        / f64::from(capture.descriptor.height.max(1));
    let mean_luminance_srgb8 = sum_luma / sample_count as f64;
    let variance = (sum_luma_sq / sample_count as f64 - mean_luminance_srgb8.powi(2)).max(0.0);
    let center_x = (f64::from(projected_min_x) + f64::from(projected_max_x)) * 0.5;
    let center_y = (f64::from(projected_min_y) + f64::from(projected_max_y)) * 0.5;
    let frame_center_x = f64::from(capture.descriptor.width) * 0.5;
    let frame_center_y = f64::from(capture.descriptor.height) * 0.5;
    let center_offset_x =
        (center_x - frame_center_x).abs() / f64::from(capture.descriptor.width.max(1));
    let center_offset_y =
        (center_y - frame_center_y).abs() / f64::from(capture.descriptor.height.max(1));
    let appearance = measure_subject_appearance(
        capture,
        semantic_aov,
        &subject_palette,
        [
            projected_min_x,
            projected_min_y,
            projected_max_x,
            projected_max_y,
        ],
        mean_luminance_srgb8,
        variance.sqrt(),
    );
    Ok(SubjectMetrics {
        min_x,
        min_y,
        max_x,
        max_y,
        fill_fraction: fill_width_fraction.max(fill_height_fraction),
        fill_width_fraction,
        fill_height_fraction,
        mean_luminance_srgb8,
        luminance_stddev_srgb8: variance.sqrt(),
        luminance_range_srgb8: max_luma - min_luma,
        background_separation_srgb8: background_delta_sum / sample_count as f64,
        background_mean_luminance_srgb8: appearance.background_mean_luminance_srgb8,
        low_clip_fraction: low_clip as f64 / sample_count as f64,
        high_clip_fraction: high_clip as f64 / sample_count as f64,
        center_offset_fraction: center_offset_x.max(center_offset_y),
        clipped_fraction,
        empty_space_fraction: appearance.empty_space_fraction,
        depth_variation: appearance.depth_variation,
        normal_variation: appearance.normal_variation,
        highlight_fraction: appearance.highlight_fraction,
        highlight_continuity: appearance.highlight_continuity,
        highlight_distribution: appearance.highlight_distribution,
        shadow_presence: appearance.shadow_presence,
        shadow_softness: appearance.shadow_softness,
        silhouette_separation: appearance.silhouette_separation,
        mean_saturation: appearance.mean_saturation,
        color_cast: appearance.color_cast,
        reflection_washout: appearance.reflection_washout,
        sample_count,
    })
}

#[derive(Debug, Clone, Copy, Default)]
struct SubjectAppearanceMeasurements {
    background_mean_luminance_srgb8: f64,
    empty_space_fraction: f64,
    depth_variation: f64,
    normal_variation: f64,
    highlight_fraction: f64,
    highlight_continuity: f64,
    highlight_distribution: f64,
    shadow_presence: f64,
    shadow_softness: f64,
    silhouette_separation: f64,
    mean_saturation: f64,
    color_cast: f64,
    reflection_washout: f64,
}

fn measure_subject_appearance(
    capture: &scena::CaptureRgba8,
    semantic_aov: &scena::SceneHostSemanticAovCaptureV1,
    subject_palette: &BTreeSet<u32>,
    projected_rect: [f32; 4],
    subject_mean_luminance_srgb8: f64,
    subject_luminance_stddev_srgb8: f64,
) -> SubjectAppearanceMeasurements {
    let width = capture.descriptor.width as usize;
    let height = capture.descriptor.height as usize;
    let is_subject = |index: usize| {
        semantic_aov
            .id_indices
            .get(index)
            .is_some_and(|palette| subject_palette.contains(palette))
    };
    let mut subject_count = 0_u64;
    let mut depth_count = 0_u64;
    let mut depth_mean = 0.0_f64;
    let mut depth_m2 = 0.0_f64;
    let mut normal_sum = scena::Vec3::ZERO;
    let mut normal_count = 0_u64;
    let mut rgb_sum = [0.0_f64; 3];
    let mut saturation_sum = 0.0_f64;
    let mut highlight_count = 0_u64;
    let mut highlight_adjacencies = 0_u64;
    let mut highlight_quadrants = [false; 4];
    let mut background_luma_sum = 0.0_f64;
    let mut background_count = 0_u64;
    let mut silhouette_delta_sum = 0.0_f64;
    let mut silhouette_count = 0_u64;
    let background = capture_background_rgba8(capture);
    let highlight_threshold =
        (subject_mean_luminance_srgb8 + subject_luminance_stddev_srgb8 * 0.8).clamp(140.0, 240.0);

    for index in 0..width.saturating_mul(height) {
        let offset = index * 4;
        let Some(pixel) = capture.rgba8.get(offset..offset + 4) else {
            continue;
        };
        let luma = pixel_luminance(pixel);
        if !is_subject(index) {
            background_luma_sum += luma;
            background_count = background_count.saturating_add(1);
            continue;
        }
        subject_count = subject_count.saturating_add(1);
        for channel in 0..3 {
            rgb_sum[channel] += f64::from(pixel[channel]);
        }
        let maximum = f64::from(pixel[0].max(pixel[1]).max(pixel[2]));
        let minimum = f64::from(pixel[0].min(pixel[1]).min(pixel[2]));
        saturation_sum += if maximum > 0.0 {
            (maximum - minimum) / maximum
        } else {
            0.0
        };
        if let Some(depth) = semantic_aov
            .depth_meters
            .get(index)
            .copied()
            .filter(|depth| depth.is_finite() && *depth > 0.0)
        {
            depth_count = depth_count.saturating_add(1);
            let delta = f64::from(depth) - depth_mean;
            depth_mean += delta / depth_count as f64;
            depth_m2 += delta * (f64::from(depth) - depth_mean);
        }
        if let Some(normal) = semantic_aov.world_normals.get(index) {
            let normal = scena::Vec3::from_array(*normal).normalize_or_zero();
            if normal.length_squared() > 0.5 {
                normal_sum += normal;
                normal_count = normal_count.saturating_add(1);
            }
        }
        if luma >= highlight_threshold {
            highlight_count = highlight_count.saturating_add(1);
            let x = index % width;
            let y = index / width;
            highlight_quadrants[usize::from(x >= width / 2) + 2 * usize::from(y >= height / 2)] =
                true;
            if x + 1 < width && is_subject(index + 1) {
                let neighbor = &capture.rgba8[(index + 1) * 4..(index + 1) * 4 + 4];
                highlight_adjacencies +=
                    u64::from(pixel_luminance(neighbor) >= highlight_threshold);
            }
            if y + 1 < height && is_subject(index + width) {
                let neighbor = &capture.rgba8[(index + width) * 4..(index + width) * 4 + 4];
                highlight_adjacencies +=
                    u64::from(pixel_luminance(neighbor) >= highlight_threshold);
            }
        }
        let x = index % width;
        let y = index / width;
        let boundary = (x > 0 && !is_subject(index - 1))
            || (x + 1 < width && !is_subject(index + 1))
            || (y > 0 && !is_subject(index - width))
            || (y + 1 < height && !is_subject(index + width));
        if boundary {
            silhouette_delta_sum += f64::from(max_rgb_delta_rgba8(pixel, background));
            silhouette_count = silhouette_count.saturating_add(1);
        }
    }

    let background_mean_luminance_srgb8 = if background_count > 0 {
        background_luma_sum / background_count as f64
    } else {
        pixel_luminance(&background)
    };
    let mut shadow_count = 0_u64;
    let mut soft_shadow_count = 0_u64;
    let min_x = projected_rect[0].floor().max(0.0) as usize;
    let max_x = projected_rect[2].ceil().min(width as f32) as usize;
    let start_y = projected_rect[3].floor().max(0.0) as usize;
    let end_y = (start_y + (height / 10).max(2)).min(height);
    let mut support_samples = 0_u64;
    for y in start_y..end_y {
        for x in min_x..max_x {
            let index = y * width + x;
            if is_subject(index) {
                continue;
            }
            let luma = pixel_luminance(&capture.rgba8[index * 4..index * 4 + 4]);
            let delta = background_mean_luminance_srgb8 - luma;
            support_samples = support_samples.saturating_add(1);
            if delta >= 6.0 {
                shadow_count = shadow_count.saturating_add(1);
                soft_shadow_count += u64::from(delta <= 48.0);
            }
        }
    }
    let subject_count_f64 = subject_count.max(1) as f64;
    let mean_rgb = rgb_sum.map(|sum| sum / subject_count_f64);
    let mean_channel = (mean_rgb[0] + mean_rgb[1] + mean_rgb[2]) / 3.0;
    let color_cast = mean_rgb
        .into_iter()
        .map(|channel| (channel - mean_channel).abs())
        .fold(0.0, f64::max)
        / 255.0;
    let mean_saturation = saturation_sum / subject_count_f64;
    let highlight_fraction = highlight_count as f64 / subject_count_f64;
    SubjectAppearanceMeasurements {
        background_mean_luminance_srgb8,
        empty_space_fraction: 1.0
            - (((projected_rect[2] - projected_rect[0]).max(0.0)
                * (projected_rect[3] - projected_rect[1]).max(0.0)) as f64
                / (width.max(1) * height.max(1)) as f64)
                .clamp(0.0, 1.0),
        depth_variation: if depth_count > 1 && depth_mean > 1.0e-6 {
            (depth_m2 / (depth_count - 1) as f64).sqrt() / depth_mean
        } else {
            0.0
        },
        normal_variation: if normal_count > 0 {
            1.0 - f64::from((normal_sum / normal_count as f32).length().clamp(0.0, 1.0))
        } else {
            0.0
        },
        highlight_fraction,
        highlight_continuity: if highlight_count > 0 {
            (highlight_adjacencies as f64 / (highlight_count as f64 * 2.0)).clamp(0.0, 1.0)
        } else {
            0.0
        },
        highlight_distribution: highlight_quadrants
            .into_iter()
            .filter(|value| *value)
            .count() as f64
            / 4.0,
        shadow_presence: shadow_count as f64 / support_samples.max(1) as f64,
        shadow_softness: soft_shadow_count as f64 / shadow_count.max(1) as f64,
        silhouette_separation: silhouette_delta_sum / silhouette_count.max(1) as f64 / 255.0,
        mean_saturation,
        color_cast,
        reflection_washout: (highlight_fraction * (1.0 - mean_saturation)).clamp(0.0, 1.0),
    }
}

fn pixel_luminance(pixel: &[u8]) -> f64 {
    0.2126 * f64::from(pixel[0]) + 0.7152 * f64::from(pixel[1]) + 0.0722 * f64::from(pixel[2])
}

fn capture_background_rgba8(capture: &scena::CaptureRgba8) -> [u8; 4] {
    capture
        .rgba8
        .get(0..4)
        .map(|pixel| [pixel[0], pixel[1], pixel[2], pixel[3]])
        .unwrap_or([0, 0, 0, 255])
}

fn max_rgb_delta_rgba8(pixel: &[u8], background: [u8; 4]) -> u8 {
    (0..3)
        .map(|channel| pixel[channel].abs_diff(background[channel]))
        .max()
        .unwrap_or(0)
}

pub(crate) fn select_camera_behavior_subject(
    manifest: &scena::SceneRecipeBuildV1,
    target: Option<&scena::SceneRecipeTargetV1>,
) -> Result<SubjectSelection, CliFailure> {
    let target = match target {
        Some(target) => target.clone(),
        None => {
            let Some(import) = manifest.imports.first() else {
                return Err(CliFailure::new(
                    CliErrorKind::InvalidInput,
                    "photo render requires an imported or authored subject",
                ));
            };
            scena::SceneRecipeTargetV1::Import {
                id: import.id.clone(),
            }
        }
    };
    let handles = resolve_photo_subject_handles(manifest, &target)?;
    let (target_kind, id) = photo_target_kind_and_id(&target);
    let root_handle = match &target {
        scena::SceneRecipeTargetV1::Import { id } => manifest
            .imports
            .iter()
            .find(|import| import.id == *id)
            .and_then(|import| {
                import
                    .primary_root
                    .or_else(|| import.root_handles.first().copied())
            })
            .or_else(|| handles.first().copied())
            .ok_or_else(|| {
                CliFailure::new(
                    CliErrorKind::Runtime,
                    format!("photo subject import '{id}' resolved to no root handle"),
                )
            })?,
        scena::SceneRecipeTargetV1::Node { .. } => handles[0],
        scena::SceneRecipeTargetV1::World { .. } => unreachable!("world targets are rejected"),
    };
    let draw_handles = handles.into_iter().collect::<BTreeSet<_>>();
    Ok(SubjectSelection {
        target_kind,
        id,
        root_handle,
        draw_handles,
    })
}

fn resolve_photo_subject_handles(
    manifest: &scena::SceneRecipeBuildV1,
    target: &scena::SceneRecipeTargetV1,
) -> Result<Vec<u64>, CliFailure> {
    scena::resolve_scene_recipe_target_handles(
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
            format!("failed to resolve photo subject target: {}", error.message)
        } else {
            format!(
                "failed to resolve photo subject target: {}; nearest candidates: {}",
                error.message,
                error.candidates.join(", ")
            )
        };
        CliFailure::new(kind, message)
    })
}

fn photo_target_kind_and_id(target: &scena::SceneRecipeTargetV1) -> (String, String) {
    match target {
        scena::SceneRecipeTargetV1::Import { id } => ("import".to_owned(), id.clone()),
        scena::SceneRecipeTargetV1::Node { id } => ("node".to_owned(), id.clone()),
        scena::SceneRecipeTargetV1::World { .. } => ("world".to_owned(), "world".to_owned()),
    }
}

pub(crate) fn apply_camera_behavior_setup_with_plan(
    host: &mut scena::SceneHostCore,
    subject: &SubjectSelection,
    authored_lights: bool,
    planning: &scena::PhotoCandidatePlanV1,
    gpu: bool,
) -> Result<ShadedCandidateSelection, CliFailure> {
    let surface_report = host
        .apply_photographic_surface(subject.root_handle)
        .map_err(runtime_failure)?;
    ensure_photographic_asset_usable(&surface_report)?;
    let _surroundings = host
        .apply_photographic_surroundings(subject.root_handle)
        .map_err(runtime_failure)?;
    configure_camera_behavior_renderer(host);
    let shaded_selection = select_camera_behavior_shaded_candidate(
        host,
        subject,
        planning,
        authored_lights,
        surface_report,
        gpu,
    )?;
    let selected = selected_shaded_composition_candidate(planning, &shaded_selection)?;
    host.frame_node_with_photo_candidate(subject.root_handle, selected)
        .map_err(runtime_failure)?;
    let selected_adjustment = shaded_selection
        .candidates
        .iter()
        .find(|candidate| candidate.id == shaded_selection.selected_candidate_id)
        .and_then(|candidate| candidate.lighting_adjustment);
    if !authored_lights || selected_adjustment.is_some() {
        let adjustment = selected_adjustment.unwrap_or_default();
        host.apply_photographic_lighting_adjusted(subject.root_handle, adjustment)
            .map_err(runtime_failure)?;
    }
    configure_camera_behavior_renderer(host);
    Ok(shaded_selection)
}

pub(crate) fn camera_behavior_composition_plan(
    host: &scena::SceneHostCore,
    root_handle: u64,
    preserve_authored_camera: bool,
) -> Result<scena::PhotoCandidatePlanV1, CliFailure> {
    let subject_bounds = host
        .node_world_bounds(root_handle)
        .map_err(runtime_failure)?
        .ok_or_else(|| {
            CliFailure::new(
                CliErrorKind::Runtime,
                "photo subject has no world bounds for camera behavior candidate planning",
            )
        })?;
    let request =
        scena::PhotoCandidateRequest::camera_behavior(subject_bounds, host.viewport_size())
            .fill_range(
                CAMERA_BEHAVIOR_MIN_FILL_WIDTH,
                CAMERA_BEHAVIOR_MAX_FILL_WIDTH,
            )
            .max_candidates(CAMERA_BEHAVIOR_COMPOSITION_CANDIDATE_BUDGET);
    let request = request.preserve_authored_camera(preserve_authored_camera);
    let mut plan = scena::camera_behavior_candidate_plan(request).map_err(runtime_failure)?;
    if preserve_authored_camera
        && let Some(candidate) = plan.candidates.first_mut()
        && let Some(camera_key) = host.scene().active_camera()
        && let Some(scena::Camera::Perspective(camera)) = host.scene().camera(camera_key)
    {
        let sensor_height_mm = candidate.physical_camera.sensor_height_mm;
        let focal_length_mm = sensor_height_mm
            / (2.0 * f64::from(camera.vertical_fov.radians() * 0.5).tan()).max(1.0e-6);
        candidate.focal_length_mm = focal_length_mm;
        candidate.physical_camera.focal_length_mm = focal_length_mm;
        if let Some(camera_node) = host.scene().camera_node(camera_key)
            && let Some(transform) = host.scene().world_transform(camera_node)
        {
            candidate.physical_camera.focus_distance_m = f64::from(
                (transform.translation - subject_bounds.center())
                    .length()
                    .max(0.001),
            );
        }
    }
    Ok(plan)
}

fn render_free_photo_plan_scoring(
    planning: &scena::PhotoCandidatePlanV1,
) -> scena::PhotoCandidateScoringReport {
    let scores = planning
        .candidates
        .iter()
        .map(|candidate| scena::PhotoCandidateScore {
            candidate_id: candidate.id.clone(),
            order: candidate.order,
            score: 100.0 - candidate.order as f64,
            reason_codes: if candidate.id == planning.selected_candidate_id {
                Vec::new()
            } else {
                vec!["lower_deterministic_plan_rank".to_owned()]
            },
        })
        .collect::<Vec<_>>();
    scena::PhotoCandidateScoringReport {
        selected_candidate_id: planning.selected_candidate_id.clone(),
        degraded: true,
        reason_codes: vec!["render_free_plan_no_shaded_scoring".to_owned()],
        scores,
    }
}

fn unique_staging_choices(
    planning: &scena::PhotoCandidatePlanV1,
) -> Vec<scena::PhotoCandidateStagingV1> {
    let mut seen = BTreeSet::new();
    let mut choices = Vec::new();
    for candidate in &planning.candidates {
        if seen.insert(candidate.staging.id.clone()) {
            choices.push(candidate.staging.clone());
        }
    }
    choices
}

fn plan_rejection_reasons(
    planning: &scena::PhotoCandidatePlanV1,
    scoring: &scena::PhotoCandidateScoringReport,
) -> std::collections::BTreeMap<String, Vec<String>> {
    scoring
        .scores
        .iter()
        .filter(|score| score.candidate_id != planning.selected_candidate_id)
        .map(|score| {
            let reasons = if score.reason_codes.is_empty() {
                vec!["not_selected".to_owned()]
            } else {
                score.reason_codes.clone()
            };
            (score.candidate_id.clone(), reasons)
        })
        .collect()
}

fn configure_camera_behavior_renderer(host: &mut scena::SceneHostCore) {
    let renderer = host.renderer_mut();
    renderer.set_tonemapper(scena::Tonemapper::Aces);
    if renderer.auto_exposure().is_none() && !renderer.has_explicit_exposure_ev() {
        renderer.set_auto_exposure(
            scena::AutoExposureConfig::new(0.20)
                .with_ev_range(-8.0, 8.0)
                .with_highlight_guard(0.92, 0.78),
        );
    }
    renderer.set_bloom(Some(scena::PostBloomConfig::new(232, 0.04, 3)));
    if renderer.screen_space_ambient_occlusion().is_none() {
        renderer.set_screen_space_ambient_occlusion(Some(
            scena::ScreenSpaceAmbientOcclusionConfig::new(4, 0.32, 0.025),
        ));
    }
}

fn select_camera_behavior_shaded_candidate(
    host: &mut scena::SceneHostCore,
    subject: &SubjectSelection,
    planning: &scena::PhotoCandidatePlanV1,
    authored_lights: bool,
    surface_report: scena::PhotographicSurfaceReportV1,
    gpu: bool,
) -> Result<ShadedCandidateSelection, CliFailure> {
    let original_size = host.viewport_size();
    host.resize(
        CAMERA_BEHAVIOR_SHADED_CANDIDATE_WIDTH as f32,
        CAMERA_BEHAVIOR_SHADED_CANDIDATE_HEIGHT as f32,
        1.0,
    )
    .map_err(runtime_failure)?;

    let result = render_camera_behavior_shaded_candidates(
        host,
        subject,
        planning,
        authored_lights,
        surface_report,
        gpu,
    );
    let restore = host.resize(original_size.0 as f32, original_size.1 as f32, 1.0);
    match (result, restore) {
        (Ok(selection), Ok(())) => Ok(selection),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(runtime_failure(error)),
    }
}

fn render_camera_behavior_shaded_candidates(
    host: &mut scena::SceneHostCore,
    subject: &SubjectSelection,
    planning: &scena::PhotoCandidatePlanV1,
    authored_lights: bool,
    surface_report: scena::PhotographicSurfaceReportV1,
    gpu: bool,
) -> Result<ShadedCandidateSelection, CliFailure> {
    let mut scored_plan = planning.clone();
    scored_plan
        .candidates
        .truncate(CAMERA_BEHAVIOR_SHADED_CANDIDATE_BUDGET);
    scored_plan.budget = scored_plan.candidates.len();
    scored_plan.selected_candidate_id = scored_plan
        .candidates
        .first()
        .map(|candidate| candidate.id.clone())
        .ok_or_else(|| {
            CliFailure::new(
                CliErrorKind::Internal,
                "photo candidate plan had no candidates to shade-score",
            )
        })?;

    let mut candidates = Vec::with_capacity(scored_plan.candidates.len());
    let mut observations = Vec::with_capacity(scored_plan.candidates.len());
    let mut work_metrics = PhotoLoopWorkMetrics::default();
    for candidate in &scored_plan.candidates {
        host.frame_node_with_photo_candidate(subject.root_handle, candidate)
            .map_err(runtime_failure)?;
        let mut generated_lighting = if authored_lights {
            None
        } else {
            Some(
                host.apply_photographic_lighting(subject.root_handle)
                    .map_err(runtime_failure)?,
            )
        };
        let mut capture = render_capture(host, &mut work_metrics)?;
        let mut semantic_aov = capture_camera_behavior_semantic_aovs(host, gpu)?;
        let inspection_json = host.inspect_json().map_err(runtime_failure)?;
        let mut inspection: scena::SceneInspectionReportV1 = serde_json::from_str(&inspection_json)
            .map_err(|error| {
                CliFailure::new(
                    CliErrorKind::Internal,
                    format!("failed to decode shaded candidate inspection report: {error}"),
                )
            })?;
        let mut metrics = match measure_subject(&capture, &inspection, &semantic_aov, subject) {
            Ok(metrics) => metrics,
            Err(error) if photo_subject_measurement_can_degrade(&error.message) => {
                empty_subject_metrics()
            }
            Err(error) => return Err(error),
        };
        work_metrics.record_subject_samples(metrics.sample_count);
        let mut lighting_adjustment = None;
        if let Some(adjustment) = corrected_photographic_lighting(metrics) {
            if let Some(previous) = generated_lighting.take() {
                remove_generated_photographic_lights(host, previous)?;
            }
            generated_lighting = Some(
                host.apply_photographic_lighting_adjusted(subject.root_handle, adjustment)
                    .map_err(runtime_failure)?,
            );
            capture = render_capture(host, &mut work_metrics)?;
            semantic_aov = capture_camera_behavior_semantic_aovs(host, gpu)?;
            let inspection_json = host.inspect_json().map_err(runtime_failure)?;
            inspection = serde_json::from_str(&inspection_json).map_err(|error| {
                CliFailure::new(
                    CliErrorKind::Internal,
                    format!(
                        "failed to decode adjusted shaded candidate inspection report: {error}"
                    ),
                )
            })?;
            metrics = match measure_subject(&capture, &inspection, &semantic_aov, subject) {
                Ok(metrics) => metrics,
                Err(error) if photo_subject_measurement_can_degrade(&error.message) => {
                    empty_subject_metrics()
                }
                Err(error) => return Err(error),
            };
            work_metrics.record_subject_samples(metrics.sample_count);
            lighting_adjustment = Some(adjustment);
        }
        let quality = shaded_candidate_quality(host, &capture, &inspection, metrics);
        observations.push(shaded_candidate_observation(
            &candidate.id,
            metrics,
            &capture,
        ));
        candidates.push(ShadedCandidate {
            id: candidate.id.clone(),
            order: candidate.order,
            metrics,
            render_quality: quality,
            lighting_adjusted: lighting_adjustment.is_some(),
            lighting_adjustment,
        });
        if let Some(generated_lighting) = generated_lighting {
            remove_generated_photographic_lights(host, generated_lighting)?;
        }
    }
    let scoring = scena::score_camera_behavior_candidates(&scored_plan, &observations)
        .map_err(runtime_failure)?;
    Ok(ShadedCandidateSelection {
        selected_candidate_id: scoring.selected_candidate_id.clone(),
        low_resolution: [
            CAMERA_BEHAVIOR_SHADED_CANDIDATE_WIDTH,
            CAMERA_BEHAVIOR_SHADED_CANDIDATE_HEIGHT,
        ],
        candidate_budget: CAMERA_BEHAVIOR_SHADED_CANDIDATE_BUDGET,
        candidates,
        scoring,
        work_metrics,
        surface_report,
    })
}

fn remove_generated_photographic_lights(
    host: &mut scena::SceneHostCore,
    report: scena::scene_host::PhotographicLightingReportV1,
) -> Result<(), CliFailure> {
    for light in report.lights {
        host.remove_node(light.node).map_err(runtime_failure)?;
    }
    Ok(())
}

fn ensure_photographic_asset_usable(
    report: &scena::PhotographicSurfaceReportV1,
) -> Result<(), CliFailure> {
    let unrecoverable = report
        .issues
        .iter()
        .filter(|issue| issue.class == scena::PhotographicAssetIssueClassV1::Unrecoverable)
        .collect::<Vec<_>>();
    if report.coherent_visible_subject && unrecoverable.is_empty() {
        return Ok(());
    }
    let details = unrecoverable
        .iter()
        .map(|issue| {
            let required = issue
                .required_input
                .as_deref()
                .map(|value| format!("; required: {value}"))
                .unwrap_or_default();
            format!("{}: {}{required}", issue.code, issue.message)
        })
        .collect::<Vec<_>>()
        .join(" | ");
    Err(CliFailure::new(
        CliErrorKind::InvalidInput,
        format!(
            "photographic asset rejected because safe repair cannot recover a coherent visible subject: {details}"
        ),
    ))
}

fn record_texture_resolution_health(
    report: &mut scena::PhotographicSurfaceReportV1,
    metrics: SubjectMetrics,
    width: u32,
    height: u32,
) {
    let Some(minimum_texture_dimension) = report.minimum_texture_dimension else {
        return;
    };
    let projected_dimension = ((metrics.max_x - metrics.min_x)
        .max(metrics.max_y - metrics.min_y)
        .max(0.0)) as u32;
    if projected_dimension <= minimum_texture_dimension.saturating_mul(2) {
        return;
    }
    report.issues.push(scena::PhotographicAssetIssueV1 {
        class: scena::PhotographicAssetIssueClassV1::AppearanceChangeRequired,
        code: "texture_resolution_below_output_demand".to_owned(),
        node: None,
        message: format!(
            "smallest authored texture is {minimum_texture_dimension}px while the subject spans {projected_dimension}px in a {width}x{height} output"
        ),
        required_input: Some(
            "supply a higher-resolution authorized texture or explicitly accept visible upscaling"
                .to_owned(),
        ),
    });
}

fn shaded_candidate_quality(
    host: &scena::SceneHostCore,
    capture: &scena::CaptureRgba8,
    inspection: &scena::SceneInspectionReportV1,
    metrics: SubjectMetrics,
) -> scena::RenderQualityReportV1 {
    let introspection = host.renderer().introspect_capture(
        capture,
        inspection,
        scena::RenderIntrospectionOptions::summary(),
    );
    let expectation = camera_behavior_shaded_quality_expectation();
    scena::evaluate_render_quality_rgba8_region(
        scena::RenderQualityRgba8Input {
            rgba8: &capture.rgba8,
            width: capture.descriptor.width,
            height: capture.descriptor.height,
            capabilities: introspection.capabilities,
            visible_pixel_fraction: metrics.fill_width_fraction as f32,
            tiny_in_frame: metrics.fill_width_fraction < CAMERA_BEHAVIOR_MIN_FILL_WIDTH,
            fit_fraction: metrics.fill_width_fraction as f32,
        },
        subject_render_quality_region(capture, metrics),
        Some(&expectation),
    )
}

fn camera_behavior_shaded_quality_expectation() -> scena::SceneRecipeQualityExpectationV1 {
    scena::SceneRecipeQualityExpectationV1 {
        profile: "product".to_owned(),
        exposure: Some(scena::SceneRecipeQualityExposureV1 {
            min_mean_luminance_srgb8: Some(CAMERA_BEHAVIOR_MIN_MEAN_LUMA),
            max_mean_luminance_srgb8: Some(CAMERA_BEHAVIOR_MAX_MEAN_LUMA),
            max_low_clip_fraction: Some(CAMERA_BEHAVIOR_MAX_LOW_CLIP),
            max_high_clip_fraction: Some(CAMERA_BEHAVIOR_MAX_HIGH_CLIP),
            max_clipped_highlight_fraction: Some(CAMERA_BEHAVIOR_MAX_HIGH_CLIP),
        }),
        contrast: Some(scena::SceneRecipeQualityContrastV1 {
            min_luminance_range: Some(CAMERA_BEHAVIOR_MIN_LUMA_RANGE / 255.0),
            min_sobel_energy: None,
            min_subject_luminance_range: Some(CAMERA_BEHAVIOR_MIN_LUMA_RANGE / 255.0),
        }),
        noise: None,
        text: None,
        line: None,
        geometry: None,
        reflection: None,
        area_light: None,
        grounding: None,
        depth_of_field: None,
    }
}

fn subject_render_quality_region(
    capture: &scena::CaptureRgba8,
    metrics: SubjectMetrics,
) -> scena::RenderQualityRegion {
    let x = metrics.min_x.floor().max(0.0) as u32;
    let y = metrics.min_y.floor().max(0.0) as u32;
    let max_x = metrics
        .max_x
        .ceil()
        .max(metrics.min_x)
        .min(capture.descriptor.width as f32) as u32;
    let max_y = metrics
        .max_y
        .ceil()
        .max(metrics.min_y)
        .min(capture.descriptor.height as f32) as u32;
    scena::RenderQualityRegion {
        kind: "subject",
        handle: None,
        x,
        y,
        width: max_x.saturating_sub(x).max(1),
        height: max_y.saturating_sub(y).max(1),
    }
}

fn shaded_candidate_observation(
    candidate_id: &str,
    metrics: SubjectMetrics,
    capture: &scena::CaptureRgba8,
) -> scena::PhotoCandidateObservation {
    let touches_frame_edge = metrics.min_x <= 1.0
        || metrics.min_y <= 1.0
        || metrics.max_x >= capture.descriptor.width.saturating_sub(1) as f32
        || metrics.max_y >= capture.descriptor.height.saturating_sub(1) as f32
        || metrics.max_x <= metrics.min_x + 1.0
        || metrics.max_y <= metrics.min_y + 1.0;
    scena::PhotoCandidateObservation::new(candidate_id)
        .visible_fill_fraction(metrics.fill_width_fraction)
        .center_offset_fraction(metrics.center_offset_fraction)
        .low_clip_fraction(metrics.low_clip_fraction)
        .high_clip_fraction(metrics.high_clip_fraction)
        .luminance_stddev_srgb8(metrics.luminance_stddev_srgb8)
        .luminance_range_srgb8(metrics.luminance_range_srgb8)
        .clipped_fraction(if touches_frame_edge { 1.0 } else { 0.0 })
        .occlusion_estimate(0.0)
        .floor_fraction((1.0 - metrics.fill_width_fraction).max(0.0) * 0.5)
        .silhouette_area_fraction(metrics.fill_width_fraction * metrics.fill_height_fraction)
        .aspect_fit_error((metrics.fill_width_fraction - metrics.fill_height_fraction).abs())
        .depth_variation(metrics.depth_variation)
        .normal_variation(metrics.normal_variation)
        .anchor_visibility_fraction(1.0)
        .background_separation(metrics.silhouette_separation)
        .appearance(
            metrics.empty_space_fraction,
            [
                metrics.highlight_fraction,
                metrics.highlight_continuity,
                metrics.highlight_distribution,
            ],
            [metrics.shadow_presence, metrics.shadow_softness],
            [
                metrics.silhouette_separation,
                metrics.mean_saturation,
                metrics.color_cast,
                metrics.reflection_washout,
            ],
        )
        .semantic_aov(true)
}

fn selected_composition_candidate(
    planning: &scena::PhotoCandidatePlanV1,
) -> Result<&scena::PhotoCompositionCandidateV1, CliFailure> {
    planning
        .candidates
        .iter()
        .find(|candidate| candidate.id == planning.selected_candidate_id)
        .ok_or_else(|| {
            CliFailure::new(
                CliErrorKind::Internal,
                format!(
                    "photo candidate plan selected missing candidate '{}'",
                    planning.selected_candidate_id
                ),
            )
        })
}

pub(crate) fn selected_shaded_composition_candidate<'a>(
    planning: &'a scena::PhotoCandidatePlanV1,
    selection: &ShadedCandidateSelection,
) -> Result<&'a scena::PhotoCompositionCandidateV1, CliFailure> {
    planning
        .candidates
        .iter()
        .find(|candidate| candidate.id == selection.selected_candidate_id)
        .or_else(|| selected_composition_candidate(planning).ok())
        .ok_or_else(|| {
            CliFailure::new(
                CliErrorKind::Internal,
                format!(
                    "shaded photo candidate selection referenced missing candidate '{}'",
                    selection.selected_candidate_id
                ),
            )
        })
}

fn photo_source(args: &PhotoRenderArgs) -> Result<PhotoSource, CliFailure> {
    photo_source_for(&args.input, args.width, args.height)
}

fn photo_source_for(input: &Path, width: u32, height: u32) -> Result<PhotoSource, CliFailure> {
    if input
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
    {
        let text = std::fs::read_to_string(input).map_err(|error| {
            CliFailure::new(
                CliErrorKind::InputNotFound,
                format!("failed to read recipe '{}': {error}", input.display()),
            )
        })?;
        let text = recipe_text_with_capture_override(text, input, width, height)?;
        return Ok(PhotoSource {
            recipe_text: text,
            recipe_path: input.display().to_string(),
            source_kind: "recipe",
        });
    }

    let recipe_path = std::env::current_dir()
        .map(|cwd| cwd.join("scena-photo.generated.recipe.json"))
        .unwrap_or_else(|_| PathBuf::from("scena-photo.generated.recipe.json"));
    Ok(PhotoSource {
        recipe_text: serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [{
                "id": "subject",
                "uri": path_for_json(input),
            }],
            "photo": {
                "intent": "camera_behavior",
                "subject": { "kind": "import", "id": "subject" },
            },
            "capture": {
                "width": width,
                "height": height,
            },
        }))
        .expect("generated photo recipe serializes"),
        recipe_path: recipe_path.display().to_string(),
        source_kind: "asset",
    })
}

fn photo_source_subject_target(
    source: &PhotoSource,
    override_target: Option<&scena::SceneRecipeTargetV1>,
) -> Result<Option<scena::SceneRecipeTargetV1>, CliFailure> {
    if let Some(target) = override_target {
        return Ok(Some(target.clone()));
    }
    if source.source_kind != "recipe" {
        return Ok(None);
    }
    let recipe =
        serde_json::from_str::<scena::SceneRecipeV1>(&source.recipe_text).map_err(|error| {
            CliFailure::new(
                CliErrorKind::InvalidInput,
                format!(
                    "photo recipe '{}' is not a valid scene recipe: {error}",
                    source.recipe_path
                ),
            )
        })?;
    Ok(recipe
        .photo
        .as_ref()
        .and_then(|photo| photo.subject.as_ref())
        .map(|subject| subject.target().clone()))
}

fn emit_effective_recipe(args: &PhotoRenderArgs, source: &PhotoSource) -> Result<(), CliFailure> {
    let Some(path) = &args.emit_recipe else {
        return Ok(());
    };
    ensure_parent_dir(path)?;
    std::fs::write(path, &source.recipe_text).map_err(|error| {
        CliFailure::new(
            CliErrorKind::Io,
            format!(
                "failed to write emitted recipe '{}': {error}",
                path.display()
            ),
        )
    })
}

fn recipe_text_with_capture_override(
    text: String,
    input: &Path,
    width: u32,
    height: u32,
) -> Result<String, CliFailure> {
    let mut value: Value = serde_json::from_str(&text).map_err(|error| {
        CliFailure::new(
            CliErrorKind::InvalidInput,
            format!("recipe '{}' is not valid JSON: {error}", input.display()),
        )
    })?;
    let Some(object) = value.as_object_mut() else {
        return Err(CliFailure::new(
            CliErrorKind::InvalidInput,
            format!("recipe '{}' must be a JSON object", input.display()),
        ));
    };
    object.insert(
        "capture".to_owned(),
        json!({
            "width": width,
            "height": height,
        }),
    );
    serde_json::to_string_pretty(&value)
        .map_err(|error| CliFailure::new(CliErrorKind::Internal, error.to_string()))
}

impl PhotoRenderArgs {
    fn parse(args: &[String]) -> Result<Self, CliUsageError> {
        let Some(input) = args.first() else {
            return Err(CliUsageError::from(photo_render_usage()));
        };
        let mut intent = None;
        let mut out = None;
        let mut report = None;
        let mut emit_recipe = None;
        let mut width = 1280_u32;
        let mut height = 840_u32;
        let mut gpu = false;
        let mut max_imports = None;
        let mut allow_roots = Vec::new();
        let mut subject = None;
        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
                "--intent" => {
                    let value = flag_value(args, index, "--intent")?;
                    intent = Some(normalize_intent(&value)?);
                    index += 2;
                }
                "--out" => {
                    out = Some(PathBuf::from(flag_value(args, index, "--out")?));
                    index += 2;
                }
                "--report" => {
                    report = Some(PathBuf::from(flag_value(args, index, "--report")?));
                    index += 2;
                }
                "--emit-recipe" => {
                    emit_recipe = Some(PathBuf::from(flag_value(args, index, "--emit-recipe")?));
                    index += 2;
                }
                "--width" => {
                    width = parse_dimension("--width", flag_value(args, index, "--width")?)?;
                    index += 2;
                }
                "--height" => {
                    height = parse_dimension("--height", flag_value(args, index, "--height")?)?;
                    index += 2;
                }
                "--gpu" => {
                    gpu = true;
                    index += 1;
                }
                "--subject" => {
                    subject = Some(parse_subject_override(flag_value(
                        args,
                        index,
                        "--subject",
                    )?)?);
                    index += 2;
                }
                "--max-imports" => {
                    max_imports = Some(parse_positive_usize(
                        "--max-imports",
                        flag_value(args, index, "--max-imports")?,
                    )?);
                    index += 2;
                }
                "--allow-root" => {
                    push_allow_root(args, index, &mut allow_roots)?;
                    index += 2;
                }
                "--json" => index += 1,
                flag => {
                    return Err(CliUsageError::from(format!(
                        "unknown photo render flag '{flag}'; {}",
                        photo_render_usage()
                    )));
                }
            }
        }
        let intent = intent.unwrap_or_else(|| CAMERA_BEHAVIOR_INTENT.to_owned());
        if intent != CAMERA_BEHAVIOR_INTENT {
            return Err(CliUsageError::from(format!(
                "unsupported photo intent '{intent}'; use camera-behavior"
            )));
        }
        Ok(Self {
            input: PathBuf::from(input),
            intent,
            out: out.ok_or_else(|| {
                CliUsageError::from(format!("missing --out <png>; {}", photo_render_usage()))
            })?,
            report: report.ok_or_else(|| {
                CliUsageError::from(format!("missing --report <json>; {}", photo_render_usage()))
            })?,
            emit_recipe,
            width,
            height,
            gpu,
            max_imports,
            allow_roots,
            subject,
        })
    }
}

impl PhotoPlanArgs {
    fn parse(args: &[String]) -> Result<Self, CliUsageError> {
        let Some(input) = args.first() else {
            return Err(CliUsageError::from(photo_plan_usage()));
        };
        let mut intent = None;
        let mut out = None;
        let mut width = 1280_u32;
        let mut height = 840_u32;
        let mut max_imports = None;
        let mut allow_roots = Vec::new();
        let mut subject = None;
        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
                "--intent" => {
                    let value = flag_value(args, index, "--intent")?;
                    intent = Some(normalize_intent(&value)?);
                    index += 2;
                }
                "--out" => {
                    out = Some(PathBuf::from(flag_value(args, index, "--out")?));
                    index += 2;
                }
                "--width" => {
                    width = parse_dimension("--width", flag_value(args, index, "--width")?)?;
                    index += 2;
                }
                "--height" => {
                    height = parse_dimension("--height", flag_value(args, index, "--height")?)?;
                    index += 2;
                }
                "--subject" => {
                    subject = Some(parse_subject_override(flag_value(
                        args,
                        index,
                        "--subject",
                    )?)?);
                    index += 2;
                }
                "--max-imports" => {
                    max_imports = Some(parse_positive_usize(
                        "--max-imports",
                        flag_value(args, index, "--max-imports")?,
                    )?);
                    index += 2;
                }
                "--allow-root" => {
                    push_allow_root(args, index, &mut allow_roots)?;
                    index += 2;
                }
                "--json" => index += 1,
                flag => {
                    return Err(CliUsageError::from(format!(
                        "unknown photo plan flag '{flag}'; {}",
                        photo_plan_usage()
                    )));
                }
            }
        }
        let intent = intent.unwrap_or_else(|| CAMERA_BEHAVIOR_INTENT.to_owned());
        if intent != CAMERA_BEHAVIOR_INTENT {
            return Err(CliUsageError::from(format!(
                "unsupported photo intent '{intent}'; use camera-behavior"
            )));
        }
        Ok(Self {
            input: PathBuf::from(input),
            intent,
            out: out.ok_or_else(|| {
                CliUsageError::from(format!("missing --out <plan.json>; {}", photo_plan_usage()))
            })?,
            width,
            height,
            max_imports,
            allow_roots,
            subject,
        })
    }
}

fn normalize_intent(value: &str) -> Result<String, CliUsageError> {
    match value {
        "camera-behavior" | "camera_behavior" | "product-hero" | "product_hero" => {
            Ok(CAMERA_BEHAVIOR_INTENT.to_owned())
        }
        other => Err(CliUsageError::from(format!(
            "unsupported photo intent '{other}'; use camera-behavior"
        ))),
    }
}

fn parse_subject_override(value: String) -> Result<scena::SceneRecipeTargetV1, CliUsageError> {
    let value = value.trim();
    let target = if let Some(id) = value.strip_prefix("import:") {
        if id.is_empty() {
            return Err(CliUsageError::from(
                "--subject requires a non-empty target id",
            ));
        }
        scena::SceneRecipeTargetV1::Import { id: id.to_owned() }
    } else if let Some(id) = value.strip_prefix("node:") {
        if id.is_empty() {
            return Err(CliUsageError::from(
                "--subject requires a non-empty target id",
            ));
        }
        scena::SceneRecipeTargetV1::Node { id: id.to_owned() }
    } else if value.contains(':') {
        return Err(CliUsageError::from(format!(
            "unsupported photo subject target '{value}'; use --subject import:<id> or --subject node:<id>"
        )));
    } else {
        if value.is_empty() {
            return Err(CliUsageError::from(
                "--subject requires a non-empty target id",
            ));
        }
        scena::SceneRecipeTargetV1::Import {
            id: value.to_owned(),
        }
    };
    Ok(target)
}

fn parse_dimension(flag: &str, value: String) -> Result<u32, CliUsageError> {
    let parsed = value
        .parse::<u32>()
        .map_err(|_| format!("{flag} requires a positive integer, got '{value}'"))?;
    if parsed == 0 {
        return Err(CliUsageError::from(format!(
            "{flag} requires a positive integer, got 0"
        )));
    }
    Ok(parsed)
}

fn parse_positive_usize(flag: &str, value: String) -> Result<usize, CliUsageError> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("{flag} requires an unsigned integer, got '{value}'"))?;
    if parsed == 0 {
        return Err(CliUsageError::from(format!(
            "{flag} requires a positive integer, got 0"
        )));
    }
    Ok(parsed)
}

fn flag_value(args: &[String], index: usize, flag: &str) -> Result<String, CliUsageError> {
    args.get(index + 1)
        .cloned()
        .ok_or_else(|| CliUsageError::from(format!("{flag} requires a value")))
}

fn round2(value: f32) -> f64 {
    ((value as f64) * 100.0).round() / 100.0
}

fn round2_f64(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

fn decimal_id(value: f64, precision: usize) -> String {
    format!("{value:.precision$}").replace('.', "_")
}

fn runtime_failure(error: impl std::fmt::Display) -> CliFailure {
    CliFailure::new(CliErrorKind::Runtime, error.to_string())
}

fn photo_usage() -> &'static str {
    "usage: scena photo plan <asset-or-recipe> [--intent camera-behavior] --out <plan.json> [--subject import:<id>|node:<id>] [--width <px>] [--height <px>] [--max-imports <n>] [--allow-root <directory>]...; scena photo render <asset-or-recipe> [--intent camera-behavior] --out <png> --report <json> [--emit-recipe <recipe.json>] [--subject import:<id>|node:<id>] [--width <px>] [--height <px>] [--gpu] [--max-imports <n>] [--allow-root <directory>]..."
}

fn photo_render_usage() -> &'static str {
    "usage: scena photo render <asset-or-recipe> [--intent camera-behavior] --out <png> --report <json> [--emit-recipe <recipe.json>] [--subject import:<id>|node:<id>] [--width <px>] [--height <px>] [--gpu] [--max-imports <n>] [--allow-root <directory>]..."
}

fn photo_plan_usage() -> &'static str {
    "usage: scena photo plan <asset-or-recipe> [--intent camera-behavior] --out <plan.json> [--subject import:<id>|node:<id>] [--width <px>] [--height <px>] [--max-imports <n>] [--allow-root <directory>]..."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn photo_intent_parser_canonicalizes_legacy_product_hero_aliases() {
        for spelling in [
            "camera_behavior",
            "camera-behavior",
            "product_hero",
            "product-hero",
        ] {
            assert_eq!(
                normalize_intent(spelling).expect("known photo intent parses"),
                "camera_behavior",
                "{spelling} should normalize to the canonical camera-behavior intent"
            );
        }
    }

    fn sane_camera_subject() -> SubjectMetrics {
        SubjectMetrics {
            min_x: 32.0,
            min_y: 20.0,
            max_x: 224.0,
            max_y: 148.0,
            fill_fraction: 0.75,
            fill_width_fraction: 0.75,
            fill_height_fraction: 0.76,
            mean_luminance_srgb8: 90.0,
            luminance_stddev_srgb8: 18.0,
            luminance_range_srgb8: 88.0,
            background_separation_srgb8: 72.0,
            background_mean_luminance_srgb8: 42.0,
            low_clip_fraction: 0.0,
            high_clip_fraction: 0.0,
            center_offset_fraction: 0.02,
            clipped_fraction: 0.0,
            empty_space_fraction: 0.43,
            depth_variation: 0.22,
            normal_variation: 0.28,
            highlight_fraction: 0.08,
            highlight_continuity: 0.36,
            highlight_distribution: 0.50,
            shadow_presence: 0.14,
            shadow_softness: 0.72,
            silhouette_separation: 0.42,
            mean_saturation: 0.24,
            color_cast: 0.03,
            reflection_washout: 0.06,
            sample_count: 24_576,
        }
    }

    fn assert_failure(metrics: SubjectMetrics, expected: &'static str) {
        let failures = camera_behavior_failure_codes(metrics);
        assert!(
            failures.contains(&expected),
            "expected {expected}, got {failures:?}"
        );
    }

    #[test]
    fn camera_behavior_oracle_rejects_known_bad_mutations() {
        assert!(camera_behavior_failure_codes(sane_camera_subject()).is_empty());
        assert!(
            camera_behavior_acceptance_failure_codes(CameraBehaviorGateEvidence::from_metrics(
                sane_camera_subject()
            ))
            .is_empty()
        );

        let mut average_metered_silhouette = sane_camera_subject();
        average_metered_silhouette.mean_luminance_srgb8 = 29.8;
        average_metered_silhouette.low_clip_fraction = 0.57;
        assert_failure(average_metered_silhouette, "subject_luminance_below_min");
        assert_failure(average_metered_silhouette, "subject_low_clip_above_max");

        let mut stale_subject_mask = sane_camera_subject();
        stale_subject_mask.sample_count = 0;
        stale_subject_mask.fill_fraction = 0.0;
        stale_subject_mask.fill_width_fraction = 0.0;
        stale_subject_mask.fill_height_fraction = 0.0;
        assert_failure(stale_subject_mask, "subject_visible_pixels_missing");

        let mut wrong_subject_target = sane_camera_subject();
        wrong_subject_target.sample_count = 0;
        wrong_subject_target.fill_fraction = 0.0;
        wrong_subject_target.fill_width_fraction = 0.0;
        wrong_subject_target.fill_height_fraction = 0.0;
        assert_failure(wrong_subject_target, "subject_visible_pixels_missing");

        let mut old_ev_cap = sane_camera_subject();
        old_ev_cap.mean_luminance_srgb8 = 72.0;
        assert_failure(old_ev_cap, "subject_luminance_below_min");

        let mut pulled_back_empty_slab = sane_camera_subject();
        pulled_back_empty_slab.fill_fraction = 0.50;
        pulled_back_empty_slab.fill_width_fraction = 0.25;
        assert_failure(pulled_back_empty_slab, "subject_fill_below_min");

        let mut actionably_too_narrow = sane_camera_subject();
        actionably_too_narrow.fill_fraction = 0.72;
        actionably_too_narrow.fill_width_fraction = 0.53;
        actionably_too_narrow.fill_height_fraction = 0.72;
        assert_failure(actionably_too_narrow, "subject_fill_below_min");

        let mut off_center = sane_camera_subject();
        off_center.center_offset_fraction = 0.31;
        assert_failure(off_center, "subject_center_offset_above_max");

        let mut blown_highlights = sane_camera_subject();
        blown_highlights.high_clip_fraction = 0.12;
        assert_failure(blown_highlights, "subject_high_clip_above_max");

        let mut flat_gray_metal = sane_camera_subject();
        flat_gray_metal.luminance_stddev_srgb8 = 0.4;
        flat_gray_metal.luminance_range_srgb8 = 2.0;
        assert_failure(flat_gray_metal, "subject_luminance_structure_below_min");

        let mut missing_steel_reflection_structure = sane_camera_subject();
        missing_steel_reflection_structure.luminance_stddev_srgb8 = 2.0;
        missing_steel_reflection_structure.luminance_range_srgb8 = 12.0;
        assert_failure(
            missing_steel_reflection_structure,
            "subject_luminance_structure_below_min",
        );

        let post_tonemap_metering = CameraBehaviorGateEvidence::from_metrics(sane_camera_subject())
            .with_metering_domain_rejection("metering_domain_encoded_output_feedback");
        assert!(
            camera_behavior_acceptance_failure_codes(post_tonemap_metering)
                .contains(&"metering_domain_encoded_output_feedback"),
            "strict camera-behavior gate must reject encoded-output feedback metering"
        );

        let wrong_focus = CameraBehaviorGateEvidence::from_metrics(sane_camera_subject())
            .with_focus_rejection("subject_focus_unresolved");
        assert!(
            camera_behavior_acceptance_failure_codes(wrong_focus)
                .contains(&"subject_focus_unresolved"),
            "strict camera-behavior gate must reject unresolved or wrong subject focus"
        );
    }

    #[test]
    fn camera_behavior_lighting_correction_targets_surface_readability() {
        let mut flat = sane_camera_subject();
        flat.luminance_stddev_srgb8 = 1.0;
        flat.luminance_range_srgb8 = 8.0;
        let flat_adjustment = corrected_photographic_lighting(flat)
            .expect("flat subject should request a lighting correction");
        assert!(flat_adjustment.fill_scale < 1.0);
        assert!(flat_adjustment.environment_rotation_offset_degrees.abs() >= 20.0);

        let mut merged = sane_camera_subject();
        merged.background_separation_srgb8 = 3.0;
        merged.silhouette_separation = 0.02;
        let merged_adjustment = corrected_photographic_lighting(merged)
            .expect("merged subject should request rim separation");
        assert!(merged_adjustment.rim_scale > 1.0);

        assert!(
            corrected_photographic_lighting(sane_camera_subject()).is_none(),
            "healthy photographic structure must not churn lighting"
        );

        let mut broken_specular = sane_camera_subject();
        broken_specular.highlight_continuity = 0.01;
        broken_specular.highlight_distribution = 0.0;
        let specular_adjustment = corrected_photographic_lighting(broken_specular)
            .expect("fragmented highlights should rotate and reshape illumination");
        assert!(specular_adjustment.key_scale > 1.0);
        assert!(specular_adjustment.environment_rotation_offset_degrees >= 28.0);

        let mut washed_out = sane_camera_subject();
        washed_out.reflection_washout = 0.45;
        let washout_adjustment = corrected_photographic_lighting(washed_out)
            .expect("washed-out reflections should reduce and rotate illumination");
        assert!(washout_adjustment.environment_intensity_scale < 1.0);
    }

    #[test]
    fn camera_behavior_retry_policy_is_bounded_camera_and_exposure_loop() {
        assert_eq!(CAMERA_BEHAVIOR_MAX_ATTEMPTS, 6);

        let mut underexposed = sane_camera_subject();
        underexposed.mean_luminance_srgb8 = 72.0;
        let next_ev = corrected_exposure_ev(0.0, underexposed)
            .expect("underexposed subject should request one EV correction");
        assert!(
            next_ev > 0.0 && next_ev <= 4.5,
            "retry correction must be positive and clamped, got {next_ev}"
        );

        let mut overexposed_at_legacy_floor = sane_camera_subject();
        overexposed_at_legacy_floor.mean_luminance_srgb8 = 102.67;
        let next_ev = corrected_exposure_ev(-1.5, overexposed_at_legacy_floor)
            .expect("measured overexposure must remain correctable below the legacy floor");
        assert!(
            next_ev < -1.5,
            "camera behavior must apply the measured correction instead of pinning EV at -1.5, got {next_ev}"
        );

        let mut passed_after_retry = underexposed;
        passed_after_retry.mean_luminance_srgb8 = 90.0;
        passed_after_retry.low_clip_fraction = 0.0;
        assert!(
            camera_behavior_failure_codes(passed_after_retry).is_empty(),
            "retry target should pass the acceptance oracle"
        );

        let first = PhotoCandidate {
            id: "candidate_1".to_owned(),
            exposure_ev: 0.0,
            composition_fill_fraction: 0.50,
            camera: test_candidate_camera(),
            metrics: underexposed,
            status: "failed",
            failure_codes: camera_behavior_failure_codes(underexposed),
            adjustment: Some("initial_camera_composition"),
        };
        let second = PhotoCandidate {
            id: "candidate_2".to_owned(),
            exposure_ev: next_ev,
            composition_fill_fraction: 0.75,
            camera: test_candidate_camera(),
            metrics: passed_after_retry,
            status: "passed",
            failure_codes: Vec::new(),
            adjustment: Some("exposure_delta"),
        };
        let report = retry_json(&[first, second]);
        assert_eq!(report["policy"]["max_attempts"], 6);
        assert_eq!(report["policy"]["max_retries"], 5);
        assert_eq!(
            report["policy"]["allowed_adjustments"],
            json!(["camera_composition", "exposure_compensation_ev"])
        );
        assert_eq!(report["attempts"], 2);
        assert_eq!(report["retry_used"], true);
        assert_eq!(report["budget_exhausted"], false);
        assert_eq!(report["suggestion"]["kind"], "exposure_compensation_ev");
        assert_eq!(report["retry_input"]["candidate_id"], "candidate_2");
    }

    #[test]
    fn camera_behavior_solver_recomposes_off_center_subjects_without_fill_change() {
        let current = scena::PhotoCompositionCandidateV1 {
            id: "candidate".to_owned(),
            order: 0,
            view: "camera_solver".to_owned(),
            lens: "measured".to_owned(),
            focal_length_mm: 70.0,
            physical_camera: test_physical_camera(),
            fill_fraction: CAMERA_BEHAVIOR_TARGET_FILL_WIDTH,
            azimuth_deg: 45.0,
            elevation_deg: 30.0,
            subject_yaw_deg: 0.0,
            preserve_authored_camera: false,
            front_hint: None,
            up_hint: None,
            staging: scena::PhotoCandidateStagingV1 {
                id: "dark_studio".to_owned(),
                environment: "studio".to_owned(),
                background: "dark_studio".to_owned(),
                ground: "matte_shadow_catcher".to_owned(),
                grid: false,
            },
            keep_visible_anchors: Vec::new(),
        };
        let mut metrics = sane_camera_subject();
        metrics.fill_fraction = CAMERA_BEHAVIOR_TARGET_FILL_WIDTH;
        metrics.fill_width_fraction = CAMERA_BEHAVIOR_TARGET_FILL_WIDTH;
        metrics.center_offset_fraction = CAMERA_BEHAVIOR_MAX_CENTER_OFFSET + 0.08;

        let next = corrected_composition_candidate(&current, metrics)
            .expect("off-center subject must request camera-composition correction");
        assert_eq!(next.fill_fraction, current.fill_fraction);
        assert_ne!(
            next.azimuth_deg, current.azimuth_deg,
            "off-center correction should try another camera angle when fill is already sane"
        );
        assert!(
            next.id.starts_with("camera_solver_recenter_"),
            "off-center correction should be explicitly identified as solver recentering: {}",
            next.id
        );
    }

    #[test]
    fn camera_behavior_solver_pulls_back_cropped_subjects() {
        let current = scena::PhotoCompositionCandidateV1 {
            id: "candidate".to_owned(),
            order: 0,
            view: "camera_solver".to_owned(),
            lens: "measured".to_owned(),
            focal_length_mm: 70.0,
            physical_camera: test_physical_camera(),
            fill_fraction: CAMERA_BEHAVIOR_TARGET_FILL_WIDTH,
            azimuth_deg: 45.0,
            elevation_deg: 30.0,
            subject_yaw_deg: 0.0,
            preserve_authored_camera: false,
            front_hint: None,
            up_hint: None,
            staging: scena::PhotoCandidateStagingV1 {
                id: "dark_studio".to_owned(),
                environment: "studio".to_owned(),
                background: "dark_studio".to_owned(),
                ground: "matte_shadow_catcher".to_owned(),
                grid: false,
            },
            keep_visible_anchors: Vec::new(),
        };
        let mut metrics = sane_camera_subject();
        metrics.fill_fraction = CAMERA_BEHAVIOR_TARGET_FILL_WIDTH;
        metrics.fill_width_fraction = CAMERA_BEHAVIOR_TARGET_FILL_WIDTH;
        metrics.clipped_fraction = 0.12;

        assert!(
            camera_behavior_failure_codes(metrics).contains(&"subject_clipped_by_frame"),
            "cropped subject must fail the final acceptance gate"
        );
        let next = corrected_composition_candidate(&current, metrics)
            .expect("cropped subject must request camera-composition correction");
        assert!(
            next.fill_fraction < current.fill_fraction,
            "cropped correction should pull back or widen, got {next:#?}"
        );
    }

    #[test]
    fn camera_loop_budget_exhaustion_report_keeps_all_attempts() {
        let mut metrics = sane_camera_subject();
        metrics.mean_luminance_srgb8 = 20.0;
        let candidates = (0..CAMERA_BEHAVIOR_MAX_ATTEMPTS)
            .map(|index| PhotoCandidate {
                id: format!("candidate_{}", index + 1),
                exposure_ev: 4.5,
                composition_fill_fraction: 0.75,
                camera: test_candidate_camera(),
                metrics,
                status: "failed",
                failure_codes: camera_behavior_failure_codes(metrics),
                adjustment: if index == 0 {
                    Some("initial_camera_composition")
                } else {
                    Some("exposure_delta")
                },
            })
            .collect::<Vec<_>>();
        let report = retry_json(&candidates);
        assert_eq!(report["policy"]["max_attempts"], 6);
        assert_eq!(report["attempts"], 6);
        assert_eq!(report["retry_used"], true);
        assert_eq!(report["budget_exhausted"], true);
        assert_eq!(report["final_attempt_id"], "candidate_6");
    }

    fn test_candidate_camera() -> PhotoCandidateCamera {
        PhotoCandidateCamera {
            world_transform: Some(scena::Transform::IDENTITY),
            projection: Some(scena::CaptureProjection::Perspective {
                vertical_fov_radians: 1.0,
                aspect: 1.0,
                near: 0.01,
                far: 1000.0,
            }),
            vertical_fov_degrees: Some(f64::from(1.0_f32).to_degrees()),
            focus_distance_m: Some(2.0),
        }
    }

    fn test_physical_camera() -> scena::PhotoPhysicalCameraV1 {
        scena::PhotoPhysicalCameraV1 {
            sensor_width_mm: 36.0,
            sensor_height_mm: 24.0,
            focal_length_mm: 70.0,
            aperture_f_stop: 8.0,
            focus_distance_m: 3.0,
            shutter_seconds: 1.0 / 125.0,
            sensitivity_iso: 100.0,
            exposure_compensation_ev: 0.0,
            circle_of_confusion_mm: 0.03,
            aperture_blades: 9,
        }
    }
}
