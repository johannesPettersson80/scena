use super::scena_args::ValidateRecipeCommandArgs;
use super::scena_input::{RecipeReadError, read_recipe_text};
use super::scena_output::{CliOutcome, json_outcome};
use super::scena_policy::effective_recipe_policy;

pub(crate) fn run_validate_recipe_command(args: &[String]) -> Result<CliOutcome, String> {
    let args = ValidateRecipeCommandArgs::parse(args)?;
    let recipe_path = args.recipe;
    let policy = effective_recipe_policy(&args.allow_roots, args.max_imports)?;
    let text = match read_recipe_text(&recipe_path, &policy) {
        Ok(text) => text,
        Err(RecipeReadError::TooLarge(mut report)) => {
            report.policy = Some(Box::new(policy.to_schema_report()));
            return emit_report(report);
        }
        Err(RecipeReadError::Io(error)) => {
            return Err(format!(
                "failed to read recipe '{}': {error}",
                recipe_path.display()
            ));
        }
    };
    let recipe_path = recipe_path
        .to_str()
        .ok_or_else(|| format!("recipe path '{}' is not valid UTF-8", recipe_path.display()))?;
    let report = if args.syntax_only {
        scena::validate_scene_recipe_json_syntax_with_policy(&text, &policy)
    } else {
        let assets = scena::Assets::new();
        pollster::block_on(scena::validate_scene_recipe_json_with_assets_and_policy(
            recipe_path,
            &text,
            &assets,
            &policy,
        ))
    };
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
