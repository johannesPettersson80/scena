use super::scena_output::{CliOutcome, json_success};

pub(crate) fn run_recipe_policy_command() -> Result<CliOutcome, String> {
    json_success(
        &scena::RecipeBuildPolicy::testing().to_schema_report(),
        "failed to serialize effective recipe policy",
    )
}
