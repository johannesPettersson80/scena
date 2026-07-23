use super::scena_output::{CliOutcome, json_success};

pub(crate) fn run_schema_list_command() -> Result<CliOutcome, String> {
    json_success(
        &scena::schema_catalog_v1(),
        "failed to serialize schema catalog",
    )
}

pub(crate) fn run_schema_get_command(schema: &str) -> Result<CliOutcome, String> {
    let report = scena::schema_entry_report_v1(schema).ok_or_else(|| {
        let suggestion = scena::nearest_schema_name(schema)
            .map(|name| format!("; did you mean '{name}'?"))
            .unwrap_or_default();
        format!("unknown schema '{schema}'{suggestion}")
    })?;
    json_success(&report, "failed to serialize schema entry")
}

pub(crate) fn run_schema_json_command(schema: &str) -> Result<CliOutcome, String> {
    let report = scena::contract_json_schema_export_v1(schema).ok_or_else(|| {
        let suggestion = scena::nearest_schema_name(schema)
            .map(|name| format!("; did you mean '{name}'?"))
            .unwrap_or_default();
        format!("unknown schema '{schema}'{suggestion}")
    })?;
    json_success(&report, "failed to serialize JSON Schema export")
}
