use std::env;
#[cfg(feature = "inspection")]
use std::fs;
#[cfg(feature = "inspection")]
use std::path::{Path, PathBuf};
use std::process;

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
        [command, rest @ ..] if command == "render" => run_render_command(rest),
        [command, rest @ ..] if command == "inspect" => run_inspect_command(rest),
        [command, rest @ ..] if command == "diagnose" => run_diagnose_command(rest),
        _ => Err(
            "unknown command; expected 'schema list', 'schema get <scena.*.vN>', \
             'render <asset> --introspect --out <png>', or \
             'inspect <asset>', or \
             'diagnose <asset> --visibility [--handle <u64>]'"
                .to_string(),
        ),
    }
}

#[cfg(feature = "inspection")]
fn run_render_command(args: &[String]) -> Result<CliOutcome, String> {
    let args = RenderCommandArgs::parse(args)?;
    let first = pollster::block_on(
        scena::headless_gltf_viewer(args.asset.as_str())
            .size(args.width, args.height)
            .with_default_light()
            .render(),
    )
    .map_err(|error| format!("failed to render '{}': {error}", args.asset))?;
    let capture = first
        .capture()
        .map_err(|error| format!("failed to capture '{}': {error}", args.asset))?;

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
    let viewer = pollster::block_on(
        scena::headless_gltf_viewer(args.asset.as_str())
            .size(args.width, args.height)
            .with_default_light()
            .build(),
    )
    .map_err(|error| format!("failed to inspect '{}': {error}", args.asset))?;
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
    let first = pollster::block_on(
        scena::headless_gltf_viewer(args.asset.as_str())
            .size(args.width, args.height)
            .with_default_light()
            .render(),
    )
    .map_err(|error| format!("failed to render '{}': {error}", args.asset))?;
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
#[derive(Debug, Clone, PartialEq, Eq)]
struct RenderCommandArgs {
    asset: String,
    out: PathBuf,
    width: u32,
    height: u32,
    detail: bool,
}

#[cfg(feature = "inspection")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct DiagnoseCommandArgs {
    asset: String,
    handle: Option<u64>,
    width: u32,
    height: u32,
    detail: bool,
}

#[cfg(feature = "inspection")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct InspectCommandArgs {
    asset: String,
    width: u32,
    height: u32,
}

#[cfg(feature = "inspection")]
impl RenderCommandArgs {
    fn parse(args: &[String]) -> Result<Self, String> {
        let Some(asset) = args.first() else {
            return Err(render_usage());
        };
        let mut introspect = false;
        let mut out = None;
        let mut width = 800;
        let mut height = 600;
        let mut detail = false;

        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
                "--introspect" => {
                    introspect = true;
                    index += 1;
                }
                "--out" => {
                    let value = flag_value(args, index, "--out")?;
                    out = Some(PathBuf::from(value));
                    index += 2;
                }
                "--width" => {
                    width = parse_positive_u32("--width", flag_value(args, index, "--width")?)?;
                    index += 2;
                }
                "--height" => {
                    height = parse_positive_u32("--height", flag_value(args, index, "--height")?)?;
                    index += 2;
                }
                "--detail" => {
                    detail = true;
                    index += 1;
                }
                "--json" => {
                    index += 1;
                }
                flag => return Err(format!("unknown render flag '{flag}'; {}", render_usage())),
            }
        }

        if !introspect {
            return Err(format!("missing --introspect; {}", render_usage()));
        }
        let out = out.ok_or_else(|| format!("missing --out <png>; {}", render_usage()))?;

        Ok(Self {
            asset: asset.clone(),
            out,
            width,
            height,
            detail,
        })
    }
}

#[cfg(feature = "inspection")]
impl InspectCommandArgs {
    fn parse(args: &[String]) -> Result<Self, String> {
        let Some(asset) = args.first() else {
            return Err(inspect_usage());
        };
        let mut width = 800;
        let mut height = 600;

        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
                "--width" => {
                    width = parse_positive_u32("--width", flag_value(args, index, "--width")?)?;
                    index += 2;
                }
                "--height" => {
                    height = parse_positive_u32("--height", flag_value(args, index, "--height")?)?;
                    index += 2;
                }
                "--json" => {
                    index += 1;
                }
                flag => {
                    return Err(format!(
                        "unknown inspect flag '{flag}'; {}",
                        inspect_usage()
                    ));
                }
            }
        }

        Ok(Self {
            asset: asset.clone(),
            width,
            height,
        })
    }
}

#[cfg(feature = "inspection")]
impl DiagnoseCommandArgs {
    fn parse(args: &[String]) -> Result<Self, String> {
        let Some(asset) = args.first() else {
            return Err(diagnose_usage());
        };
        let mut visibility = false;
        let mut handle = None;
        let mut width = 800;
        let mut height = 600;
        let mut detail = false;

        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
                "--visibility" => {
                    visibility = true;
                    index += 1;
                }
                "--handle" => {
                    handle = Some(parse_u64("--handle", flag_value(args, index, "--handle")?)?);
                    index += 2;
                }
                "--width" => {
                    width = parse_positive_u32("--width", flag_value(args, index, "--width")?)?;
                    index += 2;
                }
                "--height" => {
                    height = parse_positive_u32("--height", flag_value(args, index, "--height")?)?;
                    index += 2;
                }
                "--detail" => {
                    detail = true;
                    index += 1;
                }
                "--json" => {
                    index += 1;
                }
                flag => {
                    return Err(format!(
                        "unknown diagnose flag '{flag}'; {}",
                        diagnose_usage()
                    ));
                }
            }
        }

        if !visibility {
            return Err(format!("missing --visibility; {}", diagnose_usage()));
        }

        Ok(Self {
            asset: asset.clone(),
            handle,
            width,
            height,
            detail,
        })
    }
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
fn render_introspection_options(detail: bool) -> scena::RenderIntrospectionOptions {
    if detail {
        scena::RenderIntrospectionOptions::detail()
    } else {
        scena::RenderIntrospectionOptions::summary()
    }
}

#[cfg(feature = "inspection")]
fn flag_value(args: &[String], index: usize, flag: &str) -> Result<String, String> {
    args.get(index + 1)
        .cloned()
        .ok_or_else(|| format!("{flag} requires a value"))
}

#[cfg(feature = "inspection")]
fn parse_positive_u32(flag: &str, value: String) -> Result<u32, String> {
    let parsed = value
        .parse::<u32>()
        .map_err(|_| format!("{flag} requires a positive integer, got '{value}'"))?;
    if parsed == 0 {
        return Err(format!("{flag} requires a positive integer, got 0"));
    }
    Ok(parsed)
}

#[cfg(feature = "inspection")]
fn parse_u64(flag: &str, value: String) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("{flag} requires an unsigned integer, got '{value}'"))
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

#[cfg(feature = "inspection")]
fn render_usage() -> String {
    "usage: scena render <asset> --introspect --out <png> [--width <px>] [--height <px>] [--detail]"
        .to_string()
}

#[cfg(feature = "inspection")]
fn inspect_usage() -> String {
    "usage: scena inspect <asset> [--width <px>] [--height <px>]".to_string()
}

#[cfg(feature = "inspection")]
fn diagnose_usage() -> String {
    "usage: scena diagnose <asset> --visibility [--handle <u64>] [--width <px>] [--height <px>] [--detail]"
        .to_string()
}

fn help_json() -> String {
    serde_json::json!({
        "schema": "scena.cli_help.v1",
        "commands": [
            "schema list",
            "schema get <scena.*.vN>",
            "render <asset> --introspect --out <png>",
            "inspect <asset>",
            "diagnose <asset> --visibility [--handle <u64>]"
        ]
    })
    .to_string()
}
