use super::*;

pub(super) fn validate_photographic_surface(
    material_path: &str,
    value: &Value,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let path = format!("{material_path}.photographic_surface");
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_photographic_surface",
            "error",
            path,
            "photographic_surface must be an object",
            "emit photographic_surface:{kind:\"brushed_metal\",variation:0.6}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields(&path, object, PHOTOGRAPHIC_SURFACE_FIELDS, diagnostics);
    match object.get("kind").and_then(Value::as_str) {
        Some(kind) if PhotographicSurfaceKind::from_name(kind).is_some() => {}
        Some(kind) => diagnostics.push(diagnostic(
            "invalid_photographic_surface_kind",
            "error",
            format!("{path}.kind"),
            format!("photographic surface kind '{kind}' is not supported"),
            format!("use one of: {}", PhotographicSurfaceKind::NAMES.join(", ")),
            None,
            false,
        )),
        None => diagnostics.push(diagnostic(
            "invalid_photographic_surface_kind",
            "error",
            format!("{path}.kind"),
            "photographic surface kind must be a supported string",
            format!("use one of: {}", PhotographicSurfaceKind::NAMES.join(", ")),
            None,
            false,
        )),
    }
    validate_optional_positive(
        &format!("{path}.tile_size_m"),
        object.get("tile_size_m"),
        diagnostics,
    );
    validate_optional_positive(
        &format!("{path}.feature_scale_m"),
        object.get("feature_scale_m"),
        diagnostics,
    );
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
    validate_unit_float(
        &format!("{path}.variation"),
        object.get("variation"),
        diagnostics,
    );
    validate_unit_float(&format!("{path}.wear"), object.get("wear"), diagnostics);
    if let Some(seed) = object.get("seed")
        && seed.as_u64().is_none()
    {
        diagnostics.push(diagnostic(
            "invalid_photographic_surface_seed",
            "error",
            format!("{path}.seed"),
            "photographic surface seed must be an unsigned integer",
            "use an integer between 0 and 18446744073709551615",
            None,
            false,
        ));
    }
    if let Some(resolution) = object.get("resolution") {
        match resolution.as_u64() {
            Some(value) if (16..=1_024).contains(&value) && value.is_power_of_two() => {}
            _ => diagnostics.push(diagnostic(
                "invalid_photographic_surface_resolution",
                "error",
                format!("{path}.resolution"),
                "photographic surface resolution must be a power of two from 16 through 1024",
                "use 256 for previews or 512/1024 for close product stills",
                None,
                false,
            )),
        }
    }
}

pub(super) fn validate_material_pack(
    material_path: &str,
    value: &Value,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let path = format!("{material_path}.material_pack");
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_material_pack",
            "error",
            path,
            "material_pack must be an object",
            "emit material_pack:{uri:\"materials/steel/scena-material-pack.json\"}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields(&path, object, MATERIAL_PACK_FIELDS, diagnostics);
    match object.get("uri").and_then(Value::as_str) {
        Some(uri) if !uri.trim().is_empty() => {}
        _ => diagnostics.push(diagnostic(
            "invalid_material_pack_uri",
            "error",
            format!("{path}.uri"),
            "material pack uri must be a non-empty string",
            "reference a compiled scena-material-pack.json file",
            None,
            false,
        )),
    }
    if let Some(hash) = object.get("expected_archive_sha256") {
        match hash.as_str() {
            Some(hash) if hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()) => {
            }
            _ => diagnostics.push(diagnostic(
                "invalid_material_pack_source_sha256",
                "error",
                format!("{path}.expected_archive_sha256"),
                "expected material archive SHA-256 must contain exactly 64 hexadecimal digits",
                "copy source.archive_sha256 from the compiled material pack manifest",
                None,
                false,
            )),
        }
    }
    validate_optional_positive(
        &format!("{path}.tile_size_m"),
        object.get("tile_size_m"),
        diagnostics,
    );
}

pub(super) fn validate_advanced_pbr_fields(
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

pub(super) fn reject_advanced_pbr_fields_for_non_pbr(
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
