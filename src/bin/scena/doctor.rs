use super::CliOutcome;
#[cfg(feature = "inspection")]
use super::scena_args::DoctorCommandArgs;
#[cfg(feature = "inspection")]
use super::scena_input::resolve_scene_input;
#[cfg(all(feature = "inspection", feature = "scene-host"))]
use super::scena_input::scene_host_manifest_from_resolved_recipe;
#[cfg(feature = "inspection")]
use super::scena_output::json_outcome;
#[cfg(feature = "inspection")]
use super::scena_policy::{effective_recipe_policy, ensure_recipe_policy_applies};

#[cfg(feature = "inspection")]
pub(crate) fn run_doctor_command(args: &[String]) -> Result<CliOutcome, String> {
    let args = DoctorCommandArgs::parse(args)?;
    let policy = effective_recipe_policy(&args.allow_roots, None)?;
    let input = match resolve_scene_input(&args.input, policy) {
        Ok(input) => input,
        Err(outcome) => return Ok(outcome),
    };
    ensure_recipe_policy_applies(input.is_recipe(), &args.allow_roots)?;
    if input.is_recipe() {
        return run_doctor_recipe(input);
    }
    let report = pollster::block_on(scena::Assets::new().doctor_asset_path(input.asset.as_str()));
    let exit_code = if report.ok { 0 } else { 1 };
    json_outcome(
        &report,
        exit_code,
        "failed to serialize asset doctor report",
    )
}

#[cfg(all(feature = "inspection", feature = "scene-host"))]
fn run_doctor_recipe(input: super::scena_input::ResolvedSceneInput) -> Result<CliOutcome, String> {
    let report = pollster::block_on(scene_host_manifest_from_resolved_recipe(&input))?;
    let exit_code = if report.ok { 0 } else { 1 };
    json_outcome(
        &report,
        exit_code,
        "failed to serialize recipe doctor build result",
    )
}

#[cfg(all(feature = "inspection", not(feature = "scene-host")))]
fn run_doctor_recipe(_input: super::scena_input::ResolvedSceneInput) -> Result<CliOutcome, String> {
    Err("doctor for scene recipes requires the scene-host feature".to_owned())
}

#[cfg(not(feature = "inspection"))]
pub(crate) fn run_doctor_command(_args: &[String]) -> Result<CliOutcome, String> {
    Err("doctor requires building the scena binary with the 'inspection' feature".to_string())
}
