use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::scene::recipe::types::SceneRecipeDiagnosticV1;
use crate::scene::recipe::validation::diagnostic;

use super::super::{finite_vec3, validate_known_fields, validate_required_id};

const MORPH_FIELDS: &[&str] = &["id", "source_geometry", "targets"];
const MORPH_TARGET_FIELDS: &[&str] = &["position_deltas"];
const SKIN_FIELDS: &[&str] = &["id", "source_geometry", "joints", "weights"];

#[derive(Debug, Default)]
pub(in crate::scene::recipe::validation::authoring) struct MorphValidationInfo {
    pub(in crate::scene::recipe::validation::authoring) ids: BTreeSet<String>,
    pub(in crate::scene::recipe::validation::authoring) vertex_counts: BTreeMap<String, usize>,
    pub(in crate::scene::recipe::validation::authoring) target_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Default)]
pub(in crate::scene::recipe::validation::authoring) struct SkinValidationInfo {
    pub(in crate::scene::recipe::validation::authoring) ids: BTreeSet<String>,
    pub(in crate::scene::recipe::validation::authoring) vertex_counts: BTreeMap<String, usize>,
    pub(in crate::scene::recipe::validation::authoring) target_counts: BTreeMap<String, usize>,
    pub(in crate::scene::recipe::validation::authoring) max_joint_indices:
        BTreeMap<String, SkinJointIndexLimit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::scene::recipe::validation::authoring) struct SkinJointIndexLimit {
    pub(in crate::scene::recipe::validation::authoring) index: usize,
    pub(in crate::scene::recipe::validation::authoring) path: String,
}

pub(in crate::scene::recipe::validation::authoring) fn geometry_vertex_counts(
    value: Option<&Value>,
) -> BTreeMap<String, usize> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|geometry| {
            let id = geometry.get("id")?.as_str()?;
            let count = geometry
                .get("mesh")?
                .get("positions")?
                .as_array()
                .map(Vec::len)?;
            Some((id.to_owned(), count))
        })
        .collect()
}

pub(in crate::scene::recipe::validation::authoring) fn validate_morphs(
    value: Option<&Value>,
    source_geometry_ids: &BTreeSet<String>,
    source_vertex_counts: &BTreeMap<String, usize>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> MorphValidationInfo {
    let mut info = MorphValidationInfo::default();
    let Some(value) = value else {
        return info;
    };
    let Some(morphs) = value.as_array() else {
        diagnostics.push(diagnostic(
            "invalid_morphs",
            "error",
            "$.morphs",
            "morphs must be an array",
            "emit morphs:[{id,source_geometry,targets}]",
            None,
            false,
        ));
        return info;
    };

    for (index, morph) in morphs.iter().enumerate() {
        let path = format!("$.morphs[{index}]");
        let Some(object) = morph.as_object() else {
            diagnostics.push(diagnostic(
                "invalid_morph",
                "error",
                &path,
                "morph entry must be an object",
                "emit {id,source_geometry,targets}",
                None,
                false,
            ));
            continue;
        };
        validate_known_fields(&path, object, MORPH_FIELDS, diagnostics);
        validate_required_id(&path, object.get("id"), diagnostics);
        let Some(id) = object.get("id").and_then(Value::as_str) else {
            continue;
        };
        info.ids.insert(id.to_owned());
        let source = match validate_source_geometry(
            &format!("{path}.source_geometry"),
            object.get("source_geometry"),
            source_geometry_ids,
            diagnostics,
        ) {
            Some(source) => source,
            None => continue,
        };
        if let Some(count) = source_vertex_counts.get(source) {
            info.vertex_counts.insert(id.to_owned(), *count);
        }
        let target_count = validate_morph_targets(
            &format!("{path}.targets"),
            object.get("targets"),
            source_vertex_counts.get(source).copied(),
            diagnostics,
        );
        if let Some(target_count) = target_count {
            info.target_counts.insert(id.to_owned(), target_count);
        }
    }

    info
}

pub(in crate::scene::recipe::validation::authoring) fn validate_skins(
    value: Option<&Value>,
    source_geometry_ids: &BTreeSet<String>,
    source_vertex_counts: &BTreeMap<String, usize>,
    source_morph_target_counts: &BTreeMap<String, usize>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> SkinValidationInfo {
    let mut info = SkinValidationInfo::default();
    let Some(value) = value else {
        return info;
    };
    let Some(skins) = value.as_array() else {
        diagnostics.push(diagnostic(
            "invalid_skins",
            "error",
            "$.skins",
            "skins must be an array",
            "emit skins:[{id,source_geometry,joints,weights}]",
            None,
            false,
        ));
        return info;
    };

    for (index, skin) in skins.iter().enumerate() {
        let path = format!("$.skins[{index}]");
        let Some(object) = skin.as_object() else {
            diagnostics.push(diagnostic(
                "invalid_skin",
                "error",
                &path,
                "skin entry must be an object",
                "emit {id,source_geometry,joints,weights}",
                None,
                false,
            ));
            continue;
        };
        validate_known_fields(&path, object, SKIN_FIELDS, diagnostics);
        validate_required_id(&path, object.get("id"), diagnostics);
        let Some(id) = object.get("id").and_then(Value::as_str) else {
            continue;
        };
        info.ids.insert(id.to_owned());
        let source = match validate_source_geometry(
            &format!("{path}.source_geometry"),
            object.get("source_geometry"),
            source_geometry_ids,
            diagnostics,
        ) {
            Some(source) => source,
            None => continue,
        };
        let source_vertex_count = source_vertex_counts.get(source).copied();
        if let Some(limit) = validate_skin_joints(
            &format!("{path}.joints"),
            object.get("joints"),
            source_vertex_count,
            diagnostics,
        ) {
            info.max_joint_indices.insert(id.to_owned(), limit);
        }
        validate_skin_weights(
            &format!("{path}.weights"),
            object.get("weights"),
            source_vertex_count,
            diagnostics,
        );
        if let Some(count) = source_vertex_count {
            info.vertex_counts.insert(id.to_owned(), count);
        }
        if let Some(target_count) = source_morph_target_counts.get(source) {
            info.target_counts.insert(id.to_owned(), *target_count);
        }
    }

    info
}

fn validate_source_geometry<'a>(
    path: &str,
    value: Option<&'a Value>,
    ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> Option<&'a str> {
    match value.and_then(Value::as_str) {
        Some(value) if ids.contains(value) => Some(value),
        Some(value) => {
            diagnostics.push(diagnostic(
                "unknown_geometry_ref",
                "error",
                path,
                format!("geometry reference '{value}' does not name a declared geometry"),
                "declare the source geometry before referencing it",
                None,
                false,
            ));
            None
        }
        None => {
            diagnostics.push(diagnostic(
                "missing_geometry_ref",
                "error",
                path,
                "deformation must include a source_geometry string",
                "set source_geometry to a declared geometry id",
                None,
                false,
            ));
            None
        }
    }
}

fn validate_morph_targets(
    path: &str,
    value: Option<&Value>,
    source_vertex_count: Option<usize>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> Option<usize> {
    let Some(targets) = value
        .and_then(Value::as_array)
        .filter(|targets| !targets.is_empty())
    else {
        diagnostics.push(diagnostic(
            "invalid_morph",
            "error",
            path,
            "morph targets must be a non-empty array",
            "emit at least one target with position_deltas",
            None,
            false,
        ));
        return None;
    };
    for (target_index, target) in targets.iter().enumerate() {
        let target_path = format!("{path}[{target_index}]");
        let Some(object) = target.as_object() else {
            diagnostics.push(diagnostic(
                "invalid_morph",
                "error",
                &target_path,
                "morph target must be an object",
                "emit {position_deltas:[[x,y,z],...]}",
                None,
                false,
            ));
            continue;
        };
        validate_known_fields(&target_path, object, MORPH_TARGET_FIELDS, diagnostics);
        let Some(deltas) = object
            .get("position_deltas")
            .and_then(Value::as_array)
            .filter(|deltas| !deltas.is_empty())
        else {
            diagnostics.push(diagnostic(
                "invalid_morph",
                "error",
                format!("{target_path}.position_deltas"),
                "morph target position_deltas must be a non-empty array",
                "emit one [x,y,z] delta per source vertex",
                None,
                false,
            ));
            continue;
        };
        if let Some(expected) = source_vertex_count
            && deltas.len() != expected
        {
            diagnostics.push(diagnostic(
                "invalid_morph",
                "error",
                format!("{target_path}.position_deltas"),
                format!(
                    "morph target has {} position deltas but source geometry has {expected} vertices",
                    deltas.len()
                ),
                "emit exactly one [x,y,z] delta per source vertex",
                None,
                false,
            ));
        }
        for (delta_index, delta) in deltas.iter().enumerate() {
            if finite_vec3(delta).is_none() {
                diagnostics.push(diagnostic(
                    "invalid_morph",
                    "error",
                    format!("{target_path}.position_deltas[{delta_index}]"),
                    "morph delta must be a finite [x,y,z] array",
                    "emit three finite numbers",
                    None,
                    false,
                ));
            }
        }
    }
    Some(targets.len())
}

fn validate_skin_joints(
    path: &str,
    value: Option<&Value>,
    source_vertex_count: Option<usize>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> Option<SkinJointIndexLimit> {
    let Some(rows) = value
        .and_then(Value::as_array)
        .filter(|rows| !rows.is_empty())
    else {
        diagnostics.push(diagnostic(
            "invalid_skin",
            "error",
            path,
            "skin joints must be a non-empty array",
            "emit one four-index joint row per source vertex",
            None,
            false,
        ));
        return None;
    };
    if let Some(expected) = source_vertex_count
        && rows.len() != expected
    {
        diagnostics.push(diagnostic(
            "invalid_skin",
            "error",
            path,
            format!(
                "skin declares {} joint rows but source geometry has {expected} vertices",
                rows.len()
            ),
            "emit exactly one four-index joint row per source vertex",
            None,
            false,
        ));
    }
    let mut max_index = None;
    for (row_index, row) in rows.iter().enumerate() {
        let Some(values) = row.as_array() else {
            diagnostics.push(diagnostic(
                "invalid_skin",
                "error",
                format!("{path}[{row_index}]"),
                "skin joint row must be an array of four integers",
                "emit [j0,j1,j2,j3]",
                None,
                false,
            ));
            continue;
        };
        if values.len() != 4 {
            diagnostics.push(diagnostic(
                "invalid_skin",
                "error",
                format!("{path}[{row_index}]"),
                "skin joint row must contain exactly four indices",
                "emit [j0,j1,j2,j3]",
                None,
                false,
            ));
        }
        for (joint_index, value) in values.iter().enumerate() {
            let value_path = format!("{path}[{row_index}][{joint_index}]");
            match value.as_u64().and_then(|raw| usize::try_from(raw).ok()) {
                Some(index) => {
                    if max_index
                        .as_ref()
                        .is_none_or(|max: &SkinJointIndexLimit| index > max.index)
                    {
                        max_index = Some(SkinJointIndexLimit {
                            index,
                            path: value_path,
                        });
                    }
                }
                None => {
                    diagnostics.push(diagnostic(
                        "invalid_skin",
                        "error",
                        value_path,
                        "skin joint index must be a non-negative usize-compatible integer",
                        "emit integer joint indices into the node skin_binding joint list",
                        None,
                        false,
                    ));
                }
            }
        }
    }
    max_index
}

fn validate_skin_weights(
    path: &str,
    value: Option<&Value>,
    source_vertex_count: Option<usize>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(rows) = value
        .and_then(Value::as_array)
        .filter(|rows| !rows.is_empty())
    else {
        diagnostics.push(diagnostic(
            "invalid_skin",
            "error",
            path,
            "skin weights must be a non-empty array",
            "emit one four-weight row per source vertex",
            None,
            false,
        ));
        return;
    };
    if let Some(expected) = source_vertex_count
        && rows.len() != expected
    {
        diagnostics.push(diagnostic(
            "invalid_skin",
            "error",
            path,
            format!(
                "skin declares {} weight rows but source geometry has {expected} vertices",
                rows.len()
            ),
            "emit exactly one four-weight row per source vertex",
            None,
            false,
        ));
    }
    for (row_index, row) in rows.iter().enumerate() {
        let Some(values) = row.as_array() else {
            diagnostics.push(diagnostic(
                "invalid_skin",
                "error",
                format!("{path}[{row_index}]"),
                "skin weight row must be an array of four numbers",
                "emit [w0,w1,w2,w3]",
                None,
                false,
            ));
            continue;
        };
        if values.len() != 4 {
            diagnostics.push(diagnostic(
                "invalid_skin",
                "error",
                format!("{path}[{row_index}]"),
                "skin weight row must contain exactly four values",
                "emit [w0,w1,w2,w3]",
                None,
                false,
            ));
        }
        let mut any_positive = false;
        for (weight_index, value) in values.iter().enumerate() {
            match value.as_f64() {
                Some(weight) if weight.is_finite() && (0.0..=1.0).contains(&weight) => {
                    any_positive |= weight > 0.0;
                }
                _ => diagnostics.push(diagnostic(
                    "invalid_skin",
                    "error",
                    format!("{path}[{row_index}][{weight_index}]"),
                    "skin weight must be finite and within [0,1]",
                    "emit normalized non-negative weights",
                    None,
                    false,
                )),
            }
        }
        if !any_positive {
            diagnostics.push(diagnostic(
                "invalid_skin",
                "error",
                format!("{path}[{row_index}]"),
                "skin weight row must contain at least one positive influence",
                "assign the vertex to at least one joint",
                None,
                false,
            ));
        }
    }
}
