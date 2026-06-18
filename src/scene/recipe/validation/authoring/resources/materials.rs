use std::collections::BTreeSet;

use serde_json::Value;

use crate::scene::recipe::types::SceneRecipeDiagnosticV1;

use super::super::{validate_known_fields, validate_required_id};
use super::material_fields::{
    validate_alpha_mode, validate_color_ref, validate_optional_finite, validate_optional_ior,
    validate_optional_non_negative, validate_optional_positive, validate_optional_range,
    validate_texture_slot, validate_unit_float,
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
    "clearcoat_factor",
    "clearcoat_roughness_factor",
    "clearcoat_normal_scale",
    "clearcoat_texture",
    "clearcoat_roughness_texture",
    "clearcoat_normal_texture",
    "sheen_color_factor",
    "sheen_roughness_factor",
    "sheen_color_texture",
    "sheen_roughness_texture",
    "anisotropy_strength_factor",
    "anisotropy_rotation_radians",
    "anisotropy_texture",
    "iridescence_factor",
    "iridescence_ior",
    "iridescence_thickness_minimum_nm",
    "iridescence_thickness_maximum_nm",
    "iridescence_texture",
    "iridescence_thickness_texture",
    "dispersion_factor",
    "transmission_factor",
    "ior",
    "thickness_factor",
    "attenuation_distance",
    "attenuation_color",
    "transmission_texture",
    "thickness_texture",
];

const ADVANCED_PBR_SCALAR_FIELDS: &[&str] = &[
    "clearcoat_factor",
    "clearcoat_roughness_factor",
    "clearcoat_normal_scale",
    "sheen_color_factor",
    "sheen_roughness_factor",
    "anisotropy_strength_factor",
    "anisotropy_rotation_radians",
    "iridescence_factor",
    "iridescence_ior",
    "iridescence_thickness_minimum_nm",
    "iridescence_thickness_maximum_nm",
    "dispersion_factor",
    "transmission_factor",
    "ior",
    "thickness_factor",
    "attenuation_distance",
    "attenuation_color",
];

const GPU_SUPPORTED_ADVANCED_PBR_TEXTURE_FIELDS: &[&str] = &[
    "clearcoat_texture",
    "clearcoat_roughness_texture",
    "clearcoat_normal_texture",
    "sheen_color_texture",
    "sheen_roughness_texture",
    "anisotropy_texture",
    "iridescence_texture",
    "iridescence_thickness_texture",
];

const GPU_UNSUPPORTED_VOLUME_TEXTURE_FIELDS: &[&str] =
    &["transmission_texture", "thickness_texture"];

const ADVANCED_PBR_TEXTURE_FIELDS: &[&str] = &[
    "clearcoat_texture",
    "clearcoat_roughness_texture",
    "clearcoat_normal_texture",
    "sheen_color_texture",
    "sheen_roughness_texture",
    "anisotropy_texture",
    "iridescence_texture",
    "iridescence_thickness_texture",
    "transmission_texture",
    "thickness_texture",
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
        for field in GPU_SUPPORTED_ADVANCED_PBR_TEXTURE_FIELDS {
            validate_texture_slot(&format!("{path}.{field}"), object.get(*field), diagnostics);
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
                validate_advanced_pbr_fields(&path, object, colors, diagnostics);
            }
            Some(_) | None => {}
        }
        if kind != Some("pbr_metallic_roughness") {
            reject_advanced_pbr_fields_for_non_pbr(&path, object, diagnostics);
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

fn validate_advanced_pbr_fields(
    path: &str,
    object: &serde_json::Map<String, Value>,
    colors: &BTreeSet<String>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    for field in [
        "clearcoat_factor",
        "clearcoat_roughness_factor",
        "sheen_roughness_factor",
        "anisotropy_strength_factor",
        "iridescence_factor",
        "transmission_factor",
    ] {
        validate_unit_float(&format!("{path}.{field}"), object.get(field), diagnostics);
    }
    for field in [
        "clearcoat_normal_scale",
        "iridescence_thickness_minimum_nm",
        "iridescence_thickness_maximum_nm",
        "dispersion_factor",
        "thickness_factor",
    ] {
        validate_optional_non_negative(&format!("{path}.{field}"), object.get(field), diagnostics);
    }
    validate_optional_finite(
        &format!("{path}.anisotropy_rotation_radians"),
        object.get("anisotropy_rotation_radians"),
        diagnostics,
    );
    validate_optional_positive(
        &format!("{path}.iridescence_ior"),
        object.get("iridescence_ior"),
        diagnostics,
    );
    validate_optional_positive(
        &format!("{path}.attenuation_distance"),
        object.get("attenuation_distance"),
        diagnostics,
    );
    validate_optional_ior(&format!("{path}.ior"), object.get("ior"), diagnostics);
    for field in ["sheen_color_factor", "attenuation_color"] {
        if let Some(value) = object.get(field) {
            validate_color_ref(&format!("{path}.{field}"), Some(value), colors, diagnostics);
        }
    }
    for field in GPU_UNSUPPORTED_VOLUME_TEXTURE_FIELDS {
        if object.contains_key(*field) {
            diagnostics.push(diagnostic(
                "unsupported_feature",
                "error",
                format!("{path}.{field}"),
                format!(
                    "{field} is not exposed by scene_recipe.v1 until the GPU path supports it without exceeding the WebGL2 texture-unit floor"
                ),
                "remove this texture slot; transmission_factor remains supported for recipe-authored glass",
                None,
                false,
            ));
        }
    }
}

fn reject_advanced_pbr_fields_for_non_pbr(
    path: &str,
    object: &serde_json::Map<String, Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    for field in ADVANCED_PBR_SCALAR_FIELDS
        .iter()
        .chain(ADVANCED_PBR_TEXTURE_FIELDS)
    {
        if object.contains_key(*field) {
            diagnostics.push(diagnostic(
                "unsupported_feature",
                "error",
                format!("{path}.{field}"),
                format!("{field} only applies to pbr_metallic_roughness materials"),
                "remove the field or use kind:\"pbr_metallic_roughness\"",
                None,
                false,
            ));
        }
    }
}
