use std::env;
use std::fs;
use std::path::Path;
#[cfg(feature = "inspection")]
use std::path::PathBuf;
use std::process;

#[path = "scena/args.rs"]
mod scena_args;
#[path = "scena/place.rs"]
mod scena_place;

use scena_args::ValidateRecipeCommandArgs;
#[cfg(feature = "inspection")]
use scena_args::{DiagnoseCommandArgs, InspectCommandArgs, RenderCommandArgs, RepairCommandArgs};

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

struct CliOutcome {
    stdout: String,
    exit_code: i32,
}

fn run(args: Vec<String>) -> Result<CliOutcome, String> {
    if args.is_empty() || args == ["--help"] || args == ["-h"] {
        return Ok(success(help_json()));
    }

    match args.as_slice() {
        [command, subcommand] if command == "schema" && subcommand == "list" => json_success(
            &scena::schema_catalog_v1(),
            "failed to serialize schema catalog",
        ),
        [command, subcommand, schema] if command == "schema" && subcommand == "get" => {
            let report = scena::schema_entry_report_v1(schema).ok_or_else(|| {
                let suggestion = scena::nearest_schema_name(schema)
                    .map(|name| format!("; did you mean '{name}'?"))
                    .unwrap_or_default();
                format!("unknown schema '{schema}'{suggestion}")
            })?;
            json_success(&report, "failed to serialize schema entry")
        }
        [command, rest @ ..] if command == "validate-recipe" => run_validate_recipe_command(rest),
        [command, rest @ ..] if command == "place" => scena_place::run_place_command(rest),
        [command, rest @ ..] if command == "render" => run_render_command(rest),
        [command, rest @ ..] if command == "inspect" => run_inspect_command(rest),
        [command, rest @ ..] if command == "diagnose" => run_diagnose_command(rest),
        [command, rest @ ..] if command == "repair" => run_repair_command(rest),
        _ => Err(
            "unknown command; expected 'schema list', 'schema get <scena.*.vN>', \
             'validate-recipe <recipe.json>', \
             'place <recipe.json> --import <id> --verb <verb>', \
             'render <asset> --introspect --out <png>', or \
             'inspect <asset>', or \
             'diagnose <asset> --visibility [--handle <u64>]', or \
             'repair <asset-or-recipe> --from <report.json>'"
                .to_string(),
        ),
    }
}

fn run_validate_recipe_command(args: &[String]) -> Result<CliOutcome, String> {
    let recipe_path = ValidateRecipeCommandArgs::parse(args)?.recipe;
    let text = fs::read_to_string(&recipe_path)
        .map_err(|error| format!("failed to read recipe '{}': {error}", recipe_path.display()))?;
    let report = scena::validate_scene_recipe_json(&text);
    let exit_code = if report.ok { 0 } else { 1 };
    json_outcome(
        &report,
        exit_code,
        "failed to serialize scene recipe validation report",
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
    let first = pollster::block_on(
        viewer_builder(input.asset.as_str(), width, height, input.transform)
            .with_default_light()
            .render(),
    )
    .map_err(|error| format!("failed to render '{}': {error}", input.asset))?;
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
    let viewer = pollster::block_on(
        viewer_builder(input.asset.as_str(), width, height, input.transform)
            .with_default_light()
            .build(),
    )
    .map_err(|error| format!("failed to inspect '{}': {error}", input.asset))?;
    let report = viewer
        .scene()
        .inspect_with_assets(viewer.assets())
        .to_schema_report();
    json_success(&report, "failed to serialize scene inspection report")
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
    let first = pollster::block_on(
        viewer_builder(input.asset.as_str(), width, height, input.transform)
            .with_default_light()
            .render(),
    )
    .map_err(|error| format!("failed to render '{}': {error}", input.asset))?;
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

fn success(stdout: String) -> CliOutcome {
    CliOutcome {
        stdout,
        exit_code: 0,
    }
}

fn json_success<T: serde::Serialize>(value: &T, context: &str) -> Result<CliOutcome, String> {
    json_outcome(value, 0, context)
}

fn json_outcome<T: serde::Serialize>(
    value: &T,
    exit_code: i32,
    context: &str,
) -> Result<CliOutcome, String> {
    Ok(CliOutcome {
        stdout: serde_json::to_string_pretty(value)
            .map_err(|error| format!("{context}: {error}"))?,
        exit_code,
    })
}

#[cfg(feature = "inspection")]
#[derive(Debug, Clone, PartialEq)]
struct ResolvedSceneInput {
    asset: String,
    transform: Option<scena::Transform>,
    width: Option<u32>,
    height: Option<u32>,
}

#[cfg(feature = "inspection")]
fn resolve_scene_input(input: &str) -> Result<ResolvedSceneInput, CliOutcome> {
    match try_load_recipe(input)? {
        Some(recipe) => {
            let import = recipe
                .imports
                .first()
                .expect("validated scene recipe contains an import");
            let asset = resolve_recipe_asset_uri(input, &import.uri);
            Ok(ResolvedSceneInput {
                asset,
                transform: import.transform,
                width: recipe.capture.as_ref().map(|capture| capture.width),
                height: recipe.capture.as_ref().map(|capture| capture.height),
            })
        }
        None => Ok(ResolvedSceneInput {
            asset: input.to_owned(),
            transform: None,
            width: None,
            height: None,
        }),
    }
}

#[cfg(feature = "inspection")]
fn viewer_builder(
    asset: &str,
    width: u32,
    height: u32,
    transform: Option<scena::Transform>,
) -> scena::HeadlessGltfViewerBuilder {
    let builder = scena::headless_gltf_viewer(asset).size(width, height);
    if let Some(transform) = transform {
        builder.with_import_transform(transform)
    } else {
        builder
    }
}

#[cfg(feature = "inspection")]
fn try_load_recipe(input: &str) -> Result<Option<scena::SceneRecipeV1>, CliOutcome> {
    let path = Path::new(input);
    let Ok(text) = fs::read_to_string(path) else {
        return Ok(None);
    };
    let is_recipe_path = input.ends_with(".recipe.json");
    let parsed = serde_json::from_str::<serde_json::Value>(&text);
    let is_recipe_schema = parsed
        .as_ref()
        .ok()
        .and_then(|value| value.get("schema"))
        .and_then(serde_json::Value::as_str)
        == Some(scena::SCENE_RECIPE_SCHEMA_V1);
    if !is_recipe_path && !is_recipe_schema {
        return Ok(None);
    }
    match scena::parse_valid_scene_recipe_json(&text) {
        Ok(recipe) => Ok(Some(recipe)),
        Err(report) => {
            let outcome = json_outcome(
                &report,
                1,
                "failed to serialize scene recipe validation report",
            )
            .expect("scene recipe validation report serializes");
            Err(outcome)
        }
    }
}

fn resolve_recipe_asset_uri(recipe_path: &str, uri: &str) -> String {
    let uri_path = Path::new(uri);
    if uri_path.is_absolute() || uri.contains("://") || uri.starts_with("data:") {
        return uri.to_owned();
    }
    let relative_to_recipe = Path::new(recipe_path)
        .parent()
        .map(|parent| parent.join(uri));
    if let Some(path) = relative_to_recipe.filter(|path| path.exists()) {
        return path.display().to_string();
    }
    uri.to_owned()
}

#[cfg(feature = "inspection")]
fn render_introspection_options(detail: bool) -> scena::RenderIntrospectionOptions {
    if detail {
        scena::RenderIntrospectionOptions::detail()
    } else {
        scena::RenderIntrospectionOptions::summary()
    }
}

#[cfg(feature = "inspection")]
fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|error| {
            format!("failed to create directory '{}': {error}", parent.display())
        })?;
    }
    Ok(())
}

#[cfg(feature = "inspection")]
fn capture_descriptor_path(png_path: &Path) -> PathBuf {
    let stem = png_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("capture");
    png_path.with_file_name(format!("{stem}.capture.json"))
}

#[cfg(feature = "inspection")]
fn path_for_json(path: &Path) -> String {
    path.display().to_string()
}

fn help_json() -> String {
    serde_json::json!({
        "schema": "scena.cli_help.v1",
        "commands": [
            "schema list",
            "schema get <scena.*.vN>",
            "validate-recipe <recipe.json>",
            "place <recipe.json> --import <id> --verb <verb>",
            "render <asset-or-recipe> --introspect --out <png>",
            "inspect <asset-or-recipe>",
            "diagnose <asset-or-recipe> --visibility [--handle <u64>]",
            "repair <asset-or-recipe> --from <report.json>"
        ]
    })
    .to_string()
}
