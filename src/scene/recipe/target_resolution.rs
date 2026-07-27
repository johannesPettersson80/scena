use std::collections::BTreeSet;

use crate::diagnostics::nearest_name_candidates;

use super::{SceneRecipeBuildV1, SceneRecipeTargetV1};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SceneRecipeTargetResolutionMode {
    /// Resolve to one concrete node handle. Whole-import targets are rejected.
    SingleHandle,
    /// Resolve to every handle that belongs to the declared subject target.
    Subject,
    /// Resolve subject handles even when authored recipe visibility hides the target.
    ///
    /// Measurement and composition callers use this mode so they can report
    /// `subject_hidden` from the rendered evidence instead of failing target
    /// resolution before a structured render report exists.
    SubjectIncludingHidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SceneRecipeTargetResolutionErrorKind {
    Unresolved,
    Unsupported,
    Hidden,
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneRecipeTargetResolutionError {
    pub kind: SceneRecipeTargetResolutionErrorKind,
    pub message: String,
    pub candidates: Vec<String>,
}

pub fn resolve_scene_recipe_target_handles(
    manifest: &SceneRecipeBuildV1,
    target: &SceneRecipeTargetV1,
    mode: SceneRecipeTargetResolutionMode,
) -> Result<Vec<u64>, SceneRecipeTargetResolutionError> {
    match target {
        SceneRecipeTargetV1::Node { id } => resolve_node_target(manifest, id, mode),
        SceneRecipeTargetV1::Import { id } => resolve_import_target(manifest, id, mode),
        SceneRecipeTargetV1::World { .. } => Err(SceneRecipeTargetResolutionError {
            kind: SceneRecipeTargetResolutionErrorKind::Unsupported,
            message: "world targets do not resolve to stable scene handles".to_owned(),
            candidates: Vec::new(),
        }),
    }
}

fn resolve_node_target(
    manifest: &SceneRecipeBuildV1,
    id: &str,
    mode: SceneRecipeTargetResolutionMode,
) -> Result<Vec<u64>, SceneRecipeTargetResolutionError> {
    if let Some(node) = manifest.nodes.iter().find(|node| node.id == id) {
        if node.visible == Some(false)
            && mode != SceneRecipeTargetResolutionMode::SubjectIncludingHidden
        {
            return Err(SceneRecipeTargetResolutionError {
                kind: SceneRecipeTargetResolutionErrorKind::Hidden,
                message: format!("target node id '{id}' is hidden by recipe visibility"),
                candidates: Vec::new(),
            });
        }
        return Ok(vec![node.handle]);
    }
    if let Some(handle) = manifest
        .imports
        .iter()
        .find_map(|import| import.nodes_by_path.get(id).copied())
    {
        return Ok(vec![handle]);
    }
    Err(SceneRecipeTargetResolutionError {
        kind: SceneRecipeTargetResolutionErrorKind::Unresolved,
        message: format!("target node id '{id}' was not in the build manifest"),
        candidates: nearest_name_candidates(id, node_candidates(manifest), 3),
    })
}

fn resolve_import_target(
    manifest: &SceneRecipeBuildV1,
    id: &str,
    mode: SceneRecipeTargetResolutionMode,
) -> Result<Vec<u64>, SceneRecipeTargetResolutionError> {
    if mode == SceneRecipeTargetResolutionMode::SingleHandle {
        return Err(SceneRecipeTargetResolutionError {
            kind: SceneRecipeTargetResolutionErrorKind::Unsupported,
            message: format!("target import id '{id}' requires subject-style resolution"),
            candidates: nearest_name_candidates(id, import_candidates(manifest), 3),
        });
    }
    let Some(import) = manifest.imports.iter().find(|import| import.id == id) else {
        return Err(SceneRecipeTargetResolutionError {
            kind: SceneRecipeTargetResolutionErrorKind::Unresolved,
            message: format!("target import id '{id}' was not in the build manifest"),
            candidates: nearest_name_candidates(id, import_candidates(manifest), 3),
        });
    };
    let mut handles = BTreeSet::new();
    handles.insert(import.import_handle);
    handles.extend(import.root_handles.iter().copied());
    handles.extend(import.primary_root);
    handles.extend(import.nodes_by_path.values().copied());
    if handles.is_empty() {
        return Err(SceneRecipeTargetResolutionError {
            kind: SceneRecipeTargetResolutionErrorKind::Empty,
            message: format!("target import id '{id}' resolved to no addressable handles"),
            candidates: Vec::new(),
        });
    }
    Ok(handles.into_iter().collect())
}

fn import_candidates(manifest: &SceneRecipeBuildV1) -> Vec<String> {
    manifest
        .imports
        .iter()
        .map(|import| import.id.clone())
        .collect()
}

fn node_candidates(manifest: &SceneRecipeBuildV1) -> Vec<String> {
    let mut candidates = BTreeSet::new();
    for node in &manifest.nodes {
        candidates.insert(node.id.clone());
    }
    for import in &manifest.imports {
        candidates.extend(import.nodes_by_path.keys().cloned());
    }
    candidates.into_iter().collect()
}
