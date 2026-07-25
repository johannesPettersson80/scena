use super::scena_cli_error::CliFailure;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use scena::BrowserProofRunV1;

use super::scena_output::{CliOutcome, json_outcome};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BrowserProofLane {
    SceneHost,
    M6,
}

#[derive(Debug, Clone)]
struct BrowserProofArgs {
    lane: BrowserProofLane,
    backend: String,
    dry_run: bool,
}

pub(crate) fn run_browser_proof_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    let args = BrowserProofArgs::parse(args)?;
    let mut report = args.to_report();
    if args.dry_run {
        return json_outcome(&report, 0, "failed to serialize browser proof run report");
    }

    remove_stale_artifact(&report.artifact_json_path)?;
    if let Some(path) = &report.artifact_png_path {
        remove_stale_artifact(path)?;
    }

    let command_args = report.command.clone();
    let Some((program, rest)) = command_args.split_first() else {
        return Err(CliFailure::invalid_arguments(
            "browser-proof command cannot be empty",
        ));
    };
    let mut command = Command::new(program);
    command.args(rest);
    command.env("SCENA_BROWSER_BACKENDS", &args.backend);
    let output = command
        .output()
        .map_err(|error| format!("failed to run browser proof command: {error}"))?;
    let exit_code = output.status.code().unwrap_or(1);
    report.dry_run = false;
    report.status = if output.status.success() {
        "passed".to_owned()
    } else {
        "failed".to_owned()
    };
    report.exit_code = exit_code;
    report.artifact_json_exists = Some(Path::new(&report.artifact_json_path).exists());
    report.artifact_png_exists = report
        .artifact_png_path
        .as_ref()
        .map(|path| Path::new(path).exists());
    if !output.status.success() {
        report.stdout_tail = text_tail(&output.stdout, 4000);
        report.stderr_tail = text_tail(&output.stderr, 4000);
    }
    json_outcome(
        &report,
        if output.status.success() { 0 } else { 1 },
        "failed to serialize browser proof run report",
    )
}

fn remove_stale_artifact(path: &str) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "failed to remove stale browser proof artifact '{path}': {error}"
        )),
    }
}

fn text_tail(bytes: &[u8], limit: usize) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    let text = String::from_utf8_lossy(bytes);
    if text.len() <= limit {
        return Some(text.into_owned());
    }
    let start = text
        .char_indices()
        .map(|(index, _)| index)
        .find(|index| text.len() - index <= limit)
        .unwrap_or(0);
    Some(text[start..].to_owned())
}

impl BrowserProofArgs {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut lane = BrowserProofLane::SceneHost;
        let mut backend = "webgl2".to_owned();
        let mut dry_run = false;
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "scene-host" => lane = BrowserProofLane::SceneHost,
                "m6" => lane = BrowserProofLane::M6,
                "--backend" => {
                    index += 1;
                    backend = args
                        .get(index)
                        .ok_or_else(|| "browser-proof --backend requires a value".to_string())?
                        .clone();
                    if backend.trim().is_empty() {
                        return Err("browser-proof --backend must not be empty".to_string());
                    }
                }
                "--dry-run" => dry_run = true,
                "--help" | "-h" => {
                    return Err(
                        "usage: scena browser-proof [scene-host|m6] [--backend webgl2] [--dry-run]"
                            .to_string(),
                    );
                }
                other => {
                    return Err(format!(
                        "unknown browser-proof argument '{other}'; expected scene-host, m6, --backend, or --dry-run"
                    ));
                }
            }
            index += 1;
        }
        Ok(Self {
            lane,
            backend,
            dry_run,
        })
    }

    fn to_report(&self) -> BrowserProofRunV1 {
        let mut env = BTreeMap::new();
        env.insert("SCENA_BROWSER_BACKENDS".to_owned(), self.backend.clone());
        BrowserProofRunV1::dry_run(
            self.lane.name(),
            self.lane.script(),
            self.lane.command(),
            env,
            self.lane.artifact_json_path(),
            self.lane.artifact_png_path().map(str::to_owned),
        )
    }
}

impl BrowserProofLane {
    const fn name(self) -> &'static str {
        match self {
            Self::SceneHost => "scene-host",
            Self::M6 => "m6",
        }
    }

    const fn script(self) -> &'static str {
        match self {
            Self::SceneHost => "browser:scene-host-proof",
            Self::M6 => "browser:m6",
        }
    }

    fn command(self) -> Vec<String> {
        match self {
            Self::SceneHost => vec![
                "npm".to_owned(),
                "run".to_owned(),
                self.script().to_owned(),
            ],
            Self::M6 => vec![
                "bash".to_owned(),
                "-lc".to_owned(),
                "wasm-pack build --dev --target web --out-dir target/m6-browser-pkg . --features browser-probe && npm run browser:m6".to_owned(),
            ],
        }
    }

    const fn artifact_json_path(self) -> &'static str {
        match self {
            Self::SceneHost => {
                "target/gate-artifacts/scene-host-browser-proof/scene-host-browser-proof.json"
            }
            Self::M6 => "target/gate-artifacts/m6-rust-wasm-renderer-probe.json",
        }
    }

    const fn artifact_png_path(self) -> Option<&'static str> {
        match self {
            Self::SceneHost => {
                Some("target/gate-artifacts/scene-host-browser-proof/scene-host-browser-proof.png")
            }
            Self::M6 => Some("target/gate-artifacts/scena-viewer-element-browser-proof.png"),
        }
    }
}
