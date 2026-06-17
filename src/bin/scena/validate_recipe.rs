use super::scena_args::ValidateRecipeCommandArgs;
use super::scena_output::{CliOutcome, json_outcome};
use std::fs;

pub(crate) fn run_validate_recipe_command(args: &[String]) -> Result<CliOutcome, String> {
    let recipe_path = ValidateRecipeCommandArgs::parse(args)?.recipe;
    let text = fs::read_to_string(&recipe_path)
        .map_err(|error| format!("failed to read recipe '{}': {error}", recipe_path.display()))?;
    let recipe_path = recipe_path
        .to_str()
        .ok_or_else(|| format!("recipe path '{}' is not valid UTF-8", recipe_path.display()))?;
    let assets = scena::Assets::new();
    let report = pollster::block_on(scena::validate_scene_recipe_json_with_assets(
        recipe_path,
        &text,
        &assets,
    ));
    emit_report(report)
}

fn emit_report(report: scena::SceneRecipeValidationReportV1) -> Result<CliOutcome, String> {
    let exit_code = if report.ok { 0 } else { 1 };
    json_outcome(
        &report,
        exit_code,
        "failed to serialize scene recipe validation report",
    )
}
