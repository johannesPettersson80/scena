use serde_json::Value;

use crate::scene::recipe::field_model::PRIMITIVE_KINDS;
use crate::scene::recipe::types::SceneRecipeDiagnosticV1;

use super::super::{validate_known_fields, validate_required_id};
use super::common::{
    validate_indices, validate_optional_min_u32, validate_optional_positive_u32,
    validate_optional_string_array, validate_optional_vec2_array, validate_optional_vec3_array,
    validate_points, validate_positive_number, validate_positive_number_list, validate_vec3,
};
use crate::scene::recipe::validation::diagnostic;

const GEOMETRY_FIELDS: &[&str] = &["id", "primitive", "mesh"];
const PRIMITIVE_FIELDS: &[&str] = &[
    "kind",
    "size",
    "radius",
    "major_radius",
    "minor_radius",
    "height",
    "bevel",
    "fillet",
    "segments",
    "rings",
    "divisions",
    "length",
    "start",
    "end",
    "points",
];
const MESH_FIELDS: &[&str] = &[
    "topology",
    "positions",
    "normals",
    "indices",
    "colors",
    "uvs",
];

pub(in crate::scene::recipe::validation::authoring) fn validate_geometries(
    value: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(geometries) = value.as_array() else {
        diagnostics.push(diagnostic(
            "invalid_geometries",
            "error",
            "$.geometries",
            "geometries must be an array",
            "emit geometries:[{id,primitive}]",
            None,
            false,
        ));
        return;
    };
    for (index, geometry) in geometries.iter().enumerate() {
        let path = format!("$.geometries[{index}]");
        let Some(object) = geometry.as_object() else {
            diagnostics.push(diagnostic(
                "invalid_geometry",
                "error",
                &path,
                "geometry entry must be an object",
                "emit geometry entries as {id, primitive}",
                None,
                false,
            ));
            continue;
        };
        validate_known_fields(&path, object, GEOMETRY_FIELDS, diagnostics);
        validate_required_id(&path, object.get("id"), diagnostics);
        match (object.get("primitive"), object.get("mesh")) {
            (Some(primitive), None) => {
                validate_primitive(&format!("{path}.primitive"), primitive, diagnostics)
            }
            (None, Some(mesh)) => validate_mesh(&format!("{path}.mesh"), mesh, diagnostics),
            (Some(_), Some(_)) => diagnostics.push(diagnostic(
                "invalid_geometry",
                "error",
                &path,
                "geometry must define exactly one of primitive or mesh",
                "remove one geometry source",
                None,
                false,
            )),
            (None, None) => diagnostics.push(diagnostic(
                "missing_geometry_source",
                "error",
                &path,
                "geometry must include primitive or mesh",
                "emit either primitive:{...} or mesh:{...}",
                None,
                false,
            )),
        }
    }
}

fn validate_primitive(path: &str, value: &Value, diagnostics: &mut Vec<SceneRecipeDiagnosticV1>) {
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_primitive",
            "error",
            path,
            "primitive must be an object",
            "emit primitive:{kind:\"box\",size:[x,y,z]}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields(path, object, PRIMITIVE_FIELDS, diagnostics);
    validate_bevel_aliases(path, object, diagnostics);
    match object.get("kind").and_then(Value::as_str) {
        Some("box") => {
            validate_positive_number_list(
                &format!("{path}.size"),
                object.get("size"),
                3,
                "box primitive size must contain three finite positive dimensions",
                "use size:[width,height,depth] in meters",
                diagnostics,
            );
            validate_optional_bevel_for_supported_kind(path, object, diagnostics);
        }
        Some("plane") => {
            reject_bevel_for_kind(path, object, "plane", diagnostics);
            validate_positive_number_list(
                &format!("{path}.size"),
                object.get("size"),
                2,
                "plane primitive size must contain finite positive width and depth",
                "use size:[width,depth] in meters",
                diagnostics,
            );
        }
        Some("sphere") => {
            reject_bevel_for_kind(path, object, "sphere", diagnostics);
            validate_positive_number(&format!("{path}.radius"), object.get("radius"), diagnostics);
            validate_optional_positive_u32(
                &format!("{path}.segments"),
                object.get("segments"),
                diagnostics,
            );
            validate_optional_positive_u32(
                &format!("{path}.rings"),
                object.get("rings"),
                diagnostics,
            );
        }
        Some("cylinder") => {
            validate_positive_number(&format!("{path}.radius"), object.get("radius"), diagnostics);
            validate_positive_number(&format!("{path}.height"), object.get("height"), diagnostics);
            validate_optional_bevel_for_supported_kind(path, object, diagnostics);
            validate_optional_positive_u32(
                &format!("{path}.segments"),
                object.get("segments"),
                diagnostics,
            );
        }
        Some("cone") => {
            reject_bevel_for_kind(path, object, "cone", diagnostics);
            validate_positive_number(&format!("{path}.radius"), object.get("radius"), diagnostics);
            validate_positive_number(&format!("{path}.height"), object.get("height"), diagnostics);
            validate_optional_min_u32(
                &format!("{path}.segments"),
                object.get("segments"),
                3,
                diagnostics,
            );
        }
        Some("disc") => {
            reject_bevel_for_kind(path, object, "disc", diagnostics);
            validate_positive_number(&format!("{path}.radius"), object.get("radius"), diagnostics);
            validate_optional_min_u32(
                &format!("{path}.segments"),
                object.get("segments"),
                3,
                diagnostics,
            );
        }
        Some("torus") => {
            reject_bevel_for_kind(path, object, "torus", diagnostics);
            validate_positive_number(
                &format!("{path}.major_radius"),
                object.get("major_radius"),
                diagnostics,
            );
            validate_positive_number(
                &format!("{path}.minor_radius"),
                object.get("minor_radius"),
                diagnostics,
            );
            if let (Some(major_radius), Some(minor_radius)) = (
                object.get("major_radius").and_then(Value::as_f64),
                object.get("minor_radius").and_then(Value::as_f64),
            ) && major_radius.is_finite()
                && minor_radius.is_finite()
                && major_radius > 0.0
                && minor_radius > 0.0
                && minor_radius >= major_radius
            {
                diagnostics.push(diagnostic(
                    "invalid_primitive",
                    "error",
                    format!("{path}.minor_radius"),
                    "torus minor_radius must be smaller than major_radius",
                    "use a tube radius below the torus major radius",
                    None,
                    false,
                ));
            }
            validate_optional_min_u32(
                &format!("{path}.segments"),
                object.get("segments"),
                3,
                diagnostics,
            );
            validate_optional_min_u32(
                &format!("{path}.rings"),
                object.get("rings"),
                3,
                diagnostics,
            );
        }
        Some("wedge") => {
            reject_bevel_for_kind(path, object, "wedge", diagnostics);
            validate_positive_number_list(
                &format!("{path}.size"),
                object.get("size"),
                3,
                "wedge primitive size must contain three finite positive dimensions",
                "use size:[width,height,depth] in meters",
                diagnostics,
            );
        }
        Some("line" | "arrow") => {
            reject_bevel_for_kind(path, object, "line or arrow", diagnostics);
            validate_vec3(&format!("{path}.start"), object.get("start"), diagnostics);
            validate_vec3(&format!("{path}.end"), object.get("end"), diagnostics);
        }
        Some("polyline") => {
            reject_bevel_for_kind(path, object, "polyline", diagnostics);
            validate_points(
                &format!("{path}.points"),
                object.get("points"),
                2,
                diagnostics,
            );
        }
        Some("grid") => {
            reject_bevel_for_kind(path, object, "grid", diagnostics);
            if object.get("length").is_some() {
                validate_positive_number(
                    &format!("{path}.length"),
                    object.get("length"),
                    diagnostics,
                );
            } else {
                validate_positive_number_list(
                    &format!("{path}.size"),
                    object.get("size"),
                    1,
                    "grid primitive size must contain one finite positive dimension",
                    "use length or size:[size] in meters",
                    diagnostics,
                );
            }
            validate_optional_positive_u32(
                &format!("{path}.divisions"),
                object.get("divisions"),
                diagnostics,
            );
        }
        Some("axes") => {
            reject_bevel_for_kind(path, object, "axes", diagnostics);
            validate_positive_number(&format!("{path}.length"), object.get("length"), diagnostics)
        }
        Some(kind) => diagnostics.push(diagnostic(
            "unsupported_feature",
            "error",
            format!("{path}.kind"),
            format!("primitive kind '{kind}' is not supported"),
            format!("use one of: {}", PRIMITIVE_KINDS.join(", ")),
            None,
            false,
        )),
        None => diagnostics.push(diagnostic(
            "missing_primitive_kind",
            "error",
            format!("{path}.kind"),
            "primitive must include a kind string",
            "use kind:\"box\"",
            None,
            false,
        )),
    }
}

fn validate_mesh(path: &str, value: &Value, diagnostics: &mut Vec<SceneRecipeDiagnosticV1>) {
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_mesh",
            "error",
            path,
            "mesh must be an object",
            "emit mesh:{topology,positions,indices}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields(path, object, MESH_FIELDS, diagnostics);
    match object.get("topology").and_then(Value::as_str) {
        Some("triangles" | "lines") => {}
        Some(topology) => diagnostics.push(diagnostic(
            "unsupported_feature",
            "error",
            format!("{path}.topology"),
            format!("mesh topology '{topology}' is not supported"),
            "use topology:\"triangles\" or topology:\"lines\"",
            None,
            false,
        )),
        None => diagnostics.push(diagnostic(
            "missing_topology",
            "error",
            format!("{path}.topology"),
            "custom mesh must include topology",
            "use topology:\"triangles\" or topology:\"lines\"",
            None,
            false,
        )),
    }
    let vertex_count = validate_points(
        &format!("{path}.positions"),
        object.get("positions"),
        1,
        diagnostics,
    );
    validate_optional_vec3_array(
        &format!("{path}.normals"),
        object.get("normals"),
        vertex_count,
        diagnostics,
    );
    validate_optional_string_array(
        &format!("{path}.colors"),
        object.get("colors"),
        vertex_count,
        diagnostics,
    );
    validate_optional_vec2_array(
        &format!("{path}.uvs"),
        object.get("uvs"),
        vertex_count,
        diagnostics,
    );
    validate_indices(
        &format!("{path}.indices"),
        object.get("indices"),
        vertex_count,
        object.get("topology").and_then(Value::as_str),
        diagnostics,
    );
}

fn validate_bevel_aliases(
    path: &str,
    object: &serde_json::Map<String, Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if object.contains_key("bevel") && object.contains_key("fillet") {
        diagnostics.push(diagnostic(
            "invalid_primitive",
            "error",
            path,
            "primitive may specify bevel or fillet, not both",
            "remove one bevel alias",
            None,
            false,
        ));
    }
}

fn validate_optional_bevel_for_supported_kind(
    path: &str,
    object: &serde_json::Map<String, Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    for field in ["bevel", "fillet"] {
        if object.contains_key(field) {
            validate_positive_number(&format!("{path}.{field}"), object.get(field), diagnostics);
        }
    }
}

fn reject_bevel_for_kind(
    path: &str,
    object: &serde_json::Map<String, Value>,
    kind: &str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if object.contains_key("bevel") || object.contains_key("fillet") {
        diagnostics.push(diagnostic(
            "unsupported_feature",
            "error",
            path,
            format!("primitive kind '{kind}' does not support bevel or fillet"),
            "use bevel on box or cylinder primitives",
            None,
            false,
        ));
    }
}
