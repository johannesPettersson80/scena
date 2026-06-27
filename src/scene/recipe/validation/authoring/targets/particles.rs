use std::collections::BTreeSet;

use serde_json::Value;

use crate::scene::recipe::types::SceneRecipeDiagnosticV1;
use crate::scene::recipe::validation::diagnostic;

use super::super::validate_known_fields;
use super::common::{
    TransformUse, validate_optional_i16, validate_optional_u64, validate_transform, validate_vec3,
};
use super::extras::validate_color_value;
use super::import_refs::validate_node_ref;

const PARTICLE_SET_FIELDS: &[&str] = &[
    "id",
    "parent",
    "transform",
    "visible",
    "layer_mask",
    "render_group",
    "particles",
];
const PARTICLE_FIELDS: &[&str] = &["id", "position", "color", "size_px", "rotation_degrees"];

pub(in crate::scene::recipe::validation::authoring) fn has_authored_particle_sets(
    value: Option<&Value>,
) -> bool {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|entry| {
            entry
                .get("particles")
                .and_then(Value::as_array)
                .is_some_and(|particles| !particles.is_empty())
        })
}

pub(in crate::scene::recipe::validation::authoring) fn validate_particle_sets(
    value: Option<&Value>,
    colors: &BTreeSet<String>,
    node_ids: &BTreeSet<String>,
    import_ids: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(particle_sets) = value.as_array() else {
        diagnostics.push(diagnostic(
            "invalid_particles",
            "error",
            "$.particles",
            "particles must be an array of particle set objects",
            "emit particles:[{id,particles:[...]}]",
            None,
            false,
        ));
        return;
    };
    for (index, particle_set) in particle_sets.iter().enumerate() {
        let path = format!("$.particles[{index}]");
        let Some(object) = particle_set.as_object() else {
            diagnostics.push(diagnostic(
                "invalid_particle_set",
                "error",
                &path,
                "particle set entry must be an object",
                "emit {id, particles:[{id,position,color,size_px}]}",
                None,
                false,
            ));
            continue;
        };
        validate_known_fields(&path, object, PARTICLE_SET_FIELDS, diagnostics);
        validate_particle_set_id(&format!("{path}.id"), object.get("id"), diagnostics);
        if let Some(parent) = object.get("parent") {
            validate_node_ref(
                &format!("{path}.parent"),
                Some(parent),
                node_ids,
                import_ids,
                diagnostics,
            );
        }
        if let Some(transform) = object.get("transform") {
            validate_transform(
                &format!("{path}.transform"),
                transform,
                TransformUse::Node,
                node_ids,
                import_ids,
                diagnostics,
            );
        }
        if object
            .get("visible")
            .is_some_and(|value| !value.is_boolean())
        {
            diagnostics.push(diagnostic(
                "invalid_visible",
                "error",
                format!("{path}.visible"),
                "visible must be a boolean",
                "use true or false",
                None,
                false,
            ));
        }
        validate_optional_u64(
            &format!("{path}.layer_mask"),
            object.get("layer_mask"),
            diagnostics,
        );
        validate_optional_i16(
            &format!("{path}.render_group"),
            object.get("render_group"),
            diagnostics,
        );
        validate_particles(&path, object.get("particles"), colors, diagnostics);
    }
}

fn validate_particles(
    path: &str,
    value: Option<&Value>,
    colors: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        diagnostics.push(diagnostic(
            "missing_particles",
            "error",
            format!("{path}.particles"),
            "particle set must include at least one particle",
            "emit particles:[{id,position,color,size_px}]",
            None,
            false,
        ));
        return;
    };
    let Some(particles) = value.as_array().filter(|particles| !particles.is_empty()) else {
        diagnostics.push(diagnostic(
            "invalid_particles",
            "error",
            format!("{path}.particles"),
            "particles must be a non-empty array",
            "emit one object per host-supplied particle",
            None,
            false,
        ));
        return;
    };
    let mut ids = BTreeSet::new();
    for (index, particle) in particles.iter().enumerate() {
        let particle_path = format!("{path}.particles[{index}]");
        let Some(object) = particle.as_object() else {
            diagnostics.push(diagnostic(
                "invalid_particle",
                "error",
                &particle_path,
                "particle entry must be an object",
                "emit {id, position:[x,y,z], color, size_px}",
                None,
                false,
            ));
            continue;
        };
        validate_known_fields(&particle_path, object, PARTICLE_FIELDS, diagnostics);
        validate_particle_id(
            &format!("{particle_path}.id"),
            object.get("id"),
            &mut ids,
            diagnostics,
        );
        validate_vec3(
            &format!("{particle_path}.position"),
            object.get("position"),
            diagnostics,
        );
        validate_color_value(
            &format!("{particle_path}.color"),
            object.get("color"),
            colors,
            diagnostics,
        );
        match object.get("size_px").and_then(Value::as_f64) {
            Some(size) if size.is_finite() && size > 0.0 && size <= f64::from(f32::MAX) => {}
            _ => diagnostics.push(diagnostic(
                "invalid_particle",
                "error",
                format!("{particle_path}.size_px"),
                "particle size_px must be finite and positive",
                "use a positive screen-space sprite size in pixels",
                None,
                false,
            )),
        }
        if let Some(rotation) = object.get("rotation_degrees") {
            match rotation.as_f64() {
                Some(rotation) if rotation.is_finite() => {}
                _ => diagnostics.push(diagnostic(
                    "invalid_particle",
                    "error",
                    format!("{particle_path}.rotation_degrees"),
                    "particle rotation_degrees must be finite when present",
                    "omit rotation_degrees or use a finite degree value",
                    None,
                    false,
                )),
            }
        }
    }
}

fn validate_particle_set_id(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    match value.and_then(Value::as_str) {
        Some(id) if !id.trim().is_empty() => {}
        Some(_) => diagnostics.push(diagnostic(
            "invalid_id",
            "error",
            path,
            "particle set id must not be empty",
            "use a stable caller-owned id",
            None,
            false,
        )),
        None => diagnostics.push(diagnostic(
            "missing_id",
            "error",
            path,
            "particle set must include an id string",
            "add a stable caller-owned id",
            None,
            false,
        )),
    }
}

fn validate_particle_id(
    path: &str,
    value: Option<&Value>,
    ids: &mut BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    match value.and_then(Value::as_str) {
        Some(id) if !id.trim().is_empty() && ids.insert(id.to_owned()) => {}
        Some(id) if id.trim().is_empty() => diagnostics.push(diagnostic(
            "invalid_id",
            "error",
            path,
            "particle id must not be empty",
            "use a stable per-particle id",
            None,
            false,
        )),
        Some(id) => diagnostics.push(diagnostic(
            "duplicate_id",
            "error",
            path,
            format!("particle id '{id}' is used more than once in this set"),
            "make per-particle ids unique within the set",
            None,
            false,
        )),
        None => diagnostics.push(diagnostic(
            "missing_id",
            "error",
            path,
            "particle entry must include an id string",
            "add a stable per-particle id",
            None,
            false,
        )),
    }
}
