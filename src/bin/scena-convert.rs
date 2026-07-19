use std::env;
use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

#[path = "scena/process_output_shared.rs"]
mod process_output;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConvertOptions {
    input: PathBuf,
    output: PathBuf,
    tool: String,
    dry_run: bool,
}

fn main() {
    match run(env::args().skip(1).collect()) {
        Ok(Some(stdout)) => {
            if let Err(error) = process_output::write_stdout_line(&stdout) {
                if error.kind() == io::ErrorKind::BrokenPipe {
                    return;
                }
                process_output::write_stdout_error(&error);
                process::exit(process_output::IO_ERROR_EXIT_CODE);
            }
        }
        Ok(None) => {}
        Err(error) => {
            process_output::write_stderr_line(&error);
            process::exit(2);
        }
    }
}

fn run(args: Vec<String>) -> Result<Option<String>, String> {
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        return Ok(Some(help_text().to_owned()));
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
        return serde_json::to_string(&serde_json::json!({
            "status": "planned",
            "workflow": "FBX to glTF",
            "tool": &options.tool,
            "input": input.as_ref(),
            "output": output.as_ref(),
            "command": command,
        }))
        .map(Some)
        .map_err(|error| format!("failed to serialize conversion plan: {error}"));
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
        Ok(None)
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

fn help_text() -> &'static str {
    "scena-convert\n\nPlans or runs the FBX to glTF/GLB asset-conversion workflow.\n\nUsage:\n  scena-convert --input model.fbx --output model.glb [--tool FBX2glTF] [--dry-run]\n\nThe command delegates actual conversion to FBX2glTF or a compatible converter. Use --dry-run in CI to verify the workflow without requiring the external tool."
}

#[cfg(test)]
mod tests {
    use super::{ConvertOptions, run};

    #[test]
    fn dry_run_machine_json_round_trips_all_controls_and_unicode() {
        let controls = (0_u8..=0x1f).map(char::from).collect::<String>();
        let tool = format!("tool-{controls}-€");
        let output = run(vec![
            "--input".to_owned(),
            "model.fbx".to_owned(),
            "--output".to_owned(),
            "model.glb".to_owned(),
            "--tool".to_owned(),
            tool.clone(),
            "--dry-run".to_owned(),
        ])
        .expect("dry run succeeds")
        .expect("dry run emits JSON");
        let value: serde_json::Value = serde_json::from_str(&output).expect("output is valid JSON");
        assert_eq!(value["tool"], tool);
        assert!(!output.bytes().any(|byte| byte < 0x20));
    }

    #[test]
    fn convert_options_type_remains_constructible_for_parser_contract() {
        let options = ConvertOptions {
            input: "model.fbx".into(),
            output: "model.glb".into(),
            tool: "converter".to_owned(),
            dry_run: true,
        };
        assert!(options.dry_run);
    }
}
