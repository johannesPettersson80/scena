use std::collections::{BTreeMap, BTreeSet};

use serde_json::json;

use crate::NodeKey;
use crate::assets::DefaultAssetFetcher;
use crate::scene::recipe::{
    SceneRecipeBuildNamedStateV1, SceneRecipeDiagnosticV1, SceneRecipeNamedStateV1,
    SceneRecipeSpatialTargetV1, SceneRecipeStateTintV1, SceneRecipeStateTransformV1,
    SceneRecipeStateVisibilityV1,
};
use crate::scene_host::{
    SceneHostCore, SceneHostVisualStateV1, VisualPatchTintV1, VisualPatchTransformV1,
    VisualPatchV1, VisualPatchVisibilityV1,
};

use super::{SpatialBuildInputs, resolve_target};
use crate::scene_host::recipe::authoring::{authored_color, local_transform_from_recipe};
use crate::scene_host::recipe::{error_diagnostic, scene_host_error_diagnostic};

pub(super) fn build_named_states(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    colors: &BTreeMap<String, crate::SceneRecipeColorV1>,
    recipes: &[SceneRecipeNamedStateV1],
    context: &SpatialBuildInputs<'_>,
    manifest: &mut Vec<SceneRecipeBuildNamedStateV1>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let recipes_by_id = recipes
        .iter()
        .map(|recipe| (recipe.id.as_str(), recipe))
        .collect::<BTreeMap<_, _>>();
    let mut resolved = BTreeMap::<String, ResolvedState>::new();
    for recipe in recipes {
        let mut visiting = BTreeSet::new();
        if let Err(diagnostic) =
            resolve_state(&recipe.id, &recipes_by_id, &mut visiting, &mut resolved)
        {
            diagnostics.push(diagnostic);
        }
    }
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == "error")
    {
        return;
    }

    let mut active = None;
    for (index, recipe) in recipes.iter().enumerate() {
        let path = format!("$.named_states[{index}]");
        let Some(state) = resolved.get(&recipe.id) else {
            continue;
        };
        let patch = match visual_patch(host, state, context, colors) {
            Ok(patch) => patch,
            Err((code, message)) => {
                diagnostics.push(error_diagnostic(
                    &path,
                    code,
                    message,
                    "fix named-state targets, transforms, and colors",
                ));
                continue;
            }
        };
        let stored = SceneHostVisualStateV1::new(recipe.id.clone(), patch).with_metadata(json!({
            "recipe_id": recipe.id,
            "inherits": recipe.inherits,
            "identity_scope": "persistent_recipe_id"
        }));
        if let Err(error) = host.store_visual_state(stored) {
            diagnostics.push(scene_host_error_diagnostic(
                &path,
                "named_state_store_failed",
                error,
            ));
            continue;
        }
        manifest.push(SceneRecipeBuildNamedStateV1 {
            id: recipe.id.clone(),
            identity_scope: "persistent_recipe_id".to_owned(),
            active: recipe.active,
            inherited_from: recipe.inherits.clone(),
            transform_count: state.transforms.len(),
            tint_count: state.tints.len(),
            visibility_count: state.visibility.len(),
            status: "resolved".to_owned(),
        });
        if recipe.active {
            active = Some(recipe.id.clone());
        }
    }
    if let Some(active) = active {
        match host.apply_visual_state(&active) {
            Ok(result) if result.failed.is_empty() => {}
            Ok(result) => diagnostics.push(error_diagnostic(
                "$.named_states",
                "named_state_apply_failed",
                format!(
                    "active named state '{active}' had failed entries: {:?}",
                    result.failed
                ),
                "fix the persistent target mapping before applying the state",
            )),
            Err(error) => diagnostics.push(scene_host_error_diagnostic(
                "$.named_states",
                "named_state_apply_failed",
                error,
            )),
        }
    }
}

#[derive(Debug, Clone, Default)]
struct ResolvedState {
    transforms: BTreeMap<SceneRecipeSpatialTargetV1, SceneRecipeStateTransformV1>,
    tints: BTreeMap<SceneRecipeSpatialTargetV1, SceneRecipeStateTintV1>,
    visibility: BTreeMap<SceneRecipeSpatialTargetV1, SceneRecipeStateVisibilityV1>,
}

// Keep the structured diagnostic unboxed at this recursive, build-time-only
// boundary; callers immediately append it to the recipe diagnostic report.
#[allow(clippy::result_large_err)]
fn resolve_state(
    id: &str,
    recipes: &BTreeMap<&str, &SceneRecipeNamedStateV1>,
    visiting: &mut BTreeSet<String>,
    resolved: &mut BTreeMap<String, ResolvedState>,
) -> Result<ResolvedState, SceneRecipeDiagnosticV1> {
    if let Some(state) = resolved.get(id) {
        return Ok(state.clone());
    }
    if !visiting.insert(id.to_owned()) {
        return Err(error_diagnostic(
            "$.named_states",
            "state_inheritance_cycle",
            format!("named state '{id}' participates in an inheritance cycle"),
            "use acyclic single inheritance",
        ));
    }
    let Some(recipe) = recipes.get(id).copied() else {
        return Err(error_diagnostic(
            "$.named_states",
            "unknown_state_parent",
            format!("named state '{id}' does not exist"),
            "reference a declared named state",
        ));
    };
    let mut state = match recipe.inherits.as_deref() {
        Some(parent) => resolve_state(parent, recipes, visiting, resolved)?,
        None => ResolvedState::default(),
    };
    for entry in &recipe.transforms {
        state.transforms.insert(entry.target.clone(), entry.clone());
    }
    for entry in &recipe.tints {
        state.tints.insert(entry.target.clone(), entry.clone());
    }
    for entry in &recipe.visibility {
        state.visibility.insert(entry.target.clone(), entry.clone());
    }
    visiting.remove(id);
    resolved.insert(id.to_owned(), state.clone());
    Ok(state)
}

fn visual_patch(
    host: &SceneHostCore<DefaultAssetFetcher>,
    state: &ResolvedState,
    context: &SpatialBuildInputs<'_>,
    colors: &BTreeMap<String, crate::SceneRecipeColorV1>,
) -> Result<VisualPatchV1, (&'static str, String)> {
    let transforms = state
        .transforms
        .values()
        .map(|entry| {
            let node = resolve_target(host, &entry.target, context)
                .map_err(|message| ("unknown_spatial_target", message))?;
            let transform = local_transform_from_recipe(Some(&entry.transform))
                .map_err(|diagnostic| ("invalid_spatial_transform", diagnostic.message.clone()))?;
            Ok::<_, (&'static str, String)>(VisualPatchTransformV1 {
                node: context_handle(host, node)?,
                transform,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let tints = state
        .tints
        .values()
        .map(|entry| {
            let node = resolve_target(host, &entry.target, context)
                .map_err(|message| ("unknown_spatial_target", message))?;
            let tint = authored_color(colors, &entry.color)
                .map_err(|diagnostic| ("invalid_state_tint", diagnostic.message.clone()))?;
            Ok::<_, (&'static str, String)>(VisualPatchTintV1 {
                node: context_handle(host, node)?,
                tint: Some(tint),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let visibility = state
        .visibility
        .values()
        .map(|entry| {
            let node = resolve_target(host, &entry.target, context)
                .map_err(|message| ("unknown_spatial_target", message))?;
            Ok::<_, (&'static str, String)>(VisualPatchVisibilityV1 {
                node: context_handle(host, node)?,
                visible: entry.visible,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(VisualPatchV1 {
        transforms,
        tints,
        visibility,
        ..VisualPatchV1::default()
    })
}

fn context_handle(
    host: &SceneHostCore<DefaultAssetFetcher>,
    node: NodeKey,
) -> Result<u64, (&'static str, String)> {
    host.node_handle_map.get(&node).copied().ok_or_else(|| {
        (
            "unknown_spatial_target",
            format!("resolved node {node:?} has no build-scoped host handle"),
        )
    })
}
