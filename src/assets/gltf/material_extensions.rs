use ::gltf::Document;

use crate::diagnostics::AssetError;
use crate::material::{Color, TextureTransform};

use super::super::AssetPath;

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
pub(super) struct AnisotropyExtension {
    pub(super) strength: f32,
    pub(super) rotation: f32,
    pub(super) texture: Option<ExtensionTextureInfo>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct IridescenceExtension {
    pub(super) factor: f32,
    pub(super) ior: f32,
    pub(super) thickness_minimum: f32,
    pub(super) thickness_maximum: f32,
    pub(super) texture: Option<ExtensionTextureInfo>,
    pub(super) thickness_texture: Option<ExtensionTextureInfo>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct DispersionExtension {
    pub(super) factor: f32,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct TransmissionExtension {
    pub(super) factor: f32,
    pub(super) texture: Option<ExtensionTextureInfo>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct IorExtension {
    pub(super) ior: f32,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct VolumeExtension {
    pub(super) thickness_factor: f32,
    pub(super) thickness_texture: Option<ExtensionTextureInfo>,
    pub(super) attenuation_distance: f32,
    pub(super) attenuation_color: Color,
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

pub(super) fn anisotropy_extension(
    document: &Document,
    material_index: usize,
) -> Option<AnisotropyExtension> {
    let extension = document
        .as_json()
        .materials
        .get(material_index)?
        .extensions
        .as_ref()?
        .others
        .get("KHR_materials_anisotropy")?;
    Some(AnisotropyExtension {
        strength: read_factor(extension, "anisotropyStrength").unwrap_or(0.0),
        rotation: read_factor(extension, "anisotropyRotation").unwrap_or(0.0),
        texture: read_extension_texture_info(extension, "anisotropyTexture"),
    })
}

pub(super) fn iridescence_extension(
    document: &Document,
    material_index: usize,
) -> Option<IridescenceExtension> {
    let extension = document
        .as_json()
        .materials
        .get(material_index)?
        .extensions
        .as_ref()?
        .others
        .get("KHR_materials_iridescence")?;
    Some(IridescenceExtension {
        factor: read_factor(extension, "iridescenceFactor").unwrap_or(0.0),
        ior: read_factor(extension, "iridescenceIor").unwrap_or(1.3),
        thickness_minimum: read_factor(extension, "iridescenceThicknessMinimum").unwrap_or(100.0),
        thickness_maximum: read_factor(extension, "iridescenceThicknessMaximum").unwrap_or(400.0),
        texture: read_extension_texture_info(extension, "iridescenceTexture"),
        thickness_texture: read_extension_texture_info(extension, "iridescenceThicknessTexture"),
    })
}

pub(super) fn dispersion_extension(
    document: &Document,
    material_index: usize,
) -> Option<DispersionExtension> {
    let extension = document
        .as_json()
        .materials
        .get(material_index)?
        .extensions
        .as_ref()?
        .others
        .get("KHR_materials_dispersion")?;
    Some(DispersionExtension {
        factor: read_factor(extension, "dispersion").unwrap_or(0.0),
    })
}

pub(super) fn transmission_extension(
    document: &Document,
    material_index: usize,
) -> Option<TransmissionExtension> {
    let extension = document
        .as_json()
        .materials
        .get(material_index)?
        .extensions
        .as_ref()?
        .others
        .get("KHR_materials_transmission")?;
    Some(TransmissionExtension {
        factor: read_factor(extension, "transmissionFactor").unwrap_or(0.0),
        texture: read_extension_texture_info(extension, "transmissionTexture"),
    })
}

pub(super) fn ior_extension(document: &Document, material_index: usize) -> Option<IorExtension> {
    let extension = document
        .as_json()
        .materials
        .get(material_index)?
        .extensions
        .as_ref()?
        .others
        .get("KHR_materials_ior")?;
    Some(IorExtension {
        ior: read_factor(extension, "ior").unwrap_or(1.5),
    })
}

pub(super) fn volume_extension(
    document: &Document,
    material_index: usize,
) -> Option<VolumeExtension> {
    let extension = document
        .as_json()
        .materials
        .get(material_index)?
        .extensions
        .as_ref()?
        .others
        .get("KHR_materials_volume")?;
    let attenuation_color =
        read_vec3_factor(extension, "attenuationColor").unwrap_or([1.0, 1.0, 1.0]);
    Some(VolumeExtension {
        thickness_factor: read_factor(extension, "thicknessFactor").unwrap_or(0.0),
        thickness_texture: read_extension_texture_info(extension, "thicknessTexture"),
        attenuation_distance: read_factor(extension, "attenuationDistance")
            .unwrap_or(f32::INFINITY),
        attenuation_color: Color::from_linear_rgb(
            attenuation_color[0],
            attenuation_color[1],
            attenuation_color[2],
        ),
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

pub(super) fn validate_material_texture_indices(
    path: &AssetPath,
    document: &Document,
    texture_count: usize,
) -> Result<(), AssetError> {
    let raw = document.as_json();
    for (material_index, material) in raw.materials.iter().enumerate() {
        validate_texture_transform_tex_coords(path, material_index, material)?;
        let pbr = &material.pbr_metallic_roughness;
        validate_texture_info(
            path,
            "baseColorTexture",
            pbr.base_color_texture
                .as_ref()
                .map(|info| info.index.value()),
            texture_count,
        )?;
        validate_texture_info(
            path,
            "metallicRoughnessTexture",
            pbr.metallic_roughness_texture
                .as_ref()
                .map(|info| info.index.value()),
            texture_count,
        )?;
        for (slot, index) in [
            (
                "normalTexture",
                material
                    .normal_texture
                    .as_ref()
                    .map(|info| info.index.value()),
            ),
            (
                "occlusionTexture",
                material
                    .occlusion_texture
                    .as_ref()
                    .map(|info| info.index.value()),
            ),
            (
                "emissiveTexture",
                material
                    .emissive_texture
                    .as_ref()
                    .map(|info| info.index.value()),
            ),
        ] {
            validate_texture_info(path, slot, index, texture_count)?;
        }
        validate_extension_texture_slots(path, material_index, material, texture_count)?;
    }
    Ok(())
}

fn validate_texture_transform_tex_coords(
    path: &AssetPath,
    material_index: usize,
    material: &::gltf::json::material::Material,
) -> Result<(), AssetError> {
    let value = serde_json::to_value(material).map_err(|error| AssetError::Parse {
        path: path.as_str().to_string(),
        reason: format!(
            "material {material_index} could not be inspected for texture transforms: {error}"
        ),
    })?;
    validate_texture_transform_tex_coords_value(path, material_index, "$", &value)
}

fn validate_texture_transform_tex_coords_value(
    path: &AssetPath,
    material_index: usize,
    json_path: &str,
    value: &serde_json::Value,
) -> Result<(), AssetError> {
    match value {
        serde_json::Value::Object(object) => {
            if let Some(transform) = object.get("KHR_texture_transform")
                && let Some(tex_coord) = transform
                    .get("texCoord")
                    .and_then(serde_json::Value::as_u64)
                && tex_coord != 0
            {
                return Err(AssetError::Parse {
                    path: path.as_str().to_string(),
                    reason: format!(
                        "material {material_index} uses KHR_texture_transform texCoord {tex_coord} at {json_path}; scena supports only TEXCOORD_0"
                    ),
                });
            }
            for (key, child) in object {
                let child_path = format!("{json_path}.{key}");
                validate_texture_transform_tex_coords_value(
                    path,
                    material_index,
                    &child_path,
                    child,
                )?;
            }
        }
        serde_json::Value::Array(array) => {
            for (index, child) in array.iter().enumerate() {
                let child_path = format!("{json_path}[{index}]");
                validate_texture_transform_tex_coords_value(
                    path,
                    material_index,
                    &child_path,
                    child,
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_extension_texture_slots(
    path: &AssetPath,
    _material_index: usize,
    material: &::gltf::json::material::Material,
    texture_count: usize,
) -> Result<(), AssetError> {
    let Some(extensions) = material.extensions.as_ref() else {
        return Ok(());
    };
    for (extension, slots) in [
        (
            "KHR_materials_clearcoat",
            &[
                ("clearcoatTexture", "clearcoatTexture"),
                ("clearcoatRoughnessTexture", "clearcoatRoughnessTexture"),
                ("clearcoatNormalTexture", "clearcoatNormalTexture"),
            ][..],
        ),
        (
            "KHR_materials_sheen",
            &[
                ("sheenColorTexture", "sheenColorTexture"),
                ("sheenRoughnessTexture", "sheenRoughnessTexture"),
            ][..],
        ),
        (
            "KHR_materials_anisotropy",
            &[("anisotropyTexture", "anisotropyTexture")][..],
        ),
        (
            "KHR_materials_iridescence",
            &[
                ("iridescenceTexture", "iridescenceTexture"),
                ("iridescenceThicknessTexture", "iridescenceThicknessTexture"),
            ][..],
        ),
        (
            "KHR_materials_transmission",
            &[("transmissionTexture", "transmissionTexture")][..],
        ),
        (
            "KHR_materials_volume",
            &[("thicknessTexture", "thicknessTexture")][..],
        ),
    ] {
        let Some(value) = extensions.others.get(extension) else {
            continue;
        };
        for (slot, key) in slots {
            validate_texture_info(
                path,
                slot,
                read_extension_texture_info(value, key).map(|info| info.index),
                texture_count,
            )?;
        }
    }
    Ok(())
}

fn validate_texture_info(
    path: &AssetPath,
    material_slot: &'static str,
    index: Option<usize>,
    texture_count: usize,
) -> Result<(), AssetError> {
    if let Some(index) = index
        && index >= texture_count
    {
        return Err(AssetError::MissingTexture {
            path: path.as_str().to_string(),
            material_slot: material_slot.to_string(),
            texture_index: index,
            help: "export the referenced image or remove the broken material slot",
        });
    }
    Ok(())
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
    Some(TextureTransform::new(offset, rotation, scale))
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
