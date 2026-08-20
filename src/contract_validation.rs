use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const CONTRACT_VALIDATION_SCHEMA_V1: &str = "scena.contract_validation.v1";
pub const JSON_SCHEMA_EXPORT_SCHEMA_V1: &str = "scena.json_schema_export.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractValidationReportV1 {
    pub schema: String,
    /// Whether the checks that ran all passed.
    ///
    /// G07: this is **not** "the payload is fully valid". A contract with
    /// envelope-only support reports `ok: true` after checking the wrapper
    /// alone. Key on [`Self::fully_validated`] before treating a value as
    /// verified.
    pub ok: bool,
    /// Whether the payload itself was validated, not just its envelope.
    ///
    /// Added in 1.10.0; defaults to `false` when deserializing an older
    /// fixture, which is the fail-closed reading.
    #[serde(default)]
    pub fully_validated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract: Option<String>,
    pub validation_level: String,
    pub diagnostics: Vec<ContractValidationDiagnosticV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractValidationDiagnosticV1 {
    pub code: String,
    pub severity: String,
    pub path: String,
    pub message: String,
    pub help: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonSchemaExportV1 {
    pub schema: String,
    pub contract: String,
    pub json_schema: Value,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub limitations: Vec<String>,
}

pub fn validate_contract_json_v1(text: &str) -> ContractValidationReportV1 {
    let value = match serde_json::from_str::<Value>(text) {
        Ok(value) => value,
        Err(error) => {
            return failure(
                None,
                "none",
                diagnostic(
                    "malformed_json",
                    "$",
                    format!("file is not valid JSON: {error}"),
                    "repair the JSON syntax before contract validation",
                ),
            );
        }
    };
    let Some(object) = value.as_object() else {
        return failure(
            None,
            "none",
            diagnostic(
                "contract_mismatch",
                "$",
                "public scena contracts must be JSON objects",
                "emit an object containing a versioned scena.*.vN schema field",
            ),
        );
    };
    let Some(contract) = object
        .get("schema")
        .and_then(Value::as_str)
        .map(str::to_owned)
    else {
        return failure(
            None,
            "none",
            diagnostic(
                "missing_schema",
                "$.schema",
                "contract is missing its versioned schema field",
                "set schema to a name returned by `scena schema list`",
            ),
        );
    };
    if crate::schema_catalog_entry(&contract).is_none() {
        let candidates = crate::nearest_name_candidates(
            &contract,
            crate::schema_catalog_v1()
                .entries
                .iter()
                .map(|entry| entry.schema.as_str()),
            3,
        );
        let mut report = failure(
            Some(&contract),
            "none",
            diagnostic(
                "unknown_schema",
                "$.schema",
                format!("schema '{contract}' is not in the public catalog"),
                "select a schema returned by `scena schema list`",
            ),
        );
        report.diagnostics[0].candidates = candidates;
        return report;
    }

    match contract.as_str() {
        crate::SCENE_RECIPE_SCHEMA_V1 => validate_recipe(text),
        #[cfg(feature = "inspection")]
        crate::render::appearance::APPEARANCE_EXPECTATION_SCHEMA_V1 => {
            validate_typed::<crate::render::appearance::AppearanceExpectationV1>(
                value,
                &contract,
                |expectation| expectation.validate_schema(),
            )
        }
        #[cfg(feature = "scene-host")]
        crate::scene_host::INTERACTION_EXPECTATION_SCHEMA_V1 => {
            validate_typed::<crate::scene_host::InteractionExpectationV1>(
                value,
                &contract,
                |expectation| expectation.validate_schema(),
            )
        }
        #[cfg(feature = "scene-host")]
        crate::PHOTO_PLAN_SCHEMA_V1 => validate_typed::<crate::PhotoPlanV1>(
            value,
            &contract,
            |plan| plan.validate_contract().map_err(str::to_owned),
        ),
        #[cfg(feature = "scene-host")]
        crate::PHOTO_REPORT_SCHEMA_V1 => validate_typed::<crate::PhotoReportV1>(
            value,
            &contract,
            |report| report.validate_contract().map_err(str::to_owned),
        ),
        crate::SCENE_RECIPE_PATCH_SCHEMA_V1 => {
            validate_typed::<crate::SceneRecipePatchResultV1>(value, &contract, |patch| {
                patch.validate_schema()
            })
        }
        crate::CAPABILITY_REPORT_SCHEMA_V1 => {
            validate_typed::<crate::CapabilityReportV1>(value, &contract, |report| {
                (report.schema == crate::CAPABILITY_REPORT_SCHEMA_V1)
                    .then_some(())
                    .ok_or_else(|| "capability report schema does not match its type".to_owned())
            })
        }
        #[cfg(feature = "inspection")]
        crate::FOCUS_REPORT_SCHEMA_V1 => {
            validate_typed::<crate::FocusReportV1>(value, &contract, |report| {
                report.validate_contract().map_err(str::to_owned)
            })
        }
        #[cfg(feature = "inspection")]
        crate::EXPOSURE_REPORT_SCHEMA_V1 => {
            validate_typed::<crate::ExposureReportV1>(value, &contract, |report| {
                report.validate_contract().map_err(str::to_owned)
            })
        }
        #[cfg(feature = "inspection")]
        crate::SUBJECT_OBSERVATION_SCHEMA_V1 => {
            validate_typed::<crate::SubjectObservationV1>(value, &contract, |report| {
                report.validate_contract().map_err(str::to_owned)
            })
        }
        _ => ContractValidationReportV1 {
            schema: CONTRACT_VALIDATION_SCHEMA_V1.to_owned(),
            ok: true,
            fully_validated: false,
            contract: Some(contract),
            validation_level: "envelope".to_owned(),
            diagnostics: vec![ContractValidationDiagnosticV1 {
                code: "envelope_validation_only".to_owned(),
                severity: "warning".to_owned(),
                path: "$".to_owned(),
                message:
                    "only the contract envelope was validated; the payload was not checked"
                        .to_owned(),
                help:
                    "validate the producing workflow for runtime semantics, or key on fully_validated"
                        .to_owned(),
                candidates: Vec::new(),
            }],
            limitations: vec![
                "this emitted/report contract has envelope validation only; validate its producing workflow for runtime semantics"
                    .to_owned(),
            ],
        },
    }
}

pub fn contract_json_schema_export_v1(contract: &str) -> Option<JsonSchemaExportV1> {
    crate::schema_catalog_entry(contract)?;
    let (mut json_schema, limitations) = if contract == crate::SCENE_RECIPE_SCHEMA_V1 {
        (
            crate::scene_recipe_json_schema_v1(),
            vec![
                "JSON Schema cannot prove filesystem/resource resolution, sandbox policy, cross-resource identity, or backend/runtime capability availability"
                    .to_owned(),
            ],
        )
    } else {
        (
            json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "$id": format!("https://scena.rs/schema/{contract}"),
                "type": "object",
                "required": ["schema"],
                "properties": {
                    "schema": {"const": contract}
                },
                "additionalProperties": true
            }),
            vec![
                "this export validates the versioned envelope only; use `scena validate` for typed owner validation when available"
                    .to_owned(),
                "runtime, cross-field, filesystem, policy, and backend semantics are outside this JSON Schema"
                    .to_owned(),
            ],
        )
    };
    if let Some(object) = json_schema.as_object_mut() {
        object.entry("$schema".to_owned()).or_insert_with(|| {
            Value::String("https://json-schema.org/draft/2020-12/schema".to_owned())
        });
    }
    Some(JsonSchemaExportV1 {
        schema: JSON_SCHEMA_EXPORT_SCHEMA_V1.to_owned(),
        contract: contract.to_owned(),
        json_schema,
        limitations,
    })
}

fn validate_recipe(text: &str) -> ContractValidationReportV1 {
    let report = crate::validate_scene_recipe_json(text);
    ContractValidationReportV1 {
        schema: CONTRACT_VALIDATION_SCHEMA_V1.to_owned(),
        ok: report.ok,
        fully_validated: report.ok,
        contract: Some(crate::SCENE_RECIPE_SCHEMA_V1.to_owned()),
        validation_level: "typed".to_owned(),
        diagnostics: report
            .diagnostics
            .into_iter()
            .map(|diagnostic| ContractValidationDiagnosticV1 {
                code: diagnostic.code,
                severity: diagnostic.severity,
                path: diagnostic.path,
                message: diagnostic.message,
                help: diagnostic.help,
                candidates: diagnostic.candidates,
            })
            .collect(),
        limitations: vec![
            "recipe validation is syntax/semantic only; run `validate-recipe --full` for resource and sandbox resolution"
                .to_owned(),
        ],
    }
}

fn validate_typed<T: DeserializeOwned>(
    value: Value,
    contract: &str,
    validate: impl FnOnce(&T) -> Result<(), String>,
) -> ContractValidationReportV1 {
    match serde_json::from_value::<T>(value) {
        Ok(parsed) => match validate(&parsed) {
            Ok(()) => ContractValidationReportV1 {
                schema: CONTRACT_VALIDATION_SCHEMA_V1.to_owned(),
                ok: true,
                fully_validated: true,
                contract: Some(contract.to_owned()),
                validation_level: "typed".to_owned(),
                diagnostics: Vec::new(),
                limitations: Vec::new(),
            },
            Err(error) => failure(
                Some(contract),
                "typed",
                diagnostic(
                    "contract_mismatch",
                    "$",
                    error,
                    "repair the value using the owning schema and workflow contract",
                ),
            ),
        },
        Err(error) => failure(
            Some(contract),
            "typed",
            diagnostic(
                "contract_mismatch",
                "$",
                format!("value does not match {contract}: {error}"),
                "repair field types and required fields using `scena schema get`",
            ),
        ),
    }
}

fn failure(
    contract: Option<&str>,
    validation_level: &str,
    diagnostic: ContractValidationDiagnosticV1,
) -> ContractValidationReportV1 {
    ContractValidationReportV1 {
        schema: CONTRACT_VALIDATION_SCHEMA_V1.to_owned(),
        ok: false,
        fully_validated: false,
        contract: contract.map(str::to_owned),
        validation_level: validation_level.to_owned(),
        diagnostics: vec![diagnostic],
        limitations: Vec::new(),
    }
}

fn diagnostic(
    code: &str,
    path: &str,
    message: impl Into<String>,
    help: impl Into<String>,
) -> ContractValidationDiagnosticV1 {
    ContractValidationDiagnosticV1 {
        code: code.to_owned(),
        severity: "error".to_owned(),
        path: path.to_owned(),
        message: message.into(),
        help: help.into(),
        candidates: Vec::new(),
    }
}
