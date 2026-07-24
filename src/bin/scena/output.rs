#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliOutcome {
    pub(crate) stdout: String,
    pub(crate) exit_code: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CliOutputFormat {
    round_floats: Option<u8>,
    json_style: Option<CliJsonStyle>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliJsonStyle {
    Pretty,
    Compact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[cfg(feature = "inspection")]
pub(crate) struct CliBackendSelectionV1 {
    source: &'static str,
    requested: scena::Backend,
    selected: Option<scena::Backend>,
    fallback_used: bool,
    reason: Option<&'static str>,
    remedy: Option<&'static str>,
}

#[cfg(feature = "inspection")]
impl CliBackendSelectionV1 {
    pub(crate) fn new(gpu_flag: bool, selected: Option<scena::Backend>) -> Self {
        let requested = if gpu_flag {
            scena::Backend::HeadlessGpu
        } else {
            scena::Backend::Headless
        };
        let fallback_used = matches!(
            (requested, selected),
            (scena::Backend::HeadlessGpu, Some(scena::Backend::Headless))
        );
        Self {
            source: if gpu_flag { "cli_flag" } else { "default" },
            requested,
            selected,
            fallback_used,
            reason: fallback_used.then_some(
                "the requested headless GPU backend was unavailable, so the renderer selected the deterministic CPU headless backend",
            ),
            remedy: fallback_used.then_some(
                "run `scena capabilities --gpu` to inspect adapter initialization, then install or configure a supported graphics driver",
            ),
        }
    }
}

pub(crate) fn success(stdout: String) -> CliOutcome {
    CliOutcome {
        stdout,
        exit_code: 0,
    }
}

pub(crate) fn json_success<T: serde::Serialize>(
    value: &T,
    context: &str,
) -> Result<CliOutcome, String> {
    json_outcome(value, 0, context)
}

pub(crate) fn json_outcome<T: serde::Serialize>(
    value: &T,
    exit_code: i32,
    context: &str,
) -> Result<CliOutcome, String> {
    Ok(CliOutcome {
        stdout: serde_json::to_string_pretty(value)
            .map_err(|error| format!("{context}: {error}"))?,
        exit_code,
    })
}

#[cfg(feature = "inspection")]
pub(crate) fn json_outcome_with_backend_selection<T: serde::Serialize>(
    value: &T,
    exit_code: i32,
    context: &str,
    selection: CliBackendSelectionV1,
) -> Result<CliOutcome, String> {
    let mut value = serde_json::to_value(value).map_err(|error| format!("{context}: {error}"))?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| format!("{context}: result envelope must be a JSON object"))?;
    object.insert(
        "backend_selection".to_owned(),
        serde_json::to_value(selection).expect("backend selection is serializable"),
    );
    json_outcome(&value, exit_code, context)
}

#[cfg(all(feature = "inspection", feature = "scene-host"))]
pub(crate) fn add_backend_selection_to_outcome(
    mut outcome: CliOutcome,
    selection: CliBackendSelectionV1,
) -> Result<CliOutcome, String> {
    let mut value: serde_json::Value = serde_json::from_str(&outcome.stdout)
        .map_err(|error| format!("failed to attach backend selection to JSON output: {error}"))?;
    let object = value.as_object_mut().ok_or_else(|| {
        "failed to attach backend selection: result envelope must be a JSON object".to_owned()
    })?;
    object.insert(
        "backend_selection".to_owned(),
        serde_json::to_value(selection).expect("backend selection is serializable"),
    );
    outcome.stdout = serde_json::to_string_pretty(&value)
        .map_err(|error| format!("failed to serialize backend selection: {error}"))?;
    Ok(outcome)
}

#[cfg(feature = "inspection")]
pub(crate) fn add_recipe_policy_to_outcome(
    mut outcome: CliOutcome,
    policy: &scena::RecipeBuildPolicyReportV1,
) -> Result<CliOutcome, String> {
    let mut value: serde_json::Value = serde_json::from_str(&outcome.stdout)
        .map_err(|error| format!("failed to attach recipe policy to JSON output: {error}"))?;
    let object = value.as_object_mut().ok_or_else(|| {
        "failed to attach recipe policy: result envelope must be a JSON object".to_owned()
    })?;
    object.insert(
        "policy".to_owned(),
        serde_json::to_value(policy).expect("recipe policy is serializable"),
    );
    outcome.stdout = serde_json::to_string_pretty(&value)
        .map_err(|error| format!("failed to serialize recipe policy: {error}"))?;
    Ok(outcome)
}

pub(crate) fn parse_output_format_args(
    args: Vec<String>,
) -> Result<(Vec<String>, CliOutputFormat), String> {
    let mut filtered = Vec::with_capacity(args.len());
    let mut round_floats = None;
    let mut json_style = None;
    let mut index = 0;
    while index < args.len() {
        if matches!(args[index].as_str(), "--compact" | "--pretty") {
            let next = if args[index] == "--compact" {
                CliJsonStyle::Compact
            } else {
                CliJsonStyle::Pretty
            };
            if let Some(previous) = json_style
                && previous != next
            {
                return Err("--compact and --pretty are mutually exclusive".to_owned());
            }
            json_style = Some(next);
            index += 1;
        } else if args[index] == "--round-floats" {
            let Some(value) = args.get(index + 1) else {
                return Err("--round-floats requires a value".to_string());
            };
            let digits = value.parse::<u8>().map_err(|_| {
                format!("--round-floats requires an integer from 0 to 6, got '{value}'")
            })?;
            if digits > 6 {
                return Err(format!(
                    "--round-floats requires an integer from 0 to 6, got '{value}'"
                ));
            }
            round_floats = Some(digits);
            index += 2;
        } else {
            filtered.push(args[index].clone());
            index += 1;
        }
    }
    Ok((
        filtered,
        CliOutputFormat {
            round_floats,
            json_style,
        },
    ))
}

pub(crate) fn requested_json_style(args: &[String]) -> CliJsonStyle {
    let compact = args.iter().any(|arg| arg == "--compact");
    let pretty = args.iter().any(|arg| arg == "--pretty");
    if compact && !pretty {
        CliJsonStyle::Compact
    } else {
        CliJsonStyle::Pretty
    }
}

pub(crate) fn serialize_json<T: serde::Serialize>(
    value: &T,
    style: CliJsonStyle,
) -> Result<String, serde_json::Error> {
    match style {
        CliJsonStyle::Pretty => serde_json::to_string_pretty(value),
        CliJsonStyle::Compact => serde_json::to_string(value),
    }
}

pub(crate) fn apply_output_format(
    outcome: &mut CliOutcome,
    output_format: CliOutputFormat,
) -> Result<(), String> {
    if output_format.round_floats.is_none() && output_format.json_style.is_none() {
        return Ok(());
    }
    let mut value =
        serde_json::from_str::<serde_json::Value>(&outcome.stdout).map_err(|error| {
            format!("JSON output formatting requires a JSON command result: {error}")
        })?;
    if let Some(digits) = output_format.round_floats {
        round_json_numbers(&mut value, digits);
    }
    outcome.stdout = serialize_json(
        &value,
        output_format.json_style.unwrap_or(CliJsonStyle::Pretty),
    )
    .map_err(|error| format!("failed to serialize formatted JSON output: {error}"))?;
    Ok(())
}

fn round_json_numbers(value: &mut serde_json::Value, digits: u8) {
    match value {
        serde_json::Value::Number(number) => {
            if number.is_i64() || number.is_u64() {
                return;
            }
            let Some(float) = number.as_f64() else {
                return;
            };
            if !float.is_finite() {
                return;
            }
            let scale = 10_f64.powi(i32::from(digits));
            let rounded = (float * scale).round() / scale;
            if let Some(next) = serde_json::Number::from_f64(rounded) {
                *number = next;
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                round_json_numbers(value, digits);
            }
        }
        serde_json::Value::Object(values) => {
            for value in values.values_mut() {
                round_json_numbers(value, digits);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::{CliJsonStyle, json_outcome, serialize_json};

    #[test]
    fn explicit_json_styles_match_byte_stable_goldens() {
        let value = serde_json::json!({
            "ok": true,
            "schema": "test.a12.format_fixture",
            "values": [1, 2, 3],
        });
        let compact = serialize_json(&value, CliJsonStyle::Compact).expect("compact serializes");
        let pretty = serialize_json(&value, CliJsonStyle::Pretty).expect("pretty serializes");
        assert_eq!(
            compact,
            include_str!("../../../tests/assets/cli-golden/a12_compact.json").trim_end()
        );
        assert_eq!(
            pretty,
            include_str!("../../../tests/assets/cli-golden/a12_pretty.json").trim_end()
        );
    }

    #[cfg(feature = "inspection")]
    #[test]
    fn gpu_fallback_selection_is_actionable_machine_data() {
        let value = serde_json::to_value(super::CliBackendSelectionV1::new(
            true,
            Some(scena::Backend::Headless),
        ))
        .expect("backend selection serializes");
        assert_eq!(value["requested"], "headless_gpu");
        assert_eq!(value["selected"], "headless");
        assert_eq!(value["fallback_used"], true);
        assert!(
            value["reason"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
        );
        assert!(
            value["remedy"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
        );
    }

    #[test]
    fn machine_json_round_trips_all_controls_and_unicode() {
        let controls = (0_u8..=0x1f).map(char::from).collect::<String>();
        let value = format!("before-{controls}-€-after");
        let outcome = json_outcome(
            &serde_json::json!({"ok": false, "value": &value}),
            1,
            "control fixture serializes",
        )
        .expect("machine JSON serializes");
        let parsed: serde_json::Value =
            serde_json::from_str(&outcome.stdout).expect("machine output is valid JSON");
        assert_eq!(parsed["value"], value);
        assert!(
            !outcome
                .stdout
                .bytes()
                .any(|byte| byte < 0x20 && byte != b'\n')
        );
    }
}
