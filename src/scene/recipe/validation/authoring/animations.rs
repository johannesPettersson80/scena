use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::scene::recipe::RecipeBuildPolicy;
use crate::scene::recipe::types::SceneRecipeDiagnosticV1;
use crate::scene::recipe::validation::diagnostic;

use super::{validate_known_fields, validate_required_id};

mod values;
use values::validate_values;

const ANIMATION_FIELDS: &[&str] = &["id", "duration", "channels"];
const CHANNEL_FIELDS: &[&str] = &["target", "path", "interpolation", "times", "values"];
const TARGET_FIELDS: &[&str] = &["kind", "id"];
const CHANNEL_PATHS: &[&str] = &["translation", "rotation", "scale", "weights"];
const INTERPOLATIONS: &[&str] = &["linear", "step", "cubic_spline"];

struct AnimationValidationState<'a, 'd> {
    policy: &'a RecipeBuildPolicy,
    total_channels: &'a mut usize,
    total_keyframes: &'a mut usize,
    authored_target_ids: &'a BTreeSet<String>,
    target_ids: &'a BTreeSet<String>,
    import_ids: &'a BTreeSet<String>,
    authored_morph_target_counts: &'a BTreeMap<String, usize>,
    diagnostics: &'d mut Vec<SceneRecipeDiagnosticV1>,
}

pub(super) fn validate_animations(
    value: Option<&Value>,
    policy: &RecipeBuildPolicy,
    authored_target_ids: &BTreeSet<String>,
    target_ids: &BTreeSet<String>,
    import_ids: &BTreeSet<String>,
    authored_morph_target_counts: &BTreeMap<String, usize>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(animations) = value.as_array() else {
        diagnostics.push(diagnostic(
            "invalid_animations",
            "error",
            "$.animations",
            "animations must be an array",
            "emit animations:[{id,duration,channels}]",
            None,
            false,
        ));
        return;
    };
    if animations.len() > policy.max_animations() {
        diagnostics.push(diagnostic(
            "policy_violation",
            "error",
            "$.animations",
            format!(
                "recipe declares {} animations, exceeding RecipeBuildPolicy max_animations {}",
                animations.len(),
                policy.max_animations()
            ),
            "reduce animation count or raise the operator-owned max_animations policy",
            None,
            false,
        ));
    }
    let mut total_channels = 0usize;
    let mut total_keyframes = 0usize;
    for (index, animation) in animations.iter().enumerate() {
        let path = format!("$.animations[{index}]");
        let Some(object) = animation.as_object() else {
            diagnostics.push(diagnostic(
                "invalid_animation",
                "error",
                &path,
                "animation entry must be an object",
                "emit {id,duration,channels}",
                None,
                false,
            ));
            continue;
        };
        validate_known_fields(&path, object, ANIMATION_FIELDS, diagnostics);
        validate_required_id(&path, object.get("id"), diagnostics);
        let duration = validate_duration(&path, object.get("duration"), diagnostics);
        let mut state = AnimationValidationState {
            policy,
            total_channels: &mut total_channels,
            total_keyframes: &mut total_keyframes,
            authored_target_ids,
            target_ids,
            import_ids,
            authored_morph_target_counts,
            diagnostics,
        };
        validate_channels(&path, object.get("channels"), duration, &mut state);
    }
}

fn validate_duration(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> Option<f64> {
    match value.and_then(Value::as_f64) {
        Some(duration)
            if duration.is_finite() && duration > 0.0 && duration <= f64::from(f32::MAX) =>
        {
            Some(duration)
        }
        _ => {
            diagnostics.push(diagnostic(
                "invalid_animation_duration",
                "error",
                format!("{path}.duration"),
                "animation duration must be a finite positive number of seconds",
                "emit a duration such as 1.0",
                None,
                false,
            ));
            None
        }
    }
}

fn validate_channels(
    path: &str,
    value: Option<&Value>,
    duration: Option<f64>,
    state: &mut AnimationValidationState<'_, '_>,
) {
    let Some(channels) = value
        .and_then(Value::as_array)
        .filter(|channels| !channels.is_empty())
    else {
        state.diagnostics.push(diagnostic(
            "invalid_animation_channels",
            "error",
            format!("{path}.channels"),
            "animation channels must be a non-empty array",
            "emit at least one channel with target, path, times, and values",
            None,
            false,
        ));
        return;
    };

    *state.total_channels = (*state.total_channels).saturating_add(channels.len());
    if *state.total_channels > state.policy.max_animation_channels() {
        state.diagnostics.push(diagnostic(
            "policy_violation",
            "error",
            format!("{path}.channels"),
            format!(
                "recipe declares {} animation channels, exceeding RecipeBuildPolicy max_animation_channels {}",
                *state.total_channels,
                state.policy.max_animation_channels()
            ),
            "reduce animation channel count or raise the operator-owned max_animation_channels policy",
            None,
            false,
        ));
    }

    for (index, channel) in channels.iter().enumerate() {
        let channel_path = format!("{path}.channels[{index}]");
        let Some(object) = channel.as_object() else {
            state.diagnostics.push(diagnostic(
                "invalid_animation_channel",
                "error",
                &channel_path,
                "animation channel must be an object",
                "emit {target,path,interpolation,times,values}",
                None,
                false,
            ));
            continue;
        };
        validate_known_fields(&channel_path, object, CHANNEL_FIELDS, state.diagnostics);
        let target_id = validate_target(
            &format!("{channel_path}.target"),
            object.get("target"),
            state.target_ids,
            state.import_ids,
            state.diagnostics,
        );
        let channel_kind = validate_channel_path(
            &format!("{channel_path}.path"),
            object.get("path"),
            state.diagnostics,
        );
        let authored_weight_count = if channel_kind == Some("weights") {
            target_id.as_deref().and_then(|id| {
                if state.authored_target_ids.contains(id) {
                    match state.authored_morph_target_counts.get(id).copied() {
                        Some(count) => Some(count),
                        None => {
                            state.diagnostics.push(diagnostic(
                                "invalid_animation_target",
                                "error",
                                format!("{channel_path}.target.id"),
                                "authored weights animation requires a morph-capable target node",
                                "target a node using a morph-derived geometry",
                                None,
                                false,
                            ));
                            None
                        }
                    }
                } else {
                    None
                }
            })
        } else {
            None
        };
        validate_interpolation(
            &format!("{channel_path}.interpolation"),
            object.get("interpolation"),
            state.diagnostics,
        );
        let times = validate_times(
            &format!("{channel_path}.times"),
            object.get("times"),
            duration,
            state.diagnostics,
        );
        if let Some(times) = times {
            *state.total_keyframes = (*state.total_keyframes).saturating_add(times);
            if *state.total_keyframes > state.policy.max_animation_keyframes() {
                state.diagnostics.push(diagnostic(
                    "policy_violation",
                    "error",
                    &channel_path,
                    format!(
                        "recipe declares {} animation keyframes, exceeding RecipeBuildPolicy max_animation_keyframes {}",
                        *state.total_keyframes,
                        state.policy.max_animation_keyframes()
                    ),
                    "reduce keyframe count or raise the operator-owned max_animation_keyframes policy",
                    None,
                    false,
                ));
            }
        }
        validate_values(
            &format!("{channel_path}.values"),
            object.get("values"),
            channel_kind,
            object
                .get("interpolation")
                .and_then(Value::as_str)
                .unwrap_or("linear"),
            times,
            authored_weight_count,
            state.diagnostics,
        );
    }
}

fn validate_target(
    path: &str,
    value: Option<&Value>,
    target_ids: &BTreeSet<String>,
    import_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> Option<String> {
    let Some(object) = value.and_then(Value::as_object) else {
        diagnostics.push(diagnostic(
            "invalid_animation_target",
            "error",
            path,
            "animation target must be {kind:\"node\",id}",
            "target an authored node id or an imported child id such as machine:/Arm",
            None,
            false,
        ));
        return None;
    };
    validate_known_fields(path, object, TARGET_FIELDS, diagnostics);
    match (
        object.get("kind").and_then(Value::as_str),
        object.get("id").and_then(Value::as_str),
    ) {
        (Some("node"), Some(id))
            if target_ids.contains(id) || import_child_ref_is_plausible(id, import_ids) =>
        {
            Some(id.to_owned())
        }
        (Some("node"), Some(id)) => {
            diagnostics.push(diagnostic(
                "unknown_animation_target",
                "error",
                format!("{path}.id"),
                format!("animation target references unknown node id '{id}'"),
                "target a node from nodes, labels, instance_sets, or <import_id>:/<path>",
                None,
                false,
            ));
            Some(id.to_owned())
        }
        (Some("node"), None) => {
            diagnostics.push(diagnostic(
                "missing_animation_target",
                "error",
                format!("{path}.id"),
                "node animation target requires an id",
                "target a node from the recipe manifest",
                None,
                false,
            ));
            None
        }
        (Some(kind), _) => {
            diagnostics.push(diagnostic(
                "unsupported_feature",
                "error",
                format!("{path}.kind"),
                format!("animation target kind '{kind}' is not supported"),
                "use kind:\"node\"",
                None,
                false,
            ));
            None
        }
        (None, _) => {
            diagnostics.push(diagnostic(
                "missing_animation_target_kind",
                "error",
                format!("{path}.kind"),
                "animation target must include kind:\"node\"",
                "use target:{kind:\"node\",id:\"...\"}",
                None,
                false,
            ));
            None
        }
    }
}

fn import_child_ref_is_plausible(id: &str, import_ids: &BTreeSet<String>) -> bool {
    let Some((import, path)) = id.split_once(":/") else {
        return false;
    };
    import_ids.contains(import) && !path.trim().is_empty()
}

fn validate_channel_path(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> Option<&'static str> {
    match value.and_then(Value::as_str) {
        Some(kind) if CHANNEL_PATHS.contains(&kind) => CHANNEL_PATHS
            .iter()
            .copied()
            .find(|candidate| *candidate == kind),
        Some(kind) => {
            diagnostics.push(diagnostic(
                "invalid_animation_path",
                "error",
                path,
                format!("animation path '{kind}' is not supported"),
                "use translation, rotation, scale, or weights",
                None,
                false,
            ));
            None
        }
        None => {
            diagnostics.push(diagnostic(
                "missing_animation_path",
                "error",
                path,
                "animation channel must include a path string",
                "use path:\"translation\"",
                None,
                false,
            ));
            None
        }
    }
}

fn validate_interpolation(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    match value.and_then(Value::as_str) {
        None => {}
        Some(interpolation) if INTERPOLATIONS.contains(&interpolation) => {}
        Some(interpolation) => diagnostics.push(diagnostic(
            "invalid_animation_interpolation",
            "error",
            path,
            format!("animation interpolation '{interpolation}' is not supported"),
            "use linear, step, or cubic_spline",
            None,
            false,
        )),
    }
}

fn validate_times(
    path: &str,
    value: Option<&Value>,
    duration: Option<f64>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> Option<usize> {
    let Some(times) = value
        .and_then(Value::as_array)
        .filter(|times| !times.is_empty())
    else {
        diagnostics.push(diagnostic(
            "invalid_animation_times",
            "error",
            path,
            "animation times must be a non-empty array",
            "emit finite non-negative seconds in strictly increasing order",
            None,
            false,
        ));
        return None;
    };
    let mut previous = None;
    for (index, time) in times.iter().enumerate() {
        let Some(time) = time.as_f64() else {
            diagnostics.push(diagnostic(
                "invalid_animation_time",
                "error",
                format!("{path}[{index}]"),
                "animation time must be numeric",
                "emit finite non-negative seconds",
                None,
                false,
            ));
            continue;
        };
        if !time.is_finite() || time < 0.0 || time > f64::from(f32::MAX) {
            diagnostics.push(diagnostic(
                "invalid_animation_time",
                "error",
                format!("{path}[{index}]"),
                format!("animation time must be finite and non-negative, got {time}"),
                "emit finite non-negative seconds",
                None,
                false,
            ));
        } else if duration.is_some_and(|duration| time > duration) {
            diagnostics.push(diagnostic(
                "invalid_animation_duration",
                "error",
                format!("{path}[{index}]"),
                format!("animation keyframe time {time} exceeds clip duration"),
                "set duration to at least the largest keyframe time or lower the keyframe time",
                None,
                false,
            ));
        } else if previous.is_some_and(|previous| time <= previous) {
            diagnostics.push(diagnostic(
                "invalid_animation_times",
                "error",
                format!("{path}[{index}]"),
                "animation times must be strictly increasing",
                "sort times and remove duplicates",
                None,
                false,
            ));
        }
        previous = Some(time);
    }
    Some(times.len())
}
