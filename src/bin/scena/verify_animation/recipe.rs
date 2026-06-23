use super::VerifyAnimationCommandArgs;
use super::expectations::{apply_expected_node_status, expected_node_status};
use super::observations::{
    AnimationObservation, samples_from_observations, selected_observed_node,
};
use crate::scena_input::{ResolvedSceneInput, scene_host_build_from_resolved_recipe};
use crate::scena_output::{CliOutcome, json_outcome};

pub(super) async fn run_verify_recipe_animation(
    args: VerifyAnimationCommandArgs,
    input: ResolvedSceneInput,
    width: u32,
    height: u32,
) -> Result<CliOutcome, String> {
    let build = scene_host_build_from_resolved_recipe(&input, width, height, false).await?;
    let manifest = build.manifest;
    let mut host = build.host;
    let animation_handle = if let Some(animation) = manifest
        .animations
        .iter()
        .find(|animation| animation.id == args.clip)
        .cloned()
    {
        animation.handle
    } else if let Some(handle) = play_imported_clip(&mut host, &manifest, &args.clip)? {
        handle
    } else {
        let mut available = manifest
            .animations
            .iter()
            .map(|animation| animation.id.clone())
            .collect::<Vec<_>>();
        available.extend(imported_clip_names(&host, &manifest));
        available.sort();
        available.dedup();
        let report = scena::AnimationIntrospectionReportV1::missing_clip(&args.clip, available);
        return json_outcome(
            &report,
            1,
            "failed to serialize animation introspection report",
        );
    };
    let clip = host
        .animation_clip_for_handle(animation_handle)
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
        host.seek_animation(animation_handle, f64::from(*time_seconds))
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
        let node_rendered_coverage =
            super::rendered_coverage::rendered_node_coverages(&capture, &inspection);
        let node_transforms = inspection
            .nodes
            .iter()
            .map(|node| (node.handle, node.world_transform))
            .collect();
        observations.push(AnimationObservation {
            time_seconds: *time_seconds,
            transform_revision: inspection.revisions.transform,
            appearance_revision: inspection.revisions.appearance,
            payload_fnv1a64: capture.descriptor.payload.fnv1a64.clone(),
            capture,
            inspection,
            node_transforms,
            node_rendered_coverage,
        });
    }
    let selected_node = args.expected_node_handle.or_else(|| {
        (args.expect_change || args.expected_translations.is_some())
            .then(|| selected_observed_node(&observations, 0.0001))
            .flatten()
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

fn play_imported_clip(
    host: &mut scena::SceneHostCore,
    manifest: &scena::SceneRecipeBuildV1,
    clip_name: &str,
) -> Result<Option<u64>, String> {
    for import in &manifest.imports {
        let inventory = match host.animation_inventory_json(import.import_handle) {
            Ok(inventory) => inventory,
            Err(_) => continue,
        };
        let inventory: scena::SceneHostAnimationInventoryV1 = serde_json::from_str(&inventory)
            .map_err(|error| format!("failed to decode imported animation inventory: {error}"))?;
        if !inventory.clips.iter().any(|clip| clip.name == clip_name) {
            continue;
        }
        let handle = host
            .play_animation(
                import.import_handle,
                clip_name,
                scena::SceneHostAnimationPlayOptions::default(),
            )
            .map_err(|error| {
                format!(
                    "failed to play imported animation '{}' from import '{}': {error}",
                    clip_name, import.id
                )
            })?;
        return Ok(Some(handle));
    }
    Ok(None)
}

fn imported_clip_names(
    host: &scena::SceneHostCore,
    manifest: &scena::SceneRecipeBuildV1,
) -> Vec<String> {
    manifest
        .imports
        .iter()
        .filter_map(|import| host.animation_inventory_json(import.import_handle).ok())
        .filter_map(|inventory| {
            serde_json::from_str::<scena::SceneHostAnimationInventoryV1>(&inventory).ok()
        })
        .flat_map(|inventory| inventory.clips.into_iter().map(|clip| clip.name))
        .collect()
}
