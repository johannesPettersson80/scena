use std::collections::{BTreeMap, BTreeSet};

use crate::scene::recipe::types::{
    SceneRecipeDiagnosticV1, SceneRecipeNamedStateV1, SceneRecipeSpatialTargetV1,
};

use super::{error, validate_local_transform, validate_non_empty_optional, validate_target};

pub(super) fn validate_states(
    states: &[SceneRecipeNamedStateV1],
    node_ids: &BTreeSet<String>,
    import_ids: &BTreeSet<String>,
    animated_nodes: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if states.iter().filter(|state| state.active).count() > 1 {
        diagnostics.push(error(
            "multiple_active_named_states",
            "$.named_states",
            "at most one named state may be active",
            "set active:true on zero or one named state",
        ));
    }
    let by_id = states
        .iter()
        .map(|state| (state.id.as_str(), state))
        .collect::<BTreeMap<_, _>>();
    for (index, state) in states.iter().enumerate() {
        let path = format!("$.named_states[{index}]");
        validate_inheritance(state, &path, &by_id, diagnostics);
        validate_transforms(
            state,
            &path,
            node_ids,
            import_ids,
            animated_nodes,
            diagnostics,
        );
        validate_tints(state, &path, node_ids, import_ids, diagnostics);
        validate_visibility(state, &path, node_ids, import_ids, diagnostics);
    }
}

fn validate_inheritance(
    state: &SceneRecipeNamedStateV1,
    path: &str,
    by_id: &BTreeMap<&str, &SceneRecipeNamedStateV1>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if let Some(parent) = &state.inherits
        && !by_id.contains_key(parent.as_str())
    {
        diagnostics.push(error(
            "unknown_state_parent",
            format!("{path}.inherits"),
            format!(
                "named state '{}' inherits missing state '{parent}'",
                state.id
            ),
            "reference another named state id",
        ));
    }
    if state_cycle(&state.id, by_id) {
        diagnostics.push(error(
            "state_inheritance_cycle",
            format!("{path}.inherits"),
            format!(
                "named state '{}' participates in an inheritance cycle",
                state.id
            ),
            "use acyclic single inheritance",
        ));
    }
}

fn validate_transforms(
    state: &SceneRecipeNamedStateV1,
    path: &str,
    node_ids: &BTreeSet<String>,
    import_ids: &BTreeSet<String>,
    animated_nodes: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let mut targets = BTreeSet::new();
    for (entry_index, entry) in state.transforms.iter().enumerate() {
        let entry_path = format!("{path}.transforms[{entry_index}]");
        validate_target(
            &format!("{entry_path}.target"),
            &entry.target,
            node_ids,
            import_ids,
            diagnostics,
        );
        validate_local_transform(
            &format!("{entry_path}.transform"),
            &entry.transform,
            diagnostics,
        );
        reject_duplicate_state_target(&mut targets, &entry.target, &entry_path, diagnostics);
        if let SceneRecipeSpatialTargetV1::Node { id } = &entry.target
            && animated_nodes.contains(id)
        {
            diagnostics.push(error(
                "animated_state_transform_conflict",
                format!("{entry_path}.target"),
                format!("named-state transform targets animated node '{id}'"),
                "remove the transform entry or the recipe animation channel",
            ));
        }
    }
}

fn validate_tints(
    state: &SceneRecipeNamedStateV1,
    path: &str,
    node_ids: &BTreeSet<String>,
    import_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let mut targets = BTreeSet::new();
    for (entry_index, entry) in state.tints.iter().enumerate() {
        let entry_path = format!("{path}.tints[{entry_index}]");
        validate_target(
            &format!("{entry_path}.target"),
            &entry.target,
            node_ids,
            import_ids,
            diagnostics,
        );
        reject_duplicate_state_target(&mut targets, &entry.target, &entry_path, diagnostics);
        validate_non_empty_optional(
            &format!("{entry_path}.color"),
            Some(&entry.color),
            diagnostics,
        );
    }
}

fn validate_visibility(
    state: &SceneRecipeNamedStateV1,
    path: &str,
    node_ids: &BTreeSet<String>,
    import_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let mut targets = BTreeSet::new();
    for (entry_index, entry) in state.visibility.iter().enumerate() {
        let entry_path = format!("{path}.visibility[{entry_index}]");
        validate_target(
            &format!("{entry_path}.target"),
            &entry.target,
            node_ids,
            import_ids,
            diagnostics,
        );
        reject_duplicate_state_target(&mut targets, &entry.target, &entry_path, diagnostics);
    }
}

fn reject_duplicate_state_target(
    targets: &mut BTreeSet<SceneRecipeSpatialTargetV1>,
    target: &SceneRecipeSpatialTargetV1,
    path: &str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if !targets.insert(target.clone()) {
        diagnostics.push(error(
            "duplicate_state_target",
            path,
            "a named-state channel may target each persistent object only once",
            "keep one entry per channel and target",
        ));
    }
}

fn state_cycle(start: &str, states: &BTreeMap<&str, &SceneRecipeNamedStateV1>) -> bool {
    let mut seen = BTreeSet::new();
    let mut current = Some(start);
    while let Some(id) = current {
        if !seen.insert(id) {
            return true;
        }
        current = states.get(id).and_then(|state| state.inherits.as_deref());
    }
    false
}
