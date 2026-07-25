use super::scena_cli_error::{CliFailure, CliUsageError};
use std::fs;
use std::path::PathBuf;

use super::scena_input::{
    ResolvedRecipeBuild, resolve_scene_input, scene_host_build_from_resolved_recipe,
};
use super::scena_output::{CliOutcome, json_outcome};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerifyInteractionCommandArgs {
    input: String,
    expect: PathBuf,
}

pub(crate) fn run_verify_interaction_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    let args = VerifyInteractionCommandArgs::parse(args)?;
    let text = fs::read_to_string(&args.expect).map_err(|error| {
        format!(
            "failed to read interaction expectation '{}': {error}",
            args.expect.display()
        )
    })?;
    let expectation: scena::InteractionExpectationV1 =
        serde_json::from_str(&text).map_err(|error| {
            format!(
                "failed to parse interaction expectation '{}': {error}",
                args.expect.display()
            )
        })?;
    expectation.validate_schema()?;

    let input = match resolve_scene_input(&args.input, scena::RecipeBuildPolicy::testing()) {
        Ok(input) => input,
        Err(outcome) => return Ok(outcome),
    };
    let artifacts = scena::InteractionVerificationArtifactsV1::from_viewport(expectation.viewport);
    let mut host = if input.is_recipe() {
        let build = pollster::block_on(scene_host_build_from_resolved_recipe(
            &input,
            artifacts.width_physical_px,
            artifacts.height_physical_px,
            false,
        ))?;
        match build {
            ResolvedRecipeBuild::Built(build) => build.host,
            ResolvedRecipeBuild::Rejected(outcome) => return Ok(outcome),
        }
    } else {
        let mut host = scena::SceneHostCore::headless(
            artifacts.width_physical_px,
            artifacts.height_physical_px,
        )
        .map_err(|error| format!("failed to create interaction host: {error}"))?;
        let import =
            pollster::block_on(host.instantiate_url(input.asset.as_str())).map_err(|error| {
                format!(
                    "failed to load interaction asset '{}': {error}",
                    input.asset
                )
            })?;
        if let Some(transform) = input.transform {
            for root in host
                .import_roots(import)
                .map_err(|error| format!("failed to resolve import roots: {error}"))?
            {
                host.set_transform(root, transform)
                    .map_err(|error| format!("failed to apply import transform: {error}"))?;
            }
        }
        host.frame_all()
            .map_err(|error| format!("failed to frame interaction target: {error}"))?;
        host
    };
    if artifacts.device_pixel_ratio != 1.0
        || artifacts.width_css_px != artifacts.width_physical_px as f32
        || artifacts.height_css_px != artifacts.height_physical_px as f32
    {
        host.resize(
            artifacts.width_css_px,
            artifacts.height_css_px,
            artifacts.device_pixel_ratio,
        )
        .map_err(|error| format!("failed to set interaction viewport: {error}"))?;
    }

    host.prepare()
        .map_err(|error| format!("failed to prepare interaction target: {error}"))?;
    host.render()
        .map_err(|error| format!("failed to render interaction target: {error}"))?;
    let _ = host.drain_events();

    let mut steps = Vec::with_capacity(expectation.steps.len());
    for (index, step) in expectation.steps.iter().enumerate() {
        let coordinates = scena::InteractionCoordinatesV1::from_step(step, expectation.viewport)?;
        let handle = match step.action.as_str() {
            "pick" => host
                .pick(coordinates.x_css_px, coordinates.y_css_px)
                .map_err(|error| format!("interaction pick failed: {error}"))?,
            "hover" => host
                .hover(coordinates.x_css_px, coordinates.y_css_px)
                .map_err(|error| format!("interaction hover failed: {error}"))?,
            "select" => host
                .select(coordinates.x_css_px, coordinates.y_css_px)
                .map_err(|error| format!("interaction select failed: {error}"))?,
            other => {
                return Err(CliFailure::invalid_arguments(format!(
                    "unsupported interaction action '{other}'"
                )));
            }
        };
        let hover_handle = host.hover_handle();
        let selection_handle = host.primary_selection_handle();
        let events = host
            .drain_events()
            .iter()
            .map(scena::host_event_kind_name)
            .map(str::to_owned)
            .collect::<Vec<_>>();
        steps.push(scena::InteractionStepReportV1 {
            index,
            action: step.action.clone(),
            coordinates,
            expected: scena::InteractionStepExpectedV1::from(step),
            observed: scena::InteractionStepObservedV1 {
                hit: handle.is_some(),
                handle,
                hover_handle,
                selection_handle,
                events,
            },
        });
    }

    let report = scena::InteractionVerificationReportV1::from_steps(artifacts, steps);
    let exit_code = if report.ok { 0 } else { 1 };
    json_outcome(
        &report,
        exit_code,
        "failed to serialize interaction verification report",
    )
}

impl VerifyInteractionCommandArgs {
    fn parse(args: &[String]) -> Result<Self, CliUsageError> {
        let Some(input) = args.first() else {
            return Err(CliUsageError::from(verify_interaction_usage()));
        };
        let mut expect = None;
        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
                "--expect" => {
                    expect = Some(PathBuf::from(flag_value(args, index, "--expect")?));
                    index += 2;
                }
                "--json" => {
                    index += 1;
                }
                flag => {
                    return Err(CliUsageError::from(format!(
                        "unknown verify interaction flag '{flag}'; {}",
                        verify_interaction_usage()
                    )));
                }
            }
        }
        Ok(Self {
            input: input.clone(),
            expect: expect.ok_or_else(|| {
                CliUsageError::from({
                    format!("missing --expect <json>; {}", verify_interaction_usage())
                })
            })?,
        })
    }
}

fn flag_value(args: &[String], index: usize, flag: &str) -> Result<String, CliUsageError> {
    args.get(index + 1)
        .cloned()
        .ok_or_else(|| CliUsageError::from(format!("{flag} requires a value")))
}

fn verify_interaction_usage() -> String {
    "usage: scena verify interaction <asset-or-recipe> --expect <interaction-expectation.json>"
        .to_string()
}
