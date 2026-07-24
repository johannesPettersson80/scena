use super::*;

pub(super) fn finding_for_load_warning(warning: &AssetLoadWarning) -> AssetDoctorFindingV1 {
    match warning {
        AssetLoadWarning::ExternalBufferMissing {
            path,
            index,
            reason,
        } => AssetDoctorFindingV1 {
            severity: AssetDoctorSeverityV1::Warning,
            code: "external_buffer_missing".to_owned(),
            path: Some(path.as_str().to_owned()),
            field: Some(format!("buffers[{index}]")),
            extension: None,
            message: format!("external buffer {index} was missing: {reason}"),
            help: "the asset can only be trusted when all referenced buffer bytes are available"
                .to_owned(),
            suggested_fix:
                "Serve the buffer next to the glTF, correct the URI, or embed the buffer into a GLB."
                    .to_owned(),
            source: "asset_load_report".to_owned(),
        },
        AssetLoadWarning::ExternalImageMissing { path, reason } => AssetDoctorFindingV1 {
            severity: AssetDoctorSeverityV1::Warning,
            code: "external_image_missing".to_owned(),
            path: Some(path.as_str().to_owned()),
            field: Some("images".to_owned()),
            extension: None,
            message: format!("external image was missing: {reason}"),
            help: "the material will not match the authored asset until referenced image bytes are available".to_owned(),
            suggested_fix: "Serve the image next to the glTF, correct the URI, or embed the image before approval.".to_owned(),
            source: "asset_load_report".to_owned(),
        },
        AssetLoadWarning::ComputedFlatNormals {
            path,
            mesh_index,
            primitive_index,
            triangle_count,
        } => AssetDoctorFindingV1 {
            severity: AssetDoctorSeverityV1::Info,
            code: "flat_normals_computed".to_owned(),
            path: Some(path.as_str().to_owned()),
            field: Some(format!(
                "meshes[{mesh_index}].primitives[{primitive_index}].attributes.NORMAL"
            )),
            extension: None,
            message: format!(
                "computed flat normals for {triangle_count} triangles because NORMAL was omitted"
            ),
            help: "glTF defines flat shading when a triangle primitive omits NORMAL".to_owned(),
            suggested_fix: "Author NORMAL when a specific smooth or split-normal result is required."
                .to_owned(),
            source: "asset_load_report".to_owned(),
        },
        AssetLoadWarning::SkinInfluencesTruncated {
            path,
            mesh_index,
            primitive_index,
            affected_vertices,
            source_influences,
            retained_influences,
        } => AssetDoctorFindingV1 {
            severity: AssetDoctorSeverityV1::Warning,
            code: "skin_influences_truncated".to_owned(),
            path: Some(path.as_str().to_owned()),
            field: Some(format!(
                "meshes[{mesh_index}].primitives[{primitive_index}].attributes.WEIGHTS_1"
            )),
            extension: None,
            message: format!(
                "selected the strongest {retained_influences} of {source_influences} skin influences for {affected_vertices} vertices"
            ),
            help: "scena prepares four skin influences per vertex and reports every source vertex that exceeded that limit".to_owned(),
            suggested_fix: "Limit authored skinning to four nonzero influences per vertex when exact cross-tool parity is required."
                .to_owned(),
            source: "asset_load_report".to_owned(),
        },
        AssetLoadWarning::InvalidMaterialVariantMapping {
            path,
            mesh_index,
            primitive_index,
            mapping_index,
            material_index,
            variant_indices,
            material_count,
        } => AssetDoctorFindingV1 {
            severity: AssetDoctorSeverityV1::Warning,
            code: "invalid_material_variant_mapping".to_owned(),
            path: Some(path.as_str().to_owned()),
            field: Some(format!(
                "meshes[{mesh_index}].primitives[{primitive_index}].extensions.KHR_materials_variants.mappings[{mapping_index}].material"
            )),
            extension: Some("KHR_materials_variants".to_owned()),
            message: format!(
                "material variant mapping for variants {variant_indices:?} references material {material_index:?}, but the asset resolved {material_count} materials"
            ),
            help: "the primitive remains loadable with its default material, but this variant mapping cannot be applied".to_owned(),
            suggested_fix: "Point the mapping at an in-range glTF material index or remove the invalid mapping.".to_owned(),
            source: "asset_load_report".to_owned(),
        },
        AssetLoadWarning::TextureDownscaled {
            path,
            original_width,
            original_height,
            decoded_width,
            decoded_height,
            maximum_dimension,
        } => AssetDoctorFindingV1 {
            severity: AssetDoctorSeverityV1::Warning,
            code: "texture_downscaled".to_owned(),
            path: Some(path.as_str().to_owned()),
            field: Some("images".to_owned()),
            extension: None,
            message: format!(
                "browser decode resized texture from {original_width}x{original_height} to {decoded_width}x{decoded_height}"
            ),
            help: format!(
                "the browser-safe texture limit is {maximum_dimension}px and the resize changes source pixels"
            ),
            suggested_fix:
                "Preprocess the texture to the intended browser dimensions or use an explicit lower-resolution asset variant."
                    .to_owned(),
            source: "asset_load_report".to_owned(),
        },
    }
}

pub(super) fn finding_for_material_fallback(
    fallback: &AssetMaterialFallback,
) -> AssetDoctorFindingV1 {
    AssetDoctorFindingV1 {
        severity: AssetDoctorSeverityV1::Warning,
        code: "material_fallback_used".to_owned(),
        path: Some(fallback.source_path.as_str().to_owned()),
        field: Some(fallback.material_slot.clone()),
        extension: Some("KHR_texture_basisu".to_owned()),
        message: format!(
            "{} used fallback texture {}",
            fallback.source_path.as_str(),
            fallback.fallback_path.as_str()
        ),
        help: fallback.reason.clone(),
        suggested_fix:
            "Enable the required decoder feature or keep the fallback texture packaged with the asset."
                .to_owned(),
        source: "asset_load_report".to_owned(),
    }
}

pub(super) fn finding_for_asset_error(
    error: &AssetError,
    fallback_path: &str,
) -> AssetDoctorFindingV1 {
    let (code, path, field, extension, suggested_fix) = match error {
        AssetError::NotFound { path } => (
            "asset_not_found",
            path.clone(),
            Some("source".to_owned()),
            None,
            "Fix the path, configure the asset fetcher, or serve the asset before rendering.",
        ),
        AssetError::Io { path, .. } => (
            "asset_io",
            path.clone(),
            Some("source".to_owned()),
            None,
            "Fix filesystem or network access, then retry the load.",
        ),
        AssetError::PolicyViolation { path, .. } => (
            "asset_policy_violation",
            path.clone(),
            Some("source".to_owned()),
            None,
            "Use a smaller asset or raise the operator-owned load policy.",
        ),
        AssetError::Parse { path, .. } => (
            "asset_parse",
            path.clone(),
            Some("source".to_owned()),
            None,
            "Validate the source file with the glTF validator and re-export valid glTF/GLB.",
        ),
        AssetError::InvalidTextureIdentity { identity, .. } => (
            "invalid_texture_identity",
            format!("memory://{identity}"),
            Some("identity".to_owned()),
            None,
            "Use a stable, non-empty application-owned texture identity.",
        ),
        AssetError::InvalidTextureData { identity, .. } => (
            "invalid_texture_data",
            format!("memory://{identity}"),
            Some("pixels".to_owned()),
            None,
            "Provide the exact checked pixel count and finite values required by the constructor.",
        ),
        AssetError::TextureSizeLimit { path, .. } => (
            "texture_size_limit",
            path.clone(),
            Some("dimensions".to_owned()),
            None,
            "Resize the texture before loading it or choose an explicit application policy for a capable backend.",
        ),
        AssetError::TextureIdentityCollision { identity } => (
            "texture_identity_collision",
            format!("memory://{identity}"),
            Some("identity".to_owned()),
            None,
            "Mint a new identity when generated pixels or texture options change.",
        ),
        AssetError::TextureColorSpaceMismatch { identity, slot, .. } => (
            "texture_color_space_mismatch",
            format!("memory://{identity}"),
            Some(slot.clone()),
            None,
            "Use the slot-typed texture constructor or loader.",
        ),
        AssetError::MorphWeightWidthMismatch {
            path,
            clip_index,
            channel_index,
            node_index,
            primitive_index,
            ..
        } => (
            "morph_weight_width_mismatch",
            path.clone(),
            Some(format!(
                "animations[{clip_index}].channels[{channel_index}].target(node={node_index},primitive={primitive_index})"
            )),
            None,
            "Export exactly one animation weight per morph target for every primitive bound to the target node.",
        ),
        AssetError::UnsupportedRequiredExtension { path, extension } => (
            "unsupported_required_extension",
            path.clone(),
            Some("extensionsRequired".to_owned()),
            Some(extension.clone()),
            "Remove the required extension, make it optional with a visual fallback, export a fallback material/path, or enable the matching decoder feature when one exists.",
        ),
        AssetError::UnsupportedOptionalExtensionUsed {
            path, extension, ..
        } => (
            "unsupported_optional_extension_used",
            path.clone(),
            Some("extensionsUsed".to_owned()),
            Some(extension.clone()),
            "Inspect extension_diagnostics, then export a fallback or keep the extension optional until the target renderer lane supports it.",
        ),
        AssetError::MissingTexture {
            path,
            material_slot,
            ..
        } => (
            "missing_texture",
            path.clone(),
            Some(material_slot.clone()),
            None,
            "Fix the material texture index or export the referenced image bytes.",
        ),
        AssetError::UnsupportedTextureFormat { path, .. } => (
            "unsupported_texture_format",
            path.clone(),
            Some("images".to_owned()),
            None,
            "Use PNG, JPEG, WebP, or a decoder-backed compressed texture feature supported by the build.",
        ),
        AssetError::Ktx2ColorSpaceMismatch {
            path,
            material_slot,
            ..
        } => (
            "ktx2_color_space_mismatch",
            path.clone(),
            Some(material_slot.clone()),
            Some("KHR_texture_basisu".to_owned()),
            "Re-encode the KTX2 DFD color primaries and transfer function for the material slot's color or non-color role.",
        ),
        AssetError::Cancelled { path, .. } => (
            "asset_load_cancelled",
            path.clone(),
            Some("source".to_owned()),
            None,
            "Retry the load with a fresh AssetLoadControl when the host still needs this asset.",
        ),
        AssetError::UnsupportedEnvironmentFormat { path, .. } => (
            "unsupported_environment_format",
            path.clone(),
            Some("environment".to_owned()),
            None,
            "Use an equirectangular HDR environment or a bundled supported environment preset.",
        ),
        AssetError::ReloadRequiresRetain { path, .. } => (
            "reload_requires_retain",
            path.clone(),
            Some("retain_policy".to_owned()),
            None,
            "Set RetainPolicy::Always before loading assets that need hot reload.",
        ),
        AssetError::GeometryHandleNotFound { .. }
        | AssetError::MaterialHandleNotFound { .. }
        | AssetError::TextureHandleNotFound { .. }
        | AssetError::EnvironmentHandleNotFound { .. } => (
            "asset_handle_not_found",
            fallback_path.to_owned(),
            Some("handle".to_owned()),
            None,
            "Verify the handle came from the same Assets store and has not been released.",
        ),
    };
    AssetDoctorFindingV1 {
        severity: AssetDoctorSeverityV1::Error,
        code: code.to_owned(),
        path: Some(path),
        field,
        extension,
        message: error.to_string(),
        help: error.help().to_owned(),
        suggested_fix: suggested_fix.to_owned(),
        source: "scena_asset_doctor".to_owned(),
    }
}
