use super::scena_cli_error::{CliErrorKind, CliFailure};
use std::fs;
use std::time::{Duration, Instant};

use super::scena_args::{
    DiagnoseCommandArgs, InspectCommandArgs, RenderCommandArgs, RepairCommandArgs,
};
use super::scena_input::{
    asset_doctor_outcome_or_error, capture_descriptor_path, ensure_parent_dir, path_for_json,
    render_introspection_options, resolve_scene_input, viewer_builder,
};
use super::scena_output::{
    CliBackendSelectionV1, CliOutcome, add_recipe_policy_to_outcome, json_outcome,
    json_outcome_with_backend_selection, json_success,
};
use super::scena_policy::{effective_recipe_policy, ensure_recipe_policy_applies};

#[cfg(feature = "scene-host")]
use super::scena_input::{
    ResolvedRecipeBuild, scene_host_build_from_resolved_recipe,
    scene_host_manifest_from_resolved_recipe,
};
#[cfg(feature = "scene-host")]
use super::scena_output::add_backend_selection_to_outcome;

pub(crate) fn run_render_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    let args = RenderCommandArgs::parse(args)?;
    let total_started = Instant::now();
    let policy = effective_recipe_policy(&args.allow_roots, None)?;
    let input = match resolve_scene_input(&args.input, policy) {
        Ok(input) => input,
        Err(outcome) => return Ok(outcome),
    };
    ensure_recipe_policy_applies(input.is_recipe(), &args.allow_roots)?;
    let width = args.width.or(input.width).unwrap_or(800);
    let height = args.height.or(input.height).unwrap_or(600);
    if input.is_recipe() {
        return run_render_scene_host_recipe(input, width, height, args);
    }
    let first = match pollster::block_on(
        viewer_builder(
            input.asset.as_str(),
            width,
            height,
            input.transform,
            args.gpu,
        )
        .with_default_light()
        .render(),
    ) {
        Ok(first) => first,
        Err(error) => {
            return asset_doctor_outcome_or_error(&input.asset, "render", error.to_string());
        }
    };
    let capture_started = Instant::now();
    let capture = first
        .capture()
        .map_err(|error| format!("failed to capture '{}': {error}", input.asset))?;
    let capture_duration = capture_started.elapsed();

    ensure_parent_dir(&args.out)?;
    capture.write_png(&args.out).map_err(|error| {
        CliFailure::new(
            CliErrorKind::Io,
            format!("failed to write PNG '{}': {error}", args.out.display()),
        )
    })?;

    let descriptor_path = capture_descriptor_path(&args.out);
    ensure_parent_dir(&descriptor_path)?;
    fs::write(
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

    let inspection = first
        .scene()
        .inspect_with_assets(first.assets())
        .to_schema_report();
    let mut options = render_introspection_options(args.detail)
        .with_capture_png_path(path_for_json(&args.out))
        .with_capture_descriptor_path(path_for_json(&descriptor_path));
    if args.timings {
        options = options.with_timings(scena::RenderIntrospectionTimingsV1::measured_monotonic(
            duration_ms(first.prepare_duration()),
            duration_ms(first.render_duration()),
            duration_ms(capture_duration),
            duration_ms(total_started.elapsed()),
        ));
    }
    let report = first
        .renderer()
        .introspect_capture(&capture, &inspection, options);
    let exit_code = if report.ok { 0 } else { 1 };
    json_outcome_with_backend_selection(
        &report,
        exit_code,
        "failed to serialize render introspection report",
        CliBackendSelectionV1::new(args.gpu, Some(first.renderer().capabilities().backend)),
    )
}

#[cfg(feature = "scene-host")]
fn run_render_scene_host_recipe(
    input: super::scena_input::ResolvedSceneInput,
    width: u32,
    height: u32,
    args: RenderCommandArgs,
) -> Result<CliOutcome, CliFailure> {
    let total_started = Instant::now();
    let policy = input.policy.to_schema_report();
    let build = pollster::block_on(scene_host_build_from_resolved_recipe(
        &input, width, height, args.gpu,
    ))?;
    let mut host = match build {
        ResolvedRecipeBuild::Built(build) => build.host,
        ResolvedRecipeBuild::Rejected(outcome) => {
            return add_recipe_policy_to_outcome(
                add_backend_selection_to_outcome(
                    outcome,
                    CliBackendSelectionV1::new(args.gpu, None),
                )?,
                &policy,
            );
        }
    };
    let prepare_started = Instant::now();
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

    ensure_parent_dir(&args.out)?;
    capture.write_png(&args.out).map_err(|error| {
        CliFailure::new(
            CliErrorKind::Io,
            format!("failed to write PNG '{}': {error}", args.out.display()),
        )
    })?;

    let descriptor_path = capture_descriptor_path(&args.out);
    ensure_parent_dir(&descriptor_path)?;
    fs::write(
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
    let mut options = render_introspection_options(args.detail)
        .with_capture_png_path(path_for_json(&args.out))
        .with_capture_descriptor_path(path_for_json(&descriptor_path));
    if args.timings {
        options = options.with_timings(scena::RenderIntrospectionTimingsV1::measured_monotonic(
            duration_ms(prepare_duration),
            duration_ms(render_duration),
            duration_ms(capture_duration),
            duration_ms(total_started.elapsed()),
        ));
    }
    let report = host
        .renderer()
        .introspect_capture(&capture, &inspection, options);
    let exit_code = if report.ok { 0 } else { 1 };
    add_recipe_policy_to_outcome(
        json_outcome_with_backend_selection(
            &report,
            exit_code,
            "failed to serialize render introspection report",
            CliBackendSelectionV1::new(args.gpu, Some(host.backend())),
        )?,
        &policy,
    )
}

fn duration_ms(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(not(feature = "scene-host"))]
fn run_render_scene_host_recipe(
    _input: super::scena_input::ResolvedSceneInput,
    _width: u32,
    _height: u32,
    _args: RenderCommandArgs,
) -> Result<CliOutcome, CliFailure> {
    Err(CliFailure::feature_unavailable(
        "recipe overlay directives require building the scena binary with the 'scene-host' feature",
    ))
}

pub(crate) fn run_inspect_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    let args = InspectCommandArgs::parse(args)?;
    let policy = effective_recipe_policy(&args.allow_roots, None)?;
    let input = match resolve_scene_input(&args.input, policy) {
        Ok(input) => input,
        Err(outcome) => return Ok(outcome),
    };
    ensure_recipe_policy_applies(input.is_recipe(), &args.allow_roots)?;
    let width = args.width.or(input.width).unwrap_or(800);
    let height = args.height.or(input.height).unwrap_or(600);
    if input.is_recipe() {
        return run_inspect_scene_host_recipe(input, width, height);
    }
    let viewer = match pollster::block_on(
        viewer_builder(input.asset.as_str(), width, height, input.transform, false)
            .with_default_light()
            .build(),
    ) {
        Ok(viewer) => viewer,
        Err(error) => {
            return asset_doctor_outcome_or_error(&input.asset, "inspect", error.to_string());
        }
    };
    let report = viewer
        .scene()
        .inspect_with_assets(viewer.assets())
        .to_schema_report();
    json_success(&report, "failed to serialize scene inspection report")
}

#[cfg(feature = "scene-host")]
fn run_inspect_scene_host_recipe(
    input: super::scena_input::ResolvedSceneInput,
    width: u32,
    height: u32,
) -> Result<CliOutcome, CliFailure> {
    let policy = input.policy.to_schema_report();
    let build = pollster::block_on(scene_host_build_from_resolved_recipe(
        &input, width, height, false,
    ))?;
    let host = match build {
        ResolvedRecipeBuild::Built(build) => build.host,
        ResolvedRecipeBuild::Rejected(outcome) => {
            return add_recipe_policy_to_outcome(outcome, &policy);
        }
    };
    let text = host
        .inspect_json()
        .map_err(|error| format!("failed to inspect recipe scene: {error}"))?;
    add_recipe_policy_to_outcome(
        CliOutcome {
            stdout: text,
            exit_code: 0,
            payload: super::scena_output::CliPayload::Json,
        },
        &policy,
    )
}

#[cfg(not(feature = "scene-host"))]
fn run_inspect_scene_host_recipe(
    _input: super::scena_input::ResolvedSceneInput,
    _width: u32,
    _height: u32,
) -> Result<CliOutcome, CliFailure> {
    Err(CliFailure::feature_unavailable(
        "recipe overlay directives require building the scena binary with the 'scene-host' feature",
    ))
}

pub(crate) fn run_diagnose_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    let args = DiagnoseCommandArgs::parse(args)?;
    let policy = effective_recipe_policy(&args.allow_roots, None)?;
    let input = match resolve_scene_input(&args.input, policy) {
        Ok(input) => input,
        Err(outcome) => return Ok(outcome),
    };
    ensure_recipe_policy_applies(input.is_recipe(), &args.allow_roots)?;
    let width = args.width.or(input.width).unwrap_or(800);
    let height = args.height.or(input.height).unwrap_or(600);
    if input.is_recipe() {
        return run_diagnose_scene_host_recipe(input, width, height, args);
    }
    let first = match pollster::block_on(
        viewer_builder(input.asset.as_str(), width, height, input.transform, false)
            .with_default_light()
            .render(),
    ) {
        Ok(first) => first,
        Err(error) => {
            return asset_doctor_outcome_or_error(&input.asset, "diagnose", error.to_string());
        }
    };
    let inspection = first
        .scene()
        .inspect_with_assets(first.assets())
        .to_schema_report();
    let options = if args.detail {
        scena::VisibilityDiagnosisOptions::detail()
    } else {
        scena::VisibilityDiagnosisOptions::summary()
    };
    let report = first
        .renderer()
        .diagnose_visibility(&inspection, args.handle, options);
    let exit_code = if report.ok { 0 } else { 1 };
    json_outcome(
        &report,
        exit_code,
        "failed to serialize visibility diagnosis report",
    )
}

#[cfg(feature = "scene-host")]
fn run_diagnose_scene_host_recipe(
    input: super::scena_input::ResolvedSceneInput,
    width: u32,
    height: u32,
    args: DiagnoseCommandArgs,
) -> Result<CliOutcome, CliFailure> {
    let policy = input.policy.to_schema_report();
    let build = pollster::block_on(scene_host_build_from_resolved_recipe(
        &input, width, height, false,
    ))?;
    let mut host = match build {
        ResolvedRecipeBuild::Built(build) => build.host,
        ResolvedRecipeBuild::Rejected(outcome) => {
            return add_recipe_policy_to_outcome(outcome, &policy);
        }
    };
    host.prepare()
        .map_err(|error| format!("failed to prepare recipe scene for diagnosis: {error}"))?;
    host.render()
        .map_err(|error| format!("failed to render recipe scene for diagnosis: {error}"))?;
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
    let options = if args.detail {
        scena::VisibilityDiagnosisOptions::detail()
    } else {
        scena::VisibilityDiagnosisOptions::summary()
    };
    let report = host
        .renderer()
        .diagnose_visibility(&inspection, args.handle, options);
    let exit_code = if report.ok { 0 } else { 1 };
    add_recipe_policy_to_outcome(
        json_outcome(
            &report,
            exit_code,
            "failed to serialize visibility diagnosis report",
        )?,
        &policy,
    )
}

#[cfg(not(feature = "scene-host"))]
fn run_diagnose_scene_host_recipe(
    _input: super::scena_input::ResolvedSceneInput,
    _width: u32,
    _height: u32,
    _args: DiagnoseCommandArgs,
) -> Result<CliOutcome, CliFailure> {
    Err(CliFailure::feature_unavailable(
        "recipe overlay directives require building the scena binary with the 'scene-host' feature",
    ))
}

pub(crate) fn run_repair_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    let args = RepairCommandArgs::parse(args)?;
    let policy = effective_recipe_policy(&args.allow_roots, None)?;
    let input = match resolve_scene_input(&args.input, policy) {
        Ok(input) => input,
        Err(outcome) => return Ok(outcome),
    };
    ensure_recipe_policy_applies(input.is_recipe(), &args.allow_roots)?;
    let policy = input.policy.to_schema_report();
    if input.is_recipe()
        && let Some(outcome) = validate_repair_recipe_input(&input)?
    {
        return Ok(outcome);
    }
    if !input.is_recipe()
        && let Some(outcome) = validate_repair_asset_input(&input.asset)?
    {
        return Ok(outcome);
    }
    let text = fs::read_to_string(&args.from)
        .map_err(|error| format!("failed to read report '{}': {error}", args.from.display()))?;
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|error| format!("failed to parse report '{}': {error}", args.from.display()))?;
    let schema = value
        .get("schema")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "repair input report is missing a string schema field".to_string())?;
    let plan = match schema {
        scena::VISIBILITY_DIAGNOSIS_SCHEMA_V1 => {
            let report: scena::VisibilityDiagnosisReportV1 = serde_json::from_value(value)
                .map_err(|error| {
                    format!(
                        "failed to decode visibility diagnosis '{}': {error}",
                        args.from.display()
                    )
                })?;
            scena::VisualRepairPlanV1::from_visibility_diagnosis(&report)
        }
        scena::RENDER_INTROSPECTION_SCHEMA_V1 => {
            let report: scena::RenderIntrospectionReportV1 = serde_json::from_value(value)
                .map_err(|error| {
                    format!(
                        "failed to decode render introspection '{}': {error}",
                        args.from.display()
                    )
                })?;
            scena::VisualRepairPlanV1::from_render_introspection(&report)
        }
        other => {
            return Err(CliFailure::invalid_arguments(format!(
                "repair --from expected '{}' or '{}', got '{other}'",
                scena::VISIBILITY_DIAGNOSIS_SCHEMA_V1,
                scena::RENDER_INTROSPECTION_SCHEMA_V1
            )));
        }
    };
    if plan.status == "irreducible" || args.iteration_budget == 0 {
        let loop_result = scena::AgentLoopResultV1::irreducible(
            plan,
            args.iteration_budget,
            args.iteration_budget,
        );
        return add_recipe_policy_to_outcome(
            json_outcome(&loop_result, 1, "failed to serialize agent loop result")?,
            &policy,
        );
    }
    let exit_code = if plan.auto_fixable { 0 } else { 1 };
    add_recipe_policy_to_outcome(
        json_outcome(&plan, exit_code, "failed to serialize visual repair plan")?,
        &policy,
    )
}

fn validate_repair_asset_input(asset: &str) -> Result<Option<CliOutcome>, CliFailure> {
    let report = pollster::block_on(scena::Assets::new().doctor_asset_path(asset));
    if report.ok {
        Ok(None)
    } else {
        json_outcome(
            &report,
            1,
            "failed to serialize repair target asset doctor report",
        )
        .map(Some)
    }
}

#[cfg(feature = "scene-host")]
fn validate_repair_recipe_input(
    input: &super::scena_input::ResolvedSceneInput,
) -> Result<Option<CliOutcome>, CliFailure> {
    let report = pollster::block_on(scene_host_manifest_from_resolved_recipe(input))?;
    if report.ok {
        Ok(None)
    } else {
        json_outcome(
            &report,
            1,
            "failed to serialize repair target recipe build result",
        )
        .map(Some)
    }
}

#[cfg(not(feature = "scene-host"))]
fn validate_repair_recipe_input(
    _input: &super::scena_input::ResolvedSceneInput,
) -> Result<Option<CliOutcome>, CliFailure> {
    Err(CliFailure::feature_unavailable(
        "repair for scene recipes requires the scene-host feature",
    ))
}
