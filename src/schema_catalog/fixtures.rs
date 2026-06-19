use super::{SCHEMA_CATALOG_SCHEMA_V1, SCHEMA_ENTRY_SCHEMA_V1};

pub(super) fn schema_fixture_json(schema: &str) -> Option<&'static str> {
    schema_fixture_map()
        .iter()
        .find_map(|(candidate, fixture)| (*candidate == schema).then_some(*fixture))
}

pub fn nearest_schema_name(input: &str) -> Option<&'static str> {
    schema_fixture_map()
        .iter()
        .map(|(schema, _)| (*schema, edit_distance(input, schema)))
        .filter(|(_, distance)| *distance <= 8)
        .min_by(|(left_schema, left), (right_schema, right)| {
            left.cmp(right).then_with(|| left_schema.cmp(right_schema))
        })
        .map(|(schema, _)| schema)
}

fn schema_fixture_map() -> &'static [(&'static str, &'static str)] {
    &[
        (
            SCHEMA_CATALOG_SCHEMA_V1,
            include_str!("../../tests/assets/stable-contracts/schema_catalog.v1.json"),
        ),
        (
            SCHEMA_ENTRY_SCHEMA_V1,
            include_str!("../../tests/assets/stable-contracts/schema_entry.v1.json"),
        ),
        (
            "scena.capability_report.v1",
            include_str!("../../tests/assets/stable-contracts/capability_report.v1.json"),
        ),
        (
            "scena.scene_inspection.v1",
            include_str!("../../tests/assets/stable-contracts/scene_inspection.v1.json"),
        ),
        (
            "scena.capture.v1",
            include_str!("../../tests/assets/stable-contracts/capture.v1.json"),
        ),
        (
            "scena.capture_baseline.v1",
            include_str!("../../tests/assets/stable-contracts/capture_baseline.v1.json"),
        ),
        (
            "scena.render_introspection.v1",
            include_str!("../../tests/assets/stable-contracts/render_introspection.v1.json"),
        ),
        (
            "scena.render_quality.v1",
            include_str!("../../tests/assets/stable-contracts/render_quality.v1.json"),
        ),
        (
            "scena.visibility_diagnosis.v1",
            include_str!("../../tests/assets/stable-contracts/visibility_diagnosis.v1.json"),
        ),
        (
            "scena.visual_repair_plan.v1",
            include_str!("../../tests/assets/stable-contracts/visual_repair_plan.v1.json"),
        ),
        (
            "scena.agent_loop_result.v1",
            include_str!("../../tests/assets/stable-contracts/agent_loop_result.v1.json"),
        ),
        (
            "scena.agent_smoke_template.v1",
            include_str!("../../tests/assets/stable-contracts/agent_smoke_template.v1.json"),
        ),
        (
            "scena.browser_proof_run.v1",
            include_str!("../../tests/assets/stable-contracts/browser_proof_run.v1.json"),
        ),
        (
            "scena.appearance_expectation.v1",
            include_str!("../../tests/assets/stable-contracts/appearance_expectation.v1.json"),
        ),
        (
            "scena.appearance_introspection.v1",
            include_str!("../../tests/assets/stable-contracts/appearance_introspection.v1.json"),
        ),
        (
            "scena.animation_introspection.v1",
            include_str!("../../tests/assets/stable-contracts/animation_introspection.v1.json"),
        ),
        (
            "scena.interaction_expectation.v1",
            include_str!("../../tests/assets/stable-contracts/interaction_expectation.v1.json"),
        ),
        (
            "scena.interaction_verification.v1",
            include_str!("../../tests/assets/stable-contracts/interaction_verification.v1.json"),
        ),
        (
            "scena.scene_host_gizmo_drag.v1",
            include_str!("../../tests/assets/stable-contracts/scene_host_gizmo_drag.v1.json"),
        ),
        (
            "scena.connector_browser.v1",
            include_str!("../../tests/assets/stable-contracts/connector_browser.v1.json"),
        ),
        (
            "scena.product_options.v1",
            include_str!("../../tests/assets/stable-contracts/product_options.v1.json"),
        ),
        (
            "scena.presentation_timeline.v1",
            include_str!("../../tests/assets/stable-contracts/presentation_timeline.v1.json"),
        ),
        (
            "scena.scene_host_grounding.v1",
            include_str!("../../tests/assets/stable-contracts/scene_host_grounding.v1.json"),
        ),
        (
            "scena.scene_recipe.v1",
            include_str!("../../tests/assets/stable-contracts/scene_recipe.v1.json"),
        ),
        (
            "scena.scene_recipe_validation.v1",
            include_str!("../../tests/assets/stable-contracts/scene_recipe_validation.v1.json"),
        ),
        (
            "scena.scene_recipe_build.v1",
            include_str!("../../tests/assets/stable-contracts/scene_recipe_build.v1.json"),
        ),
        (
            "scena.recipe_render_result.v1",
            include_str!("../../tests/assets/stable-contracts/recipe_render_result.v1.json"),
        ),
        (
            "scena.placement_result.v1",
            include_str!("../../tests/assets/stable-contracts/placement_result.v1.json"),
        ),
        (
            "scena.annotation_projection.v1",
            include_str!("../../tests/assets/stable-contracts/annotation_projection.v1.json"),
        ),
        (
            "scena.asset_geometry_summary.v1",
            include_str!("../../tests/assets/stable-contracts/asset_geometry_summary.v1.json"),
        ),
        (
            "scena.asset_load_report.v1",
            include_str!("../../tests/assets/stable-contracts/asset_load_report.v1.json"),
        ),
        (
            "scena.asset_doctor.v1",
            include_str!("../../tests/assets/stable-contracts/asset_doctor.v1.json"),
        ),
        (
            "scena.asset_catalog.v1",
            include_str!("../../tests/assets/stable-contracts/asset_catalog.v1.json"),
        ),
        (
            "scena.asset_readiness_report.v1",
            include_str!("../../tests/assets/stable-contracts/asset_readiness_report.v1.json"),
        ),
        (
            "scena.scene_host_asset_import.v1",
            include_str!("../../tests/assets/stable-contracts/scene_host_asset_import.v1.json"),
        ),
        (
            "scena.subtree.v1",
            include_str!("../../tests/assets/stable-contracts/scene_host_subtree.v1.json"),
        ),
        (
            "scena.scene_host_measurement_overlay.v1",
            include_str!(
                "../../tests/assets/stable-contracts/scene_host_measurement_overlay.v1.json"
            ),
        ),
        (
            "scena.scene_host_section_box.v1",
            include_str!("../../tests/assets/stable-contracts/scene_host_section_box.v1.json"),
        ),
        (
            "scena.scene_host_visual_state.v1",
            include_str!("../../tests/assets/stable-contracts/scene_host_visual_state.v1.json"),
        ),
        (
            "scena.scene_host_visual_states.v1",
            include_str!("../../tests/assets/stable-contracts/scene_host_visual_states.v1.json"),
        ),
        (
            "scena.animation_inventory.v1",
            include_str!(
                "../../tests/assets/stable-contracts/scene_host_animation_inventory.v1.json"
            ),
        ),
        (
            "scena.visual_patch.v1",
            include_str!("../../tests/assets/stable-contracts/visual_patch.v1.json"),
        ),
        (
            "scena.host_event.v1",
            include_str!("../../tests/assets/stable-contracts/host_event.v1.json"),
        ),
    ]
}

fn edit_distance(left: &str, right: &str) -> usize {
    let right_chars = right.chars().collect::<Vec<_>>();
    let mut previous = (0..=right_chars.len()).collect::<Vec<_>>();
    let mut current = vec![0; right_chars.len() + 1];

    for (left_index, left_char) in left.chars().enumerate() {
        current[0] = left_index + 1;
        for (right_index, right_char) in right_chars.iter().enumerate() {
            let substitution = usize::from(left_char != *right_char);
            current[right_index + 1] = (previous[right_index + 1] + 1)
                .min(current[right_index] + 1)
                .min(previous[right_index] + substitution);
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous[right_chars.len()]
}
