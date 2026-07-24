use std::env;
use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

#[path = "scena/process_output_shared.rs"]
mod process_output;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputMode {
    Json,
    Human,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConvertOptions {
    input: PathBuf,
    output: PathBuf,
    tool: String,
    dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConvertOutcome {
    stdout: Option<String>,
    stderr: Option<String>,
    exit_code: i32,
}

fn main() {
    let outcome = run(env::args().skip(1).collect());
    if let Some(stdout) = outcome.stdout
        && let Err(error) = process_output::write_stdout_line(&stdout)
    {
        if error.kind() == io::ErrorKind::BrokenPipe {
            return;
        }
        process_output::write_stdout_error(&error, true);
        process::exit(process_output::IO_ERROR_EXIT_CODE);
    }
    if let Some(stderr) = outcome.stderr {
        process_output::write_stderr_line(&stderr);
    }
    if outcome.exit_code != 0 {
        process::exit(outcome.exit_code);
    }
}

fn run(args: Vec<String>) -> ConvertOutcome {
    let (args, explicit_mode) = match parse_output_mode(args) {
        Ok(parsed) => parsed,
        Err(error) => return human_error(error, 2),
    };
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        return help_outcome(explicit_mode.unwrap_or(OutputMode::Human));
    }
    let mode = explicit_mode.unwrap_or(OutputMode::Json);
    let options = match parse_options(args).and_then(|options| {
        validate_extensions(&options)?;
        Ok(options)
    }) {
        Ok(options) => options,
        Err(error) => return error_outcome(mode, error, 2, None),
    };

    if options.dry_run {
        return conversion_outcome(
            mode,
            report_for_options(
                &options,
                scena::AssetConversionStatusV1::Planned,
                true,
                "conversion planned; no external tool was started".to_owned(),
                Vec::new(),
                None,
            ),
            0,
        );
    }

    match mode {
        OutputMode::Json => run_machine_conversion(&options),
        OutputMode::Human => run_human_conversion(&options),
    }
}

fn run_machine_conversion(options: &ConvertOptions) -> ConvertOutcome {
    match converter_command(options).output() {
        Ok(output) => {
            let ok = output.status.success();
            let status = if ok {
                scena::AssetConversionStatusV1::Converted
            } else {
                scena::AssetConversionStatusV1::ConversionFailed
            };
            let message = if ok {
                "conversion completed successfully".to_owned()
            } else {
                format!(
                    "{} exited with status {}; inspect captured diagnostics",
                    options.tool, output.status
                )
            };
            let diagnostics = captured_diagnostics(&output.stdout, &output.stderr, ok);
            conversion_outcome(
                OutputMode::Json,
                report_for_options(
                    options,
                    status,
                    ok,
                    message,
                    diagnostics,
                    output.status.code(),
                ),
                if ok { 0 } else { 1 },
            )
        }
        Err(error) => conversion_outcome(
            OutputMode::Json,
            report_for_options(
                options,
                scena::AssetConversionStatusV1::ToolUnavailable,
                false,
                format!(
                    "failed to start {}: {error}; install FBX2glTF or pass --tool <converter>",
                    options.tool
                ),
                Vec::new(),
                None,
            ),
            1,
        ),
    }
}

fn run_human_conversion(options: &ConvertOptions) -> ConvertOutcome {
    match converter_command(options).status() {
        Ok(status) if status.success() => ConvertOutcome {
            stdout: Some(format!(
                "Converted {} to {} with {}.",
                options.input.display(),
                options.output.display(),
                options.tool
            )),
            stderr: None,
            exit_code: 0,
        },
        Ok(status) => human_error(
            format!(
                "{} exited with status {status}; inspect converter diagnostics",
                options.tool
            ),
            1,
        ),
        Err(error) => human_error(
            format!(
                "failed to start {}: {error}; install FBX2glTF or pass --tool <converter>",
                options.tool
            ),
            1,
        ),
    }
}

fn converter_command(options: &ConvertOptions) -> Command {
    let mut command = Command::new(&options.tool);
    command
        .arg("--input")
        .arg(&options.input)
        .arg("--output")
        .arg(&options.output);
    command
}

fn report_for_options(
    options: &ConvertOptions,
    status: scena::AssetConversionStatusV1,
    ok: bool,
    message: String,
    diagnostics: Vec<scena::AssetConversionDiagnosticV1>,
    tool_exit_code: Option<i32>,
) -> scena::AssetConversionReportV1 {
    scena::AssetConversionReportV1 {
        schema: scena::ASSET_CONVERSION_SCHEMA_V1.to_owned(),
        ok,
        status,
        workflow: "fbx_to_gltf".to_owned(),
        tool: Some(options.tool.clone()),
        input: Some(options.input.to_string_lossy().into_owned()),
        output: Some(options.output.to_string_lossy().into_owned()),
        command: command_args(options),
        diagnostics,
        tool_exit_code,
        message,
    }
}

fn error_outcome(
    mode: OutputMode,
    message: String,
    exit_code: i32,
    options: Option<&ConvertOptions>,
) -> ConvertOutcome {
    if mode == OutputMode::Human {
        return human_error(message, exit_code);
    }
    let report = scena::AssetConversionReportV1 {
        schema: scena::ASSET_CONVERSION_SCHEMA_V1.to_owned(),
        ok: false,
        status: scena::AssetConversionStatusV1::InvalidRequest,
        workflow: "fbx_to_gltf".to_owned(),
        tool: options.map(|options| options.tool.clone()),
        input: options.map(|options| options.input.to_string_lossy().into_owned()),
        output: options.map(|options| options.output.to_string_lossy().into_owned()),
        command: options.map(command_args).unwrap_or_default(),
        diagnostics: Vec::new(),
        tool_exit_code: None,
        message,
    };
    conversion_outcome(OutputMode::Json, report, exit_code)
}

fn conversion_outcome(
    mode: OutputMode,
    report: scena::AssetConversionReportV1,
    exit_code: i32,
) -> ConvertOutcome {
    if mode == OutputMode::Human {
        let text = format!(
            "Planned FBX to glTF conversion: {} -> {} with {}.",
            report.input.as_deref().unwrap_or("<missing input>"),
            report.output.as_deref().unwrap_or("<missing output>"),
            report.tool.as_deref().unwrap_or("<missing tool>")
        );
        return ConvertOutcome {
            stdout: Some(text),
            stderr: None,
            exit_code,
        };
    }
    match serde_json::to_string(&report) {
        Ok(stdout) => ConvertOutcome {
            stdout: Some(stdout),
            stderr: None,
            exit_code,
        },
        Err(error) => human_error(
            format!("failed to serialize conversion report: {error}"),
            70,
        ),
    }
}

fn captured_diagnostics(
    stdout: &[u8],
    stderr: &[u8],
    success: bool,
) -> Vec<scena::AssetConversionDiagnosticV1> {
    let mut diagnostics = Vec::new();
    append_diagnostics(
        &mut diagnostics,
        stdout,
        scena::AssetConversionDiagnosticStreamV1::Stdout,
        scena::AssetConversionDiagnosticSeverityV1::Info,
    );
    append_diagnostics(
        &mut diagnostics,
        stderr,
        scena::AssetConversionDiagnosticStreamV1::Stderr,
        if success {
            scena::AssetConversionDiagnosticSeverityV1::Warning
        } else {
            scena::AssetConversionDiagnosticSeverityV1::Error
        },
    );
    diagnostics
}

fn append_diagnostics(
    diagnostics: &mut Vec<scena::AssetConversionDiagnosticV1>,
    bytes: &[u8],
    stream: scena::AssetConversionDiagnosticStreamV1,
    severity: scena::AssetConversionDiagnosticSeverityV1,
) {
    for message in String::from_utf8_lossy(bytes).lines() {
        diagnostics.push(scena::AssetConversionDiagnosticV1 {
            stream,
            severity,
            message: message.to_owned(),
        });
    }
}

fn command_args(options: &ConvertOptions) -> Vec<String> {
    vec![
        options.tool.clone(),
        "--input".to_owned(),
        options.input.to_string_lossy().into_owned(),
        "--output".to_owned(),
        options.output.to_string_lossy().into_owned(),
    ]
}

fn parse_output_mode(args: Vec<String>) -> Result<(Vec<String>, Option<OutputMode>), String> {
    let mut filtered = Vec::with_capacity(args.len());
    let mut mode = None;
    for arg in args {
        let requested = match arg.as_str() {
            "--json" => Some(OutputMode::Json),
            "--human" => Some(OutputMode::Human),
            _ => None,
        };
        if let Some(requested) = requested {
            if mode.is_some_and(|mode| mode != requested) {
                return Err("--json and --human cannot be used together".to_owned());
            }
            mode = Some(requested);
        } else {
            filtered.push(arg);
        }
    }
    Ok((filtered, mode))
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

fn help_outcome(mode: OutputMode) -> ConvertOutcome {
    if mode == OutputMode::Json {
        let help = serde_json::json!({
            "schema": "scena.cli_help.v1",
            "scope": "binary",
            "command": "scena-convert",
            "usage": usage(),
            "output_modes": ["--json", "--human"],
            "output_schemas": [scena::ASSET_CONVERSION_SCHEMA_V1],
        });
        return ConvertOutcome {
            stdout: Some(help.to_string()),
            stderr: None,
            exit_code: 0,
        };
    }
    ConvertOutcome {
        stdout: Some(help_text().to_owned()),
        stderr: None,
        exit_code: 0,
    }
}

fn human_error(message: String, exit_code: i32) -> ConvertOutcome {
    ConvertOutcome {
        stdout: None,
        stderr: Some(message),
        exit_code,
    }
}

fn usage() -> &'static str {
    "scena-convert --input model.fbx --output model.glb [--tool FBX2glTF] [--dry-run] [--json|--human]"
}

fn help_text() -> &'static str {
    "scena-convert\n\nPlans or runs the FBX to glTF/GLB asset-conversion workflow.\n\nUsage:\n  scena-convert --input model.fbx --output model.glb [--tool FBX2glTF] [--dry-run] [--json|--human]\n\nMachine mode is the compatibility default for conversion commands and emits exactly one scena.asset_conversion.v1 document. Pass --human for plain text and streamed converter diagnostics. The command delegates actual conversion to FBX2glTF or a compatible converter. Use --dry-run in CI to verify the workflow without requiring the external tool."
}

#[cfg(test)]
mod tests {
    use super::{ConvertOptions, OutputMode, run};

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
        ]);
        assert_eq!(output.exit_code, 0);
        let output = output.stdout.expect("dry run emits JSON");
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
        assert_ne!(OutputMode::Json, OutputMode::Human);
    }
}
