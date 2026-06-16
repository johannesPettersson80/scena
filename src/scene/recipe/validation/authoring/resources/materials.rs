use std::collections::BTreeSet;

use serde_json::Value;

use crate::scene::recipe::types::SceneRecipeDiagnosticV1;

use super::super::{validate_known_fields, validate_required_id};
use super::material_fields::{
    validate_alpha_mode, validate_color_ref, validate_optional_non_negative,
    validate_optional_positive, validate_optional_range, validate_texture_slot,
    validate_unit_float,
};
use crate::scene::recipe::validation::diagnostic;

const MATERIAL_FIELDS: &[&str] = &[
    "id",
    "kind",
    "base_color",
    "metallic",
    "roughness",
    "double_sided",
    "emissive",
    "emissive_strength",
    "alpha_mode",
    "stroke_width_px",
    "edge_angle_threshold_degrees",
    "base_color_texture",
    "normal_texture",
    "metallic_roughness_texture",
    "occlusion_texture",
    "emissive_texture",
];

pub(in crate::scene::recipe::validation::authoring) fn validate_materials(
    value: Option<&Value>,
    colors: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(materials) = value.as_array() else {
        diagnostics.push(diagnostic(
            "invalid_materials",
            "error",
            "$.materials",
            "materials must be an array",
            "emit materials:[{id,kind,base_color}]",
            None,
            false,
        ));
        return;
    };
    for (index, material) in materials.iter().enumerate() {
        let path = format!("$.materials[{index}]");
        let Some(object) = material.as_object() else {
            diagnostics.push(diagnostic(
                "invalid_material",
                "error",
                &path,
                "material entry must be an object",
                "emit material entries as {id, kind, base_color}",
                None,
                false,
            ));
            continue;
        };
        validate_known_fields(&path, object, MATERIAL_FIELDS, diagnostics);
        validate_required_id(&path, object.get("id"), diagnostics);
        let kind = object.get("kind").and_then(Value::as_str);
        match kind {
            Some("unlit" | "pbr_metallic_roughness" | "line" | "wireframe" | "edge") => {}
            Some(kind) => diagnostics.push(diagnostic(
                "unsupported_feature",
                "error",
                format!("{path}.kind"),
                format!("material kind '{kind}' is not implemented in this slice"),
                "use kind:\"unlit\", \"pbr_metallic_roughness\", \"line\", \"wireframe\", or \"edge\"",
                None,
                false,
            )),
            None => diagnostics.push(diagnostic(
                "missing_material_kind",
                "error",
                format!("{path}.kind"),
                "material must include a kind string",
                "use kind:\"unlit\", \"pbr_metallic_roughness\", \"line\", \"wireframe\", or \"edge\"",
                None,
                false,
            )),
        }
        validate_color_ref(
            &format!("{path}.base_color"),
            object.get("base_color"),
            colors,
            diagnostics,
        );
        if let Some(emissive) = object.get("emissive") {
            validate_color_ref(
                &format!("{path}.emissive"),
                Some(emissive),
                colors,
                diagnostics,
            );
        }
        validate_optional_non_negative(
            &format!("{path}.emissive_strength"),
            object.get("emissive_strength"),
            diagnostics,
        );
        validate_alpha_mode(
            &format!("{path}.alpha_mode"),
            object.get("alpha_mode"),
            diagnostics,
        );
        for field in [
            "base_color_texture",
            "normal_texture",
            "metallic_roughness_texture",
            "occlusion_texture",
            "emissive_texture",
        ] {
            validate_texture_slot(&format!("{path}.{field}"), object.get(field), diagnostics);
        }
        match kind {
            Some("unlit" | "line" | "wireframe" | "edge") => {
                for field in ["metallic", "roughness"] {
                    if object.contains_key(field) {
                        diagnostics.push(diagnostic(
                            "unsupported_feature",
                            "error",
                            format!("{path}.{field}"),
                            format!("unlit materials do not use {field}"),
                            "remove the field or use kind:\"pbr_metallic_roughness\"",
                            None,
                            false,
                        ));
                    }
                }
            }
            Some("pbr_metallic_roughness") => {
                validate_unit_float(
                    &format!("{path}.metallic"),
                    object.get("metallic"),
                    diagnostics,
                );
                validate_unit_float(
                    &format!("{path}.roughness"),
                    object.get("roughness"),
                    diagnostics,
                );
            }
            Some(_) | None => {}
        }
        match kind {
            Some("line" | "wireframe" | "edge") => validate_optional_positive(
                &format!("{path}.stroke_width_px"),
                object.get("stroke_width_px"),
                diagnostics,
            ),
            Some(_) | None => {
                if object.contains_key("stroke_width_px") {
                    diagnostics.push(diagnostic(
                        "unsupported_feature",
                        "error",
                        format!("{path}.stroke_width_px"),
                        "stroke_width_px only applies to line, wireframe, and edge materials",
                        "remove the field or use a stroke material kind",
                        None,
                        false,
                    ));
                }
            }
        }
        if kind == Some("edge") {
            validate_optional_range(
                &format!("{path}.edge_angle_threshold_degrees"),
                object.get("edge_angle_threshold_degrees"),
                0.0,
                180.0,
                diagnostics,
            );
        } else if object.contains_key("edge_angle_threshold_degrees") {
            diagnostics.push(diagnostic(
                "unsupported_feature",
                "error",
                format!("{path}.edge_angle_threshold_degrees"),
                "edge_angle_threshold_degrees only applies to edge materials",
                "remove the field or use kind:\"edge\"",
                None,
                false,
            ));
        }
    }
}
