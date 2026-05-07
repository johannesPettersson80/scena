use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConvertOptions {
    input: PathBuf,
    output: PathBuf,
    tool: String,
    dry_run: bool,
}

fn main() {
    match run(env::args().skip(1).collect()) {
        Ok(()) => {}
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return Ok(());
    }

    let options = parse_options(args)?;
    validate_extensions(&options)?;

    let input = options.input.to_string_lossy();
    let output = options.output.to_string_lossy();
    let command = [
        options.tool.as_str(),
        "--input",
        &input,
        "--output",
        &output,
    ];

    if options.dry_run {
        println!(
            "{{\"status\":\"planned\",\"workflow\":\"FBX to glTF\",\"tool\":\"{}\",\"input\":\"{}\",\"output\":\"{}\",\"command\":[\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"]}}",
            json_escape(&options.tool),
            json_escape(&options.input.to_string_lossy()),
            json_escape(&options.output.to_string_lossy()),
            json_escape(command[0]),
            json_escape(command[1]),
            json_escape(command[2]),
            json_escape(command[3]),
            json_escape(command[4]),
        );
        return Ok(());
    }

    let status = Command::new(&options.tool)
        .arg("--input")
        .arg(&options.input)
        .arg("--output")
        .arg(&options.output)
        .status()
        .map_err(|error| {
            format!(
                "failed to start {}: {error}; install FBX2glTF or pass --tool <converter>",
                options.tool
            )
        })?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "{} exited with status {status}; inspect converter diagnostics",
            options.tool
        ))
    }
}

fn parse_options(args: Vec<String>) -> Result<ConvertOptions, String> {
    let mut input = None;
    let mut output = None;
    let mut tool = "FBX2glTF".to_string();
    let mut dry_run = false;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--input" | "-i" => input = args.next().map(PathBuf::from),
            "--output" | "-o" => output = args.next().map(PathBuf::from),
            "--tool" => {
                tool = args
                    .next()
                    .filter(|value| !value.is_empty())
                    .ok_or("--tool requires a converter executable".to_string())?;
            }
            "--dry-run" => dry_run = true,
            other => {
                return Err(format!(
                    "unknown argument '{other}'; run scena-convert --help"
                ));
            }
        }
    }

    Ok(ConvertOptions {
        input: input.ok_or("--input <file.fbx> is required".to_string())?,
        output: output.ok_or("--output <file.gltf|file.glb> is required".to_string())?,
        tool,
        dry_run,
    })
}

fn validate_extensions(options: &ConvertOptions) -> Result<(), String> {
    if !has_extension(&options.input, "fbx") {
        return Err(format!(
            "input must be an FBX file, got {}",
            options.input.display()
        ));
    }
    if !(has_extension(&options.output, "gltf") || has_extension(&options.output, "glb")) {
        return Err(format!(
            "output must end in .gltf or .glb, got {}",
            options.output.display()
        ));
    }
    Ok(())
}

fn has_extension(path: &Path, expected: &str) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| extension.eq_ignore_ascii_case(expected))
}

fn json_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn print_help() {
    println!(
        "scena-convert\n\nPlans or runs the FBX to glTF/GLB asset-conversion workflow.\n\nUsage:\n  scena-convert --input model.fbx --output model.glb [--tool FBX2glTF] [--dry-run]\n\nThe command delegates actual conversion to FBX2glTF or a compatible converter. Use --dry-run in CI to verify the workflow without requiring the external tool."
    );
}
