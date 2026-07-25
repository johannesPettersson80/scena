use super::scena_cli_error::CliFailure;
/// What a command wrote to stdout.
///
/// X02: response shaping (`--compact`, `--fields`, `--round-floats`,
/// `--include`) rewrites a JSON envelope. Applying it to Markdown produced
/// `internal_error` at exit 70, which tells an agent to file a bug about its
/// own flag choice. The payload kind is carried on the outcome so the decision
/// is made on the type, never by inspecting the bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliPayload {
    /// A serialized JSON envelope; response shaping applies.
    Json,
    /// Human-facing Markdown; response shaping does not apply.
    Markdown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliOutcome {
    pub(crate) stdout: String,
    pub(crate) exit_code: i32,
    pub(crate) payload: CliPayload,
}

/// Why response shaping could not be applied.
///
/// X01/X02: the caller classifies on this type. A wording change in any message
/// below cannot move an invocation between exit classes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CliOutputFormatError {
    /// The caller asked for JSON-only shaping on a non-JSON payload. The fix
    /// belongs to the caller, so this is a usage error.
    JsonShapingOnNonJsonPayload(String),
    /// A payload declared as JSON could not be parsed or re-serialized.
    Internal(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CliOutputFormat {
    round_floats: Option<u8>,
    json_style: Option<CliJsonStyle>,
    /// G08: top-level keys an agent asked to keep. `schema` and `ok` are
    /// always retained so every response stays self-describing.
    fields: Option<Vec<String>>,
    /// G08: sections an agent explicitly asked to receive in full.
    include: Vec<String>,
}

impl CliOutputFormat {
    /// Whether the caller asked for the full constant policy block.
    ///
    /// G08: the block is byte-identical on every call and was 40% of a
    /// measured render response, so it is replaced by a digest by default.
    pub(crate) fn includes_policy(&self) -> bool {
        self.include.iter().any(|section| section == "policy")
    }
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

/// A successful command whose stdout is already-serialized JSON.
pub(crate) fn success(stdout: String) -> CliOutcome {
    CliOutcome {
        stdout,
        exit_code: 0,
        payload: CliPayload::Json,
    }
}

/// A successful command whose stdout is Markdown, not JSON.
pub(crate) fn markdown_success(stdout: String) -> CliOutcome {
    CliOutcome {
        stdout,
        exit_code: 0,
        payload: CliPayload::Markdown,
    }
}

pub(crate) fn json_success<T: serde::Serialize>(
    value: &T,
    context: &str,
) -> Result<CliOutcome, CliFailure> {
    json_outcome(value, 0, context)
}

pub(crate) fn json_outcome<T: serde::Serialize>(
    value: &T,
    exit_code: i32,
    context: &str,
) -> Result<CliOutcome, CliFailure> {
    Ok(CliOutcome {
        stdout: serde_json::to_string_pretty(value)
            .map_err(|error| format!("{context}: {error}"))?,
        exit_code,
        payload: CliPayload::Json,
    })
}

#[cfg(feature = "inspection")]
pub(crate) fn json_outcome_with_backend_selection<T: serde::Serialize>(
    value: &T,
    exit_code: i32,
    context: &str,
    selection: CliBackendSelectionV1,
) -> Result<CliOutcome, CliFailure> {
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
) -> Result<CliOutcome, CliFailure> {
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

/// Stable digest of the recipe policy block.
///
/// G08: agents need to know *which* policy applied without re-reading an
/// identical 1.3 KB block every turn. The digest changes if and only if the
/// policy does.
#[cfg(feature = "inspection")]
pub(crate) fn recipe_policy_digest(policy: &scena::RecipeBuildPolicyReportV1) -> String {
    use sha2::Digest as _;
    let canonical = serde_json::to_vec(policy).expect("recipe policy is serializable");
    let digest = sha2::Sha256::digest(&canonical);
    let hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("sha256:{hex}")
}

#[cfg(feature = "inspection")]
pub(crate) fn add_recipe_policy_to_outcome(
    mut outcome: CliOutcome,
    policy: &scena::RecipeBuildPolicyReportV1,
) -> Result<CliOutcome, CliFailure> {
    let mut value: serde_json::Value = serde_json::from_str(&outcome.stdout)
        .map_err(|error| format!("failed to attach recipe policy to JSON output: {error}"))?;
    let object = value.as_object_mut().ok_or_else(|| {
        "failed to attach recipe policy: result envelope must be a JSON object".to_owned()
    })?;
    object.insert(
        "policy_digest".to_owned(),
        serde_json::Value::String(recipe_policy_digest(policy)),
    );
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
    let mut fields: Option<Vec<String>> = None;
    let mut include: Vec<String> = Vec::new();
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
        } else if args[index] == "--fields" {
            let Some(value) = args.get(index + 1) else {
                return Err("--fields requires a comma-separated field list".to_owned());
            };
            let requested = value
                .split(',')
                .map(str::trim)
                .filter(|field| !field.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>();
            if requested.is_empty() {
                return Err("--fields requires at least one field name".to_owned());
            }
            fields = Some(requested);
            index += 2;
        } else if args[index] == "--include" {
            let Some(value) = args.get(index + 1) else {
                return Err("--include requires a section name".to_owned());
            };
            for section in value.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                if section != "policy" {
                    return Err(format!("--include supports 'policy', got '{section}'"));
                }
                include.push(section.to_owned());
            }
            index += 2;
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
            fields,
            include,
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
) -> Result<(), CliOutputFormatError> {
    // The policy block is stripped unconditionally unless requested, so this
    // runs even when no formatting flag was supplied.
    let strips_policy = !output_format.includes_policy();
    let requested_json_shaping = output_format.round_floats.is_some()
        || output_format.json_style.is_some()
        || output_format.fields.is_some()
        || !output_format.include.is_empty();
    if outcome.payload == CliPayload::Markdown {
        // X02: shaping a Markdown payload is the caller's mistake, not an
        // internal fault, and the implicit policy strip must not touch it.
        if requested_json_shaping {
            return Err(CliOutputFormatError::JsonShapingOnNonJsonPayload(
                "JSON response shaping (--compact, --pretty, --round-floats, --fields, --include) \
                 does not apply to Markdown output; drop the flag or request --json instead"
                    .to_owned(),
            ));
        }
        return Ok(());
    }
    if !requested_json_shaping && !strips_policy {
        return Ok(());
    }
    let mut value =
        serde_json::from_str::<serde_json::Value>(&outcome.stdout).map_err(|error| {
            CliOutputFormatError::Internal(format!(
                "JSON output formatting requires a JSON command result: {error}"
            ))
        })?;
    if let Some(digits) = output_format.round_floats {
        round_json_numbers(&mut value, digits);
    }
    if strips_policy
        && let Some(object) = value.as_object_mut()
        && object.contains_key("policy_digest")
    {
        object.remove("policy");
    }
    if let Some(fields) = output_format.fields.as_deref()
        && let Some(object) = value.as_object_mut()
    {
        // `schema` and `ok` always survive so a projected response stays
        // self-describing and its success is still readable.
        object.retain(|key, _| {
            key == "schema" || key == "ok" || fields.iter().any(|field| field == key)
        });
    }
    outcome.stdout = serialize_json(
        &value,
        output_format.json_style.unwrap_or(CliJsonStyle::Pretty),
    )
    .map_err(|error| {
        CliOutputFormatError::Internal(format!(
            "failed to serialize formatted JSON output: {error}"
        ))
    })?;
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
