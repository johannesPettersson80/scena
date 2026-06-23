use std::collections::BTreeMap;

use serde_json::json;

use super::checks::{checked_check, error_check, observed_pairs};
use super::helpers::ground_candidate_handles;
use crate::{SceneCompositionCheckV1, SceneInspectionReportV1, SceneRecipeV1, SceneSetupPreset};

pub(super) fn composition_grid_ownership_checks(
    recipe: &SceneRecipeV1,
    inspection: &SceneInspectionReportV1,
) -> (Vec<SceneCompositionCheckV1>, Vec<u64>) {
    let mut checks = Vec::new();
    let grid_enabled = recipe.scene.as_ref().is_some_and(|scene| {
        scene.grid.as_ref().map_or_else(
            || {
                scene
                    .preset
                    .as_deref()
                    .and_then(SceneSetupPreset::from_recipe_name)
                    .is_some()
            },
            |grid| grid.enabled,
        )
    });
    if !grid_enabled {
        return (checks, Vec::new());
    }

    let ground_handles = ground_candidate_handles(inspection);
    if ground_handles.is_empty() {
        checks.push(error_check(
            "scene.grid.ownership".to_owned(),
            "helper_render_layer",
            "grid_floor_output_missing",
            None,
            Vec::new(),
            BTreeMap::new(),
            (
                "recipe requested a grid floor but no ground-like draw output was found",
                "make the grid visible, keep it in frame, or disable scene.grid when it is not required",
            ),
        ));
    } else {
        checks.push(checked_check(
            "scene.grid.ownership".to_owned(),
            "helper_render_layer",
            "grid_floor_output_owned",
            None,
            ground_handles.clone(),
            observed_pairs([
                ("owned_ground_handles", json!(ground_handles.clone())),
                ("ground_draw_count", json!(ground_handles.len())),
            ]),
            (
                "recipe-requested grid floor output is explicitly owned by scene.grid",
                "no action needed",
            ),
        ));
    }
    (checks, ground_handles)
}
