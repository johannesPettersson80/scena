use std::collections::{BTreeMap, BTreeSet};
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
const DEFAULT_PHOTO_WIDTH: u32 = 1280;
const DEFAULT_PHOTO_HEIGHT: u32 = 840;
const DEFAULT_FINAL_PHOTO_WIDTH: u32 = 3840;
const DEFAULT_FINAL_PHOTO_HEIGHT: u32 = 2520;
const FINAL_PHOTO_MATERIAL_TEXTURE_BUDGET_BYTES: u64 = 512 * 1024 * 1024;
const FINAL_PHOTO_POLICY_JSON: &str =
    include_str!("../../../tests/assets/photo/final/photo_final_policy_v1.json");
const CAMERA_BEHAVIOR_TARGET_MEAN_LUMA: f64 = 90.0;
const CAMERA_BEHAVIOR_MIN_MEAN_LUMA: f64 = 80.0;
const CAMERA_BEHAVIOR_MAX_MEAN_LUMA: f64 = 100.0;
const CAMERA_BEHAVIOR_MAX_LOW_CLIP: f64 = 0.20;
const CAMERA_BEHAVIOR_MAX_HIGH_CLIP: f64 = 0.005;
const CAMERA_BEHAVIOR_DARK_PRODUCT_MIN_MEAN_LUMA: f64 = 20.0;
const CAMERA_BEHAVIOR_DARK_PRODUCT_MAX_MEAN_LUMA: f64 = 45.0;
const CAMERA_BEHAVIOR_HIGHLIGHT_LIMITED_MIN_CLIP: f64 = 0.001;
const CAMERA_BEHAVIOR_MIN_FILL_WIDTH: f64 = 0.65;
const CAMERA_BEHAVIOR_MAX_FILL_WIDTH: f64 = 0.85;
const CAMERA_BEHAVIOR_TARGET_FILL_WIDTH: f64 = 0.75;
const CAMERA_BEHAVIOR_MAX_FIT_FRACTION: f64 = 0.96;
const CAMERA_BEHAVIOR_MAX_CENTER_OFFSET: f64 = 0.16;
const CAMERA_BEHAVIOR_MIN_LUMA_STDDEV: f64 = 6.0;
const CAMERA_BEHAVIOR_MIN_LUMA_RANGE: f64 = 32.0;
const CAMERA_BEHAVIOR_MIN_SILHOUETTE_SEPARATION: f64 = 0.01;
const CAMERA_BEHAVIOR_MIN_EXPOSURE_EV: f32 = -8.0;
const CAMERA_BEHAVIOR_MAX_EXPOSURE_EV: f32 = 8.0;
// Scena's built-in directional studio rig is deliberately moderate. This
// fixed base keeps its zero-config product-photo output near the established
// bright studio anchor before the single bounded exposure correction runs.
const FINAL_PHOTO_BASE_EXPOSURE_EV: f32 = 0.25;
const DEFAULT_PHOTO_MAX_EXPOSURE_CORRECTION_EV: f32 = 0.75;
const CAMERA_BEHAVIOR_MAX_ATTEMPTS: usize = 6;
const CAMERA_BEHAVIOR_FOCUS_DELIVERY_MAX_ATTEMPTS: usize = 6;
const FINAL_DARK_MATERIAL_LIGHTING_MAX_RETRIES: usize = 1;
const CAMERA_BEHAVIOR_COMPOSITION_CANDIDATE_BUDGET: usize = 10;
const CAMERA_BEHAVIOR_SHADED_CANDIDATE_BUDGET: usize = 3;
const CAMERA_BEHAVIOR_SHADED_CANDIDATE_WIDTH: u32 = 160;
const CAMERA_BEHAVIOR_SHADED_CANDIDATE_HEIGHT: u32 = 105;

#[derive(Debug, Clone, PartialEq)]
struct PhotoRenderArgs {
    input: PathBuf,
    /// Whether `--width`/`--height` were passed. Without this the command
    /// cannot tell a caller-chosen size from its own default, and so cannot
    /// know whether it may overwrite a size the recipe already declares.
    capture_explicit: bool,
    intent: String,
    out: PathBuf,
    report: PathBuf,
    emit_recipe: Option<PathBuf>,
    width: u32,
    height: u32,
    gpu: bool,
    optimize: bool,
    max_imports: Option<usize>,
    allow_roots: Vec<PathBuf>,
    subject: Option<scena::SceneRecipeTargetV1>,
}

#[derive(Debug, Clone, PartialEq)]
struct PhotoPlanArgs {
    input: PathBuf,
    capture_explicit: bool,
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
    quality: scena::SceneRecipePhotoQualityV1,
    ground: scena::scene_host::PhotographicGroundV1,
}

#[derive(Debug, serde::Deserialize)]
struct FinalPhotoPolicyV1 {
    schema: String,
    version: u32,
    mode: String,
    metrics: FinalPhotoPolicyMetricsV1,
}

#[derive(Debug, serde::Deserialize)]
struct FinalPhotoPolicyMetricsV1 {
    contact_shadow_delta_mean_srgb8: FinalPhotoGroundingPolicyV1,
}

#[derive(Debug, serde::Deserialize)]
struct FinalPhotoGroundingPolicyV1 {
    blocking: bool,
    failure_code: String,
    threshold: FinalPhotoGroundingThresholdV1,
}

#[derive(Debug, serde::Deserialize)]
struct FinalPhotoGroundingThresholdV1 {
    min: f64,
    min_boundary_samples: u64,
    min_attached_fraction: f64,
}

#[derive(Debug, Clone)]
pub(crate) struct SubjectSelection {
    target_kind: String,
    id: String,
    pub(crate) root_handle: u64,
    root_handles: Vec<u64>,
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
    dark_material_mean_luminance_srgb8: Option<f64>,
    dark_material_coverage: f64,
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
    quality_execution: Value,
    quality_analysis: Value,
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
    lighting_report: Option<scena::scene_host::PhotographicLightingReportV1>,
    reflection_probe_report: Option<scena::scene_host::PhotographicReflectionProbeReportV1>,
    /// What the renderer decided to stage. Discarding this is why "is there a
    /// backdrop in this frame?" could only be answered by squinting at pixels.
    pub(crate) surroundings_report: scena::PhotographicSurroundingsReportV1,
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
    let (source, _width, _height, _capture_source) = photo_source_for(
        &args.input,
        args.capture_explicit.then_some((args.width, args.height)),
    )?;
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
    let planning =
        camera_behavior_composition_plan(&host, &subject, !build.manifest.cameras.is_empty())?;
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
    let mut args = PhotoRenderArgs::parse(args)?;
    let policy = effective_recipe_policy(&args.allow_roots, args.max_imports)?;
    let policy_report = policy.to_schema_report();
    let (source, effective_width, effective_height, capture_source) = photo_source(&args)?;
    // The recipe may declare its own capture size; adopt it so reported work
    // metrics and artifact dimensions describe the frame actually rendered.
    args.width = effective_width;
    args.height = effective_height;
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
    ensure_final_photo_backend(source.quality, host.backend())?;
    // GPU hosts must own semantic AOV resources before the next prepare, because
    // every camera-behavior measurement below reads them back.
    if args.gpu {
        host.set_semantic_aov_capture_enabled(true);
    }
    let backend_selection = CliBackendSelectionV1::new(args.gpu, Some(host.backend()));
    let planning = camera_behavior_composition_plan(&host, &subject, authored_camera)?;
    let mut shaded_selection = apply_camera_behavior_setup_with_plan(
        &mut host,
        &subject,
        authored_lights,
        &planning,
        args.gpu,
        source.quality,
        source.ground,
        args.optimize,
    )?;
    let selected_composition =
        selected_shaded_composition_candidate(&planning, &shaded_selection)?.clone();
    let mut selected = render_camera_behavior_candidates(
        &mut host,
        &subject,
        &selected_composition,
        Some(shaded_selection.surroundings_report.clone()),
        args.gpu,
        source.quality.is_final(),
        args.optimize,
    )?;
    // The loop re-sized the backdrop for the camera it settled on, so the
    // staging the report discloses has to be that one, not the setup-time guess.
    if let Some(staging) = selected.surroundings.clone() {
        shaded_selection.surroundings_report = staging;
    }
    let visible_focus =
        apply_visible_subject_physical_focus(&mut host, &subject, &selected.composition, args.gpu)?;
    // Focus resolution enables a post effect after the camera/exposure loop has
    // already accepted its frame. Revalidate the delivered pixels and keep
    // exposure correction bounded here as well: a focused frame that moved out
    // of band is not the frame the earlier loop approved.
    let mut focus_work = PhotoLoopWorkMetrics::default();
    if visible_focus.is_some() {
        render_focused_delivery(
            &mut host,
            &subject,
            args.gpu,
            &mut selected,
            &mut focus_work,
            args.optimize,
            true,
        )?;
    }
    let (mut final_aov, mut final_metrics) =
        measure_photo_subject_frame(&mut host, &selected.capture, &subject, args.gpu)?;
    for _ in 0..FINAL_DARK_MATERIAL_LIGHTING_MAX_RETRIES {
        if !args.optimize
            || !source.quality.is_final()
            || !should_retry_final_dark_material_lighting(final_metrics)
            || shaded_selection.lighting_report.is_none()
        {
            break;
        }
        let adjustment = corrected_photographic_lighting(final_metrics)
            .expect("an unreadable dark material always requests lighting correction");
        if let Some(previous) = shaded_selection.lighting_report.take() {
            remove_generated_photographic_lights(&mut host, previous)?;
        }
        shaded_selection.lighting_report = Some(
            host.apply_final_photographic_lighting_adjusted(subject.root_handle, adjustment)
                .map_err(runtime_failure)?,
        );
        shaded_selection.reflection_probe_report = Some(
            host.bake_photographic_reflection_probes(subject.root_handle)
                .map_err(runtime_failure)?,
        );
        selected.capture = render_capture(&mut host, &mut selected.work_metrics)?;
        (final_aov, final_metrics) =
            measure_photo_subject_frame(&mut host, &selected.capture, &subject, args.gpu)?;
        selected.final_candidate.adjustment = Some("final_dark_material_lighting");
    }
    if source.quality.is_final()
        && source.ground == scena::scene_host::PhotographicGroundV1::Reflective
        && let Some(planar) = host
            .capture_photographic_planar_reflection(&mut shaded_selection.surroundings_report)
            .map_err(runtime_failure)?
    {
        selected.capture = render_capture(&mut host, &mut selected.work_metrics)?;
        final_aov = capture_camera_behavior_semantic_aovs(&mut host, args.gpu)?;
        let floor_mask = planar_reflection_floor_mask(
            &final_aov,
            &shaded_selection.surroundings_report.support_nodes,
        );
        let mut rgba8 = selected.capture.rgba8.clone();
        composite_planar_reflection_rgba8(
            &mut rgba8,
            &planar.capture.rgba8,
            selected.capture.descriptor.width,
            selected.capture.descriptor.height,
            &floor_mask,
            planar.roughness,
            planar.strength,
        );
        selected.capture = host
            .capture_from_rgba8(
                selected.capture.descriptor.width,
                selected.capture.descriptor.height,
                rgba8,
            )
            .map_err(runtime_failure)?;
        final_metrics =
            measure_photo_subject_with_aov(&host, &selected.capture, &subject, &final_aov)?;
        selected.final_candidate.adjustment = Some("planar_ground_reflection");
    }
    let mut quality_analysis = photo_quality_analysis_json(
        &selected.capture,
        &final_aov,
        &subject,
        &shaded_selection.surroundings_report,
    )?;
    record_texture_resolution_health(
        &mut shaded_selection.surface_report,
        final_metrics,
        args.width,
        args.height,
    );
    selected.final_candidate.metrics = final_metrics;
    selected.final_candidate.failure_codes = camera_behavior_failure_codes(final_metrics);
    apply_final_photo_quality_policy(
        source.quality,
        &mut quality_analysis,
        &mut selected.final_candidate.failure_codes,
    )?;
    selected.final_candidate.status = if selected.final_candidate.failure_codes.is_empty() {
        "passed"
    } else {
        "failed"
    };
    let subject_bounds = host
        .nodes_world_bounds(&subject.root_handles)
        .ok()
        .flatten();
    selected.final_candidate.camera =
        PhotoCandidateCamera::from_capture(&selected.capture, subject_bounds);
    if !refresh_selected_candidate_history(&mut selected.candidates, &selected.final_candidate) {
        return Err(CliFailure::new(
            CliErrorKind::Internal,
            format!(
                "selected photo candidate '{}' is missing from its attempt history",
                selected.final_candidate.id
            ),
        ));
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

    let focus_report =
        camera_behavior_focus_report(&subject, subject_bounds, visible_focus.as_ref(), &capture);
    let exposure_report =
        camera_behavior_exposure_report(&selected.final_candidate, &capture, args.optimize);
    let subject_observation =
        camera_behavior_subject_observation(&subject, &selected.final_candidate, &capture);
    let capability_report = host.renderer().capability_report();
    let adapter = capability_report.adapter();
    let evidence_class = classify_photo_evidence(
        host.backend(),
        adapter.map(|adapter| adapter.name.as_str()),
        adapter.map(|adapter| adapter.driver.as_str()),
        adapter.map(|adapter| adapter.driver_info.as_str()),
    );
    let reconstruction = match host.renderer().reconstruction_filter() {
        scena::ReconstructionFilter::Box => "box",
        scena::ReconstructionFilter::Tent => "tent",
        scena::ReconstructionFilter::Gaussian => "gaussian",
    };
    let quality_execution = photo_quality_execution_json(PhotoQualityExecutionInput {
        quality: source.quality,
        backend: host.backend(),
        evidence_class,
        capture: [args.width, args.height],
        supersample_factor: capture.descriptor.frame.supersample_factor,
        reconstruction,
        anti_aliasing: &capture.descriptor.frame.anti_aliasing,
        environment_source_dimensions: shaded_selection
            .lighting_report
            .as_ref()
            .and_then(|report| report.environment.source_dimensions),
        environment_cubemap_resolution: shaded_selection
            .lighting_report
            .as_ref()
            .and_then(|report| report.environment.cubemap_resolution),
        reflection_probe_count: shaded_selection
            .reflection_probe_report
            .as_ref()
            .map_or(0, |report| report.probes.len()),
        shadow_mode: if shaded_selection
            .lighting_report
            .as_ref()
            .is_some_and(|report| report.source == "built_in_studio_directional")
        {
            "directional_key_shadow"
        } else if source.quality.is_final() {
            "weighted_area_visibility"
        } else {
            "prepared_area_visibility"
        },
        tonemapper: &capture.descriptor.frame.tonemapper,
        edge_rounding: build
            .manifest
            .imports
            .iter()
            .filter_map(|import| import.edge_rounding.clone())
            .collect(),
        material_resolution_selection: selected.material_resolution_selection.clone(),
    });
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
        quality_execution,
        quality_analysis,
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
            // States which size was rendered and why, so adopting a recipe's own
            // capture block is visible rather than inferred from the PNG.
            "capture": {
                "width": args.width,
                "height": args.height,
                "source": capture_source,
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

fn apply_final_photo_quality_policy(
    quality: scena::SceneRecipePhotoQualityV1,
    quality_analysis: &mut Value,
    failure_codes: &mut Vec<&'static str>,
) -> Result<(), CliFailure> {
    let same_pass_grounding_confirmed = quality_analysis
        .pointer("/grounding/contact_shadow_confirmed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !quality.is_final() {
        if same_pass_grounding_confirmed {
            failure_codes.retain(|code| *code != "contact_shadow_missing");
        }
        return Ok(());
    }

    let policy: FinalPhotoPolicyV1 =
        serde_json::from_str(FINAL_PHOTO_POLICY_JSON).map_err(|error| {
            CliFailure::new(
                CliErrorKind::Internal,
                format!("failed to parse tracked final-photo policy: {error}"),
            )
        })?;
    let grounding_policy = &policy.metrics.contact_shadow_delta_mean_srgb8;
    if policy.version != 1
        || policy.mode != "selective_blocking"
        || !grounding_policy.blocking
        || grounding_policy.failure_code != "contact_shadow_missing"
    {
        return Err(CliFailure::new(
            CliErrorKind::Internal,
            "tracked final-photo policy has an unsupported grounding contract",
        ));
    }

    let boundary_sample_count = quality_analysis
        .pointer("/grounding/boundary_sample_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let contact_shadow_delta_mean_srgb8 = quality_analysis
        .pointer("/grounding/contact_shadow_delta_mean_srgb8")
        .and_then(Value::as_f64);
    let attached_fraction = quality_analysis
        .pointer("/grounding/attached_fraction")
        .and_then(Value::as_f64);
    let threshold = &grounding_policy.threshold;
    let grounding_passed = boundary_sample_count >= threshold.min_boundary_samples
        && contact_shadow_delta_mean_srgb8.is_some_and(|value| value >= threshold.min)
        && attached_fraction.is_some_and(|value| value >= threshold.min_attached_fraction);

    let analysis = quality_analysis.as_object_mut().ok_or_else(|| {
        CliFailure::new(
            CliErrorKind::Internal,
            "final photo quality analysis must be a JSON object",
        )
    })?;
    analysis.insert("mode".to_owned(), Value::String(policy.mode.clone()));
    analysis.insert(
        "policy".to_owned(),
        json!({
            "schema": policy.schema,
            "version": policy.version,
            "mode": policy.mode,
            "blocking_metrics": ["contact_shadow_delta_mean_srgb8"],
            "checks": [{
                "metric": "contact_shadow_delta_mean_srgb8",
                "status": if grounding_passed { "checked" } else { "failed" },
                "failure_code": grounding_policy.failure_code,
                "observed": {
                    "value": contact_shadow_delta_mean_srgb8,
                    "boundary_sample_count": boundary_sample_count,
                    "attached_fraction": attached_fraction,
                },
                "threshold": {
                    "min": threshold.min,
                    "min_boundary_samples": threshold.min_boundary_samples,
                    "min_attached_fraction": threshold.min_attached_fraction,
                }
            }]
        }),
    );
    if grounding_passed {
        failure_codes.retain(|code| *code != "contact_shadow_missing");
    } else if !failure_codes.contains(&"contact_shadow_missing") {
        failure_codes.push("contact_shadow_missing");
    }
    Ok(())
}

pub(crate) struct SelectedCapture {
    pub(crate) capture: scena::CaptureRgba8,
    candidates: Vec<PhotoCandidate>,
    pub(crate) final_candidate: PhotoCandidate,
    composition: scena::PhotoCompositionCandidateV1,
    work_metrics: PhotoLoopWorkMetrics,
    /// The staging that produced the returned frame. The loop re-sizes the
    /// backdrop for each camera it tries, so the setup-time report describes a
    /// backdrop that is not the one in the delivered image.
    pub(crate) surroundings: Option<scena::PhotographicSurroundingsReportV1>,
    pub(crate) material_resolution_selection:
        Option<scena::PhotographicMaterialResolutionSelectionReportV1>,
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

fn measure_photo_subject_frame(
    host: &mut scena::SceneHostCore,
    capture: &scena::CaptureRgba8,
    subject: &SubjectSelection,
    gpu: bool,
) -> Result<(scena::SceneHostSemanticAovCaptureV1, SubjectMetrics), CliFailure> {
    let semantic_aov = capture_camera_behavior_semantic_aovs(host, gpu)?;
    let metrics = measure_photo_subject_with_aov(host, capture, subject, &semantic_aov)?;
    Ok((semantic_aov, metrics))
}

fn measure_photo_subject_with_aov(
    host: &scena::SceneHostCore,
    capture: &scena::CaptureRgba8,
    subject: &SubjectSelection,
    semantic_aov: &scena::SceneHostSemanticAovCaptureV1,
) -> Result<SubjectMetrics, CliFailure> {
    let inspection_json = host.inspect_json().map_err(runtime_failure)?;
    let inspection: scena::SceneInspectionReportV1 = serde_json::from_str(&inspection_json)
        .map_err(|error| {
            CliFailure::new(
                CliErrorKind::Internal,
                format!("failed to decode final scene inspection report: {error}"),
            )
        })?;
    let metrics = match measure_subject(capture, &inspection, semantic_aov, subject) {
        Ok(metrics) => metrics,
        Err(error) if photo_subject_measurement_can_degrade(&error.message) => {
            empty_subject_metrics()
        }
        Err(error) => return Err(error),
    };
    Ok(metrics)
}

fn photo_quality_analysis_json(
    capture: &scena::CaptureRgba8,
    semantic_aov: &scena::SceneHostSemanticAovCaptureV1,
    subject: &SubjectSelection,
    surroundings: &scena::PhotographicSurroundingsReportV1,
) -> Result<Value, CliFailure> {
    let Some(analysis) =
        photo_material_density_analysis(capture, semantic_aov, subject, Some(surroundings))?
    else {
        return Ok(json!({
            "schema": scena::PHOTO_QUALITY_ANALYSIS_SCHEMA_V1,
            "mode": "report_only",
            "identity_source": "unavailable",
            "materials": [],
            "grounding": {
                "method": "unavailable",
                "boundary_sample_count": 0,
            },
            "contour": {
                "method": "unavailable",
                "boundary_sample_count": 0,
            },
            "unavailable_metrics": [
                "same_pass_beauty_semantic_unavailable",
                "projected_texture_density_requires_beauty_identity_and_linear_depth",
            ],
        }));
    };
    serde_json::to_value(analysis).map_err(|error| {
        CliFailure::new(
            CliErrorKind::Internal,
            format!("failed to serialize final photo quality analysis: {error}"),
        )
    })
}

fn photo_material_density_analysis(
    capture: &scena::CaptureRgba8,
    semantic_aov: &scena::SceneHostSemanticAovCaptureV1,
    subject: &SubjectSelection,
    surroundings: Option<&scena::PhotographicSurroundingsReportV1>,
) -> Result<Option<scena::PhotoQualityAnalysisReportV1>, CliFailure> {
    let Some(beauty_id_indices) = semantic_aov.beauty_id_indices.as_deref() else {
        return Ok(None);
    };
    let subject_handles = subject_handles(subject);
    let support_handles = surroundings
        .map(|surroundings| surroundings.support_nodes.as_slice())
        .unwrap_or(&[]);
    let analysis = scena::analyze_photo_quality(scena::PhotoQualityAnalysisInputV1 {
        width: capture.descriptor.width,
        height: capture.descriptor.height,
        rgba8: &capture.rgba8,
        beauty_id_indices,
        depth_meters: &semantic_aov.depth_meters,
        projection: capture.descriptor.camera.projection,
        legend: &semantic_aov.legend,
        subject_handles: &subject_handles,
        support_handles,
    })
    .map_err(|code| {
        CliFailure::new(
            CliErrorKind::Runtime,
            format!("final photo quality analysis failed: {code}"),
        )
    })?;
    Ok(Some(analysis))
}

pub(crate) fn render_camera_behavior_candidates(
    host: &mut scena::SceneHostCore,
    subject: &SubjectSelection,
    base_candidate: &scena::PhotoCompositionCandidateV1,
    surroundings: Option<scena::PhotographicSurroundingsReportV1>,
    gpu: bool,
    select_material_resolutions: bool,
    optimize: bool,
) -> Result<SelectedCapture, CliFailure> {
    let mut candidates = Vec::new();
    let mut final_capture = None;
    let mut best: Option<(
        usize,
        PhotoCandidate,
        scena::CaptureRgba8,
        Option<scena::PhotographicSurroundingsReportV1>,
        scena::PhotoCompositionCandidateV1,
    )> = None;
    let mut work_metrics = PhotoLoopWorkMetrics::default();
    let mut composition = base_candidate.clone();
    let mut pending_adjustment = Some("initial_camera_composition");
    let mut surroundings = surroundings;
    let mut material_resolution_selection = None;
    let subject_bounds = host
        .nodes_world_bounds(&subject.root_handles)
        .ok()
        .flatten();
    for attempt in 0..CAMERA_BEHAVIOR_MAX_ATTEMPTS {
        host.frame_nodes_with_photo_candidate(&subject.root_handles, &composition)
            .map_err(runtime_failure)?;
        // The generated backdrop is sized for the frustum it has to fill, so it
        // is only correct for the camera it was solved against. This loop moves
        // the camera; leaving the backdrop where setup put it is what let its
        // edge into the frame.
        surroundings = resized_photographic_surroundings(host, subject, surroundings)?;
        let mut capture = render_capture(host, &mut work_metrics)?;
        let mut semantic_aov = capture_camera_behavior_semantic_aovs(host, gpu)?;
        if select_material_resolutions
            && let Some(analysis) = photo_material_density_analysis(
                &capture,
                &semantic_aov,
                subject,
                surroundings.as_ref(),
            )?
        {
            let selection = pollster::block_on(host.select_photographic_material_resolutions(
                &analysis,
                FINAL_PHOTO_MATERIAL_TEXTURE_BUDGET_BYTES,
            ))
            .map_err(runtime_failure)?;
            if selection.selections.iter().any(|entry| entry.changed) {
                material_resolution_selection = Some(selection);
                capture = render_capture(host, &mut work_metrics)?;
                semantic_aov = capture_camera_behavior_semantic_aovs(host, gpu)?;
            }
        }
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
        // Keep the least-bad attempt and the frame it produced. When no
        // candidate satisfies the gate the loop used to hand back whichever
        // attempt happened to run last, which is not the one a caller would
        // pick from the reported history.
        let failures = candidate.failure_codes.len();
        // `<=` so a later attempt wins a tie: each one folds in the previous
        // correction, so among equally-failing candidates the last is the most
        // converged.
        if best
            .as_ref()
            .is_none_or(|(best_failures, _, _, _, _)| failures <= *best_failures)
        {
            // Carry this attempt's staging with its frame. The backdrop is
            // re-sized per attempt, so the report has to describe the frame that
            // is actually returned, not whichever attempt ran last.
            best = Some((
                failures,
                candidate.clone(),
                capture.clone(),
                surroundings.clone(),
                composition.clone(),
            ));
        }
        final_capture = Some(capture);
        candidates.push(candidate.clone());
        if candidate.status == "passed" {
            return Ok(SelectedCapture {
                capture: final_capture.expect("candidate capture exists"),
                candidates,
                final_candidate: candidate,
                composition,
                work_metrics,
                surroundings,
                material_resolution_selection,
            });
        }

        if !optimize {
            if attempt == 0
                && let Some(next_ev) =
                    bounded_default_exposure_ev(FINAL_PHOTO_BASE_EXPOSURE_EV, metrics)
            {
                host.renderer_mut().clear_auto_exposure();
                host.renderer_mut().set_exposure_ev(next_ev);
                pending_adjustment = Some("bounded_exposure_delta");
                continue;
            }
            return Ok(SelectedCapture {
                capture: final_capture.expect("candidate capture exists"),
                candidates,
                final_candidate: candidate,
                composition,
                work_metrics,
                surroundings,
                material_resolution_selection,
            });
        }

        if attempt + 1 >= CAMERA_BEHAVIOR_MAX_ATTEMPTS {
            break;
        }
        // Composition and exposure are independent controls: one moves the
        // camera, the other moves the exposure, and neither correction depends
        // on the other having converged. Applying only the first while it is out
        // of band starves the second for the whole budget - a subject whose fill
        // target is unreachable (its aspect is not the frame's, so any further
        // zoom clips) re-frames six times and never once corrects exposure, then
        // reports an underexposure the loop never attempted. Measured on the
        // demo hero: `subject_fill_below_min` and `subject_luminance_below_min`
        // together, with the exposure untouched at every attempt.
        let next_composition = corrected_composition_candidate(&composition, metrics);
        let next_ev = corrected_exposure_ev(candidate.exposure_ev, metrics);
        pending_adjustment = match (next_composition.is_some(), next_ev.is_some()) {
            (true, true) => Some("camera_composition+exposure_delta"),
            (true, false) => Some("camera_composition"),
            (false, true) => Some("exposure_delta"),
            // Neither control has anything left to give; another attempt would
            // render the same frame.
            (false, false) => break,
        };
        if let Some(next_composition) = next_composition {
            composition = next_composition;
        }
        if let Some(next_ev) = next_ev {
            host.renderer_mut().clear_auto_exposure();
            host.renderer_mut().set_exposure_ev(next_ev);
        }
    }

    // No attempt passed. Report the least-bad one with its own frame, rather
    // than the last attempt the loop happened to make.
    let (final_candidate, _selected_capture, surroundings, composition) = match best {
        Some((_, candidate, capture, staging, composition)) => {
            (candidate, capture, staging, composition)
        }
        None => {
            let candidate = candidates.last().cloned().ok_or_else(|| {
                CliFailure::new(CliErrorKind::Runtime, "photo render produced no candidate")
            })?;
            let capture = final_capture.ok_or_else(|| {
                CliFailure::new(CliErrorKind::Runtime, "photo render produced no capture")
            })?;
            (candidate, capture, surroundings, composition)
        }
    };
    // Callers verify and may re-render through this host, so its camera and
    // staging must describe the selected frame, not the final attempted one.
    host.frame_nodes_with_photo_candidate(&subject.root_handles, &composition)
        .map_err(runtime_failure)?;
    let surroundings = resized_photographic_surroundings(host, subject, surroundings)?;
    host.renderer_mut().clear_auto_exposure();
    host.renderer_mut()
        .set_exposure_ev(final_candidate.exposure_ev);
    let capture = render_capture(host, &mut work_metrics)?;
    Ok(SelectedCapture {
        capture,
        candidates,
        final_candidate,
        composition,
        work_metrics,
        surroundings,
        material_resolution_selection,
    })
}

fn corrected_composition_candidate(
    current: &scena::PhotoCompositionCandidateV1,
    metrics: SubjectMetrics,
) -> Option<scena::PhotoCompositionCandidateV1> {
    if metrics.sample_count == 0 || !metrics.fill_width_fraction.is_finite() {
        return None;
    }
    // Drive the axis that limits the subject, the same one the gate scores.
    // Targeting width and height independently makes the corrector chase two
    // constraints that fight whenever the subject's aspect differs from the
    // frame's, which is why it used to stall short of the band.
    let fit = subject_fit_fraction(metrics);
    let fit_out_of_band =
        !(CAMERA_BEHAVIOR_MIN_FILL_WIDTH..=CAMERA_BEHAVIOR_MAX_FILL_WIDTH).contains(&fit);
    let over_fit = metrics.fill_fraction > CAMERA_BEHAVIOR_MAX_FIT_FRACTION;
    let off_center = metrics.center_offset_fraction > CAMERA_BEHAVIOR_MAX_CENTER_OFFSET;
    let clipped = metrics.clipped_fraction > 0.01;
    if !fit_out_of_band && !over_fit && !off_center && !clipped {
        return None;
    }
    // Clipping and over-fit pull outward; an under-filled frame pulls inward.
    let target_fit = if clipped || over_fit {
        (fit * 0.86).min(CAMERA_BEHAVIOR_MAX_FILL_WIDTH * 0.94)
    } else {
        CAMERA_BEHAVIOR_TARGET_FILL_WIDTH
    };
    let next_fill = if fit_out_of_band || over_fit || clipped {
        (current.fill_fraction * (target_fit / fit.max(0.001))).clamp(0.20, 1.0)
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

/// How much of the frame the subject fills along whichever axis limits it.
///
/// Requiring width and height to each reach the target independently is only
/// satisfiable when the subject's aspect matches the frame's. A wide subject in
/// a landscape frame cannot reach the width target without its height leaving
/// the frame, and a tall one cannot reach the height target without its width
/// shrinking below it, so the two constraints fight and no camera satisfies
/// both. That is what made the gate unsatisfiable for the valve manifold and
/// failed the demo hero at 0.644.
///
/// The subject's aspect is a property of the subject, not a framing decision, so
/// only the limiting axis is a composition target; the other follows from it.
/// Clipping remains the guard against over-filling.
fn subject_fit_fraction(metrics: SubjectMetrics) -> f64 {
    metrics.fill_fraction.max(metrics.fill_width_fraction)
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
        dark_material_mean_luminance_srgb8: None,
        dark_material_coverage: 0.0,
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
    work_metrics.record_capture(host.renderer().last_render_work_metrics());
    host.capture().map_err(runtime_failure)
}

fn planar_reflection_floor_mask(
    semantic_aov: &scena::SceneHostSemanticAovCaptureV1,
    floor_nodes: &[u64],
) -> Vec<bool> {
    let floor_nodes = floor_nodes.iter().copied().collect::<BTreeSet<_>>();
    let floor_palette = semantic_aov
        .legend
        .iter()
        .filter(|entry| floor_nodes.contains(&entry.node_handle))
        .map(|entry| entry.palette_index)
        .collect::<BTreeSet<_>>();
    semantic_aov
        .id_indices
        .iter()
        .zip(&semantic_aov.world_normals)
        .map(|(palette, normal)| floor_palette.contains(palette) && normal[1] >= 0.85)
        .collect()
}

fn composite_planar_reflection_rgba8(
    beauty: &mut [u8],
    reflected: &[u8],
    width: u32,
    height: u32,
    floor_mask: &[bool],
    roughness: f32,
    strength: f32,
) {
    let pixel_count = (width as usize).saturating_mul(height as usize);
    if beauty.len() != pixel_count.saturating_mul(4)
        || reflected.len() != beauty.len()
        || floor_mask.len() != pixel_count
    {
        return;
    }
    let radius = if roughness <= 0.0 {
        0
    } else {
        ((width.min(height) as f32 * roughness.clamp(0.0, 1.0) * 0.012).round() as usize)
            .clamp(1, 24)
    };
    let reflected = box_blur_rgba8(reflected, width as usize, height as usize, radius);
    for (index, is_floor) in floor_mask.iter().copied().enumerate() {
        if !is_floor {
            continue;
        }
        let offset = index * 4;
        let luma = (0.2126 * f32::from(beauty[offset])
            + 0.7152 * f32::from(beauty[offset + 1])
            + 0.0722 * f32::from(beauty[offset + 2]))
            / 255.0;
        let mix = strength.clamp(0.0, 1.0) * luma.clamp(0.25, 1.0);
        for channel in 0..3 {
            beauty[offset + channel] = (f32::from(beauty[offset + channel]) * (1.0 - mix)
                + f32::from(reflected[offset + channel]) * mix)
                .round()
                .clamp(0.0, 255.0) as u8;
        }
    }
}

fn box_blur_rgba8(input: &[u8], width: usize, height: usize, radius: usize) -> Vec<u8> {
    if radius == 0 || width == 0 || height == 0 {
        return input.to_vec();
    }
    let mut horizontal = vec![0_u8; input.len()];
    for y in 0..height {
        let mut sums = [0_u32; 4];
        for sample_x in 0..=radius.min(width - 1) {
            for channel in 0..4 {
                sums[channel] += u32::from(input[(y * width + sample_x) * 4 + channel]);
            }
        }
        for x in 0..width {
            let min_x = x.saturating_sub(radius);
            let max_x = (x + radius).min(width - 1);
            let count = (max_x - min_x + 1) as u32;
            for channel in 0..4 {
                horizontal[(y * width + x) * 4 + channel] = (sums[channel] / count) as u8;
            }
            let next_min = (x + 1).saturating_sub(radius);
            if next_min > min_x {
                for channel in 0..4 {
                    sums[channel] -= u32::from(input[(y * width + min_x) * 4 + channel]);
                }
            }
            let next_max = (x + 1 + radius).min(width - 1);
            if next_max > max_x {
                for channel in 0..4 {
                    sums[channel] += u32::from(input[(y * width + next_max) * 4 + channel]);
                }
            }
        }
    }
    let mut output = vec![0_u8; input.len()];
    for x in 0..width {
        let mut sums = [0_u32; 4];
        for sample_y in 0..=radius.min(height - 1) {
            for channel in 0..4 {
                sums[channel] += u32::from(horizontal[(sample_y * width + x) * 4 + channel]);
            }
        }
        for y in 0..height {
            let min_y = y.saturating_sub(radius);
            let max_y = (y + radius).min(height - 1);
            let count = (max_y - min_y + 1) as u32;
            for channel in 0..4 {
                output[(y * width + x) * 4 + channel] = (sums[channel] / count) as u8;
            }
            let next_min = (y + 1).saturating_sub(radius);
            if next_min > min_y {
                for channel in 0..4 {
                    sums[channel] -= u32::from(horizontal[(min_y * width + x) * 4 + channel]);
                }
            }
            let next_max = (y + 1 + radius).min(height - 1);
            if next_max > max_y {
                for channel in 0..4 {
                    sums[channel] += u32::from(horizontal[(next_max * width + x) * 4 + channel]);
                }
            }
        }
    }
    output
}

/// Rebuild the generated surroundings for the camera that is framed right now.
///
/// Sizing the backdrop is a function of the frustum, and the frustum changes
/// every time the composition corrector moves the camera. Regenerating keeps the
/// two consistent; without it the backdrop keeps whatever size the setup-time
/// camera implied and its edge walks into the frame as the loop zooms.
///
/// `None` in means the caller generated nothing to resize - an authored backdrop
/// or an authored environment - and that decision is preserved.
fn resized_photographic_surroundings(
    host: &mut scena::SceneHostCore,
    subject: &SubjectSelection,
    previous: Option<scena::PhotographicSurroundingsReportV1>,
) -> Result<Option<scena::PhotographicSurroundingsReportV1>, CliFailure> {
    let Some(previous) = previous else {
        return Ok(None);
    };
    if !previous.generated_floor && !previous.generated_cyclorama {
        return Ok(Some(previous));
    }
    let synthetic_contact_shadow = previous.contact_shadow_strength > 0.0;
    let ground = previous.ground;
    host.remove_photographic_surroundings(&previous)
        .map_err(runtime_failure)?;
    let mut report = host
        .apply_photographic_surroundings_with_ground(subject.root_handle, ground)
        .map_err(runtime_failure)?;
    apply_synthetic_contact_shadow_policy(host, &mut report, synthetic_contact_shadow)?;
    Ok(Some(report))
}

fn apply_synthetic_contact_shadow_policy(
    host: &mut scena::SceneHostCore,
    report: &mut scena::PhotographicSurroundingsReportV1,
    enabled: bool,
) -> Result<(), CliFailure> {
    if enabled {
        return Ok(());
    }
    // Final stills use geometry-derived area-light visibility. Surroundings are
    // regenerated after every reframe, and that generator supplies preview SSAO
    // as a fallback; clear it here as part of the same final grounding policy so
    // a resize cannot silently re-enable the depth-threshold pass.
    host.renderer_mut().set_screen_space_ambient_occlusion(None);
    if std::env::var_os("SCENA_DEBUG_LOG_STAGING").is_some() {
        eprintln!(
            "[staging] final grounding policy area_visibility=true ssao enabled={}",
            host.renderer().screen_space_ambient_occlusion().is_some()
        );
    }
    let contact_nodes = std::mem::take(&mut report.contact_shadow_nodes);
    for node in &contact_nodes {
        host.remove_node(*node).map_err(runtime_failure)?;
    }
    report
        .generated_nodes
        .retain(|node| !contact_nodes.contains(node));
    report.contact_shadow_strength = 0.0;
    Ok(())
}

fn corrected_exposure_ev(current_ev: f32, metrics: SubjectMetrics) -> Option<f32> {
    if metrics.sample_count == 0 || !metrics.mean_luminance_srgb8.is_finite() {
        return None;
    }
    if unreadable_dark_material(metrics)
        && metrics.high_clip_fraction >= CAMERA_BEHAVIOR_HIGHLIGHT_LIMITED_MIN_CLIP
    {
        // Exposure cannot open the body without worsening the already-limited
        // chrome. Hold EV so the single final lighting retry can act on the
        // readable-body problem instead of first crushing it further.
        return None;
    }
    let mut correction = if metrics.high_clip_fraction > CAMERA_BEHAVIOR_MAX_HIGH_CLIP {
        // Clipped samples no longer contain enough information to solve an
        // exact compensation. Move down conservatively and remeasure instead
        // of allowing a dark product's mean to drive chrome farther into the
        // shoulder on every retry.
        let clip_ratio =
            metrics.high_clip_fraction / CAMERA_BEHAVIOR_MAX_HIGH_CLIP.max(f64::EPSILON);
        -(0.5 * clip_ratio.log2()).clamp(0.25, 1.5)
    } else if dark_product_is_readable(metrics) {
        // A deliberately dark, structured material is correctly exposed in
        // this range. Chasing the generic subject target would turn charcoal,
        // black paint, and dark rubber gray while consuming highlight
        // headroom on adjacent metal.
        return None;
    } else if metrics.mean_luminance_srgb8 < CAMERA_BEHAVIOR_TARGET_MEAN_LUMA
        && metrics.high_clip_fraction >= CAMERA_BEHAVIOR_HIGHLIGHT_LIMITED_MIN_CLIP
    {
        // A positive EV change cannot open the body without consuming the
        // remaining highlight budget. Leave this failure for lighting or
        // material correction instead of oscillating across the clip limit.
        return None;
    } else {
        let measured =
            (CAMERA_BEHAVIOR_TARGET_MEAN_LUMA / metrics.mean_luminance_srgb8.max(1.0)).log2();
        if measured > 0.0 {
            // Preserve headroom as the clipped-pixel budget fills. Lighting is
            // responsible for opening a dark body; exposure must not turn a
            // small polished accent into a white patch to hit an average.
            let remaining_clip_headroom =
                (1.0 - metrics.high_clip_fraction / CAMERA_BEHAVIOR_MAX_HIGH_CLIP).clamp(0.0, 1.0);
            measured.min(0.15 + 0.85 * remaining_clip_headroom)
        } else {
            measured
        }
    };
    if unreadable_dark_material(metrics)
        && metrics.high_clip_fraction < CAMERA_BEHAVIOR_HIGHLIGHT_LIMITED_MIN_CLIP
    {
        // The subject-wide mean can be dominated by chrome and an indicator
        // while most of a dark body remains crushed. Give that body one small
        // exposure step, but cap it below half a stop so the next measured
        // frame retains authority over polished highlights.
        correction = correction.max(0.25).min(0.50);
    }
    if correction.abs() <= 0.03 && metrics.low_clip_fraction <= CAMERA_BEHAVIOR_MAX_LOW_CLIP {
        return None;
    }
    Some((current_ev + correction as f32).clamp(
        CAMERA_BEHAVIOR_MIN_EXPOSURE_EV,
        CAMERA_BEHAVIOR_MAX_EXPOSURE_EV,
    ))
}

fn bounded_default_exposure_ev(base_ev: f32, metrics: SubjectMetrics) -> Option<f32> {
    if metrics.sample_count == 0 || !metrics.mean_luminance_srgb8.is_finite() {
        return None;
    }
    let correction = if metrics.high_clip_fraction > CAMERA_BEHAVIOR_MAX_HIGH_CLIP {
        let clip_ratio =
            metrics.high_clip_fraction / CAMERA_BEHAVIOR_MAX_HIGH_CLIP.max(f64::EPSILON);
        -(0.5 * clip_ratio.log2()).clamp(0.25, 1.5)
    } else {
        (CAMERA_BEHAVIOR_TARGET_MEAN_LUMA / metrics.mean_luminance_srgb8.max(1.0)).log2()
    };
    if correction.abs() <= 0.03 && metrics.low_clip_fraction <= CAMERA_BEHAVIOR_MAX_LOW_CLIP {
        return None;
    }
    let bounded = base_ev
        + (correction as f32).clamp(
            -DEFAULT_PHOTO_MAX_EXPOSURE_CORRECTION_EV,
            DEFAULT_PHOTO_MAX_EXPOSURE_CORRECTION_EV,
        );
    ((bounded - base_ev).abs() > f32::EPSILON).then_some(bounded)
}

fn corrected_focus_delivery_exposure_ev(current_ev: f32, metrics: SubjectMetrics) -> Option<f32> {
    let failures = camera_behavior_failure_codes(metrics);
    if !failures.iter().any(|code| {
        matches!(
            *code,
            "subject_luminance_below_min"
                | "subject_luminance_above_max"
                | "subject_low_clip_above_max"
                | "subject_high_clip_above_max"
        )
    }) {
        return None;
    }
    corrected_exposure_ev(current_ev, metrics)
}

fn corrected_photographic_lighting(
    metrics: SubjectMetrics,
) -> Option<scena::scene_host::PhotographicLightingAdjustmentV1> {
    if metrics.sample_count == 0 {
        return None;
    }
    let mut adjustment = scena::scene_host::PhotographicLightingAdjustmentV1::default();
    let mut changed = false;
    let needs_dark_material_readability = unreadable_dark_material(metrics);
    if needs_dark_material_readability {
        // Open the dark material with broad sources and environment response,
        // not a harder key that would only make adjacent chrome clip sooner.
        adjustment.fill_scale = adjustment.fill_scale.max(3.0);
        adjustment.overhead_scale = adjustment.overhead_scale.max(1.40);
        adjustment.rim_scale = adjustment.rim_scale.max(1.50);
        adjustment.environment_intensity_scale = adjustment.environment_intensity_scale.max(1.50);
        adjustment.key_scale = adjustment.key_scale.min(0.85);
        changed = true;
    }
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
    let dark_clipping = metrics.low_clip_fraction > CAMERA_BEHAVIOR_MAX_LOW_CLIP;
    if metrics.shadow_presence < 0.01 {
        if dark_clipping {
            // A dark reflective product needs readable fill before stronger
            // key contrast. Reducing fill here made the speaker body black
            // while the chrome clipped, then exposure amplified both defects.
            adjustment.key_scale *= 0.92;
        } else {
            adjustment.key_scale *= 1.06;
            adjustment.fill_scale *= 0.76;
        }
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
    }
    if dark_clipping {
        adjustment.fill_scale = adjustment.fill_scale.max(1.8);
        adjustment.overhead_scale = adjustment.overhead_scale.max(1.10);
        adjustment.key_scale = adjustment.key_scale.min(0.92);
        changed = true;
    }
    if needs_dark_material_readability {
        // Other observations may rotate or soften the setup, but they must not
        // erase the readability correction that made this branch necessary.
        adjustment.key_scale = adjustment.key_scale.min(0.85);
        adjustment.fill_scale = adjustment.fill_scale.max(3.0);
        adjustment.rim_scale = adjustment.rim_scale.max(1.50);
        adjustment.overhead_scale = adjustment.overhead_scale.max(1.40);
        adjustment.environment_intensity_scale = adjustment.environment_intensity_scale.max(1.50);
    }
    changed.then_some(adjustment)
}

fn camera_behavior_failure_codes(metrics: SubjectMetrics) -> Vec<&'static str> {
    camera_behavior_acceptance_failure_codes(CameraBehaviorGateEvidence::from_metrics(metrics))
}

fn dark_product_is_readable(metrics: SubjectMetrics) -> bool {
    let measured_dark = metrics
        .dark_material_mean_luminance_srgb8
        .filter(|_| metrics.dark_material_coverage >= 0.05)
        .unwrap_or(metrics.mean_luminance_srgb8);
    metrics.mean_luminance_srgb8 <= CAMERA_BEHAVIOR_MAX_MEAN_LUMA
        && measured_dark >= CAMERA_BEHAVIOR_DARK_PRODUCT_MIN_MEAN_LUMA
        && measured_dark <= CAMERA_BEHAVIOR_DARK_PRODUCT_MAX_MEAN_LUMA
        && metrics.low_clip_fraction <= CAMERA_BEHAVIOR_MAX_LOW_CLIP
        && metrics.high_clip_fraction <= CAMERA_BEHAVIOR_MAX_HIGH_CLIP
        && metrics.luminance_stddev_srgb8 >= CAMERA_BEHAVIOR_MIN_LUMA_STDDEV
        && metrics.luminance_range_srgb8 >= CAMERA_BEHAVIOR_MIN_LUMA_RANGE
}

fn unreadable_dark_material(metrics: SubjectMetrics) -> bool {
    let identified_dark_material = metrics.dark_material_coverage >= 0.05
        && metrics
            .dark_material_mean_luminance_srgb8
            .is_some_and(|mean| mean < CAMERA_BEHAVIOR_DARK_PRODUCT_MIN_MEAN_LUMA);
    let missing_material_identity_with_crushed_region =
        metrics.dark_material_mean_luminance_srgb8.is_none()
            && metrics.low_clip_fraction > CAMERA_BEHAVIOR_MAX_LOW_CLIP;
    identified_dark_material || missing_material_identity_with_crushed_region
}

fn should_retry_final_dark_material_lighting(metrics: SubjectMetrics) -> bool {
    unreadable_dark_material(metrics)
        && metrics.high_clip_fraction >= CAMERA_BEHAVIOR_HIGHLIGHT_LIMITED_MIN_CLIP
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
    if metrics.silhouette_separation < CAMERA_BEHAVIOR_MIN_SILHOUETTE_SEPARATION {
        failures.push("subject_color_frame_agreement_below_min");
    }
    let fit = subject_fit_fraction(metrics);
    if fit < CAMERA_BEHAVIOR_MIN_FILL_WIDTH {
        failures.push("subject_fill_below_min");
    }
    if fit > CAMERA_BEHAVIOR_MAX_FILL_WIDTH
        || metrics.fill_fraction > CAMERA_BEHAVIOR_MAX_FIT_FRACTION
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
    if metrics.shadow_presence < 0.01 {
        failures.push("contact_shadow_missing");
    } else if metrics.shadow_softness < 0.20 {
        failures.push("shadow_too_hard");
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
        quality_execution,
        quality_analysis,
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
        "correction_mode": if args.optimize {
            "iterative_optimizer"
        } else {
            "deterministic_one_shot"
        },
        "retry": retry_json(candidates, args.optimize),
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
            "analysis": quality_analysis,
            "execution": quality_execution,
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
            "edge_rounding": manifest.imports.iter()
                .filter_map(|import| import.edge_rounding.as_ref())
                .collect::<Vec<_>>(),
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
        "final_candidate_render_budget": if args.optimize {
            CAMERA_BEHAVIOR_MAX_ATTEMPTS
        } else {
            2
        },
        "final_candidate_renders": final_work.render_calls,
        "final_candidate_width": args.width,
        "final_candidate_height": args.height,
        "final_candidate_pixels": final_candidate_pixels
            .saturating_mul(final_work.render_calls),
        "focus_delivery_render_budget": if args.optimize {
            CAMERA_BEHAVIOR_FOCUS_DELIVERY_MAX_ATTEMPTS
        } else {
            1
        },
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
    optimize: bool,
) -> scena::ExposureReportV1 {
    let metrics = candidate.metrics;
    let suggested_ev = if optimize {
        corrected_exposure_ev(candidate.exposure_ev, metrics)
    } else {
        bounded_default_exposure_ev(FINAL_PHOTO_BASE_EXPOSURE_EV, metrics)
    };
    let suggested_compensation_ev = suggested_ev
        .map(|next_ev| next_ev - candidate.exposure_ev)
        .unwrap_or(0.0);
    scena::ExposureReportV1::measured_subject(
        if optimize {
            "camera_behavior_optimizer"
        } else {
            "camera_behavior_one_shot"
        },
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
    // Reached only when the semantic-AOV measurement above was unavailable.
    // A focus distance derived from bounds geometry is a guess, not a
    // measurement: the depth range is the bounding sphere and the confidence
    // and visible-pixel count were literals. Reporting that as `resolved` told
    // callers focus had been measured from the subject when nothing had been
    // sampled, so it is reported as unresolved with the distance the geometry
    // implies named in the reason.
    let _ = (bounds, camera_transform);
    scena::FocusReportV1::unresolved(
        "subject",
        target,
        Some("subject".to_owned()),
        Some("camera_auto".to_owned()),
        "subject depth was not sampled; only bounds-derived focus geometry was available",
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

fn render_focused_delivery(
    host: &mut scena::SceneHostCore,
    subject: &SubjectSelection,
    gpu: bool,
    selected: &mut SelectedCapture,
    work_metrics: &mut PhotoLoopWorkMetrics,
    optimize: bool,
    rerender_initial: bool,
) -> Result<(), CliFailure> {
    let max_attempts = if optimize {
        CAMERA_BEHAVIOR_FOCUS_DELIVERY_MAX_ATTEMPTS
    } else {
        1
    };
    for attempt in 0..max_attempts {
        let capture = if attempt == 0 && !rerender_initial {
            selected.capture.clone()
        } else {
            render_capture(host, work_metrics)?
        };
        let semantic_aov = capture_camera_behavior_semantic_aovs(host, gpu)?;
        let inspection_json = host.inspect_json().map_err(runtime_failure)?;
        let inspection: scena::SceneInspectionReportV1 = serde_json::from_str(&inspection_json)
            .map_err(|error| {
                CliFailure::new(
                    CliErrorKind::Internal,
                    format!("failed to decode focused scene inspection report: {error}"),
                )
            })?;
        let metrics = match measure_subject(&capture, &inspection, &semantic_aov, subject) {
            Ok(metrics) => metrics,
            Err(error) if photo_subject_measurement_can_degrade(&error.message) => {
                empty_subject_metrics()
            }
            Err(error) => return Err(error),
        };
        work_metrics.record_subject_samples(metrics.sample_count);

        selected.capture = capture;
        selected.final_candidate.exposure_ev = host.renderer().exposure_ev();
        selected.final_candidate.metrics = metrics;
        selected.final_candidate.failure_codes = camera_behavior_failure_codes(metrics);
        selected.final_candidate.status = if selected.final_candidate.failure_codes.is_empty() {
            "passed"
        } else {
            "failed"
        };
        if selected.final_candidate.status == "passed" || attempt + 1 >= max_attempts {
            break;
        }

        let next_ev = if optimize {
            corrected_focus_delivery_exposure_ev(selected.final_candidate.exposure_ev, metrics)
        } else {
            bounded_default_exposure_ev(FINAL_PHOTO_BASE_EXPOSURE_EV, metrics)
        };
        let Some(next_ev) = next_ev else {
            break;
        };
        if (next_ev - selected.final_candidate.exposure_ev).abs() <= f32::EPSILON {
            break;
        }
        host.renderer_mut().clear_auto_exposure();
        host.renderer_mut().set_exposure_ev(next_ev);
        selected.final_candidate.adjustment = Some(if optimize {
            "subject_focus+exposure_delta"
        } else {
            "bounded_exposure_delta"
        });
    }
    Ok(())
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
    let mut handles = Vec::with_capacity(subject.draw_handles.len() + subject.root_handles.len());
    handles.extend(subject.root_handles.iter().copied());
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
        "staging": serde_json::to_value(&selection.surroundings_report)
            .expect("photographic surroundings report serializes"),
        "reflection_probes": selection.reflection_probe_report,
        "lighting": selection.lighting_report.as_ref().map(|report| json!({
            "source": report.source,
            "generated_local_light_count": report.lights.len(),
        })),
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

fn retry_json(candidates: &[PhotoCandidate], optimize: bool) -> Value {
    let max_attempts = if optimize {
        CAMERA_BEHAVIOR_MAX_ATTEMPTS
    } else {
        2
    };
    let budget_exhausted = candidates.len() >= max_attempts
        && candidates
            .last()
            .is_some_and(|candidate| candidate.status != "passed");
    let first_retryable_failure = candidates
        .iter()
        .find(|candidate| candidate.status != "passed")
        .and_then(|candidate| {
            if optimize {
                if candidate.failure_codes.iter().any(|code| {
                    matches!(*code, "subject_fill_below_min" | "subject_fill_above_max")
                }) {
                    return Some(json!({
                        "source_candidate_id": candidate.id,
                        "kind": "camera_composition",
                        "target_fill_width_fraction": CAMERA_BEHAVIOR_TARGET_FILL_WIDTH,
                    }));
                }
                return corrected_exposure_ev(candidate.exposure_ev, candidate.metrics).map(
                    |next_ev| {
                        json!({
                            "source_candidate_id": candidate.id,
                            "kind": "exposure_compensation_ev",
                            "delta_ev": next_ev - candidate.exposure_ev,
                            "next_exposure_ev": next_ev,
                        })
                    },
                );
            }
            bounded_default_exposure_ev(FINAL_PHOTO_BASE_EXPOSURE_EV, candidate.metrics).map(
                |next_ev| {
                    json!({
                        "source_candidate_id": candidate.id,
                        "kind": "exposure_compensation_ev",
                        "delta_ev": next_ev - FINAL_PHOTO_BASE_EXPOSURE_EV,
                        "next_exposure_ev": next_ev,
                    })
                },
            )
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
            "max_attempts": max_attempts,
            "max_retries": max_attempts.saturating_sub(1),
            "allowed_adjustments": if optimize {
                json!(["camera_composition", "exposure_compensation_ev"])
            } else {
                json!(["exposure_compensation_ev"])
            },
            "loop": if optimize { "bounded" } else { "one_shot" },
        },
        "attempts": candidates.len(),
        "retry_used": candidates.len() > 1,
        "budget_exhausted": budget_exhausted,
        "final_attempt_id": candidates.last().map(|candidate| candidate.id.clone()),
        "suggestion": first_retryable_failure,
        "retry_input": retry_input,
    })
}

fn refresh_selected_candidate_history(
    candidates: &mut [PhotoCandidate],
    selected: &PhotoCandidate,
) -> bool {
    let Some(candidate) = candidates
        .iter_mut()
        .find(|candidate| candidate.id == selected.id)
    else {
        return false;
    };
    *candidate = selected.clone();
    true
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
        "dark_material_mean_luminance_srgb8": metrics
            .dark_material_mean_luminance_srgb8
            .map(round2_f64),
        "dark_material_coverage": round4(metrics.dark_material_coverage),
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
    let pixel_count = (capture.descriptor.width * capture.descriptor.height) as usize;
    if semantic_aov.width != capture.descriptor.width
        || semantic_aov.height != capture.descriptor.height
        || semantic_aov.id_indices.len() != pixel_count
        || semantic_aov
            .beauty_id_indices
            .as_ref()
            .is_some_and(|ids| ids.len() != pixel_count)
    {
        return Err(CliFailure::new(
            CliErrorKind::Runtime,
            "photo semantic subject mask dimensions do not match the rendered frame",
        ));
    }
    let subject_mask_ids = semantic_aov
        .beauty_id_indices
        .as_deref()
        .unwrap_or(&semantic_aov.id_indices);
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
    let subject_material_by_palette = semantic_aov
        .legend
        .iter()
        .filter(|entry| {
            subject_handles.contains(&entry.node_handle)
                || entry
                    .instance_handle
                    .is_some_and(|handle| subject_handles.contains(&handle))
        })
        .filter_map(|entry| {
            entry
                .material_handle
                .map(|material| (entry.palette_index, material))
        })
        .collect::<BTreeMap<_, _>>();
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
    // Magnitude of the overflow, from the union of per-draw projected AABBs.
    // That union is conservative: for an assembly of rotated parts it bounds a
    // volume far larger than the silhouette, so on its own it reports clipping
    // for subjects that are visibly inside the frame. It is trusted only when
    // the subject's own visible pixels also reach a border, below.
    let projected_clipped_fraction = if raw_area > 0.0 {
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
    let mut material_luminance = BTreeMap::<u64, (f64, u64)>::new();
    let background = capture_background_rgba8(capture);
    for y in start_y..end_y {
        for x in start_x..end_x {
            let offset = ((y as usize) * capture.descriptor.width as usize + x as usize) * 4;
            let Some(pixel) = capture.rgba8.get(offset..offset + 4) else {
                continue;
            };
            let pixel_index = y as usize * capture.descriptor.width as usize + x as usize;
            let Some(palette_index) = subject_mask_ids
                .get(pixel_index)
                .copied()
                .filter(|index| subject_palette.contains(index))
            else {
                continue;
            };
            if pixel[3] == 0 {
                continue;
            }
            let background_delta = max_rgb_delta_rgba8(pixel, background);
            let luma = 0.2126 * f64::from(pixel[0])
                + 0.7152 * f64::from(pixel[1])
                + 0.0722 * f64::from(pixel[2]);
            sum_luma += luma;
            sum_luma_sq += luma * luma;
            if let Some(material_handle) = subject_material_by_palette.get(&palette_index) {
                let material = material_luminance.entry(*material_handle).or_default();
                material.0 += luma;
                material.1 = material.1.saturating_add(1);
            }
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
            if pixel_has_output_channel_clip(pixel) {
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
    // Only report clipping when the semantic mask itself reaches a frame edge.
    // Without this the conservative AABB above fails renders whose subject is
    // demonstrably whole, and it makes the acceptance gate unsatisfiable: the
    // corrector must zoom in to raise fill, which grows the AABB overflow and
    // trips clipping, so no candidate can satisfy both.
    let visible_touches_frame_edge = visible_min_x <= 0.5
        || visible_min_y <= 0.5
        || visible_max_x >= capture.descriptor.width.saturating_sub(1) as f32 - 0.5
        || visible_max_y >= capture.descriptor.height.saturating_sub(1) as f32 - 0.5;
    let clipped_fraction = if visible_touches_frame_edge {
        projected_clipped_fraction
    } else {
        0.0
    };
    let fill_width_fraction = f64::from((projected_max_x - projected_min_x).max(0.0))
        / f64::from(capture.descriptor.width.max(1));
    let fill_height_fraction = f64::from((projected_max_y - projected_min_y).max(0.0))
        / f64::from(capture.descriptor.height.max(1));
    let mean_luminance_srgb8 = sum_luma / sample_count as f64;
    let (dark_material_mean_luminance_srgb8, dark_material_coverage) =
        select_dark_material_region(&material_luminance, sample_count);
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
        dark_material_mean_luminance_srgb8,
        dark_material_coverage,
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

fn select_dark_material_region(
    material_luminance: &BTreeMap<u64, (f64, u64)>,
    subject_sample_count: u64,
) -> (Option<f64>, f64) {
    let minimum_samples = (subject_sample_count / 20).max(32);
    material_luminance
        .values()
        .filter(|(_, samples)| *samples >= minimum_samples)
        .map(|(sum, samples)| {
            (
                sum / *samples as f64,
                *samples as f64 / subject_sample_count.max(1) as f64,
            )
        })
        .min_by(|left, right| left.0.total_cmp(&right.0))
        .map_or((None, 0.0), |(mean, coverage)| (Some(mean), coverage))
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

const SUPPORT_SHADOW_MIN_DELTA_SRGB8: f64 = 3.0;
const SUPPORT_SHADOW_SOFT_MAX_DELTA_SRGB8: f64 = 48.0;

fn measure_local_support_shadow(
    receiver_luma: &[f64],
    width: usize,
    height: usize,
    outer_radius: usize,
) -> (f64, f64) {
    if width == 0 || height == 0 || receiver_luma.len() != width.saturating_mul(height) || width < 5
    {
        return (0.0, 0.0);
    }
    let outer_radius = outer_radius.clamp(2, width.saturating_sub(1) / 2);
    let inner_radius = (outer_radius / 2).max(1);
    let mut shadow_count = 0_u64;
    let mut soft_shadow_count = 0_u64;
    let mut support_samples = 0_u64;
    let mut row_sum = vec![0.0_f64; width + 1];
    let mut row_count = vec![0_u64; width + 1];

    for y in 0..height {
        row_sum.fill(0.0);
        row_count.fill(0);
        for x in 0..width {
            let sample = receiver_luma[y * width + x];
            row_sum[x + 1] = row_sum[x] + if sample.is_finite() { sample } else { 0.0 };
            row_count[x + 1] = row_count[x].saturating_add(u64::from(sample.is_finite()));
        }
        for x in outer_radius..width.saturating_sub(outer_radius) {
            let sample = receiver_luma[y * width + x];
            if !sample.is_finite() {
                continue;
            }
            let left_start = x - outer_radius;
            let left_end = x - inner_radius;
            let right_start = x + inner_radius + 1;
            let right_end = x + outer_radius + 1;
            let reference_sum =
                row_sum[left_end] - row_sum[left_start] + row_sum[right_end] - row_sum[right_start];
            let reference_count = row_count[left_end]
                .saturating_sub(row_count[left_start])
                .saturating_add(row_count[right_end].saturating_sub(row_count[right_start]));
            if reference_count < 2 {
                continue;
            }
            let delta = reference_sum / reference_count as f64 - sample;
            support_samples = support_samples.saturating_add(1);
            if delta >= SUPPORT_SHADOW_MIN_DELTA_SRGB8 {
                shadow_count = shadow_count.saturating_add(1);
                soft_shadow_count += u64::from(delta <= SUPPORT_SHADOW_SOFT_MAX_DELTA_SRGB8);
            }
        }
    }

    (
        shadow_count as f64 / support_samples.max(1) as f64,
        soft_shadow_count as f64 / shadow_count.max(1) as f64,
    )
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
    let subject_mask_ids = semantic_aov
        .beauty_id_indices
        .as_deref()
        .unwrap_or(&semantic_aov.id_indices);
    let is_subject = |index: usize| {
        subject_mask_ids
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
            let boundary_delta = [
                (x > 0).then_some(index - 1),
                (x + 1 < width).then_some(index + 1),
                (y > 0).then_some(index - width),
                (y + 1 < height).then_some(index + width),
            ]
            .into_iter()
            .flatten()
            .filter(|neighbor| !is_subject(*neighbor))
            .filter_map(|neighbor| {
                capture
                    .rgba8
                    .get(neighbor * 4..neighbor * 4 + 4)
                    .map(|neighbor| max_rgb_delta_between_rgba8(pixel, neighbor))
            })
            .max();
            if let Some(delta) = boundary_delta {
                silhouette_delta_sum += f64::from(delta);
                silhouette_count = silhouette_count.saturating_add(1);
            }
        }
    }

    let background_mean_luminance_srgb8 = if background_count > 0 {
        background_luma_sum / background_count as f64
    } else {
        pixel_luminance(&background)
    };
    let min_x = projected_rect[0].floor().max(0.0) as usize;
    let max_x = projected_rect[2].ceil().min(width as f32) as usize;
    let start_y = projected_rect[3].floor().max(0.0) as usize;
    let end_y = (start_y + (height / 10).max(2)).min(height);
    let support_width = max_x.saturating_sub(min_x);
    let support_height = end_y.saturating_sub(start_y);
    let mut receiver_luma = Vec::with_capacity(support_width.saturating_mul(support_height));
    for y in start_y..end_y {
        for x in min_x..max_x {
            let index = y * width + x;
            if is_subject(index) {
                receiver_luma.push(f64::NAN);
                continue;
            }
            receiver_luma.push(pixel_luminance(&capture.rgba8[index * 4..index * 4 + 4]));
        }
    }
    let shadow_radius = (support_width / 5).max(2);
    let (shadow_presence, shadow_softness) =
        measure_local_support_shadow(&receiver_luma, support_width, support_height, shadow_radius);
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
        shadow_presence,
        shadow_softness,
        silhouette_separation: silhouette_delta_sum / silhouette_count.max(1) as f64 / 255.0,
        mean_saturation,
        color_cast,
        reflection_washout: (highlight_fraction * (1.0 - mean_saturation)).clamp(0.0, 1.0),
    }
}

fn pixel_luminance(pixel: &[u8]) -> f64 {
    0.2126 * f64::from(pixel[0]) + 0.7152 * f64::from(pixel[1]) + 0.0722 * f64::from(pixel[2])
}

fn pixel_has_output_channel_clip(pixel: &[u8]) -> bool {
    pixel.get(..3).is_some_and(|rgb| rgb.contains(&u8::MAX))
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

fn max_rgb_delta_between_rgba8(left: &[u8], right: &[u8]) -> u8 {
    (0..3)
        .map(|channel| left[channel].abs_diff(right[channel]))
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
    let (root_handle, root_handles) = match &target {
        scena::SceneRecipeTargetV1::Import { id } => {
            let import = manifest
                .imports
                .iter()
                .find(|import| import.id == *id)
                .ok_or_else(|| {
                    CliFailure::new(
                        CliErrorKind::Runtime,
                        format!("photo subject import '{id}' has no build manifest entry"),
                    )
                })?;
            let root_handle = import
                .primary_root
                .or_else(|| import.root_handles.first().copied())
                .or_else(|| handles.first().copied())
                .ok_or_else(|| {
                    CliFailure::new(
                        CliErrorKind::Runtime,
                        format!("photo subject import '{id}' resolved to no root handle"),
                    )
                })?;
            let mut root_handles = import.root_handles.clone();
            if root_handles.is_empty() {
                root_handles.push(root_handle);
            }
            (root_handle, root_handles)
        }
        scena::SceneRecipeTargetV1::Node { .. } => (handles[0], vec![handles[0]]),
        scena::SceneRecipeTargetV1::World { .. } => unreachable!("world targets are rejected"),
    };
    let draw_handles = handles.into_iter().collect::<BTreeSet<_>>();
    Ok(SubjectSelection {
        target_kind,
        id,
        root_handle,
        root_handles,
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
    quality: scena::SceneRecipePhotoQualityV1,
    ground: scena::scene_host::PhotographicGroundV1,
    optimize: bool,
) -> Result<ShadedCandidateSelection, CliFailure> {
    let use_builtin_studio = use_builtin_studio_lighting(authored_lights, optimize);
    let surface_report = apply_subject_photographic_surface(host, subject)?;
    ensure_photographic_asset_usable(&surface_report)?;
    let mut surroundings_report = host
        .apply_photographic_surroundings_with_ground(subject.root_handle, ground)
        .map_err(runtime_failure)?;
    apply_synthetic_contact_shadow_policy(
        host,
        &mut surroundings_report,
        synthetic_contact_shadow_enabled(quality),
    )?;
    configure_camera_behavior_renderer(host, quality, optimize)?;
    if use_builtin_studio {
        host.add_studio_lighting().map_err(runtime_failure)?;
    }
    let mut shaded_selection = select_camera_behavior_shaded_candidate(
        host,
        subject,
        planning,
        authored_lights || use_builtin_studio,
        surface_report,
        surroundings_report,
        gpu,
        optimize,
    )?;
    let selected = selected_shaded_composition_candidate(planning, &shaded_selection)?;
    host.frame_nodes_with_photo_candidate(&subject.root_handles, selected)
        .map_err(runtime_failure)?;
    let selected_adjustment = shaded_selection
        .candidates
        .iter()
        .find(|candidate| candidate.id == shaded_selection.selected_candidate_id)
        .and_then(|candidate| candidate.lighting_adjustment);
    if !authored_lights || selected_adjustment.is_some() {
        let adjustment = selected_adjustment.unwrap_or_default();
        let mut lighting_report = if quality.is_final() {
            host.apply_final_photographic_lighting_adjusted(subject.root_handle, adjustment)
        } else {
            host.apply_photographic_lighting_adjusted(subject.root_handle, adjustment)
        }
        .map_err(runtime_failure)?;
        if use_builtin_studio {
            for light in &lighting_report.lights {
                host.remove_node(light.node).map_err(runtime_failure)?;
            }
            lighting_report.lights.clear();
            lighting_report.source = "built_in_studio_directional".to_owned();
        }
        shaded_selection.lighting_report = Some(lighting_report);
    }
    configure_camera_behavior_renderer(host, quality, optimize)?;
    if automatic_reflection_probe_bake_enabled(quality) {
        shaded_selection.reflection_probe_report = Some(
            host.bake_photographic_reflection_probes(subject.root_handle)
                .map_err(runtime_failure)?,
        );
    }
    Ok(shaded_selection)
}

const fn use_builtin_studio_lighting(authored_lights: bool, optimize: bool) -> bool {
    !authored_lights && !optimize
}

fn apply_subject_photographic_surface(
    host: &mut scena::SceneHostCore,
    subject: &SubjectSelection,
) -> Result<scena::PhotographicSurfaceReportV1, CliFailure> {
    let mut roots = subject.root_handles.clone();
    if !roots.contains(&subject.root_handle) {
        roots.push(subject.root_handle);
    }
    roots.sort_unstable();
    roots.dedup();

    let mut reports = roots.into_iter().map(|root| {
        host.apply_photographic_surface(root)
            .map_err(runtime_failure)
    });
    let mut aggregate = reports.next().transpose()?.ok_or_else(|| {
        CliFailure::new(
            CliErrorKind::Runtime,
            "photo subject resolved to no roots for photographic surface preparation",
        )
    })?;
    for report in reports {
        merge_photographic_surface_report(&mut aggregate, report?);
    }
    aggregate.subject = subject.root_handle;
    Ok(aggregate)
}

fn merge_photographic_surface_report(
    aggregate: &mut scena::PhotographicSurfaceReportV1,
    report: scena::PhotographicSurfaceReportV1,
) {
    aggregate.mesh_count += report.mesh_count;
    aggregate.repaired_normal_meshes += report.repaired_normal_meshes;
    aggregate.reversed_winding_meshes += report.reversed_winding_meshes;
    aggregate.disconnected_meshes += report.disconnected_meshes;
    aggregate.maximum_disconnected_components = aggregate
        .maximum_disconnected_components
        .max(report.maximum_disconnected_components);
    aggregate.removed_degenerate_triangles += report.removed_degenerate_triangles;
    aggregate.generated_tangent_frames += report.generated_tangent_frames;
    aggregate.micro_beveled_meshes += report.micro_beveled_meshes;
    aggregate.preserved_sharp_meshes += report.preserved_sharp_meshes;
    aggregate.micro_surface_materials += report.micro_surface_materials;
    aggregate.neutral_fallback_materials += report.neutral_fallback_materials;
    aggregate.max_bevel_m = aggregate.max_bevel_m.max(report.max_bevel_m);
    aggregate.boundary_edges += report.boundary_edges;
    aggregate.nonmanifold_edges += report.nonmanifold_edges;
    aggregate.folded_edges += report.folded_edges;
    aggregate.self_intersections += report.self_intersections;
    aggregate.duplicate_vertices_removed += report.duplicate_vertices_removed;
    aggregate.minimum_texture_dimension = match (
        aggregate.minimum_texture_dimension,
        report.minimum_texture_dimension,
    ) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (left, right) => left.or(right),
    };
    for scope in report.inspection_scope {
        if !aggregate.inspection_scope.contains(&scope) {
            aggregate.inspection_scope.push(scope);
        }
    }
    aggregate.coherent_visible_subject &= report.coherent_visible_subject;
    aggregate.issues.extend(report.issues);
    aggregate.rejected_meshes.extend(report.rejected_meshes);
    for claim in report.substance_claims {
        if !aggregate.substance_claims.contains(&claim) {
            aggregate.substance_claims.push(claim);
        }
    }
}

pub(crate) fn camera_behavior_composition_plan(
    host: &scena::SceneHostCore,
    subject: &SubjectSelection,
    preserve_authored_camera: bool,
) -> Result<scena::PhotoCandidatePlanV1, CliFailure> {
    let subject_bounds = host
        .nodes_world_bounds(&subject.root_handles)
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

fn configure_camera_behavior_renderer(
    host: &mut scena::SceneHostCore,
    quality: scena::SceneRecipePhotoQualityV1,
    optimize: bool,
) -> Result<(), CliFailure> {
    let renderer = host.renderer_mut();
    // Final stills shade full-frame samples and resolve once. Preview keeps the
    // cheaper edge-only sampling path for compatibility.
    if quality.is_final() {
        renderer.set_anti_aliasing(scena::AntiAliasing::None);
        if camera_behavior_baked_ambient_visibility_enabled(quality) {
            renderer.set_baked_ambient_occlusion(Some(
                scena::BakedAmbientOcclusionConfig::product_still(),
            ));
        } else {
            renderer.clear_baked_ambient_occlusion();
        }
        if renderer.supersample_factor() < 2 {
            renderer
                .set_supersample_factor(2)
                .map_err(runtime_failure)?;
        }
        renderer.set_reconstruction_filter(scena::ReconstructionFilter::Tent);
    } else {
        renderer.set_anti_aliasing(scena::AntiAliasing::Msaa4);
        renderer.clear_baked_ambient_occlusion();
    }
    renderer.set_tonemapper(if quality.is_final() {
        scena::Tonemapper::PbrNeutral
    } else {
        scena::Tonemapper::Aces
    });
    if optimize && renderer.auto_exposure().is_none() && !renderer.has_explicit_exposure_ev() {
        renderer.set_auto_exposure(
            scena::AutoExposureConfig::new(0.20)
                .with_ev_range(-8.0, 8.0)
                .with_highlight_guard(0.92, 0.78),
        );
    } else if !optimize {
        renderer.clear_auto_exposure();
        renderer.set_exposure_ev(FINAL_PHOTO_BASE_EXPOSURE_EV);
    }
    renderer.set_bloom(Some(scena::PostBloomConfig::new(232, 0.04, 3)));
    if !camera_behavior_ssao_enabled(quality)
        || std::env::var_os("SCENA_DEBUG_DISABLE_SSAO").is_some()
    {
        renderer.set_screen_space_ambient_occlusion(None);
    } else if renderer.screen_space_ambient_occlusion().is_none() {
        renderer.set_screen_space_ambient_occlusion(Some(
            scena::ScreenSpaceAmbientOcclusionConfig::new(4, 0.32, 0.025),
        ));
    }
    Ok(())
}

const fn camera_behavior_ssao_enabled(quality: scena::SceneRecipePhotoQualityV1) -> bool {
    !quality.is_final()
}

const fn camera_behavior_baked_ambient_visibility_enabled(
    _quality: scena::SceneRecipePhotoQualityV1,
) -> bool {
    false
}

const fn automatic_reflection_probe_bake_enabled(
    quality: scena::SceneRecipePhotoQualityV1,
) -> bool {
    quality.is_final()
}

const fn synthetic_contact_shadow_enabled(quality: scena::SceneRecipePhotoQualityV1) -> bool {
    !quality.is_final()
}

fn select_camera_behavior_shaded_candidate(
    host: &mut scena::SceneHostCore,
    subject: &SubjectSelection,
    planning: &scena::PhotoCandidatePlanV1,
    authored_lights: bool,
    surface_report: scena::PhotographicSurfaceReportV1,
    surroundings_report: scena::PhotographicSurroundingsReportV1,
    gpu: bool,
    optimize: bool,
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
        surroundings_report,
        gpu,
        optimize,
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
    surroundings_report: scena::PhotographicSurroundingsReportV1,
    gpu: bool,
    optimize: bool,
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
        host.frame_nodes_with_photo_candidate(&subject.root_handles, candidate)
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
        if optimize && let Some(adjustment) = corrected_photographic_lighting(metrics) {
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
        surroundings_report,
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
        lighting_report: None,
        reflection_probe_report: None,
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

fn photo_source(
    args: &PhotoRenderArgs,
) -> Result<(PhotoSource, u32, u32, &'static str), CliFailure> {
    photo_source_for(
        &args.input,
        args.capture_explicit.then_some((args.width, args.height)),
    )
}

/// Resolves the capture size and reports where it came from.
///
/// Precedence is explicit flags, then the recipe's own `capture` block, then
/// this command's default. Previously any recipe input had its `capture`
/// overwritten unconditionally, so `scena photo render` silently rendered a
/// different size than `scena recipe render` did for the same file.
fn photo_source_for(
    input: &Path,
    requested: Option<(u32, u32)>,
) -> Result<(PhotoSource, u32, u32, &'static str), CliFailure> {
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
        let quality = recipe_photo_quality(&text, input)?;
        let ground = recipe_photo_ground(&text, input)?;
        let declared = recipe_declared_capture(&text, input)?;
        let (width, height, capture_source) = resolve_photo_capture(quality, requested, declared);
        let text = recipe_text_with_capture_override(text, input, quality, width, height)?;
        return Ok((
            PhotoSource {
                recipe_text: text,
                recipe_path: input.display().to_string(),
                source_kind: "recipe",
                quality,
                ground,
            },
            width,
            height,
            capture_source,
        ));
    }
    let quality = scena::SceneRecipePhotoQualityV1::Preview;
    let (width, height, capture_source) = resolve_photo_capture(quality, requested, None);

    let recipe_path = std::env::current_dir()
        .map(|cwd| cwd.join("scena-photo.generated.recipe.json"))
        .unwrap_or_else(|_| PathBuf::from("scena-photo.generated.recipe.json"));
    let source = PhotoSource {
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
        quality,
        ground: scena::scene_host::PhotographicGroundV1::Matte,
    };
    Ok((source, width, height, capture_source))
}

fn recipe_photo_ground(
    text: &str,
    input: &Path,
) -> Result<scena::scene_host::PhotographicGroundV1, CliFailure> {
    let value: Value = serde_json::from_str(text).map_err(|error| {
        CliFailure::new(
            CliErrorKind::InvalidInput,
            format!("recipe '{}' is not valid JSON: {error}", input.display()),
        )
    })?;
    Ok(photo_ground_from_staging(
        value
            .pointer("/photo/staging/ground")
            .and_then(Value::as_str),
    ))
}

pub(crate) fn photo_ground_from_staging(
    ground: Option<&str>,
) -> scena::scene_host::PhotographicGroundV1 {
    match ground {
        Some("reflective") => scena::scene_host::PhotographicGroundV1::Reflective,
        _ => scena::scene_host::PhotographicGroundV1::Matte,
    }
}

fn resolve_photo_capture(
    quality: scena::SceneRecipePhotoQualityV1,
    requested: Option<(u32, u32)>,
    declared: Option<(u32, u32)>,
) -> (u32, u32, &'static str) {
    match (requested, declared, quality) {
        (Some((width, height)), _, _) => (width, height, "cli_flag"),
        (None, Some((width, height)), _) => (width, height, "recipe_capture"),
        (None, None, scena::SceneRecipePhotoQualityV1::Final) => (
            DEFAULT_FINAL_PHOTO_WIDTH,
            DEFAULT_FINAL_PHOTO_HEIGHT,
            "final_photo_default",
        ),
        (None, None, scena::SceneRecipePhotoQualityV1::Preview) => {
            (DEFAULT_PHOTO_WIDTH, DEFAULT_PHOTO_HEIGHT, "photo_default")
        }
    }
}

fn recipe_photo_quality(
    text: &str,
    input: &Path,
) -> Result<scena::SceneRecipePhotoQualityV1, CliFailure> {
    let value: Value = serde_json::from_str(text).map_err(|error| {
        CliFailure::new(
            CliErrorKind::InvalidInput,
            format!("recipe '{}' is not valid JSON: {error}", input.display()),
        )
    })?;
    Ok(
        match value.pointer("/photo/quality").and_then(Value::as_str) {
            Some("final") => scena::SceneRecipePhotoQualityV1::Final,
            _ => scena::SceneRecipePhotoQualityV1::Preview,
        },
    )
}

pub(crate) fn normalize_recipe_photo_defaults(
    text: String,
    input: &Path,
) -> Result<(String, scena::SceneRecipePhotoQualityV1), CliFailure> {
    let quality = recipe_photo_quality(&text, input)?;
    if quality.is_preview() {
        return Ok((text, quality));
    }

    let declared = recipe_declared_capture(&text, input)?;
    let (width, height, _) = resolve_photo_capture(quality, None, declared);
    let text = recipe_text_with_capture_override(text, input, quality, width, height)?;
    Ok((text, quality))
}

pub(crate) fn ensure_final_photo_backend(
    quality: scena::SceneRecipePhotoQualityV1,
    backend: scena::Backend,
) -> Result<(), CliFailure> {
    if !quality.is_final()
        || matches!(
            backend,
            scena::Backend::HeadlessGpu | scena::Backend::NativeSurface
        )
    {
        return Ok(());
    }
    Err(CliFailure::new(
        CliErrorKind::FinalPhotoUnsupported,
        format!(
            "final_photo_unsupported: backend {backend:?} cannot provide the complete native final-photo contract; use HeadlessGpu or NativeSurface"
        ),
    ))
}

struct PhotoQualityExecutionInput<'a> {
    quality: scena::SceneRecipePhotoQualityV1,
    backend: scena::Backend,
    evidence_class: &'a str,
    capture: [u32; 2],
    supersample_factor: u32,
    reconstruction: &'a str,
    anti_aliasing: &'a str,
    environment_source_dimensions: Option<[u32; 2]>,
    environment_cubemap_resolution: Option<u32>,
    reflection_probe_count: usize,
    shadow_mode: &'a str,
    tonemapper: &'a str,
    edge_rounding: Vec<scena::SceneRecipeImportEdgeRoundingReportV1>,
    material_resolution_selection: Option<scena::PhotographicMaterialResolutionSelectionReportV1>,
}

fn photo_quality_execution_json(input: PhotoQualityExecutionInput<'_>) -> Value {
    let quality = match input.quality {
        scena::SceneRecipePhotoQualityV1::Preview => "preview",
        scena::SceneRecipePhotoQualityV1::Final => "final",
    };
    let [width, height] = input.capture;
    json!({
        "schema": scena::PHOTO_QUALITY_EXECUTION_SCHEMA_V1,
        "requested": quality,
        "effective": quality,
        "backend": input.backend,
        "evidence_class": input.evidence_class,
        "capture": {
            "width": width,
            "height": height,
            "pixel_count": u64::from(width).saturating_mul(u64::from(height)),
        },
        "sampling": {
            "supersample_factor": input.supersample_factor,
            "reconstruction": input.reconstruction,
            "anti_aliasing": input.anti_aliasing,
        },
        "environment": {
            "source_dimensions": input.environment_source_dimensions,
            "cubemap_resolution": input.environment_cubemap_resolution,
        },
        "reflections": {
            "local_probe_count": input.reflection_probe_count,
        },
        "shadows": {
            "mode": input.shadow_mode,
        },
        "color": {
            "linear_scene_format": "rgba16_float",
            "tonemapper": input.tonemapper,
            "output_encoding": "srgb8_png",
        },
        "geometry": {
            "edge_rounding": input.edge_rounding,
        },
        "materials": {
            "resolution_selection": input.material_resolution_selection,
        },
    })
}

fn classify_photo_evidence(
    backend: scena::Backend,
    adapter_name: Option<&str>,
    adapter_driver: Option<&str>,
    adapter_driver_info: Option<&str>,
) -> &'static str {
    if matches!(backend, scena::Backend::WebGpu | scena::Backend::WebGl2) {
        return "browser_conformance";
    }
    let adapter = format!(
        "{} {} {}",
        adapter_name.unwrap_or_default(),
        adapter_driver.unwrap_or_default(),
        adapter_driver_info.unwrap_or_default()
    )
    .to_ascii_lowercase();
    if adapter.contains("v3d") || adapter.contains("v3dv") {
        return "v3d_diagnostic";
    }
    if adapter.contains("lavapipe")
        || adapter.contains("llvmpipe")
        || adapter.contains("swiftshader")
        || adapter.contains("software raster")
    {
        return "software_conformance";
    }
    if matches!(
        backend,
        scena::Backend::HeadlessGpu | scena::Backend::NativeSurface
    ) {
        return "supported_hardware";
    }
    "cpu_preview"
}

/// The `capture` block a recipe declares for itself, if any.
fn recipe_declared_capture(text: &str, input: &Path) -> Result<Option<(u32, u32)>, CliFailure> {
    let value: Value = serde_json::from_str(text).map_err(|error| {
        CliFailure::new(
            CliErrorKind::InvalidInput,
            format!("recipe '{}' is not valid JSON: {error}", input.display()),
        )
    })?;
    let Some(capture) = value.get("capture") else {
        return Ok(None);
    };
    let width = capture.get("width").and_then(Value::as_u64);
    let height = capture.get("height").and_then(Value::as_u64);
    match (width, height) {
        (Some(width), Some(height)) => Ok(Some((width as u32, height as u32))),
        _ => Ok(None),
    }
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
    quality: scena::SceneRecipePhotoQualityV1,
    width: u32,
    height: u32,
) -> Result<String, CliFailure> {
    let mut value: Value = serde_json::from_str(&text).map_err(|error| {
        CliFailure::new(
            CliErrorKind::InvalidInput,
            format!("recipe '{}' is not valid JSON: {error}", input.display()),
        )
    })?;
    apply_photo_quality_defaults(&mut value, quality, width, height).map_err(|error| {
        CliFailure::new(
            CliErrorKind::InvalidInput,
            format!("recipe '{}': {error}", input.display()),
        )
    })?;
    serde_json::to_string_pretty(&value)
        .map_err(|error| CliFailure::new(CliErrorKind::Internal, error.to_string()))
}

fn apply_photo_quality_defaults(
    value: &mut Value,
    quality: scena::SceneRecipePhotoQualityV1,
    width: u32,
    height: u32,
) -> Result<(), &'static str> {
    let Some(object) = value.as_object_mut() else {
        return Err("must be a JSON object");
    };
    object.insert(
        "capture".to_owned(),
        json!({
            "width": width,
            "height": height,
        }),
    );
    if quality.is_preview() {
        return Ok(());
    }
    if let Some(imports) = object.get_mut("imports").and_then(Value::as_array_mut) {
        for import in imports.iter_mut().filter_map(Value::as_object_mut) {
            import.entry("edge_rounding").or_insert_with(|| {
                json!({
                    "enabled": true,
                    "radius_fraction": 0.0025,
                    "segments": 3,
                    "edge_angle_threshold_degrees": 30.0,
                    "max_derived_triangles": 250000,
                })
            });
        }
    }
    let render = object
        .entry("render")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or("final photo render settings must be an object")?;
    render
        .entry("anti_aliasing")
        .or_insert_with(|| json!("none"));
    render.entry("supersample").or_insert_with(|| json!(2));
    render
        .entry("reconstruction")
        .or_insert_with(|| json!("tent"));
    render
        .entry("tonemapper")
        .or_insert_with(|| json!("pbr_neutral"));
    Ok(())
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
        let mut capture_explicit = false;
        let mut width = DEFAULT_PHOTO_WIDTH;
        let mut height = DEFAULT_PHOTO_HEIGHT;
        let mut gpu = false;
        let mut optimize = false;
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
                    capture_explicit = true;
                    width = parse_dimension("--width", flag_value(args, index, "--width")?)?;
                    index += 2;
                }
                "--height" => {
                    capture_explicit = true;
                    height = parse_dimension("--height", flag_value(args, index, "--height")?)?;
                    index += 2;
                }
                "--gpu" => {
                    gpu = true;
                    index += 1;
                }
                "--optimize" => {
                    optimize = true;
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
            capture_explicit,
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
            optimize,
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
        let mut capture_explicit = false;
        let mut width = DEFAULT_PHOTO_WIDTH;
        let mut height = DEFAULT_PHOTO_HEIGHT;
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
                    capture_explicit = true;
                    width = parse_dimension("--width", flag_value(args, index, "--width")?)?;
                    index += 2;
                }
                "--height" => {
                    capture_explicit = true;
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
            capture_explicit,
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
    "usage: scena photo plan <asset-or-recipe> [--intent camera-behavior] --out <plan.json> [--subject import:<id>|node:<id>] [--width <px>] [--height <px>] [--max-imports <n>] [--allow-root <directory>]...; scena photo render <asset-or-recipe> [--intent camera-behavior] --out <png> --report <json> [--emit-recipe <recipe.json>] [--subject import:<id>|node:<id>] [--width <px>] [--height <px>] [--gpu] [--optimize] [--max-imports <n>] [--allow-root <directory>]..."
}

fn photo_render_usage() -> &'static str {
    "usage: scena photo render <asset-or-recipe> [--intent camera-behavior] --out <png> --report <json> [--emit-recipe <recipe.json>] [--subject import:<id>|node:<id>] [--width <px>] [--height <px>] [--gpu] [--optimize] [--max-imports <n>] [--allow-root <directory>]..."
}

fn photo_plan_usage() -> &'static str {
    "usage: scena photo plan <asset-or-recipe> [--intent camera-behavior] --out <plan.json> [--subject import:<id>|node:<id>] [--width <px>] [--height <px>] [--max-imports <n>] [--allow-root <directory>]..."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multi_root_photo_subject_applies_surface_solver_to_every_import_root() {
        let repository_root = std::path::Path::new(".")
            .canonicalize()
            .expect("repository root canonicalizes");
        let recipe = json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [{
                "id": "machine",
                "uri": "demo/samples/connector-snap/connector_snap_assembly.glb"
            }],
            "scene": {},
            "render": {}
        });
        let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
            "hero-multi-root-surface.recipe.json",
            &serde_json::to_string_pretty(&recipe).expect("focused recipe serializes"),
            scena::RecipeBuildPolicy::testing().with_allowed_root(repository_root),
        ))
        .expect("frozen hero import builds");
        let subject = select_camera_behavior_subject(&build.manifest, None)
            .expect("hero import resolves as the photo subject");
        assert_eq!(
            subject.root_handles.len(),
            2,
            "the frozen hero keeps its load-unit and drive-unit scene roots"
        );
        let mut host = build.host;

        let report = apply_subject_photographic_surface(&mut host, &subject)
            .expect("photo surface solver completes");

        assert_eq!(
            report.mesh_count, 78,
            "the general photo path must process all 35 load-unit and 43 drive-unit meshes"
        );
    }

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

    #[test]
    fn final_photo_defaults_and_backend_support_are_fail_closed() {
        assert_eq!(
            resolve_photo_capture(scena::SceneRecipePhotoQualityV1::Final, None, None),
            (3840, 2520, "final_photo_default")
        );
        assert_eq!(
            resolve_photo_capture(scena::SceneRecipePhotoQualityV1::Preview, None, None),
            (DEFAULT_PHOTO_WIDTH, DEFAULT_PHOTO_HEIGHT, "photo_default")
        );

        let mut recipe = json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [{
                "id": "subject",
                "uri": "subject.glb"
            }],
            "photo": {
                "intent": "camera_behavior",
                "quality": "final"
            }
        });
        apply_photo_quality_defaults(
            &mut recipe,
            scena::SceneRecipePhotoQualityV1::Final,
            3840,
            2520,
        )
        .expect("final defaults apply");
        assert_eq!(recipe["capture"], json!({ "width": 3840, "height": 2520 }));
        assert_eq!(recipe["render"]["anti_aliasing"], "none");
        assert_eq!(recipe["render"]["supersample"], 2);
        assert_eq!(recipe["render"]["reconstruction"], "tent");
        assert_eq!(recipe["render"]["tonemapper"], "pbr_neutral");
        assert_eq!(
            recipe["imports"][0]["edge_rounding"],
            json!({
                "enabled": true,
                "radius_fraction": 0.0025,
                "segments": 3,
                "edge_angle_threshold_degrees": 30.0,
                "max_derived_triangles": 250000
            })
        );

        let mut preview = json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [{ "id": "subject", "uri": "subject.glb" }]
        });
        apply_photo_quality_defaults(
            &mut preview,
            scena::SceneRecipePhotoQualityV1::Preview,
            1280,
            840,
        )
        .expect("preview defaults apply");
        assert!(preview["imports"][0].get("edge_rounding").is_none());
        let execution = photo_quality_execution_json(PhotoQualityExecutionInput {
            quality: scena::SceneRecipePhotoQualityV1::Final,
            backend: scena::Backend::HeadlessGpu,
            evidence_class: "software_conformance",
            capture: [3840, 2520],
            supersample_factor: 2,
            reconstruction: "tent",
            anti_aliasing: "none",
            environment_source_dimensions: Some([1024, 512]),
            environment_cubemap_resolution: Some(512),
            reflection_probe_count: 3,
            shadow_mode: "weighted_area_visibility",
            tonemapper: "pbr_neutral",
            edge_rounding: vec![scena::SceneRecipeImportEdgeRoundingReportV1 {
                enabled: true,
                inspected_meshes: 4,
                rounded_meshes: 3,
                skipped_meshes: 1,
                eligible_edges: 24,
                rounded_edges: 24,
                skipped_edges: 8,
                rejected_edges: 0,
                removed_degenerate_triangles: 2,
                source_triangles: 120,
                derived_triangles: 288,
            }],
            material_resolution_selection: None,
        });
        assert_eq!(execution["schema"], "scena.photo_quality_execution.v1");
        assert_eq!(execution["requested"], "final");
        assert_eq!(execution["effective"], "final");
        assert_eq!(execution["capture"]["pixel_count"], 9_676_800_u64);
        assert_eq!(execution["sampling"]["supersample_factor"], 2);
        assert_eq!(execution["sampling"]["reconstruction"], "tent");
        assert_eq!(execution["sampling"]["anti_aliasing"], "none");
        assert_eq!(
            execution["environment"]["source_dimensions"],
            json!([1024, 512])
        );
        assert_eq!(execution["environment"]["cubemap_resolution"], 512);
        assert_eq!(execution["reflections"]["local_probe_count"], 3);
        assert_eq!(execution["shadows"]["mode"], "weighted_area_visibility");
        assert_eq!(execution["color"]["linear_scene_format"], "rgba16_float");
        assert_eq!(execution["color"]["tonemapper"], "pbr_neutral");
        assert_eq!(execution["color"]["output_encoding"], "srgb8_png");
        assert_eq!(
            execution["geometry"]["edge_rounding"][0]["rounded_edges"],
            24
        );
        assert_eq!(execution["backend"], "headless_gpu");
        assert_eq!(execution["evidence_class"], "software_conformance");
        assert_eq!(
            classify_photo_evidence(
                scena::Backend::HeadlessGpu,
                Some("llvmpipe (LLVM 19)"),
                Some("lavapipe"),
                Some("Mesa")
            ),
            "software_conformance"
        );
        assert_eq!(
            classify_photo_evidence(
                scena::Backend::HeadlessGpu,
                Some("V3D 7.1.10.2"),
                Some("V3DV"),
                Some("Mesa")
            ),
            "v3d_diagnostic"
        );
        assert_eq!(
            classify_photo_evidence(
                scena::Backend::HeadlessGpu,
                Some("NVIDIA RTX"),
                Some("NVIDIA"),
                Some("Vulkan")
            ),
            "supported_hardware"
        );
        assert_eq!(
            classify_photo_evidence(scena::Backend::WebGl2, None, None, None),
            "browser_conformance"
        );

        assert!(
            ensure_final_photo_backend(
                scena::SceneRecipePhotoQualityV1::Final,
                scena::Backend::HeadlessGpu
            )
            .is_ok()
        );
        assert!(
            ensure_final_photo_backend(
                scena::SceneRecipePhotoQualityV1::Final,
                scena::Backend::NativeSurface
            )
            .is_ok()
        );
        for backend in [
            scena::Backend::Headless,
            scena::Backend::SurfaceDescriptor,
            scena::Backend::WebGpu,
            scena::Backend::WebGl2,
        ] {
            let error =
                ensure_final_photo_backend(scena::SceneRecipePhotoQualityV1::Final, backend)
                    .expect_err("unsupported final-photo backend must fail closed");
            assert_eq!(error.kind, CliErrorKind::FinalPhotoUnsupported);
            assert!(
                error.message.contains("final_photo_unsupported"),
                "{error:?}"
            );
        }
        assert!(
            ensure_final_photo_backend(
                scena::SceneRecipePhotoQualityV1::Preview,
                scena::Backend::Headless
            )
            .is_ok(),
            "preview remains compatible with the CPU renderer"
        );
    }

    #[test]
    fn final_photo_color_and_dark_material_contract() {
        let mut final_host = scena::SceneHostCore::headless(64, 64).expect("final host builds");
        configure_camera_behavior_renderer(
            &mut final_host,
            scena::SceneRecipePhotoQualityV1::Final,
            false,
        )
        .expect("final renderer configures");
        assert_eq!(
            final_host.renderer().tonemapper(),
            scena::Tonemapper::PbrNeutral,
            "final photography must use the existing Khronos PBR Neutral output transform"
        );

        let mut preview_host = scena::SceneHostCore::headless(64, 64).expect("preview host builds");
        configure_camera_behavior_renderer(
            &mut preview_host,
            scena::SceneRecipePhotoQualityV1::Preview,
            false,
        )
        .expect("preview renderer configures");
        assert_eq!(
            preview_host.renderer().tonemapper(),
            scena::Tonemapper::Aces,
            "the preview compatibility path remains unchanged"
        );

        let mut readable_dark_material = sane_camera_subject();
        readable_dark_material.mean_luminance_srgb8 = 35.0;
        readable_dark_material.low_clip_fraction = 0.08;
        readable_dark_material.high_clip_fraction = 0.0;
        readable_dark_material.luminance_stddev_srgb8 = 34.0;
        readable_dark_material.luminance_range_srgb8 = 180.0;
        assert!(
            camera_behavior_failure_codes(readable_dark_material).is_empty(),
            "a structured dark material in the 20-45 sRGB range must remain intentionally dark"
        );
        assert!(
            corrected_exposure_ev(0.0, readable_dark_material).is_none(),
            "exposure correction must not lift a readable dark material out of its 20-45 sRGB range"
        );

        let mut unreadable_dark_fixture = readable_dark_material;
        unreadable_dark_fixture.mean_luminance_srgb8 = 18.0;
        unreadable_dark_fixture.dark_material_mean_luminance_srgb8 = Some(18.0);
        assert!(
            corrected_exposure_ev(0.0, unreadable_dark_fixture).is_some(),
            "a dark material below 20 sRGB with highlight headroom must still request correction"
        );

        let material_luminance = BTreeMap::from([
            (10, (5.0 * 650.0, 650)),
            (20, (210.0 * 300.0, 300)),
            (30, (95.0 * 50.0, 50)),
        ]);
        let (dark_mean, dark_coverage) = select_dark_material_region(&material_luminance, 1_000);
        assert_eq!(dark_mean, Some(5.0));
        assert!((dark_coverage - 0.65).abs() <= f64::EPSILON);

        let mut chrome_biased_dark_body = sane_camera_subject();
        chrome_biased_dark_body.mean_luminance_srgb8 = 83.0;
        chrome_biased_dark_body.dark_material_mean_luminance_srgb8 = Some(5.0);
        chrome_biased_dark_body.dark_material_coverage = 0.65;
        chrome_biased_dark_body.luminance_stddev_srgb8 = 1.0;
        chrome_biased_dark_body.luminance_range_srgb8 = 8.0;
        chrome_biased_dark_body.background_separation_srgb8 = 3.0;
        chrome_biased_dark_body.low_clip_fraction = 0.58;
        chrome_biased_dark_body.high_clip_fraction = 0.0;
        chrome_biased_dark_body.highlight_continuity = 0.01;
        chrome_biased_dark_body.highlight_distribution = 0.0;
        chrome_biased_dark_body.reflection_washout = 0.45;
        chrome_biased_dark_body.shadow_presence = 0.0;
        chrome_biased_dark_body.silhouette_separation = 0.02;
        let dark_body_lighting = corrected_photographic_lighting(chrome_biased_dark_body)
            .expect("a chrome-biased meter must still request dark-body lighting");
        assert!(
            dark_body_lighting.fill_scale >= 3.0
                && dark_body_lighting.overhead_scale >= 1.4
                && dark_body_lighting.rim_scale >= 1.5
                && dark_body_lighting.environment_intensity_scale >= 1.5
                && dark_body_lighting.key_scale <= 0.85,
            "dark-body correction must open the material without using a harder key: \
             {dark_body_lighting:#?}"
        );
        chrome_biased_dark_body.high_clip_fraction = 0.0;
        let dark_body_exposure = corrected_exposure_ev(0.0, chrome_biased_dark_body)
            .expect("a sub-20 dark material with clean highlights needs bounded exposure support");
        assert!(
            (0.25..=0.50).contains(&dark_body_exposure),
            "dark-body exposure support must be conservative enough to retain chrome headroom, \
             got {dark_body_exposure}"
        );
        let mut highlight_limited_dark_body = chrome_biased_dark_body;
        highlight_limited_dark_body.high_clip_fraction = 0.0299;
        let highlight_limited_lighting =
            corrected_photographic_lighting(highlight_limited_dark_body)
                .expect("highlight-limited dark body still needs lighting correction");
        assert!(
            highlight_limited_lighting.fill_scale >= 3.0
                && highlight_limited_lighting.overhead_scale >= 1.4
                && highlight_limited_lighting.rim_scale >= 1.5
                && highlight_limited_lighting.key_scale <= 0.85,
            "dark readability must preserve broad fill under highlight-limited exposure: \
             {highlight_limited_lighting:#?}"
        );
        assert!(
            corrected_exposure_ev(-0.096, highlight_limited_dark_body).is_none(),
            "exposure must hold while the bounded lighting retry opens an unreadable dark body"
        );

        let mut missing_material_metric = chrome_biased_dark_body;
        missing_material_metric.mean_luminance_srgb8 = 83.0;
        missing_material_metric.dark_material_mean_luminance_srgb8 = None;
        missing_material_metric.dark_material_coverage = 0.0;
        missing_material_metric.low_clip_fraction = 0.58;
        missing_material_metric.high_clip_fraction = 0.003;
        assert!(
            !dark_product_is_readable(missing_material_metric),
            "a large crushed region must fail closed when material metering is unavailable"
        );
        assert!(
            unreadable_dark_material(missing_material_metric),
            "the bounded lighting retry must remain available when a large crushed region lacks material identity"
        );
        assert_eq!(FINAL_DARK_MATERIAL_LIGHTING_MAX_RETRIES, 1);
        assert!(
            should_retry_final_dark_material_lighting(missing_material_metric),
            "highlight-limited final output with an unreadable dark region needs one lighting retry"
        );

        let valve: Value = serde_json::from_slice(
            &std::fs::read("tests/assets/photo/final/recipes/valve_manifold.recipe.json")
                .expect("valve final recipe reads"),
        )
        .expect("valve final recipe parses");
        let red_wheel_control = valve["expect"]["expect_color"]
            .as_array()
            .expect("valve must carry a rendered red-wheel control")
            .iter()
            .find(|control| control["id"] == "valve-wheel-red-dominance")
            .expect("valve red-wheel dominance control exists");
        assert_eq!(red_wheel_control["color_family"], "red");
        assert_eq!(
            red_wheel_control["target"],
            json!({
                "kind": "node",
                "id": "valve_hub"
            })
        );
        assert!(
            red_wheel_control.get("swatch_srgb8").is_none(),
            "the control checks diffuse red dominance without rejecting permitted white specular peaks"
        );
        assert!(
            !pixel_has_output_channel_clip(&[252, 252, 252, 255]),
            "bright rolled-off specular structure must not be mislabeled as output clipping"
        );
        assert!(
            pixel_has_output_channel_clip(&[255, 240, 220, 255]),
            "an RGB channel at display maximum must count as output clipping"
        );
    }

    #[test]
    fn default_photo_correction_is_one_shot_bounded_and_fail_closed() {
        let args = PhotoRenderArgs::parse(&[
            "fixture.glb".to_owned(),
            "--out".to_owned(),
            "out.png".to_owned(),
            "--report".to_owned(),
            "out.json".to_owned(),
        ])
        .expect("default photo arguments parse");
        assert!(
            !args.optimize,
            "the easy path must not enter the iterative optimizer"
        );

        let optimized = PhotoRenderArgs::parse(&[
            "fixture.glb".to_owned(),
            "--out".to_owned(),
            "out.png".to_owned(),
            "--report".to_owned(),
            "out.json".to_owned(),
            "--optimize".to_owned(),
        ])
        .expect("explicit optimizer arguments parse");
        assert!(optimized.optimize, "optimizer use must be explicit");
        assert!(
            use_builtin_studio_lighting(false, false),
            "the zero-config default must use Scena's stable built-in studio rig"
        );
        assert!(
            !use_builtin_studio_lighting(true, false) && !use_builtin_studio_lighting(false, true),
            "authored lighting and explicit optimization keep their selected lighting paths"
        );

        let mut clipped = sane_camera_subject();
        clipped.high_clip_fraction = 0.12;
        let base_ev = FINAL_PHOTO_BASE_EXPOSURE_EV;
        let next_ev = bounded_default_exposure_ev(base_ev, clipped)
            .expect("a severely clipped subject requests one correction");
        assert!(
            (next_ev - (base_ev - DEFAULT_PHOTO_MAX_EXPOSURE_CORRECTION_EV)).abs() < 1.0e-6,
            "the default correction must stop at one -0.75 EV step: base={base_ev} next={next_ev}"
        );

        let mut goodhart_frame = sane_camera_subject();
        goodhart_frame.mean_luminance_srgb8 = 59.55;
        goodhart_frame.dark_material_mean_luminance_srgb8 = Some(21.53);
        goodhart_frame.dark_material_coverage = 0.0771;
        assert!(
            dark_product_is_readable(goodhart_frame),
            "the focused dark-material diagnostic remains useful"
        );
        assert!(
            camera_behavior_failure_codes(goodhart_frame).contains(&"subject_luminance_below_min"),
            "a readable dark patch must not rubber-stamp an underexposed complete subject"
        );
        let corrected_goodhart = bounded_default_exposure_ev(base_ev, goodhart_frame)
            .expect("the one-shot default must brighten an underexposed complete subject");
        assert!(
            corrected_goodhart > base_ev,
            "the dark-patch diagnostic must not suppress whole-subject exposure correction"
        );
    }

    #[test]
    fn final_photo_ground_intent_and_planar_composite_contract() {
        let (mug, _, _, _) = photo_source_for(
            Path::new("tests/assets/photo/final/recipes/colored_travel_mug.recipe.json"),
            None,
        )
        .expect("mug final source resolves");
        let (hero, _, _, _) = photo_source_for(
            Path::new("tests/assets/photo/final/recipes/demo_hero.recipe.json"),
            None,
        )
        .expect("hero final source resolves");
        assert_eq!(
            mug.ground,
            scena::scene_host::PhotographicGroundV1::Reflective
        );
        assert_eq!(hero.ground, scena::scene_host::PhotographicGroundV1::Matte);

        let aov = scena::SceneHostSemanticAovCaptureV1 {
            schema: scena::SCENE_HOST_SEMANTIC_AOV_SCHEMA_V1.to_owned(),
            width: 3,
            height: 1,
            identity_scope: "node_material".to_owned(),
            sample_pattern: "pixel_center".to_owned(),
            depth_convention: "linear_camera_distance_scene_meters".to_owned(),
            normal_space: "world".to_owned(),
            near: 0.1,
            far: 100.0,
            id_indices: vec![1, 2, 1],
            beauty_id_indices: None,
            depth_meters: vec![2.0; 3],
            world_normals: vec![[0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            legend: vec![
                scena::SceneHostSemanticAovLegendEntryV1 {
                    palette_index: 1,
                    rgba8: scena::scene_host::palette_rgba8(1),
                    node_handle: 42,
                    material_handle: None,
                    material_kind: None,
                    metallic_factor: None,
                    roughness_factor: None,
                    effective_metallic_mean: None,
                    effective_roughness_mean: None,
                    surface_texture_min_dimension_px: None,
                    surface_tile_size_m: None,
                    instance_handle: None,
                    instance_id: None,
                },
                scena::SceneHostSemanticAovLegendEntryV1 {
                    palette_index: 2,
                    rgba8: scena::scene_host::palette_rgba8(2),
                    node_handle: 7,
                    material_handle: None,
                    material_kind: None,
                    metallic_factor: None,
                    roughness_factor: None,
                    effective_metallic_mean: None,
                    effective_roughness_mean: None,
                    surface_texture_min_dimension_px: None,
                    surface_tile_size_m: None,
                    instance_handle: None,
                    instance_id: None,
                },
            ],
            exclusions: scena::SceneHostSemanticAovExclusionsV1::default(),
        };
        let mask = planar_reflection_floor_mask(&aov, &[42]);
        assert_eq!(mask, vec![true, false, false]);
        let mut beauty = vec![40, 40, 40, 255, 80, 80, 80, 255, 120, 120, 120, 255];
        let reflected = [200, 160, 120, 255].repeat(3);
        composite_planar_reflection_rgba8(&mut beauty, &reflected, 3, 1, &mask, 0.0, 0.5);
        assert_ne!(&beauty[0..4], &[40, 40, 40, 255]);
        assert_eq!(&beauty[4..8], &[80, 80, 80, 255]);
        assert_eq!(&beauty[8..12], &[120, 120, 120, 255]);
    }

    #[test]
    fn live_speaker_semantic_material_probe_clears_sample_floor() {
        let recipe_path = "tests/assets/photo/final/recipes/dark_metal_speaker.recipe.json";
        let repository_root = std::path::Path::new(".")
            .canonicalize()
            .expect("repository root canonicalizes");
        let material_root = repository_root.join("target/photo-materials");
        let recipe_text = std::fs::read_to_string(recipe_path)
            .expect("speaker recipe reads")
            .replace(
                "../../../../../target/photo-materials",
                material_root
                    .to_str()
                    .expect("material fixture path is UTF-8"),
            );
        let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
            recipe_path,
            &recipe_text,
            scena::RecipeBuildPolicy::testing().with_allowed_root(repository_root),
        ))
        .expect("speaker recipe builds on the CPU diagnostic path");
        let subject = select_camera_behavior_subject(&build.manifest, None)
            .expect("speaker import resolves as the photo subject");
        let mut host = build.host;
        let planning = camera_behavior_composition_plan(&host, &subject, false)
            .expect("speaker camera candidates plan");
        host.resize(80.0, 53.0, 1.0)
            .expect("speaker diagnostic viewport resizes");
        host.frame_nodes_with_photo_candidate(&subject.root_handles, &planning.candidates[0])
            .expect("first speaker candidate frames");
        host.prepare().expect("speaker diagnostic prepares");
        let semantic_aov = host
            .capture_semantic_aovs()
            .expect("speaker semantic AOV captures on CPU");
        let handles = subject_handles(&subject)
            .into_iter()
            .collect::<BTreeSet<_>>();
        let material_by_palette = semantic_aov
            .legend
            .iter()
            .filter(|entry| {
                handles.contains(&entry.node_handle)
                    || entry
                        .instance_handle
                        .is_some_and(|handle| handles.contains(&handle))
            })
            .filter_map(|entry| {
                entry
                    .material_handle
                    .map(|material| (entry.palette_index, material))
            })
            .collect::<BTreeMap<_, _>>();
        let mut material_samples = BTreeMap::<u64, u64>::new();
        for palette in &semantic_aov.id_indices {
            if let Some(material) = material_by_palette.get(palette) {
                *material_samples.entry(*material).or_default() += 1;
            }
        }
        let subject_samples = material_samples.values().sum::<u64>();
        let minimum_samples = (subject_samples / 20).max(32);
        eprintln!(
            "speaker live semantic material probe: subject_samples={subject_samples}, \
             minimum_samples={minimum_samples}, material_samples={material_samples:?}"
        );
        assert!(
            !material_by_palette.is_empty(),
            "the live speaker semantic legend must carry material handles"
        );
        assert!(
            material_samples
                .values()
                .any(|samples| *samples >= minimum_samples),
            "at least one live speaker material must clear the dark-region sample floor"
        );
    }

    #[test]
    fn final_photo_policy_blocks_only_uncalibrated_same_pass_grounding() {
        let mut grounded = json!({
            "schema": scena::PHOTO_QUALITY_ANALYSIS_SCHEMA_V1,
            "mode": "report_only",
            "grounding": {
                "boundary_sample_count": 887,
                "contact_shadow_delta_mean_srgb8": 15.407,
                "attached_fraction": 0.883,
                "contact_shadow_confirmed": true
            }
        });
        let mut grounded_failures = vec!["contact_shadow_missing"];
        apply_final_photo_quality_policy(
            scena::SceneRecipePhotoQualityV1::Final,
            &mut grounded,
            &mut grounded_failures,
        )
        .expect("tracked final-photo policy applies");
        assert!(grounded_failures.is_empty());
        assert_eq!(grounded["mode"], "selective_blocking");
        assert_eq!(
            grounded["policy"]["checks"][0]["status"], "checked",
            "the weakest current four-subject positive must pass the admitted threshold"
        );

        let mut detached = json!({
            "schema": scena::PHOTO_QUALITY_ANALYSIS_SCHEMA_V1,
            "mode": "report_only",
            "grounding": {
                "boundary_sample_count": 64,
                "contact_shadow_delta_mean_srgb8": 0.0,
                "attached_fraction": 0.0,
                "contact_shadow_confirmed": false
            }
        });
        let mut detached_failures = Vec::new();
        apply_final_photo_quality_policy(
            scena::SceneRecipePhotoQualityV1::Final,
            &mut detached,
            &mut detached_failures,
        )
        .expect("tracked final-photo policy applies");
        assert_eq!(detached_failures, vec!["contact_shadow_missing"]);
        assert_eq!(detached["policy"]["checks"][0]["status"], "failed");

        let mut preview = detached.clone();
        preview["mode"] = json!("report_only");
        preview
            .as_object_mut()
            .expect("analysis is an object")
            .remove("policy");
        let mut preview_failures = Vec::new();
        apply_final_photo_quality_policy(
            scena::SceneRecipePhotoQualityV1::Preview,
            &mut preview,
            &mut preview_failures,
        )
        .expect("preview remains report-only");
        assert!(preview_failures.is_empty());
        assert_eq!(preview["mode"], "report_only");
        assert!(preview.get("policy").is_none());
    }

    #[test]
    fn final_photo_enables_automatic_reflection_probe_bake() {
        assert!(automatic_reflection_probe_bake_enabled(
            scena::SceneRecipePhotoQualityV1::Final
        ));
        assert!(!automatic_reflection_probe_bake_enabled(
            scena::SceneRecipePhotoQualityV1::Preview
        ));
        assert!(!synthetic_contact_shadow_enabled(
            scena::SceneRecipePhotoQualityV1::Final
        ));
        assert!(synthetic_contact_shadow_enabled(
            scena::SceneRecipePhotoQualityV1::Preview
        ));
        assert!(
            !camera_behavior_ssao_enabled(scena::SceneRecipePhotoQualityV1::Final),
            "final stills use geometry-derived area visibility; the depth-threshold SSAO pass bands smooth cycloramas"
        );
        assert!(
            camera_behavior_ssao_enabled(scena::SceneRecipePhotoQualityV1::Preview),
            "preview keeps the inexpensive screen-space grounding path"
        );
        assert!(
            !camera_behavior_baked_ambient_visibility_enabled(
                scena::SceneRecipePhotoQualityV1::Final
            ),
            "the 16-sample prepared ambient field creates visible polygonal patches on the \
             generated receiver; final stills must retain only physical area-light visibility"
        );
        assert!(!camera_behavior_baked_ambient_visibility_enabled(
            scena::SceneRecipePhotoQualityV1::Preview
        ));
    }

    #[test]
    fn local_support_shadow_measurement_detects_soft_contact_on_a_bright_receiver() {
        let width = 100;
        let height = 24;
        let mut receiver = (0..height)
            .flat_map(|y| (0..width).map(move |x| 226.0 + x as f64 * 0.04 + y as f64 * 0.18))
            .collect::<Vec<_>>();
        for y in 0..14 {
            for x in 25..76 {
                let dx = (x as f64 - 50.0) / 25.0;
                let dy = (y as f64 - 5.0) / 9.0;
                let falloff = (1.0 - (dx * dx + dy * dy).sqrt()).max(0.0);
                receiver[y * width + x] -= falloff * 18.0;
            }
        }

        let (presence, softness) = measure_local_support_shadow(&receiver, width, height, 20);

        assert!(
            presence > 0.01,
            "a localized soft contact shadow must be measured against the receiver, got {presence}"
        );
        assert!(
            softness > 0.80,
            "the continuous falloff should be classified as soft, got {softness}"
        );
    }

    #[test]
    fn local_support_shadow_measurement_rejects_receiver_gradients_and_backdrop_offsets() {
        let width = 100;
        let height = 24;
        let receiver = (0..height)
            .flat_map(|y| {
                (0..width).map(move |x| {
                    // The receiver is deliberately much brighter than the
                    // backdrop value used by the retired global comparison.
                    226.0 + x as f64 * 0.04 + y as f64 * 0.18
                })
            })
            .collect::<Vec<_>>();

        let (presence, _) = measure_local_support_shadow(&receiver, width, height, 20);

        assert!(
            presence < 0.005,
            "a smooth receiver-lighting gradient is not a contact shadow, got {presence}"
        );
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
            dark_material_mean_luminance_srgb8: None,
            dark_material_coverage: 0.0,
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

        let mut missing_beauty_subject = sane_camera_subject();
        missing_beauty_subject.background_separation_srgb8 = 0.0;
        missing_beauty_subject.silhouette_separation = 0.0;
        assert_failure(
            missing_beauty_subject,
            "subject_color_frame_agreement_below_min",
        );

        let mut old_ev_cap = sane_camera_subject();
        old_ev_cap.mean_luminance_srgb8 = 72.0;
        assert_failure(old_ev_cap, "subject_luminance_below_min");

        let mut highlight_limited_dark_product = sane_camera_subject();
        highlight_limited_dark_product.mean_luminance_srgb8 = 42.0;
        highlight_limited_dark_product.luminance_stddev_srgb8 = 34.0;
        highlight_limited_dark_product.luminance_range_srgb8 = 180.0;
        highlight_limited_dark_product.low_clip_fraction = 0.08;
        highlight_limited_dark_product.high_clip_fraction = 0.003;
        assert!(
            camera_behavior_failure_codes(highlight_limited_dark_product).is_empty(),
            "a readable dark product must not be exposed to a fixed mid-gray mean: {:?}",
            camera_behavior_failure_codes(highlight_limited_dark_product)
        );

        let mut pulled_back_empty_slab = sane_camera_subject();
        pulled_back_empty_slab.fill_fraction = 0.50;
        pulled_back_empty_slab.fill_width_fraction = 0.25;
        assert_failure(pulled_back_empty_slab, "subject_fill_below_min");

        // Positive control for a deliberate contract change, not a mutation.
        // A subject whose limiting axis fills the band while the other does not
        // is correctly framed: the aspect belongs to the subject, not to the
        // camera. Requiring both axes independently is only satisfiable when the
        // subject happens to be frame-shaped, which is why the valve manifold
        // (0.481 wide, 0.669 tall) could never pass however the camera moved.
        let mut narrow_but_well_framed = sane_camera_subject();
        narrow_but_well_framed.fill_fraction = 0.72;
        narrow_but_well_framed.fill_width_fraction = 0.53;
        narrow_but_well_framed.fill_height_fraction = 0.72;
        assert!(
            camera_behavior_failure_codes(narrow_but_well_framed).is_empty(),
            "a correctly framed subject that is not frame-shaped must pass, got {:?}",
            camera_behavior_failure_codes(narrow_but_well_framed)
        );

        // Under-filled on the limiting axis too, which is a real framing
        // failure and must still be rejected: relaxing the second axis must not
        // relax the floor.
        let mut actionably_too_small = sane_camera_subject();
        actionably_too_small.fill_fraction = 0.53;
        actionably_too_small.fill_width_fraction = 0.40;
        actionably_too_small.fill_height_fraction = 0.53;
        assert_failure(actionably_too_small, "subject_fill_below_min");

        let mut off_center = sane_camera_subject();
        off_center.center_offset_fraction = 0.31;
        assert_failure(off_center, "subject_center_offset_above_max");

        let mut blown_highlights = sane_camera_subject();
        blown_highlights.high_clip_fraction = 0.12;
        assert_failure(blown_highlights, "subject_high_clip_above_max");

        let mut visibly_clipped_product_highlights = sane_camera_subject();
        visibly_clipped_product_highlights.high_clip_fraction = 0.012;
        assert_failure(
            visibly_clipped_product_highlights,
            "subject_high_clip_above_max",
        );

        let mut flat_gray_metal = sane_camera_subject();
        flat_gray_metal.luminance_stddev_srgb8 = 0.4;
        flat_gray_metal.luminance_range_srgb8 = 2.0;
        assert_failure(flat_gray_metal, "subject_luminance_structure_below_min");

        let mut floating_subject = sane_camera_subject();
        floating_subject.shadow_presence = 0.0;
        floating_subject.shadow_softness = 0.0;
        assert_failure(floating_subject, "contact_shadow_missing");
        let mut reconciled_failures = camera_behavior_failure_codes(floating_subject);
        let mut confirmed_grounding = json!({
            "mode": "report_only",
            "grounding": {
                "boundary_sample_count": 1461,
                "contact_shadow_delta_mean_srgb8": 32.7,
                "attached_fraction": 0.99,
                "contact_shadow_confirmed": true
            }
        });
        apply_final_photo_quality_policy(
            scena::SceneRecipePhotoQualityV1::Final,
            &mut confirmed_grounding,
            &mut reconciled_failures,
        )
        .expect("tracked final-photo policy applies");
        assert!(
            !reconciled_failures.contains(&"contact_shadow_missing"),
            "same-pass subject/support contact must resolve the observed local-strip presence \
             and softness false negative"
        );
        let mut unconfirmed_failures = camera_behavior_failure_codes(floating_subject);
        let mut unconfirmed_grounding = json!({
            "mode": "report_only",
            "grounding": {
                "boundary_sample_count": 64,
                "contact_shadow_delta_mean_srgb8": 0.0,
                "attached_fraction": 0.0,
                "contact_shadow_confirmed": false
            }
        });
        apply_final_photo_quality_policy(
            scena::SceneRecipePhotoQualityV1::Final,
            &mut unconfirmed_grounding,
            &mut unconfirmed_failures,
        )
        .expect("tracked final-photo policy applies");
        assert!(
            unconfirmed_failures.contains(&"contact_shadow_missing"),
            "unconfirmed grounding must remain rejected"
        );

        let mut hard_cutout_shadow = sane_camera_subject();
        hard_cutout_shadow.shadow_presence = 0.08;
        hard_cutout_shadow.shadow_softness = 0.05;
        assert_failure(hard_cutout_shadow, "shadow_too_hard");

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

        let mut dark_metal = sane_camera_subject();
        dark_metal.mean_luminance_srgb8 = 35.0;
        dark_metal.low_clip_fraction = 0.58;
        dark_metal.high_clip_fraction = 0.004;
        dark_metal.shadow_presence = 0.0;
        let dark_metal_adjustment = corrected_photographic_lighting(dark_metal)
            .expect("dark metal should request readable fill illumination");
        assert!(
            dark_metal_adjustment.fill_scale >= 1.5,
            "dark metal needs materially stronger fill, got {}",
            dark_metal_adjustment.fill_scale
        );
        assert!(
            dark_metal_adjustment.key_scale <= 1.0,
            "dark metal must not trade unreadable body values for harder clipped highlights, got {}",
            dark_metal_adjustment.key_scale
        );
    }

    #[test]
    fn camera_behavior_retry_policy_is_bounded_camera_and_exposure_loop() {
        assert_eq!(CAMERA_BEHAVIOR_MAX_ATTEMPTS, 6);
        assert_eq!(CAMERA_BEHAVIOR_FOCUS_DELIVERY_MAX_ATTEMPTS, 6);

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

        let mut dark_product_with_clipped_highlights = sane_camera_subject();
        dark_product_with_clipped_highlights.mean_luminance_srgb8 = 70.0;
        dark_product_with_clipped_highlights.low_clip_fraction = 0.08;
        dark_product_with_clipped_highlights.high_clip_fraction = 0.02;
        let next_ev = corrected_exposure_ev(0.0, dark_product_with_clipped_highlights)
            .expect("clipped product highlights must request exposure protection");
        assert!(
            next_ev < 0.0,
            "highlight clipping must take precedence over a dark product mean, got {next_ev}"
        );

        let mut readable_highlight_limited_product = sane_camera_subject();
        readable_highlight_limited_product.mean_luminance_srgb8 = 42.0;
        readable_highlight_limited_product.luminance_stddev_srgb8 = 34.0;
        readable_highlight_limited_product.luminance_range_srgb8 = 180.0;
        readable_highlight_limited_product.low_clip_fraction = 0.08;
        readable_highlight_limited_product.high_clip_fraction = 0.003;
        assert!(
            corrected_exposure_ev(0.0, readable_highlight_limited_product).is_none(),
            "exposure must stop once a readable dark product has used its highlight headroom"
        );

        let mut unreadable_highlight_limited_product = readable_highlight_limited_product;
        unreadable_highlight_limited_product.mean_luminance_srgb8 = 16.0;
        unreadable_highlight_limited_product.low_clip_fraction = 0.69;
        assert!(
            corrected_exposure_ev(0.0, unreadable_highlight_limited_product).is_none(),
            "exposure must not oscillate when lighting or material correction is required"
        );

        let mut focused_underexposure = sane_camera_subject();
        focused_underexposure.mean_luminance_srgb8 = 66.44;
        let focused_next_ev =
            corrected_focus_delivery_exposure_ev(0.016_609_073, focused_underexposure)
                .expect("the delivered focused frame must re-enter exposure correction");
        assert!(
            focused_next_ev > 0.4,
            "post-effect metering must correct the delivered frame, got {focused_next_ev}"
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
        let report = retry_json(&[first, second], true);
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
    fn final_candidate_refresh_updates_the_selected_attempt_not_the_last_attempt() {
        let candidate = |id: &str| PhotoCandidate {
            id: id.to_owned(),
            exposure_ev: 0.0,
            composition_fill_fraction: 0.75,
            camera: test_candidate_camera(),
            metrics: sane_camera_subject(),
            status: "failed",
            failure_codes: vec!["subject_fill_below_min"],
            adjustment: Some("camera_composition"),
        };
        let mut candidates = vec![
            candidate("candidate_1"),
            candidate("candidate_2"),
            candidate("candidate_3"),
        ];
        let mut selected = candidate("candidate_2");
        selected.status = "passed";
        selected.failure_codes.clear();

        refresh_selected_candidate_history(&mut candidates, &selected);

        assert_eq!(candidates[1].id, "candidate_2");
        assert_eq!(candidates[1].status, "passed");
        assert_eq!(
            candidates[2].id, "candidate_3",
            "refreshing a best non-final attempt must not duplicate its ID over the last attempt"
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
        let report = retry_json(&candidates, true);
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
