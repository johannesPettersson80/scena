use super::expectations::{apply_expected_node_status, expected_node_status};
use super::{
    AnimationObservation, VerifyAnimationCommandArgs, samples_from_observations,
    selected_observed_node,
};
use crate::scena_input::{ResolvedSceneInput, scene_host_build_from_resolved_recipe};
use crate::scena_output::{CliOutcome, json_outcome};

pub(super) async fn run_verify_recipe_animation(
    args: VerifyAnimationCommandArgs,
    input: ResolvedSceneInput,
    width: u32,
    height: u32,
) -> Result<CliOutcome, String> {
    let build = scene_host_build_from_resolved_recipe(&input, width, height).await?;
    let Some(animation) = build
        .manifest
        .animations
        .iter()
        .find(|animation| animation.id == args.clip)
        .cloned()
    else {
        let report = scena::AnimationIntrospectionReportV1::missing_clip(
            &args.clip,
            build
                .manifest
                .animations
                .iter()
                .map(|animation| animation.id.clone())
                .collect(),
        );
        return json_outcome(
            &report,
            1,
            "failed to serialize animation introspection report",
        );
    };
    let mut host = build.host;
    let clip = host
        .animation_clip_for_handle(animation.handle)
        .map_err(|error| {
            format!(
                "failed to inspect recipe animation '{}': {error}",
                args.clip
            )
        })?
        .clone();
    let clip_summary = scena::AnimationClipIntrospectionV1::from_clip(&args.clip, &clip);
    let channel_counts = scena::animation_channel_change_counts(&clip, &args.times, 0.0001);

    let mut observations = Vec::with_capacity(args.times.len());
    for time_seconds in &args.times {
        host.seek_animation(animation.handle, f64::from(*time_seconds))
            .map_err(|error| format!("failed to seek recipe animation '{}': {error}", args.clip))?;
        host.prepare()
            .map_err(|error| format!("failed to prepare recipe animation sample: {error}"))?;
        host.render()
            .map_err(|error| format!("failed to render recipe animation sample: {error}"))?;
        let capture = host
            .capture()
            .map_err(|error| format!("failed to capture recipe animation sample: {error}"))?;
        let inspection_json = host
            .inspect_json()
            .map_err(|error| format!("failed to inspect recipe animation sample: {error}"))?;
        let inspection: scena::SceneInspectionReportV1 = serde_json::from_str(&inspection_json)
            .map_err(|error| {
                format!("failed to decode recipe animation inspection report: {error}")
            })?;
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
