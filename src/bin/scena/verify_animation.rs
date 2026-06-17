use std::collections::BTreeMap;

use super::scena_input::{resolve_scene_input, viewer_builder};
use super::scena_output::{CliOutcome, json_outcome};
use expectations::{apply_expected_node_status, expected_node_status};

#[path = "verify_animation/expectations.rs"]
mod expectations;
#[cfg(feature = "scene-host")]
#[path = "verify_animation/recipe.rs"]
mod recipe;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct VerifyAnimationCommandArgs {
    input: String,
    clip: String,
    times: Vec<f32>,
    expect_change: bool,
    expected_node_handle: Option<u64>,
    expected_translations: Option<Vec<scena::Vec3>>,
    expected_tolerance: f32,
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
    if input.has_scene_host_directives() {
        #[cfg(feature = "scene-host")]
        {
            return pollster::block_on(recipe::run_verify_recipe_animation(
                args, input, width, height,
            ));
        }
        #[cfg(not(feature = "scene-host"))]
        {
            return Err(
                "verify animation for authored recipes requires the scene-host feature".to_owned(),
            );
        }
    }
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

    let selected_node = args.expected_node_handle.or_else(|| {
        args.expected_translations
            .as_ref()
            .and_then(|_| selected_observed_node(&observations, 0.0001))
    });
    let samples = samples_from_observations(
        &observations,
        0.0001,
        args.expected_translations.as_deref(),
        selected_node,
        args.expected_tolerance,
    );
    let mut report = scena::AnimationIntrospectionReportV1::from_samples(
        clip_summary,
        channel_counts,
        samples,
        args.expect_change,
    );
    if let Some(handle) = args.expected_node_handle {
        apply_expected_node_status(
            &mut report,
            handle,
            expected_node_status(&observations, handle, 0.0001, args.expect_change),
        );
    }
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
        let mut expected_node_handle = None;
        let mut expected_translations = None;
        let mut expected_tolerance = 0.001;
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
                "--expect-node-handle" => {
                    expected_node_handle = Some(parse_u64_handle(
                        "--expect-node-handle",
                        flag_value(args, index, "--expect-node-handle")?,
                    )?);
                    index += 2;
                }
                "--expect-translations" => {
                    expected_translations = Some(parse_expected_translations(flag_value(
                        args,
                        index,
                        "--expect-translations",
                    )?)?);
                    index += 2;
                }
                "--expect-tolerance" => {
                    expected_tolerance = parse_positive_f32(
                        "--expect-tolerance",
                        flag_value(args, index, "--expect-tolerance")?,
                    )?;
                    index += 2;
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

        let times = times
            .ok_or_else(|| format!("missing --times <seconds>; {}", verify_animation_usage()))?;
        if let Some(expected) = &expected_translations
            && expected.len() != times.len()
        {
            return Err(format!(
                "--expect-translations requires one x,y,z triple per sample time (got {} expected values for {} times)",
                expected.len(),
                times.len()
            ));
        }

        Ok(Self {
            input: input.clone(),
            clip: clip
                .ok_or_else(|| format!("missing --clip <name>; {}", verify_animation_usage()))?,
            times,
            expect_change,
            expected_node_handle,
            expected_translations,
            expected_tolerance,
            width,
            height,
        })
    }
}

fn samples_from_observations(
    observations: &[AnimationObservation],
    tolerance: f32,
    expected_translations: Option<&[scena::Vec3]>,
    selected_node: Option<u64>,
    expected_tolerance: f32,
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
        .enumerate()
        .map(|(index, observation)| {
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
                observed_values: observed_values_for_sample(
                    observation,
                    selected_node,
                    expected_translations.and_then(|expected| expected.get(index).copied()),
                    expected_tolerance,
                ),
            }
        })
        .collect()
}

fn selected_observed_node(observations: &[AnimationObservation], tolerance: f32) -> Option<u64> {
    let baseline = observations.first()?;
    for (handle, baseline_transform) in &baseline.node_transforms {
        if observations.iter().skip(1).any(|observation| {
            observation
                .node_transforms
                .iter()
                .find(|(candidate, _)| candidate == handle)
                .is_some_and(|(_, transform)| {
                    scena::transform_differs(*baseline_transform, *transform, tolerance)
                })
        }) {
            return Some(*handle);
        }
    }
    baseline.node_transforms.first().map(|(handle, _)| *handle)
}

fn observed_values_for_sample(
    observation: &AnimationObservation,
    selected_node: Option<u64>,
    expected_translation: Option<scena::Vec3>,
    expected_tolerance: f32,
) -> Vec<scena::AnimationObservedValueV1> {
    let Some(node) = selected_node else {
        return Vec::new();
    };
    let Some((_, transform)) = observation
        .node_transforms
        .iter()
        .find(|(candidate, _)| *candidate == node)
    else {
        return Vec::new();
    };
    let within_tolerance = expected_translation.map(|expected| {
        (transform.translation.x - expected.x).abs() <= expected_tolerance
            && (transform.translation.y - expected.y).abs() <= expected_tolerance
            && (transform.translation.z - expected.z).abs() <= expected_tolerance
    });
    vec![scena::AnimationObservedValueV1 {
        id: "selected-transform".to_owned(),
        node,
        kind: "transform".to_owned(),
        transform: *transform,
        expected_translation,
        within_tolerance,
    }]
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

fn parse_expected_translations(value: String) -> Result<Vec<scena::Vec3>, String> {
    let translations = value
        .split(';')
        .filter(|part| !part.trim().is_empty())
        .map(parse_vec3)
        .collect::<Result<Vec<_>, _>>()?;
    if translations.is_empty() {
        return Err("--expect-translations requires at least one x,y,z triple".to_string());
    }
    Ok(translations)
}

fn parse_vec3(value: &str) -> Result<scena::Vec3, String> {
    let components = value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            part.parse::<f32>().map_err(|_| {
                format!("--expect-translations contains non-numeric component '{part}'")
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let [x, y, z]: [f32; 3] = components.try_into().map_err(|components: Vec<f32>| {
        format!(
            "--expect-translations entries must have exactly three components, got {} in '{value}'",
            components.len()
        )
    })?;
    if !x.is_finite() || !y.is_finite() || !z.is_finite() {
        return Err(format!(
            "--expect-translations requires finite values, got '{value}'"
        ));
    }
    Ok(scena::Vec3::new(x, y, z))
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

fn parse_positive_f32(flag: &str, value: String) -> Result<f32, String> {
    let parsed = value
        .parse::<f32>()
        .map_err(|_| format!("{flag} requires a number, got '{value}'"))?;
    if !parsed.is_finite() || parsed <= 0.0 {
        return Err(format!(
            "{flag} requires a finite positive number, got {value}"
        ));
    }
    Ok(parsed)
}

fn parse_u64_handle(flag: &str, value: String) -> Result<u64, String> {
    let parsed = value
        .parse::<u64>()
        .map_err(|_| format!("{flag} requires an unsigned integer handle, got '{value}'"))?;
    if parsed == 0 {
        return Err(format!("{flag} requires a non-zero handle"));
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
    "usage: scena verify animation <asset-or-recipe> --clip <name> --times <seconds[,seconds...]> [--expect-change] [--expect-node-handle <handle>] [--expect-translations 'x,y,z;...'] [--expect-tolerance n] [--width <px>] [--height <px>]"
        .to_string()
}
