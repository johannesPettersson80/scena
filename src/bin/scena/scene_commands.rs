use std::fs;

use super::scena_args::{
    DiagnoseCommandArgs, InspectCommandArgs, RenderCommandArgs, RepairCommandArgs,
};
use super::scena_input::{
    asset_doctor_outcome_or_error, capture_descriptor_path, ensure_parent_dir, path_for_json,
    render_introspection_options, resolve_scene_input, viewer_builder,
};
use super::scena_output::{CliOutcome, json_outcome, json_success};

#[cfg(feature = "scene-host")]
use super::scena_input::scene_host_from_resolved_recipe;

pub(crate) fn run_render_command(args: &[String]) -> Result<CliOutcome, String> {
    let args = RenderCommandArgs::parse(args)?;
    let input = match resolve_scene_input(&args.input) {
        Ok(input) => input,
        Err(outcome) => return Ok(outcome),
    };
    let width = args.width.or(input.width).unwrap_or(800);
    let height = args.height.or(input.height).unwrap_or(600);
    if input.has_scene_host_directives() {
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
    warn_gpu_fallback(args.gpu, first.renderer().capabilities().backend);
    let capture = first
        .capture()
        .map_err(|error| format!("failed to capture '{}': {error}", input.asset))?;

    ensure_parent_dir(&args.out)?;
    capture
        .write_png(&args.out)
        .map_err(|error| format!("failed to write PNG '{}': {error}", args.out.display()))?;

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
    let options = render_introspection_options(args.detail)
        .with_capture_png_path(path_for_json(&args.out))
        .with_capture_descriptor_path(path_for_json(&descriptor_path));
    let report = first
        .renderer()
        .introspect_capture(&capture, &inspection, options);
    let exit_code = if report.ok { 0 } else { 1 };
    json_outcome(
        &report,
        exit_code,
        "failed to serialize render introspection report",
    )
}

fn warn_gpu_fallback(requested_gpu: bool, backend: scena::Backend) {
    if requested_gpu && backend != scena::Backend::HeadlessGpu {
        eprintln!(
            "scena: --gpu requested but HeadlessGpu was unavailable; rendered with {backend:?}"
        );
    }
}

#[cfg(feature = "scene-host")]
fn run_render_scene_host_recipe(
    input: super::scena_input::ResolvedSceneInput,
    width: u32,
    height: u32,
    args: RenderCommandArgs,
) -> Result<CliOutcome, String> {
    let mut host = pollster::block_on(scene_host_from_resolved_recipe(
        &input, width, height, args.gpu,
    ))?;
    warn_gpu_fallback(args.gpu, host.backend());
    host.prepare()
        .map_err(|error| format!("failed to prepare recipe scene: {error}"))?;
    host.render()
        .map_err(|error| format!("failed to render recipe scene: {error}"))?;
    let capture = host
        .capture()
        .map_err(|error| format!("failed to capture recipe scene: {error}"))?;

    ensure_parent_dir(&args.out)?;
    capture
        .write_png(&args.out)
        .map_err(|error| format!("failed to write PNG '{}': {error}", args.out.display()))?;

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
        .map_err(|error| format!("failed to decode recipe scene inspection report: {error}"))?;
    let options = render_introspection_options(args.detail)
        .with_capture_png_path(path_for_json(&args.out))
        .with_capture_descriptor_path(path_for_json(&descriptor_path));
    let report = host
        .renderer()
        .introspect_capture(&capture, &inspection, options);
    let exit_code = if report.ok { 0 } else { 1 };
    json_outcome(
        &report,
        exit_code,
        "failed to serialize render introspection report",
    )
}

#[cfg(not(feature = "scene-host"))]
fn run_render_scene_host_recipe(
    _input: super::scena_input::ResolvedSceneInput,
    _width: u32,
    _height: u32,
    _args: RenderCommandArgs,
) -> Result<CliOutcome, String> {
    Err(
        "recipe overlay directives require building the scena binary with the 'scene-host' feature"
            .to_string(),
    )
}

pub(crate) fn run_inspect_command(args: &[String]) -> Result<CliOutcome, String> {
    let args = InspectCommandArgs::parse(args)?;
    let input = match resolve_scene_input(&args.input) {
        Ok(input) => input,
        Err(outcome) => return Ok(outcome),
    };
    let width = args.width.or(input.width).unwrap_or(800);
    let height = args.height.or(input.height).unwrap_or(600);
    if input.has_scene_host_directives() {
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
) -> Result<CliOutcome, String> {
    let host = pollster::block_on(scene_host_from_resolved_recipe(
        &input, width, height, false,
    ))?;
    let text = host
        .inspect_json()
        .map_err(|error| format!("failed to inspect recipe scene: {error}"))?;
    Ok(CliOutcome {
        stdout: text,
        exit_code: 0,
    })
}

#[cfg(not(feature = "scene-host"))]
fn run_inspect_scene_host_recipe(
    _input: super::scena_input::ResolvedSceneInput,
    _width: u32,
    _height: u32,
) -> Result<CliOutcome, String> {
    Err(
        "recipe overlay directives require building the scena binary with the 'scene-host' feature"
            .to_string(),
    )
}

pub(crate) fn run_diagnose_command(args: &[String]) -> Result<CliOutcome, String> {
    let args = DiagnoseCommandArgs::parse(args)?;
    let input = match resolve_scene_input(&args.input) {
        Ok(input) => input,
        Err(outcome) => return Ok(outcome),
    };
    let width = args.width.or(input.width).unwrap_or(800);
    let height = args.height.or(input.height).unwrap_or(600);
    if input.has_scene_host_directives() {
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
) -> Result<CliOutcome, String> {
    let mut host = pollster::block_on(scene_host_from_resolved_recipe(
        &input, width, height, false,
    ))?;
    host.prepare()
        .map_err(|error| format!("failed to prepare recipe scene for diagnosis: {error}"))?;
    host.render()
        .map_err(|error| format!("failed to render recipe scene for diagnosis: {error}"))?;
    let inspection_json = host
        .inspect_json()
        .map_err(|error| format!("failed to inspect recipe scene: {error}"))?;
    let inspection: scena::SceneInspectionReportV1 = serde_json::from_str(&inspection_json)
        .map_err(|error| format!("failed to decode recipe scene inspection report: {error}"))?;
    let options = if args.detail {
        scena::VisibilityDiagnosisOptions::detail()
    } else {
        scena::VisibilityDiagnosisOptions::summary()
    };
    let report = host
        .renderer()
        .diagnose_visibility(&inspection, args.handle, options);
    let exit_code = if report.ok { 0 } else { 1 };
    json_outcome(
        &report,
        exit_code,
        "failed to serialize visibility diagnosis report",
    )
}

#[cfg(not(feature = "scene-host"))]
fn run_diagnose_scene_host_recipe(
    _input: super::scena_input::ResolvedSceneInput,
    _width: u32,
    _height: u32,
    _args: DiagnoseCommandArgs,
) -> Result<CliOutcome, String> {
    Err(
        "recipe overlay directives require building the scena binary with the 'scene-host' feature"
            .to_string(),
    )
}

pub(crate) fn run_repair_command(args: &[String]) -> Result<CliOutcome, String> {
    let args = RepairCommandArgs::parse(args)?;
    let _input = args.input;
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
            return Err(format!(
                "repair --from expected '{}' or '{}', got '{other}'",
                scena::VISIBILITY_DIAGNOSIS_SCHEMA_V1,
                scena::RENDER_INTROSPECTION_SCHEMA_V1
            ));
        }
    };
    if plan.status == "irreducible" || args.iteration_budget == 0 {
        let loop_result = scena::AgentLoopResultV1::irreducible(
            plan,
            args.iteration_budget,
            args.iteration_budget,
        );
        return json_outcome(&loop_result, 1, "failed to serialize agent loop result");
    }
    let exit_code = if plan.auto_fixable { 0 } else { 1 };
    json_outcome(&plan, exit_code, "failed to serialize visual repair plan")
}
