use super::CliOutcome;
#[cfg(feature = "inspection")]
use super::scena_args::DoctorCommandArgs;
#[cfg(feature = "inspection")]
use super::{json_outcome, resolve_scene_input};

#[cfg(feature = "inspection")]
pub(crate) fn run_doctor_command(args: &[String]) -> Result<CliOutcome, String> {
    let args = DoctorCommandArgs::parse(args)?;
    let input = match resolve_scene_input(&args.input) {
        Ok(input) => input,
        Err(outcome) => return Ok(outcome),
    };
    let report = pollster::block_on(scena::Assets::new().doctor_asset_path(input.asset.as_str()));
    let exit_code = if report.ok { 0 } else { 1 };
    json_outcome(
        &report,
        exit_code,
        "failed to serialize asset doctor report",
    )
}

#[cfg(not(feature = "inspection"))]
pub(crate) fn run_doctor_command(_args: &[String]) -> Result<CliOutcome, String> {
    Err("doctor requires building the scena binary with the 'inspection' feature".to_string())
}
