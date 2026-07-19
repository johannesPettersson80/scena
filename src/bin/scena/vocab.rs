use super::scena_output::{CliOutcome, json_success};

pub(crate) fn run_vocab_list_command() -> Result<CliOutcome, String> {
    json_success(
        &scena::vocabulary_report_v1(),
        "failed to serialize vocabulary report",
    )
}

pub(crate) fn run_vocab_get_command(name: &str) -> Result<CliOutcome, String> {
    let vocabulary = scena::vocabulary_v1(name)
        .ok_or_else(|| format!("unknown vocabulary '{name}'; run 'scena vocab list'"))?;
    json_success(
        &scena::VocabularyReportV1 {
            schema: scena::VOCABULARY_SCHEMA_V1.to_owned(),
            vocabularies: vec![vocabulary],
        },
        "failed to serialize vocabulary",
    )
}
