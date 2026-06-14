use std::collections::BTreeMap;

use super::{CliOutcome, json_outcome, resolve_scene_input, viewer_builder};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct VerifyAnimationCommandArgs {
    input: String,
    clip: String,
    times: Vec<f32>,
    expect_change: bool,
    width: Option<u32>,
    height: Option<u32>,
}

#[derive(Debug, Clone)]
struct AnimationObservation {
    time_seconds: f32,
    transform_revision: u64,
    appearance_revision: u64,
    payload_fnv1a64: String,
    node_transforms: Vec<(u64, scena::Transform)>,
}

pub(crate) fn run_verify_animation_command(args: &[String]) -> Result<CliOutcome, String> {
    let args = VerifyAnimationCommandArgs::parse(args)?;
    let input = match resolve_scene_input(&args.input) {
        Ok(input) => input,
        Err(outcome) => return Ok(outcome),
    };
    let width = args.width.or(input.width).unwrap_or(800);
    let height = args.height.or(input.height).unwrap_or(600);
    let mut viewer = pollster::block_on(
        viewer_builder(input.asset.as_str(), width, height, input.transform)
            .with_default_light()
            .build(),
    )
    .map_err(|error| format!("failed to verify animation for '{}': {error}", input.asset))?;

    let mixer = match viewer.play_clip(&args.clip) {
        Ok(mixer) => mixer,
        Err(_) => {
            let report = scena::AnimationIntrospectionReportV1::missing_clip(
                &args.clip,
                available_clip_names(viewer.import()),
            );
            return json_outcome(
                &report,
                1,
                "failed to serialize animation introspection report",
            );
        }
    };
    let clip = viewer
        .scene()
        .animation_mixer(mixer)
        .map_err(|error| format!("failed to inspect animation mixer: {error}"))?
        .clip()
        .clone();
    let clip_summary = scena::AnimationClipIntrospectionV1::from_clip(&args.clip, &clip);
    let channel_counts = scena::animation_channel_change_counts(&clip, &args.times, 0.0001);

    let mut observations = Vec::with_capacity(args.times.len());
    for time_seconds in &args.times {
        viewer
            .scene_mut()
            .seek_animation(mixer, *time_seconds)
            .map_err(|error| format!("failed to seek animation '{}': {error}", args.clip))?;
        viewer
            .prepare()
            .map_err(|error| format!("failed to prepare animation sample: {error}"))?;
        viewer
            .render_next_frame()
            .map_err(|error| format!("failed to render animation sample: {error}"))?;
        let capture = viewer
            .capture()
            .map_err(|error| format!("failed to capture animation sample: {error}"))?;
        let inspection = viewer
            .scene()
            .inspect_with_assets(viewer.assets())
            .to_schema_report();
        observations.push(AnimationObservation {
            time_seconds: *time_seconds,
            transform_revision: inspection.revisions.transform,
            appearance_revision: inspection.revisions.appearance,
            payload_fnv1a64: capture.descriptor.payload.fnv1a64,
            node_transforms: inspection
                .nodes
                .into_iter()
                .map(|node| (node.handle, node.world_transform))
                .collect(),
        });
    }

    let samples = samples_from_observations(&observations, 0.0001);
    let report = scena::AnimationIntrospectionReportV1::from_samples(
        clip_summary,
        channel_counts,
        samples,
        args.expect_change,
    );
    let exit_code = if report.ok { 0 } else { 1 };
    json_outcome(
        &report,
        exit_code,
        "failed to serialize animation introspection report",
    )
}

impl VerifyAnimationCommandArgs {
    fn parse(args: &[String]) -> Result<Self, String> {
        let Some(input) = args.first() else {
            return Err(verify_animation_usage());
        };
        let mut clip = None;
        let mut times = None;
        let mut expect_change = false;
        let mut width = None;
        let mut height = None;

        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
                "--clip" => {
                    clip = Some(flag_value(args, index, "--clip")?);
                    index += 2;
                }
                "--times" => {
                    times = Some(parse_times(flag_value(args, index, "--times")?)?);
                    index += 2;
                }
                "--expect-change" => {
                    expect_change = true;
                    index += 1;
                }
                "--width" => {
                    width = Some(parse_positive_u32(
                        "--width",
                        flag_value(args, index, "--width")?,
                    )?);
                    index += 2;
                }
                "--height" => {
                    height = Some(parse_positive_u32(
                        "--height",
                        flag_value(args, index, "--height")?,
                    )?);
                    index += 2;
                }
                "--json" | "--detail" => {
                    index += 1;
                }
                flag => {
                    return Err(format!(
                        "unknown verify animation flag '{flag}'; {}",
                        verify_animation_usage()
                    ));
                }
            }
        }

        Ok(Self {
            input: input.clone(),
            clip: clip
                .ok_or_else(|| format!("missing --clip <name>; {}", verify_animation_usage()))?,
            times: times.ok_or_else(|| {
                format!("missing --times <seconds>; {}", verify_animation_usage())
            })?,
            expect_change,
            width,
            height,
        })
    }
}

fn samples_from_observations(
    observations: &[AnimationObservation],
    tolerance: f32,
) -> Vec<scena::AnimationSampleV1> {
    let baseline = observations.first().map(|observation| {
        observation
            .node_transforms
            .iter()
            .copied()
            .collect::<BTreeMap<_, _>>()
    });
    observations
        .iter()
        .map(|observation| {
            let invalid_node_count = observation
                .node_transforms
                .iter()
                .filter(|(_, transform)| !scena::transform_is_finite(*transform))
                .count();
            let moving_node_count = baseline
                .as_ref()
                .map(|baseline| {
                    observation
                        .node_transforms
                        .iter()
                        .filter(|(handle, transform)| {
                            baseline.get(handle).is_some_and(|baseline_transform| {
                                scena::transform_differs(*baseline_transform, *transform, tolerance)
                            })
                        })
                        .count()
                })
                .unwrap_or(0);
            scena::AnimationSampleV1 {
                time_seconds: round3(observation.time_seconds),
                transform_revision: observation.transform_revision,
                appearance_revision: observation.appearance_revision,
                payload_fnv1a64: observation.payload_fnv1a64.clone(),
                moving_node_count,
                invalid_node_count,
            }
        })
        .collect()
}

fn available_clip_names(import: &scena::SceneImport) -> Vec<String> {
    let Ok(clips) = import.clips() else {
        return Vec::new();
    };
    clips
        .iter()
        .filter_map(|clip| clip.name().map(str::to_owned))
        .collect()
}

fn flag_value(args: &[String], index: usize, flag: &str) -> Result<String, String> {
    args.get(index + 1)
        .cloned()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn parse_times(value: String) -> Result<Vec<f32>, String> {
    let times = value
        .split([',', ' '])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let parsed = part
                .parse::<f32>()
                .map_err(|_| format!("--times contains non-numeric value '{part}'"))?;
            if !parsed.is_finite() || parsed < 0.0 {
                return Err(format!(
                    "--times requires finite non-negative seconds, got '{part}'"
                ));
            }
            Ok(parsed)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if times.is_empty() {
        return Err("--times requires at least one sample time".to_string());
    }
    Ok(times)
}

fn parse_positive_u32(flag: &str, value: String) -> Result<u32, String> {
    let parsed = value
        .parse::<u32>()
        .map_err(|_| format!("{flag} requires an unsigned integer, got '{value}'"))?;
    if parsed == 0 {
        return Err(format!("{flag} requires a positive integer, got 0"));
    }
    Ok(parsed)
}

fn round3(value: f32) -> f32 {
    if value.is_finite() {
        (value * 1000.0).round() / 1000.0
    } else {
        0.0
    }
}

fn verify_animation_usage() -> String {
    "usage: scena verify animation <asset-or-recipe> --clip <name> --times <seconds[,seconds...]> [--expect-change] [--width <px>] [--height <px>]"
        .to_string()
}
