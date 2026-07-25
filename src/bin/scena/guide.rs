use super::scena_cli_error::CliFailure;
use super::scena_output::{CliOutcome, json_success, markdown_success};

pub(crate) fn run_agent_guide_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    match args {
        [] => json_success(
            &scena::agent_guide_v1(),
            "failed to serialize public agent guide",
        ),
        [flag] if flag == "--json" => json_success(
            &scena::agent_guide_v1(),
            "failed to serialize public agent guide",
        ),
        [flag] if flag == "--markdown" => Ok(markdown_success(scena::agent_guide_v1().markdown)),
        // G08: the machine-readable contract without the embedded prose guide.
        // The `--json` form is 93.6% markdown by bytes, so an agent that only
        // needs the command/schema/template surface paid ~7k tokens of prose
        // to reach ~250 tokens of contract.
        [flag] if flag == "--contract" => {
            let guide = scena::agent_guide_v1();
            let mut value = serde_json::to_value(&guide)
                .map_err(|error| format!("failed to serialize public agent guide: {error}"))?;
            if let Some(object) = value.as_object_mut() {
                object.remove("markdown");
            }
            json_success(&value, "failed to serialize public agent guide")
        }
        _ => Err(CliFailure::invalid_arguments(
            "guide agent accepts exactly one output mode: --json, --markdown, or --contract; usage: scena guide agent [--json|--markdown|--contract]",
        )),
    }
}
