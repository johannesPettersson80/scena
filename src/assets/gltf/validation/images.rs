use super::*;

pub(super) fn validate_material_texture_refs(
    path: &AssetPath,
    value: Option<&Value>,
    texture_count: usize,
) -> Result<(), AssetError> {
    let Some(materials) = value.and_then(Value::as_array) else {
        return Ok(());
    };
    for (material_index, material) in materials.iter().enumerate() {
        let material_name = material.get("name").and_then(Value::as_str);
        validate_texture_slots_recursive(
            path,
            material_index,
            material_name,
            material,
            texture_count,
        )?;
    }
    Ok(())
}

fn validate_texture_slots_recursive(
    path: &AssetPath,
    material_index: usize,
    material_name: Option<&str>,
    value: &Value,
    texture_count: usize,
) -> Result<(), AssetError> {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                if key.ends_with("Texture")
                    && let Some(texture_index) = child.get("index").and_then(Value::as_u64)
                    && usize::try_from(texture_index).map_or(true, |index| index >= texture_count)
                {
                    return Err(AssetError::MissingTexture {
                        path: path.as_str().to_owned(),
                        material_slot: key.clone(),
                        texture_index: usize::try_from(texture_index).unwrap_or(usize::MAX),
                        context: Box::new(crate::diagnostics::MissingTextureDetails {
                            material_index: Some(material_index),
                            material_name: material_name.map(str::to_owned),
                            image_source: None,
                            reason: format!(
                                "texture index {texture_index} is outside the document texture table of length {texture_count}"
                            ),
                        }),
                        help: "export the referenced image or remove the broken material slot",
                    });
                }
                validate_texture_slots_recursive(
                    path,
                    material_index,
                    material_name,
                    child,
                    texture_count,
                )?;
            }
        }
        Value::Array(values) => {
            for child in values {
                validate_texture_slots_recursive(
                    path,
                    material_index,
                    material_name,
                    child,
                    texture_count,
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn validate_textures(
    path: &AssetPath,
    value: Option<&Value>,
    image_count: usize,
    sampler_count: usize,
) -> Result<(), AssetError> {
    for (index, texture) in values(value).enumerate() {
        validate_index(
            path,
            texture.get("source"),
            &format!("$.textures[{index}].source"),
            image_count,
        )?;
        validate_index(
            path,
            texture.get("sampler"),
            &format!("$.textures[{index}].sampler"),
            sampler_count,
        )?;
    }
    Ok(())
}

/// Every image must name where its bytes come from. The upstream document
/// model unwraps `uri` when `bufferView` is absent and `mimeType` when it is
/// present, so both shapes are rejected here before that model is built.
pub(super) fn validate_images(
    path: &AssetPath,
    value: Option<&Value>,
    buffer_view_count: usize,
) -> Result<(), AssetError> {
    for (index, image) in values(value).enumerate() {
        validate_index(
            path,
            image.get("bufferView"),
            &format!("$.images[{index}].bufferView"),
            buffer_view_count,
        )?;
        if image.get("bufferView").is_some() {
            if !image.get("mimeType").is_some_and(Value::is_string) {
                return Err(parse_error(
                    path,
                    format!("$.images[{index}].mimeType is required when bufferView is defined"),
                ));
            }
        } else if !image.get("uri").is_some_and(Value::is_string) {
            return Err(parse_error(
                path,
                format!("$.images[{index}] must define either uri or bufferView"),
            ));
        }
    }
    Ok(())
}
