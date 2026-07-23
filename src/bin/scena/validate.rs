use std::fs;
use std::path::Path;

use super::scena_output::{CliOutcome, json_outcome};

pub(crate) fn run_validate_command(args: &[String]) -> Result<CliOutcome, String> {
    let [path] = args else {
        return Err("usage: scena validate <file>".to_owned());
    };
    let path = Path::new(path);
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read contract '{}': {error}", path.display()))?;
    let report = scena::validate_contract_json_v1(&text);
    let exit_code = if report.ok { 0 } else { 65 };
    json_outcome(
        &report,
        exit_code,
        "failed to serialize contract validation report",
    )
}
