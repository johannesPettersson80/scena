use super::scena_output::{CliOutcome, json_success, success};

pub(crate) fn run_agent_guide_command(args: &[String]) -> Result<CliOutcome, String> {
    match args {
        [] => json_success(
            &scena::agent_guide_v1(),
            "failed to serialize public agent guide",
        ),
        [flag] if flag == "--json" => json_success(
            &scena::agent_guide_v1(),
            "failed to serialize public agent guide",
        ),
        [flag] if flag == "--markdown" => Ok(success(scena::agent_guide_v1().markdown)),
        _ => Err(
            "guide agent accepts exactly one output mode: --json or --markdown; usage: scena guide agent [--json|--markdown]"
                .to_owned(),
        ),
    }
}
