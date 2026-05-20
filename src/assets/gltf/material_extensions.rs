use ::gltf::Document;

use crate::material::{Color, TextureTransform};

#[derive(Debug, Clone, Copy)]
pub(super) struct ClearcoatExtension {
    pub(super) factor: f32,
    pub(super) roughness_factor: f32,
    pub(super) texture: Option<ExtensionTextureInfo>,
    pub(super) roughness_texture: Option<ExtensionTextureInfo>,
    pub(super) normal_texture: Option<ExtensionTextureInfo>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct SheenExtension {
    pub(super) color_factor: Color,
    pub(super) roughness_factor: f32,
    pub(super) color_texture: Option<ExtensionTextureInfo>,
    pub(super) roughness_texture: Option<ExtensionTextureInfo>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ExtensionTextureInfo {
    pub(super) index: usize,
    pub(super) transform: Option<TextureTransform>,
    pub(super) scale: Option<f32>,
}

pub(super) fn clearcoat_extension(
    document: &Document,
    material_index: usize,
) -> Option<ClearcoatExtension> {
    let extension = document
        .as_json()
        .materials
        .get(material_index)?
        .extensions
        .as_ref()?
        .others
        .get("KHR_materials_clearcoat")?;
    Some(ClearcoatExtension {
        factor: read_factor(extension, "clearcoatFactor").unwrap_or(0.0),
        roughness_factor: read_factor(extension, "clearcoatRoughnessFactor").unwrap_or(0.0),
        texture: read_extension_texture_info(extension, "clearcoatTexture"),
        roughness_texture: read_extension_texture_info(extension, "clearcoatRoughnessTexture"),
        normal_texture: read_extension_texture_info(extension, "clearcoatNormalTexture"),
    })
}

pub(super) fn sheen_extension(
    document: &Document,
    material_index: usize,
) -> Option<SheenExtension> {
    let extension = document
        .as_json()
        .materials
        .get(material_index)?
        .extensions
        .as_ref()?
        .others
        .get("KHR_materials_sheen")?;
    let color_factor = read_vec3_factor(extension, "sheenColorFactor").unwrap_or([0.0, 0.0, 0.0]);
    Some(SheenExtension {
        color_factor: Color::from_linear_rgb(color_factor[0], color_factor[1], color_factor[2]),
        roughness_factor: read_factor(extension, "sheenRoughnessFactor").unwrap_or(0.0),
        color_texture: read_extension_texture_info(extension, "sheenColorTexture"),
        roughness_texture: read_extension_texture_info(extension, "sheenRoughnessTexture"),
    })
}

pub(super) fn read_extension_texture_info(
    extension: &serde_json::Value,
    key: &str,
) -> Option<ExtensionTextureInfo> {
    let info = extension.get(key)?;
    let index = usize::try_from(info.get("index")?.as_u64()?).ok()?;
    let transform = extension_texture_transform(
        info.get("extensions")
            .and_then(|extensions| extensions.get("KHR_texture_transform")),
    );
    Some(ExtensionTextureInfo {
        index,
        transform,
        scale: read_factor(info, "scale"),
    })
}

fn read_factor(value: &serde_json::Value, key: &str) -> Option<f32> {
    value
        .get(key)?
        .as_f64()
        .filter(|value| value.is_finite())
        .map(|value| value as f32)
}

pub(super) fn extension_texture_transform(
    value: Option<&serde_json::Value>,
) -> Option<TextureTransform> {
    let value = value?;
    let offset = read_vec2(value, "offset").unwrap_or([0.0, 0.0]);
    let rotation = value
        .get("rotation")
        .and_then(serde_json::Value::as_f64)
        .map(|value| value as f32)
        .unwrap_or(0.0);
    let scale = read_vec2(value, "scale").unwrap_or([1.0, 1.0]);
    let tex_coord = value
        .get("texCoord")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok());
    Some(TextureTransform::new(offset, rotation, scale, tex_coord))
}

fn read_vec2(value: &serde_json::Value, key: &str) -> Option<[f32; 2]> {
    let array = value.get(key)?.as_array()?;
    let x = array.first()?.as_f64()? as f32;
    let y = array.get(1)?.as_f64()? as f32;
    Some([x, y])
}

fn read_vec3_factor(value: &serde_json::Value, key: &str) -> Option<[f32; 3]> {
    let array = value.get(key)?.as_array()?;
    let x = finite_f32(array.first()?.as_f64()?)?;
    let y = finite_f32(array.get(1)?.as_f64()?)?;
    let z = finite_f32(array.get(2)?.as_f64()?)?;
    Some([x, y, z])
}

fn finite_f32(value: f64) -> Option<f32> {
    value.is_finite().then_some(value as f32)
}
