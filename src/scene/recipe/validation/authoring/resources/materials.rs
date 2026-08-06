use std::collections::BTreeSet;

use serde_json::Value;

use crate::scene::recipe::types::SceneRecipeDiagnosticV1;
use crate::{MaterialDesc, PhotographicSurfaceKind};

use super::super::{validate_known_fields, validate_required_id};
use super::material_fields::{
    validate_alpha_mode, validate_color_ref, validate_optional_finite, validate_optional_ior,
    validate_optional_non_negative, validate_optional_positive, validate_optional_range,
    validate_texture_slot, validate_unit_float,
};
use crate::scene::recipe::validation::diagnostic;

mod details;

use details::{
    reject_advanced_pbr_fields_for_non_pbr, validate_advanced_pbr_fields, validate_material_pack,
    validate_photographic_surface,
};

const MATERIAL_FIELDS: &[&str] = &[
    "id",
    "kind",
    "preset",
    "photographic_surface",
    "material_pack",
    "imperfection",
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

const PHOTOGRAPHIC_SURFACE_FIELDS: &[&str] = &[
    "kind",
    "tile_size_m",
    "feature_scale_m",
    "metallic",
    "roughness",
    "variation",
    "wear",
    "seed",
    "resolution",
];

const MATERIAL_PACK_FIELDS: &[&str] = &["uri", "expected_archive_sha256", "tile_size_m"];

const PHOTOGRAPHIC_SURFACE_CONFLICT_FIELDS: &[&str] = &[
    "kind",
    "preset",
    "metallic",
    "roughness",
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

const MATERIAL_PACK_CONFLICT_FIELDS: &[&str] = &[
    "kind",
    "preset",
    "photographic_surface",
    "metallic",
    "roughness",
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
        let preset = object.get("preset").and_then(Value::as_str);
        let photographic_surface = object.get("photographic_surface");
        let material_pack = object.get("material_pack");
        if let Some(imperfection) = object.get("imperfection") {
            crate::scene::recipe::validation::validate_material_imperfection(
                &format!("{path}.imperfection"),
                imperfection,
                diagnostics,
            );
            if material_pack.is_none() && photographic_surface.is_none() {
                diagnostics.push(diagnostic(
                    "material_imperfection_requires_surface_maps",
                    "error",
                    format!("{path}.imperfection"),
                    "material imperfection requires material_pack or photographic_surface normal and roughness maps",
                    "add material_pack or photographic_surface, or remove imperfection",
                    None,
                    false,
                ));
            }
        }
        let material_source_count = usize::from(object.contains_key("kind"))
            + usize::from(object.contains_key("preset"))
            + usize::from(photographic_surface.is_some())
            + usize::from(material_pack.is_some());
        if material_source_count > 1 {
            diagnostics.push(diagnostic(
                "invalid_material",
                "error",
                if material_pack.is_some() {
                    format!("{path}.material_pack")
                } else if photographic_surface.is_some() {
                    format!("{path}.photographic_surface")
                } else {
                    format!("{path}.preset")
                },
                "material must use exactly one of kind, preset, photographic_surface, or material_pack",
                "remove the other material source fields",
                None,
                false,
            ));
        }
        if let Some(surface) = photographic_surface {
            validate_photographic_surface(&path, surface, diagnostics);
            for field in PHOTOGRAPHIC_SURFACE_CONFLICT_FIELDS {
                if object.contains_key(*field) {
                    diagnostics.push(diagnostic(
                        "conflicting_photographic_surface_field",
                        "error",
                        format!("{path}.{field}"),
                        format!(
                            "{field} cannot override a generated photographic surface in recipe v1"
                        ),
                        "remove the field or author a low-level pbr_metallic_roughness material instead",
                        None,
                        false,
                    ));
                }
            }
        }
        if let Some(pack) = material_pack {
            validate_material_pack(&path, pack, diagnostics);
            for field in MATERIAL_PACK_CONFLICT_FIELDS {
                if object.contains_key(*field) {
                    diagnostics.push(diagnostic(
                        "conflicting_material_pack_field",
                        "error",
                        format!("{path}.{field}"),
                        format!(
                            "{field} cannot override a source-locked material pack in recipe v1"
                        ),
                        "remove the field or author a low-level pbr_metallic_roughness material instead",
                        None,
                        false,
                    ));
                }
            }
        }
        if let Some(value) = object.get("preset")
            && !value.is_string()
        {
            diagnostics.push(diagnostic(
                "invalid_material_preset",
                "error",
                format!("{path}.preset"),
                "material preset must be a string",
                "use a documented MaterialDesc preset name such as chrome, plastic, or brushed_steel",
                None,
                false,
            ));
        }
        if let Some(preset) = preset
            && MaterialDesc::from_preset_name(preset, None).is_none()
        {
            diagnostics.push(diagnostic(
                "invalid_material_preset",
                "error",
                format!("{path}.preset"),
                format!("material preset '{preset}' is not supported"),
                format!("use one of: {}", MaterialDesc::PRESET_NAMES.join(", ")),
                None,
                false,
            ));
        }
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
            None if preset.is_none()
                && photographic_surface.is_none()
                && material_pack.is_none() =>
            {
                diagnostics.push(diagnostic(
                "missing_material_kind",
                "error",
                format!("{path}.kind"),
                "material must include a kind, preset, photographic_surface, or material_pack",
                "use material_pack for an imported PBR surface, photographic_surface for scena-generated detail, or kind:\"pbr_metallic_roughness\"",
                None,
                false,
                ))
            }
            None => {}
        }
        if kind.is_some() || object.contains_key("base_color") || photographic_surface.is_some() {
            validate_color_ref(
                &format!("{path}.base_color"),
                object.get("base_color"),
                colors,
                diagnostics,
            );
        }
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
        if kind != Some("pbr_metallic_roughness") && preset.is_none() {
            reject_advanced_pbr_fields_for_non_pbr(&path, object, diagnostics);
        } else if preset.is_some() {
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
