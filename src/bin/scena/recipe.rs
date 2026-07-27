use super::scena_cli_error::{CliErrorKind, CliFailure, CliUsageError};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::scena_input::{
    RecipeReadError, capture_descriptor_path, ensure_parent_dir, path_for_json, read_recipe_text,
    render_introspection_options,
};
use super::scena_output::{
    CliBackendSelectionV1, CliOutcome, add_recipe_policy_to_outcome, json_outcome,
    json_outcome_with_backend_selection,
};
use super::scena_photo;
use super::scena_policy::{effective_recipe_policy, push_allow_root};

#[path = "recipe/verification.rs"]
mod verification;

use verification::{RecipeVerificationInput, verify_recipe_expectations};

#[path = "recipe/cad_inspection.rs"]
mod cad_inspection;
#[path = "recipe/capture_sequence.rs"]
mod capture_sequence;
#[path = "recipe/capture_shared.rs"]
mod capture_shared;
#[path = "recipe/semantic_aov.rs"]
mod semantic_aov;
#[path = "recipe/subject_focus.rs"]
pub(crate) mod subject_focus;
#[path = "recipe/subject_metering.rs"]
mod subject_metering;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecipeRenderCommandArgs {
    recipe: PathBuf,
    out: PathBuf,
    verify: bool,
    detail: bool,
    gpu: bool,
    timings: bool,
    max_imports: Option<usize>,
    allow_roots: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecipeBuildCommandArgs {
    recipe: PathBuf,
    max_imports: Option<usize>,
    allow_roots: Vec<PathBuf>,
}

pub(crate) fn run_recipe_build_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    let args = RecipeBuildCommandArgs::parse(args)?;
    let policy = effective_recipe_policy(&args.allow_roots, args.max_imports)?;
    let policy_report = policy.to_schema_report();
    let recipe_text = match read_recipe_text(&args.recipe, &policy) {
        Ok(text) => text,
        Err(RecipeReadError::TooLarge(report)) => {
            return add_recipe_policy_to_outcome(
                json_outcome(
                    &report,
                    1,
                    "failed to serialize scene recipe validation report",
                )?,
                &policy_report,
            );
        }
        Err(RecipeReadError::Io(error)) => {
            return Err(CliFailure::new(
                CliErrorKind::InputNotFound,
                format!("failed to read recipe '{}': {error}", args.recipe.display()),
            ));
        }
    };
    let result = pollster::block_on(scena::SceneHostCore::build_recipe_manifest_json(
        args.recipe.display().to_string(),
        &recipe_text,
        policy,
    ));
    let exit_code = if result.ok { 0 } else { 1 };
    json_outcome(
        &result,
        exit_code,
        "failed to serialize recipe build result",
    )
}

pub(crate) fn run_recipe_render_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    let args = RecipeRenderCommandArgs::parse(args)?;
    let total_started = Instant::now();
    let policy = effective_recipe_policy(&args.allow_roots, args.max_imports)?;
    let policy_report = policy.to_schema_report();
    let recipe_text = match read_recipe_text(&args.recipe, &policy) {
        Ok(text) => text,
        Err(RecipeReadError::TooLarge(report)) => {
            return add_recipe_policy_to_outcome(
                json_outcome(
                    &report,
                    1,
                    "failed to serialize scene recipe validation report",
                )?,
                &policy_report,
            );
        }
        Err(RecipeReadError::Io(error)) => {
            return Err(CliFailure::new(
                CliErrorKind::InputNotFound,
                format!("failed to read recipe '{}': {error}", args.recipe.display()),
            ));
        }
    };
    let recipe_path = args.recipe.display().to_string();
    let build = if args.gpu {
        pollster::block_on(scena::SceneHostCore::build_recipe_json_gpu(
            &recipe_path,
            &recipe_text,
            policy,
        ))
    } else {
        pollster::block_on(scena::SceneHostCore::build_recipe_json(
            &recipe_path,
            &recipe_text,
            policy,
        ))
    };
    let build = match build {
        Ok(build) => build,
        Err(manifest) => {
            let result = scena::SceneRecipeRenderResultV1::build_failed(manifest);
            return add_recipe_policy_to_outcome(
                json_outcome_with_backend_selection(
                    &result,
                    1,
                    "failed to serialize recipe render result",
                    CliBackendSelectionV1::new(args.gpu, None),
                )?,
                &policy_report,
            );
        }
    };
    let recipe: scena::SceneRecipeV1 = serde_json::from_str(&recipe_text)
        .map_err(|error| format!("validated recipe failed to decode: {error}"))?;

    let mut host = build.host;
    // GPU hosts must own semantic AOV resources before the next prepare, because
    // the camera-behavior loop and subject metering read them back.
    if args.gpu {
        host.set_semantic_aov_capture_enabled(true);
    }
    let backend_selection = CliBackendSelectionV1::new(args.gpu, Some(host.backend()));
    let photo_subject = recipe_photo_subject(&recipe, &build.manifest)?;
    if photo_subject.is_none()
        && recipe.photo.is_none()
        && !recipe.cameras.iter().any(|camera| camera.active)
    {
        host.frame_all_with_overlays()
            .map_err(|error| format!("failed to frame recipe scene including overlays: {error}"))?;
    }
    let (
        capture,
        prepare_duration,
        render_duration,
        capture_duration,
        photo_reasons,
        focus_report,
        exposure_report,
    ) = if let Some(subject) = &photo_subject {
        let photo_started = Instant::now();
        let planning = scena_photo::camera_behavior_composition_plan(
            &host,
            subject.root_handle,
            !recipe.cameras.is_empty(),
        )?;
        let shaded_selection = scena_photo::apply_camera_behavior_setup_with_plan(
            &mut host,
            subject,
            !build.manifest.lights.is_empty(),
            &planning,
            args.gpu,
        )?;
        let selected_composition =
            scena_photo::selected_shaded_composition_candidate(&planning, &shaded_selection)?
                .clone();
        let selected = scena_photo::render_camera_behavior_candidates(
            &mut host,
            subject,
            &selected_composition,
            args.gpu,
        )?;
        let duration = photo_started.elapsed();
        let reasons = photo_acceptance_reasons(&selected);
        (
            selected.capture,
            Duration::ZERO,
            duration,
            Duration::ZERO,
            reasons,
            None,
            None,
        )
    } else {
        let prepare_started = Instant::now();
        let subject_focus = subject_focus::resolve_and_apply_subject_focus(
            &mut host,
            &build.manifest,
            &recipe,
            args.gpu,
        )?;
        subject_metering::resolve_and_apply_subject_metering(
            &mut host,
            &build.manifest,
            &recipe,
            args.gpu,
        )?;
        host.prepare()
            .map_err(|error| format!("failed to prepare recipe scene: {error}"))?;
        let prepare_duration = prepare_started.elapsed();
        let render_started = Instant::now();
        host.render()
            .map_err(|error| format!("failed to render recipe scene: {error}"))?;
        let render_duration = render_started.elapsed();
        let capture_started = Instant::now();
        let capture = host
            .capture()
            .map_err(|error| format!("failed to capture recipe scene: {error}"))?;
        let capture_duration = capture_started.elapsed();
        let focus_report = subject_focus
            .as_ref()
            .map(|resolution| resolution.to_focus_report(&capture));
        let exposure_report = exposure_report_from_renderer(&host, &capture);
        (
            capture,
            prepare_duration,
            render_duration,
            capture_duration,
            Vec::new(),
            focus_report,
            exposure_report,
        )
    };

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
        format!(
            "failed to write capture descriptor '{}': {error}",
            descriptor_path.display()
        )
    })?;

    let inspection_json = host
        .inspect_json()
        .map_err(|error| format!("failed to inspect recipe scene: {error}"))?;
    let inspection: scena::SceneInspectionReportV1 = serde_json::from_str(&inspection_json)
        .map_err(|error| {
            CliFailure::new(
                CliErrorKind::InvalidInput,
                format!("failed to decode recipe scene inspection report: {error}"),
            )
        })?;
    let mut introspection_options = render_introspection_options(args.detail)
        .with_capture_png_path(path_for_json(&args.out))
        .with_capture_descriptor_path(path_for_json(&descriptor_path));
    if args.timings {
        introspection_options = introspection_options.with_timings(
            scena::RenderIntrospectionTimingsV1::measured_monotonic(
                duration_ms(prepare_duration),
                duration_ms(render_duration),
                duration_ms(capture_duration),
                duration_ms(total_started.elapsed()),
            ),
        );
    }
    if let Some(focus_report) = focus_report {
        introspection_options = introspection_options.with_focus_report(focus_report);
    }
    if let Some(exposure_report) = exposure_report {
        introspection_options = introspection_options.with_exposure_report(exposure_report);
    }
    let mut introspection =
        host.renderer()
            .introspect_capture(&capture, &inspection, introspection_options);
    if !args.verify {
        let exit_code = if introspection.ok { 0 } else { 1 };
        return add_recipe_policy_to_outcome(
            json_outcome_with_backend_selection(
                &introspection,
                exit_code,
                "failed to serialize render introspection report",
                backend_selection,
            )?,
            &policy_report,
        );
    }
    let mut verification = verify_recipe_expectations(RecipeVerificationInput {
        host: &mut host,
        manifest: &build.manifest,
        recipe: &recipe,
        expect: recipe.expect.as_ref(),
        capture: &capture,
        inspection: &inspection,
        introspection: &introspection,
        detail: args.detail,
        recipe_path: &args.recipe,
        recipe_dir: args
            .recipe
            .parent()
            .unwrap_or_else(|| std::path::Path::new(".")),
    })?;
    append_photo_reasons(&mut verification, photo_reasons);
    introspection.subject_observations = verification.subject_observations.clone();
    let result = scena::SceneRecipeRenderResultV1::new(
        build.manifest,
        capture.descriptor,
        introspection,
        verification,
    );
    let exit_code = if result.ok { 0 } else { 1 };
    add_recipe_policy_to_outcome(
        json_outcome_with_backend_selection(
            &result,
            exit_code,
            "failed to serialize recipe render result",
            backend_selection,
        )?,
        &policy_report,
    )
}

pub(crate) fn run_recipe_inspect_cad_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    cad_inspection::run_recipe_inspect_cad_command(args)
}

pub(crate) fn run_recipe_capture_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    capture_sequence::run_recipe_capture_command(args)
}

fn recipe_photo_subject(
    recipe: &scena::SceneRecipeV1,
    manifest: &scena::SceneRecipeBuildV1,
) -> Result<Option<scena_photo::SubjectSelection>, CliFailure> {
    let Some(photo) = &recipe.photo else {
        return Ok(None);
    };
    let intent = match photo.intent.as_str() {
        "camera_behavior" | "camera-behavior" | "product_hero" | "product-hero" => {
            "camera_behavior"
        }
        other => {
            return Err(CliFailure::new(
                CliErrorKind::InvalidInput,
                format!("unsupported photo intent '{other}'; use camera_behavior"),
            ));
        }
    };
    if intent != "camera_behavior" {
        return Ok(None);
    }
    let subject = photo.subject.as_ref().map(|subject| subject.target());
    if let Some(scena::SceneRecipeTargetV1::Node { id }) = subject.as_ref()
        && manifest
            .nodes
            .iter()
            .any(|node| node.id == *id && node.visible == Some(false))
    {
        // A recipe render must reach composition verification so the caller
        // receives `subject_hidden` in the typed render result. The strict
        // `scena photo render` command still rejects an unusable subject.
        return Ok(None);
    }
    scena_photo::select_camera_behavior_subject(manifest, subject).map(Some)
}

fn photo_acceptance_reasons(
    selected: &scena_photo::SelectedCapture,
) -> Vec<scena::SceneRecipeVerificationReasonV1> {
    if selected.final_candidate.status == "passed" {
        return Vec::new();
    }
    selected
        .final_candidate
        .failure_codes
        .iter()
        .map(|code| scena::SceneRecipeVerificationReasonV1 {
            code: (*code).to_owned(),
            severity: "error".to_owned(),
            source: "photo".to_owned(),
            expectation_id: Some("photo.intent.camera_behavior".to_owned()),
            affected_handles: Vec::new(),
            message: format!("camera_behavior photo acceptance failed: {code}"),
        })
        .collect()
}

fn append_photo_reasons(
    verification: &mut scena::SceneRecipeVerificationReportV1,
    mut reasons: Vec<scena::SceneRecipeVerificationReasonV1>,
) {
    if reasons.is_empty() {
        return;
    }
    verification.summary.render_checks += 1;
    verification.summary.errors += reasons
        .iter()
        .filter(|reason| reason.severity == "error")
        .count();
    verification.summary.warnings += reasons
        .iter()
        .filter(|reason| reason.severity == "warning")
        .count();
    verification.reasons.append(&mut reasons);
    verification.ok = verification.summary.errors == 0;
}

pub(crate) fn run_recipe_aov_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    semantic_aov::run_recipe_aov_command(args)
}

impl RecipeRenderCommandArgs {
    fn parse(args: &[String]) -> Result<Self, CliUsageError> {
        let Some(recipe) = args.first() else {
            return Err(CliUsageError::from(recipe_render_usage()));
        };
        let mut out = None;
        let mut verify = false;
        let mut detail = false;
        let mut gpu = false;
        let mut timings = false;
        let mut max_imports = None;
        let mut allow_roots = Vec::new();
        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
                "--out" => {
                    out = Some(PathBuf::from(flag_value(args, index, "--out")?));
                    index += 2;
                }
                "--introspect" => {
                    index += 1;
                }
                "--verify" => {
                    verify = true;
                    index += 1;
                }
                "--detail" => {
                    detail = true;
                    index += 1;
                }
                "--gpu" => {
                    gpu = true;
                    index += 1;
                }
                "--timings" => {
                    timings = true;
                    index += 1;
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
                "--json" => {
                    index += 1;
                }
                flag => {
                    return Err(CliUsageError::from(format!(
                        "unknown recipe render flag '{flag}'; {}",
                        recipe_render_usage()
                    )));
                }
            }
        }
        Ok(Self {
            recipe: PathBuf::from(recipe),
            out: out.ok_or_else(|| {
                CliUsageError::from(format!("missing --out <png>; {}", recipe_render_usage()))
            })?,
            verify,
            detail,
            gpu,
            timings,
            max_imports,
            allow_roots,
        })
    }
}

impl RecipeBuildCommandArgs {
    fn parse(args: &[String]) -> Result<Self, CliUsageError> {
        let Some(recipe) = args.first() else {
            return Err(CliUsageError::from(recipe_build_usage()));
        };
        let mut max_imports = None;
        let mut allow_roots = Vec::new();
        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
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
                        "unknown recipe build flag '{flag}'; {}",
                        recipe_build_usage()
                    )));
                }
            }
        }
        Ok(Self {
            recipe: PathBuf::from(recipe),
            max_imports,
            allow_roots,
        })
    }
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

fn recipe_render_usage() -> String {
    "usage: scena recipe render <recipe.json> [--verify] --out <png> [--introspect] [--detail] [--gpu] [--timings] [--max-imports <n>] [--allow-root <directory>]..."
        .to_owned()
}

fn duration_ms(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn exposure_report_from_renderer(
    host: &scena::SceneHostCore<scena::DefaultAssetFetcher>,
    capture: &scena::CaptureRgba8,
) -> Option<scena::ExposureReportV1> {
    let renderer = host.renderer();
    let config = renderer.auto_exposure()?;
    Some(scena::ExposureReportV1::from_auto_exposure(
        renderer.auto_exposure_status(),
        config,
        renderer.last_auto_exposure(),
        renderer.exposure_ev(),
        &capture.descriptor,
    ))
}

fn recipe_build_usage() -> String {
    "usage: scena recipe build <recipe.json> [--max-imports <n>] [--allow-root <directory>]..."
        .to_owned()
}
