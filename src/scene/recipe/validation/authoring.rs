use serde_json::{Map, Value};

use super::super::RecipeBuildPolicy;
use super::super::types::SceneRecipeDiagnosticV1;
use super::diagnostic;

mod animations;
mod ids;
mod reflective_tessellation;
mod resources;
mod targets;

pub(in crate::scene::recipe::validation) use targets::{TransformUse, validate_transform};

pub(super) fn has_authored_renderable_nodes(object: &Map<String, Value>) -> bool {
    targets::has_authored_renderable_nodes(object)
        || targets::has_authored_instance_sets(object.get("instance_sets"))
        || targets::has_authored_particle_sets(object.get("particles"))
        || targets::has_authored_labels(object.get("labels"))
}

pub(super) fn validate_authoring_sections(
    object: &Map<String, Value>,
    policy: &RecipeBuildPolicy,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    ids::validate_global_ids(object, diagnostics);
    resources::validate_colors(object.get("colors"), diagnostics);
    let color_ids = ids::id_set_from_map(object.get("colors"));
    resources::validate_geometries(object.get("geometries"), diagnostics);
    let geometry_ids = ids::id_set_from_array(object.get("geometries"));
    let mut geometry_vertex_counts = resources::geometry_vertex_counts(object.get("geometries"));
    let morph_info = resources::validate_morphs(
        object.get("morphs"),
        &geometry_ids,
        &geometry_vertex_counts,
        diagnostics,
    );
    let mut morph_source_ids = geometry_ids.clone();
    morph_source_ids.extend(morph_info.ids.iter().cloned());
    geometry_vertex_counts.extend(morph_info.vertex_counts.clone());
    let skin_info = resources::validate_skins(
        object.get("skins"),
        &morph_source_ids,
        &geometry_vertex_counts,
        &morph_info.target_counts,
        diagnostics,
    );
    geometry_vertex_counts.extend(skin_info.vertex_counts.clone());
    let mut all_geometry_ids = geometry_ids.clone();
    all_geometry_ids.extend(morph_info.ids.iter().cloned());
    all_geometry_ids.extend(skin_info.ids.iter().cloned());
    let mut morph_target_counts = morph_info.target_counts.clone();
    morph_target_counts.extend(skin_info.target_counts.clone());
    resources::validate_materials(object.get("materials"), &color_ids, diagnostics);
    let material_ids = ids::id_set_from_array(object.get("materials"));
    reflective_tessellation::validate_reflective_sphere_tessellation(object, diagnostics);
    resources::validate_fonts(object.get("fonts"), diagnostics);
    let font_ids = ids::id_set_from_array(object.get("fonts"));
    let import_ids = ids::id_set_from_array(object.get("imports"));
    let node_ids = ids::id_set_from_array(object.get("nodes"));
    targets::validate_nodes(
        object.get("nodes"),
        targets::NodeValidationResources {
            geometries: &all_geometry_ids,
            materials: &material_ids,
            morph_target_counts: &morph_target_counts,
            skinned_geometries: &skin_info.ids,
            skin_max_joint_indices: &skin_info.max_joint_indices,
            all_node_ids: &node_ids,
            imports: &import_ids,
        },
        diagnostics,
    );
    let authored_morph_targets =
        authored_morph_animation_targets(object.get("nodes"), &morph_target_counts);
    targets::validate_instance_sets(
        object.get("instance_sets"),
        &geometry_ids,
        &material_ids,
        &color_ids,
        &node_ids,
        &import_ids,
        diagnostics,
    );
    let instance_set_ids = ids::id_set_from_array(object.get("instance_sets"));
    let mut particle_parent_ids = node_ids.clone();
    particle_parent_ids.extend(instance_set_ids.iter().cloned());
    targets::validate_particle_sets(
        object.get("particles"),
        &color_ids,
        &particle_parent_ids,
        &import_ids,
        diagnostics,
    );
    let particle_set_ids = ids::id_set_from_array(object.get("particles"));
    let mut label_target_ids = particle_parent_ids;
    label_target_ids.extend(particle_set_ids.iter().cloned());
    targets::validate_labels(
        object.get("labels"),
        &color_ids,
        &font_ids,
        &label_target_ids,
        &import_ids,
        diagnostics,
    );
    let label_ids = ids::id_set_from_array(object.get("labels"));
    let mut camera_target_ids = label_target_ids;
    camera_target_ids.extend(label_ids);
    targets::validate_clipping_planes(object.get("clipping_planes"), diagnostics);
    let animation_target_ids = camera_target_ids.clone();
    animations::validate_animations(
        object.get("animations"),
        policy,
        &animation_target_ids,
        &animation_target_ids,
        &import_ids,
        &authored_morph_targets,
        diagnostics,
    );
    targets::validate_cameras(
        object.get("cameras"),
        &camera_target_ids,
        &import_ids,
        diagnostics,
    );
    targets::validate_lights(object.get("lights"), &color_ids, diagnostics);
}

fn authored_morph_animation_targets(
    nodes: Option<&Value>,
    morph_target_counts: &std::collections::BTreeMap<String, usize>,
) -> std::collections::BTreeMap<String, usize> {
    nodes
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|node| {
            let id = node.get("id")?.as_str()?;
            let geometry = node.get("geometry")?.as_str()?;
            let target_count = morph_target_counts.get(geometry).copied()?;
            Some((id.to_owned(), target_count))
        })
        .collect()
}

fn validate_required_id(
    path: &str,
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    match value.and_then(Value::as_str) {
        Some(id) if !id.trim().is_empty() => {}
        Some(_) => diagnostics.push(diagnostic(
            "invalid_id",
            "error",
            format!("{path}.id"),
            "id must not be empty",
            "use a stable caller-owned id",
            None,
            false,
        )),
        None => diagnostics.push(diagnostic(
            "missing_id",
            "error",
            format!("{path}.id"),
            "entry must include an id string",
            "add a stable caller-owned id",
            None,
            false,
        )),
    }
}

fn validate_known_fields(
    path: &str,
    object: &Map<String, Value>,
    allowed: &[&str],
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            diagnostics.push(diagnostic(
                "unknown_field",
                "error",
                format!("{path}.{key}"),
                format!("field '{key}' is not accepted here"),
                "remove the field or wait for the feature slice that owns it",
                None,
                false,
            ));
        }
    }
}

fn finite_vec3(value: &Value) -> Option<[f32; 3]> {
    let array = value.as_array()?;
    let [x, y, z] = array.as_slice() else {
        return None;
    };
    let x = x.as_f64()? as f32;
    let y = y.as_f64()? as f32;
    let z = z.as_f64()? as f32;
    (x.is_finite() && y.is_finite() && z.is_finite()).then_some([x, y, z])
}
