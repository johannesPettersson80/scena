use std::env;
#[cfg(feature = "inspection")]
use std::fs;
use std::process;

#[path = "scena/args.rs"]
mod scena_args;
#[path = "scena/browser_proof.rs"]
mod scena_browser_proof;
#[path = "scena/doctor.rs"]
mod scena_doctor;
#[cfg(feature = "inspection")]
#[path = "scena/examples_agent.rs"]
mod scena_examples_agent;
#[path = "scena/help.rs"]
mod scena_help;
#[path = "scena/input.rs"]
mod scena_input;
#[path = "scena/output.rs"]
mod scena_output;
#[path = "scena/place.rs"]
mod scena_place;
#[path = "scena/schema.rs"]
mod scena_schema;
#[path = "scena/validate_recipe.rs"]
mod scena_validate_recipe;
#[cfg(feature = "inspection")]
#[path = "scena/verify.rs"]
mod scena_verify;
#[cfg(feature = "inspection")]
#[path = "scena/verify_animation.rs"]
mod scena_verify_animation;
#[cfg(feature = "scene-host")]
#[path = "scena/verify_interaction.rs"]
mod scena_verify_interaction;

#[cfg(feature = "inspection")]
use scena_args::{DiagnoseCommandArgs, InspectCommandArgs, RenderCommandArgs, RepairCommandArgs};
use scena_input::resolve_recipe_asset_uri;
#[cfg(all(feature = "inspection", feature = "scene-host"))]
use scena_input::scene_host_from_resolved_recipe;
#[cfg(feature = "inspection")]
use scena_input::{
    appearance_introspection_options, asset_doctor_outcome_or_error, capture_descriptor_path,
    ensure_parent_dir, path_for_json, render_introspection_options, resolve_scene_input,
    viewer_builder,
};
use scena_output::{
    CliOutcome, apply_output_format, json_outcome, json_success, parse_output_format_args, success,
};

fn main() {
    match run(env::args().skip(1).collect()) {
        Ok(outcome) => {
            println!("{}", outcome.stdout);
            if outcome.exit_code != 0 {
                process::exit(outcome.exit_code);
            }
        }
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    }
}

fn run(args: Vec<String>) -> Result<CliOutcome, String> {
    let (args, output_format) = parse_output_format_args(args)?;
    if args.is_empty() || args == ["--help"] || args == ["-h"] {
        return Ok(success(scena_help::help_json()));
    }

    let mut outcome = match args.as_slice() {
        [command, subcommand] if command == "schema" && subcommand == "list" => {
            scena_schema::run_schema_list_command()
        }
        [command, subcommand, schema] if command == "schema" && subcommand == "get" => {
            scena_schema::run_schema_get_command(schema)
        }
        [command, rest @ ..] if command == "validate-recipe" => {
            scena_validate_recipe::run_validate_recipe_command(rest)
        }
        [command, rest @ ..] if command == "place" => scena_place::run_place_command(rest),
        [command, subcommand, rest @ ..] if command == "examples" && subcommand == "agent" => {
            run_examples_agent_command(rest)
        }
        [command, rest @ ..] if command == "render" => run_render_command(rest),
        [command, rest @ ..] if command == "inspect" => run_inspect_command(rest),
        [command, rest @ ..] if command == "diagnose" => run_diagnose_command(rest),
        [command, rest @ ..] if command == "doctor" => scena_doctor::run_doctor_command(rest),
        [command, rest @ ..] if command == "browser-proof" => {
            scena_browser_proof::run_browser_proof_command(rest)
        }
        [command, rest @ ..] if command == "repair" => run_repair_command(rest),
        [command, subcommand, rest @ ..] if command == "verify" && subcommand == "appearance" => {
            run_verify_appearance_command(rest)
        }
        [command, subcommand, rest @ ..] if command == "verify" && subcommand == "animation" => {
            run_verify_animation_command(rest)
        }
        [command, subcommand, rest @ ..] if command == "verify" && subcommand == "interaction" => {
            run_verify_interaction_command(rest)
        }
        _ => Err(
            "unknown command; expected 'schema list', 'schema get <scena.*.vN>', \
             'validate-recipe <recipe.json>', \
             'place <recipe.json> --import <id> --verb <verb>', \
             'examples agent <template> [--out <dir>]', \
             'render <asset> --introspect --out <png>', or \
             'inspect <asset>', or \
             'diagnose <asset> --visibility [--handle <u64>]', or \
             'doctor <asset-or-recipe>', or \
             'browser-proof [scene-host|m6] [--dry-run]', or \
             'repair <asset-or-recipe> --from <report.json>', or \
             'verify appearance <asset-or-recipe> --expect <json>', or \
             'verify animation <asset-or-recipe> --clip <name> --times <seconds>', or \
             'verify interaction <asset-or-recipe> --expect <json>'"
                .to_string(),
        ),
    }?;
    apply_output_format(&mut outcome, output_format)?;
    Ok(outcome)
}

#[cfg(feature = "inspection")]
fn run_examples_agent_command(args: &[String]) -> Result<CliOutcome, String> {
    scena_examples_agent::run_examples_agent_command(args)
}

#[cfg(not(feature = "inspection"))]
fn run_examples_agent_command(_args: &[String]) -> Result<CliOutcome, String> {
    Err(
        "examples agent requires building the scena binary with the 'inspection' feature"
            .to_string(),
    )
}

#[cfg(feature = "inspection")]
fn run_render_command(args: &[String]) -> Result<CliOutcome, String> {
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
        viewer_builder(input.asset.as_str(), width, height, input.transform)
            .with_default_light()
            .render(),
    ) {
        Ok(first) => first,
        Err(error) => {
            return asset_doctor_outcome_or_error(&input.asset, "render", error.to_string());
        }
    };
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

#[cfg(all(feature = "inspection", feature = "scene-host"))]
fn run_render_scene_host_recipe(
    input: scena_input::ResolvedSceneInput,
    width: u32,
    height: u32,
    args: RenderCommandArgs,
) -> Result<CliOutcome, String> {
    let mut host = pollster::block_on(scene_host_from_resolved_recipe(&input, width, height))?;
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

#[cfg(all(feature = "inspection", not(feature = "scene-host")))]
fn run_render_scene_host_recipe(
    _input: scena_input::ResolvedSceneInput,
    _width: u32,
    _height: u32,
    _args: RenderCommandArgs,
) -> Result<CliOutcome, String> {
    Err(
        "recipe overlay directives require building the scena binary with the 'scene-host' feature"
            .to_string(),
    )
}

#[cfg(not(feature = "inspection"))]
fn run_render_command(_args: &[String]) -> Result<CliOutcome, String> {
    Err(
        "render --introspect requires building the scena binary with the 'inspection' feature"
            .to_string(),
    )
}

#[cfg(feature = "inspection")]
fn run_inspect_command(args: &[String]) -> Result<CliOutcome, String> {
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
        viewer_builder(input.asset.as_str(), width, height, input.transform)
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

#[cfg(all(feature = "inspection", feature = "scene-host"))]
fn run_inspect_scene_host_recipe(
    input: scena_input::ResolvedSceneInput,
    width: u32,
    height: u32,
) -> Result<CliOutcome, String> {
    let host = pollster::block_on(scene_host_from_resolved_recipe(&input, width, height))?;
    let text = host
        .inspect_json()
        .map_err(|error| format!("failed to inspect recipe scene: {error}"))?;
    Ok(CliOutcome {
        stdout: text,
        exit_code: 0,
    })
}

#[cfg(all(feature = "inspection", not(feature = "scene-host")))]
fn run_inspect_scene_host_recipe(
    _input: scena_input::ResolvedSceneInput,
    _width: u32,
    _height: u32,
) -> Result<CliOutcome, String> {
    Err(
        "recipe overlay directives require building the scena binary with the 'scene-host' feature"
            .to_string(),
    )
}

#[cfg(not(feature = "inspection"))]
fn run_inspect_command(_args: &[String]) -> Result<CliOutcome, String> {
    Err("inspect requires building the scena binary with the 'inspection' feature".to_string())
}

#[cfg(feature = "inspection")]
fn run_diagnose_command(args: &[String]) -> Result<CliOutcome, String> {
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
        viewer_builder(input.asset.as_str(), width, height, input.transform)
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

#[cfg(all(feature = "inspection", feature = "scene-host"))]
fn run_diagnose_scene_host_recipe(
    input: scena_input::ResolvedSceneInput,
    width: u32,
    height: u32,
    args: DiagnoseCommandArgs,
) -> Result<CliOutcome, String> {
    let mut host = pollster::block_on(scene_host_from_resolved_recipe(&input, width, height))?;
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

#[cfg(all(feature = "inspection", not(feature = "scene-host")))]
fn run_diagnose_scene_host_recipe(
    _input: scena_input::ResolvedSceneInput,
    _width: u32,
    _height: u32,
    _args: DiagnoseCommandArgs,
) -> Result<CliOutcome, String> {
    Err(
        "recipe overlay directives require building the scena binary with the 'scene-host' feature"
            .to_string(),
    )
}

#[cfg(not(feature = "inspection"))]
fn run_diagnose_command(_args: &[String]) -> Result<CliOutcome, String> {
    Err(
        "diagnose --visibility requires building the scena binary with the 'inspection' feature"
            .to_string(),
    )
}

#[cfg(feature = "inspection")]
fn run_repair_command(args: &[String]) -> Result<CliOutcome, String> {
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

#[cfg(not(feature = "inspection"))]
fn run_repair_command(_args: &[String]) -> Result<CliOutcome, String> {
    Err("repair requires building the scena binary with the 'inspection' feature".to_string())
}

#[cfg(feature = "inspection")]
fn run_verify_appearance_command(args: &[String]) -> Result<CliOutcome, String> {
    scena_verify::run_verify_appearance_command(args)
}

#[cfg(not(feature = "inspection"))]
fn run_verify_appearance_command(_args: &[String]) -> Result<CliOutcome, String> {
    Err(
        "verify appearance requires building the scena binary with the 'inspection' feature"
            .to_string(),
    )
}

#[cfg(feature = "inspection")]
fn run_verify_animation_command(args: &[String]) -> Result<CliOutcome, String> {
    scena_verify_animation::run_verify_animation_command(args)
}

#[cfg(not(feature = "inspection"))]
fn run_verify_animation_command(_args: &[String]) -> Result<CliOutcome, String> {
    Err(
        "verify animation requires building the scena binary with the 'inspection' feature"
            .to_string(),
    )
}

#[cfg(feature = "scene-host")]
fn run_verify_interaction_command(args: &[String]) -> Result<CliOutcome, String> {
    scena_verify_interaction::run_verify_interaction_command(args)
}

#[cfg(not(feature = "scene-host"))]
fn run_verify_interaction_command(_args: &[String]) -> Result<CliOutcome, String> {
    Err(
        "verify interaction requires building the scena binary with the 'scene-host' feature"
            .to_string(),
    )
}
