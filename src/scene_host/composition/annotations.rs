use std::collections::{BTreeMap, BTreeSet};

use serde_json::json;

use super::SceneHostCore;
use super::checks::{checked_check, error_check, observed_pairs};
use super::regions::CompositionOverlaySegmentV1;
use crate::{
    AssetFetcher, CaptureScreenRegion, SceneCompositionCheckV1, SceneRecipeBuildV1,
    SceneRecipeCalloutTargetV1, SceneRecipeV1,
};

impl<F: AssetFetcher> SceneHostCore<F> {
    pub(super) fn composition_callout_checks(
        &self,
        recipe: &SceneRecipeV1,
        manifest: &SceneRecipeBuildV1,
        label_regions: &BTreeMap<u64, CaptureScreenRegion>,
        overlay_segments: &[CompositionOverlaySegmentV1],
    ) -> Vec<SceneCompositionCheckV1> {
        let mut checks = Vec::new();
        for callout in &recipe.callouts {
            let target = resolve_callout_target(&callout.target, manifest);
            let target_handle = target.handle;
            let target_id = Some(callout.id.clone());
            let state = self.scene.callout_state(&callout.id);
            let leader_line_handle = state
                .and_then(|state| self.node_handle_map.get(&state.leader_line_node()).copied());
            let label_handle =
                state.and_then(|state| self.node_handle_map.get(&state.label_node()).copied());
            let affected =
                affected_callout_handles(target_handle, leader_line_handle, label_handle);

            if let Some(message) = target.error {
                checks.push(error_check(
                    format!("annotation.callout.{}.attachment", callout.id),
                    "overlay",
                    "callout_target_unresolved",
                    target_id.clone(),
                    affected.clone(),
                    observed_pairs([("target_kind", json!(target.kind))]),
                    (
                        message.as_str(),
                        "rebuild the recipe and make the callout target resolve to a declared node, import root, or world position",
                    ),
                ));
                continue;
            }

            if state.is_none() || leader_line_handle.is_none() || label_handle.is_none() {
                checks.push(error_check(
                    format!("annotation.callout.{}.attachment", callout.id),
                    "overlay",
                    "callout_output_missing",
                    target_id.clone(),
                    affected.clone(),
                    observed_pairs([
                        ("target_kind", json!(target.kind)),
                        ("target_handle", json!(target_handle)),
                        ("leader_line_handle", json!(leader_line_handle)),
                        ("label_handle", json!(label_handle)),
                    ]),
                    (
                        "declared callout did not produce both its leader line and label nodes",
                        "rebuild the recipe or remove the stale callout declaration",
                    ),
                ));
                continue;
            }

            checks.push(checked_check(
                format!("annotation.callout.{}.attachment", callout.id),
                "overlay",
                if target_handle.is_some() {
                    "callout_target_attached"
                } else {
                    "callout_world_anchor_declared"
                },
                target_id.clone(),
                affected.clone(),
                observed_pairs([
                    ("target_kind", json!(target.kind)),
                    ("target_handle", json!(target_handle)),
                    ("leader_line_handle", json!(leader_line_handle)),
                    ("label_handle", json!(label_handle)),
                ]),
                (
                    "declared callout is attached to its recipe target and generated overlay nodes",
                    "no action needed",
                ),
            ));

            let leader_projected = leader_line_handle
                .is_some_and(|handle| segment_handles(overlay_segments).contains(&handle));
            let label_projected =
                label_handle.is_some_and(|handle| label_regions.contains_key(&handle));
            if leader_projected && label_projected {
                checks.push(checked_check(
                    format!("annotation.callout.{}.output", callout.id),
                    "overlay",
                    "callout_overlay_output_projected",
                    target_id,
                    affected,
                    observed_pairs([
                        ("leader_line_projected", json!(true)),
                        ("label_projected", json!(true)),
                    ]),
                    (
                        "declared callout leader line and label project to screen regions",
                        "no action needed",
                    ),
                ));
            } else {
                checks.push(error_check(
                    format!("annotation.callout.{}.output", callout.id),
                    "overlay",
                    "callout_overlay_output_missing",
                    target_id,
                    affected,
                    observed_pairs([
                        ("leader_line_projected", json!(leader_projected)),
                        ("label_projected", json!(label_projected)),
                    ]),
                    (
                        "declared callout generated output but it was not fully projected into the captured frame",
                        "move the callout into frame or adjust the camera/label offset",
                    ),
                ));
            }
        }
        checks
    }

    pub(super) fn composition_measurement_checks(
        &self,
        recipe: &SceneRecipeV1,
        label_regions: &BTreeMap<u64, CaptureScreenRegion>,
        overlay_segments: &[CompositionOverlaySegmentV1],
    ) -> Vec<SceneCompositionCheckV1> {
        let mut checks = Vec::new();
        let segment_handles = segment_handles(overlay_segments);
        for measurement in &recipe.measurements {
            let target_id = Some(measurement.id.clone());
            let state = self.scene.measurement_overlay_state(&measurement.id);
            let line_handle =
                state.and_then(|state| self.node_handle_map.get(&state.line_node()).copied());
            let label_handle = state
                .and_then(|state| state.label_node())
                .and_then(|node| self.node_handle_map.get(&node).copied());
            let affected = affected_measurement_handles(line_handle, label_handle);
            let line_projected =
                line_handle.is_some_and(|handle| segment_handles.contains(&handle));
            let label_expected = measurement.label.is_some();
            let label_projected = !label_expected
                || label_handle.is_some_and(|handle| label_regions.contains_key(&handle));

            if state.is_none()
                || line_handle.is_none()
                || (label_expected && label_handle.is_none())
            {
                checks.push(error_check(
                    format!("annotation.measurement.{}.output", measurement.id),
                    "overlay",
                    "measurement_overlay_output_missing",
                    target_id,
                    affected,
                    observed_pairs([
                        ("line_handle", json!(line_handle)),
                        ("label_handle", json!(label_handle)),
                        ("label_expected", json!(label_expected)),
                    ]),
                    (
                        "declared measurement did not produce its generated overlay nodes",
                        "rebuild the recipe or remove the stale measurement declaration",
                    ),
                ));
                continue;
            }

            if line_projected && label_projected {
                checks.push(checked_check(
                    format!("annotation.measurement.{}.output", measurement.id),
                    "overlay",
                    "measurement_overlay_output_projected",
                    target_id,
                    affected,
                    observed_pairs([
                        ("line_projected", json!(true)),
                        ("label_projected", json!(label_projected)),
                        ("label_expected", json!(label_expected)),
                    ]),
                    (
                        "declared measurement line and label project to screen regions",
                        "no action needed",
                    ),
                ));
            } else {
                checks.push(error_check(
                    format!("annotation.measurement.{}.output", measurement.id),
                    "overlay",
                    "measurement_overlay_output_missing",
                    target_id,
                    affected,
                    observed_pairs([
                        ("line_projected", json!(line_projected)),
                        ("label_projected", json!(label_projected)),
                        ("label_expected", json!(label_expected)),
                    ]),
                    (
                        "declared measurement generated output but it was not fully projected into the captured frame",
                        "move the measurement into frame or adjust the camera/label placement",
                    ),
                ));
            }
        }
        checks
    }
}

struct ResolvedCalloutTarget {
    kind: &'static str,
    handle: Option<u64>,
    error: Option<String>,
}

fn resolve_callout_target(
    target: &SceneRecipeCalloutTargetV1,
    manifest: &SceneRecipeBuildV1,
) -> ResolvedCalloutTarget {
    match target {
        SceneRecipeCalloutTargetV1::Node { id, .. } => {
            let handle = manifest
                .nodes
                .iter()
                .find(|node| node.id == *id)
                .map(|node| node.handle)
                .or_else(|| {
                    manifest
                        .imports
                        .iter()
                        .find_map(|import| import.nodes_by_path.get(id).copied())
                });
            ResolvedCalloutTarget {
                kind: "node",
                handle,
                error: handle
                    .is_none()
                    .then(|| format!("declared callout references unresolved node target '{id}'")),
            }
        }
        SceneRecipeCalloutTargetV1::ImportRoot { import, .. } => {
            let handle = manifest
                .imports
                .iter()
                .find(|entry| entry.id == *import)
                .and_then(|entry| entry.root_handles.first().copied());
            ResolvedCalloutTarget {
                kind: "import_root",
                handle,
                error: handle.is_none().then(|| {
                    format!("declared callout references unresolved import root '{import}'")
                }),
            }
        }
        SceneRecipeCalloutTargetV1::World { .. } => ResolvedCalloutTarget {
            kind: "world",
            handle: None,
            error: None,
        },
    }
}

fn affected_callout_handles(
    target: Option<u64>,
    leader_line: Option<u64>,
    label: Option<u64>,
) -> Vec<u64> {
    let mut handles = [target, leader_line, label]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    handles.sort_unstable();
    handles.dedup();
    handles
}

fn affected_measurement_handles(line: Option<u64>, label: Option<u64>) -> Vec<u64> {
    let mut handles = [line, label].into_iter().flatten().collect::<Vec<_>>();
    handles.sort_unstable();
    handles.dedup();
    handles
}

fn segment_handles(segments: &[CompositionOverlaySegmentV1]) -> BTreeSet<u64> {
    segments
        .iter()
        .filter_map(|segment| segment.handle)
        .collect()
}
