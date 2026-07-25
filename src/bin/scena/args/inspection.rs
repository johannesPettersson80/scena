use crate::scena_cli_error::CliUsageError;
use std::path::PathBuf;

use super::super::scena_policy::push_allow_root;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DoctorCommandArgs {
    pub(crate) input: String,
    pub(crate) allow_roots: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RenderCommandArgs {
    pub(crate) input: String,
    pub(crate) out: PathBuf,
    pub(crate) width: Option<u32>,
    pub(crate) height: Option<u32>,
    pub(crate) detail: bool,
    pub(crate) gpu: bool,
    pub(crate) timings: bool,
    pub(crate) allow_roots: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DiagnoseCommandArgs {
    pub(crate) input: String,
    pub(crate) handle: Option<u64>,
    pub(crate) width: Option<u32>,
    pub(crate) height: Option<u32>,
    pub(crate) detail: bool,
    pub(crate) allow_roots: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepairCommandArgs {
    pub(crate) input: String,
    pub(crate) from: PathBuf,
    pub(crate) iteration_budget: u32,
    pub(crate) allow_roots: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InspectCommandArgs {
    pub(crate) input: String,
    pub(crate) width: Option<u32>,
    pub(crate) height: Option<u32>,
    pub(crate) allow_roots: Vec<PathBuf>,
}

impl DoctorCommandArgs {
    pub(crate) fn parse(args: &[String]) -> Result<Self, CliUsageError> {
        let Some(input) = args.first() else {
            return Err(CliUsageError::from(doctor_usage()));
        };
        let mut allow_roots = Vec::new();
        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
                "--json" => index += 1,
                "--allow-root" => {
                    push_allow_root(args, index, &mut allow_roots)?;
                    index += 2;
                }
                flag => {
                    return Err(CliUsageError::from(format!(
                        "unknown doctor flag '{flag}'; {}",
                        doctor_usage()
                    )));
                }
            }
        }
        Ok(Self {
            input: input.clone(),
            allow_roots,
        })
    }
}

impl RenderCommandArgs {
    pub(crate) fn parse(args: &[String]) -> Result<Self, CliUsageError> {
        let Some(input) = args.first() else {
            return Err(CliUsageError::from(render_usage()));
        };
        let mut out = None;
        let mut width = None;
        let mut height = None;
        let mut detail = false;
        let mut gpu = false;
        let mut timings = false;
        let mut allow_roots = Vec::new();
        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
                "--introspect" => {
                    index += 1;
                }
                "--out" => {
                    out = Some(PathBuf::from(flag_value(args, index, "--out")?));
                    index += 2;
                }
                "--width" => {
                    width = Some(parse_positive_u32(
                        "--width",
                        flag_value(args, index, "--width")?,
                    )?);
                    index += 2;
                }
                "--height" => {
                    height = Some(parse_positive_u32(
                        "--height",
                        flag_value(args, index, "--height")?,
                    )?);
                    index += 2;
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
                "--allow-root" => {
                    push_allow_root(args, index, &mut allow_roots)?;
                    index += 2;
                }
                "--json" => index += 1,
                flag => {
                    return Err(CliUsageError::from(format!(
                        "unknown render flag '{flag}'; {}",
                        render_usage()
                    )));
                }
            }
        }
        Ok(Self {
            input: input.clone(),
            out: out.ok_or_else(|| {
                CliUsageError::from(format!("missing --out <png>; {}", render_usage()))
            })?,
            width,
            height,
            detail,
            gpu,
            timings,
            allow_roots,
        })
    }
}

impl InspectCommandArgs {
    pub(crate) fn parse(args: &[String]) -> Result<Self, CliUsageError> {
        let Some(input) = args.first() else {
            return Err(CliUsageError::from(inspect_usage()));
        };
        let mut width = None;
        let mut height = None;
        let mut allow_roots = Vec::new();
        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
                "--width" => {
                    width = Some(parse_positive_u32(
                        "--width",
                        flag_value(args, index, "--width")?,
                    )?);
                    index += 2;
                }
                "--height" => {
                    height = Some(parse_positive_u32(
                        "--height",
                        flag_value(args, index, "--height")?,
                    )?);
                    index += 2;
                }
                "--json" => index += 1,
                "--allow-root" => {
                    push_allow_root(args, index, &mut allow_roots)?;
                    index += 2;
                }
                flag => {
                    return Err(CliUsageError::from(format!(
                        "unknown inspect flag '{flag}'; {}",
                        inspect_usage()
                    )));
                }
            }
        }
        Ok(Self {
            input: input.clone(),
            width,
            height,
            allow_roots,
        })
    }
}

impl DiagnoseCommandArgs {
    pub(crate) fn parse(args: &[String]) -> Result<Self, CliUsageError> {
        let Some(input) = args.first() else {
            return Err(CliUsageError::from(diagnose_usage()));
        };
        let mut visibility = false;
        let mut handle = None;
        let mut width = None;
        let mut height = None;
        let mut detail = false;
        let mut allow_roots = Vec::new();
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
                    width = Some(parse_positive_u32(
                        "--width",
                        flag_value(args, index, "--width")?,
                    )?);
                    index += 2;
                }
                "--height" => {
                    height = Some(parse_positive_u32(
                        "--height",
                        flag_value(args, index, "--height")?,
                    )?);
                    index += 2;
                }
                "--detail" => {
                    detail = true;
                    index += 1;
                }
                "--allow-root" => {
                    push_allow_root(args, index, &mut allow_roots)?;
                    index += 2;
                }
                "--json" => index += 1,
                flag => {
                    return Err(CliUsageError::from(format!(
                        "unknown diagnose flag '{flag}'; {}",
                        diagnose_usage()
                    )));
                }
            }
        }
        if !visibility {
            return Err(CliUsageError::from(format!(
                "missing --visibility; {}",
                diagnose_usage()
            )));
        }
        Ok(Self {
            input: input.clone(),
            handle,
            width,
            height,
            detail,
            allow_roots,
        })
    }
}

impl RepairCommandArgs {
    pub(crate) fn parse(args: &[String]) -> Result<Self, CliUsageError> {
        let Some(input) = args.first() else {
            return Err(CliUsageError::from(repair_usage()));
        };
        let mut from = None;
        let mut iteration_budget = 3;
        let mut allow_roots = Vec::new();
        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
                "--from" => {
                    from = Some(PathBuf::from(flag_value(args, index, "--from")?));
                    index += 2;
                }
                "--iteration-budget" => {
                    iteration_budget = parse_u32(
                        "--iteration-budget",
                        flag_value(args, index, "--iteration-budget")?,
                    )?;
                    index += 2;
                }
                "--allow-root" => {
                    push_allow_root(args, index, &mut allow_roots)?;
                    index += 2;
                }
                "--json" => index += 1,
                flag => {
                    return Err(CliUsageError::from(format!(
                        "unknown repair flag '{flag}'; {}",
                        repair_usage()
                    )));
                }
            }
        }
        Ok(Self {
            input: input.clone(),
            from: from.ok_or_else(|| {
                CliUsageError::from(format!("missing --from <report.json>; {}", repair_usage()))
            })?,
            iteration_budget,
            allow_roots,
        })
    }
}

fn flag_value(args: &[String], index: usize, flag: &str) -> Result<String, CliUsageError> {
    args.get(index + 1)
        .cloned()
        .ok_or_else(|| CliUsageError::from(format!("{flag} requires a value")))
}

fn parse_positive_u32(flag: &str, value: String) -> Result<u32, CliUsageError> {
    let parsed = parse_u32(flag, value)?;
    if parsed == 0 {
        return Err(CliUsageError::from(format!(
            "{flag} requires a positive integer, got 0"
        )));
    }
    Ok(parsed)
}

fn parse_u32(flag: &str, value: String) -> Result<u32, CliUsageError> {
    value.parse::<u32>().map_err(|_| {
        CliUsageError::from(format!(
            "{flag} requires an unsigned integer, got '{value}'"
        ))
    })
}

fn parse_u64(flag: &str, value: String) -> Result<u64, CliUsageError> {
    value.parse::<u64>().map_err(|_| {
        CliUsageError::from(format!(
            "{flag} requires an unsigned integer, got '{value}'"
        ))
    })
}

fn render_usage() -> String {
    "usage: scena render <asset-or-recipe> --out <png> [--introspect] [--gpu] [--timings] [--width <px>] [--height <px>] [--detail] [--allow-root <directory>]... [--round-floats <0..6>]"
        .to_string()
}

fn inspect_usage() -> String {
    "usage: scena inspect <asset-or-recipe> [--width <px>] [--height <px>] [--allow-root <directory>]... [--round-floats <0..6>]"
        .to_string()
}

fn diagnose_usage() -> String {
    "usage: scena diagnose <asset-or-recipe> --visibility [--handle <u64>] [--width <px>] [--height <px>] [--detail] [--allow-root <directory>]... [--round-floats <0..6>]"
        .to_string()
}

fn doctor_usage() -> String {
    "usage: scena doctor <asset-or-recipe> [--allow-root <directory>]... [--json] [--round-floats <0..6>]".to_string()
}

fn repair_usage() -> String {
    "usage: scena repair <asset-or-recipe> --from <diagnosis-or-introspection.json> [--iteration-budget <n>] [--allow-root <directory>]... [--round-floats <0..6>]"
        .to_string()
}
