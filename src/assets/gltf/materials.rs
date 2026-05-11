//! Stage C2: material parsing now uses the `gltf` crate's typed
//! `Material` accessors. `KHR_materials_unlit` and
//! `KHR_materials_emissive_strength` are surfaced via typed methods on
//! `Material`; KHR_texture_transform is read from each `texture::Info`'s
//! `texture_transform()` accessor.

use ::gltf::Document;
use ::gltf::texture::Info;

use crate::diagnostics::AssetError;
use crate::material::{AlphaMode, Color, MaterialDesc, TextureColorSpace, TextureTransform};

use super::super::{AssetPath, AssetStorage, MaterialHandle};
use super::textures::{GltfTexture, texture_slot};

pub(super) fn parse_materials(
    path: &AssetPath,
    document: &Document,
    storage: &mut AssetStorage,
    textures: &[GltfTexture],
) -> Result<Vec<MaterialHandle>, AssetError> {
    // Stage C2: pre-validate texture references in the raw JSON
    // before we hand them to the gltf crate's typed accessors —
    // the typed Info constructors unwrap on missing texture
    // indices, which would otherwise propagate as a panic instead
    // of a structured `MissingTexture` error.
    validate_material_texture_indices(path, document, textures.len())?;
    document
        .materials()
        .filter_map(|material| material.index().map(|_| material))
        .map(|material| {
            let pbr = material.pbr_metallic_roughness();
            let base_color = pbr.base_color_factor();
            let base_color = Color::from_linear_rgba(
                base_color[0],
                base_color[1],
                base_color[2],
                base_color[3],
            );
            let metallic = pbr.metallic_factor();
            let roughness = pbr.roughness_factor();
            let mut desc = if material.unlit() {
                MaterialDesc::unlit(base_color)
            } else {
                MaterialDesc::pbr_metallic_roughness(base_color, metallic, roughness)
            };
            if let Some(info) = pbr.base_color_texture() {
                let texture = texture_slot(
                    path,
                    "baseColorTexture",
                    info.texture().index(),
                    textures,
                    storage,
                    TextureColorSpace::Srgb,
                )?;
                desc = desc.with_base_color_texture(texture);
                if let Some(transform) = texture_transform(&info) {
                    desc = desc.with_base_color_texture_transform(transform);
                }
            }
            if let Some(info) = pbr.metallic_roughness_texture() {
                let texture = texture_slot(
                    path,
                    "metallicRoughnessTexture",
                    info.texture().index(),
                    textures,
                    storage,
                    TextureColorSpace::Linear,
                )?;
                desc = desc.with_metallic_roughness_texture(texture);
                if let Some(transform) = texture_transform(&info) {
                    desc = desc.with_metallic_roughness_texture_transform(transform);
                }
            }
            if let Some(normal) = material.normal_texture() {
                let texture = texture_slot(
                    path,
                    "normalTexture",
                    normal.texture().index(),
                    textures,
                    storage,
                    TextureColorSpace::Linear,
                )?;
                desc = desc.with_normal_texture(texture);
                if let Some(transform) = normal_texture_transform(&normal) {
                    desc = desc.with_normal_texture_transform(transform);
                }
            }
            if let Some(occlusion) = material.occlusion_texture() {
                let texture = texture_slot(
                    path,
                    "occlusionTexture",
                    occlusion.texture().index(),
                    textures,
                    storage,
                    TextureColorSpace::Linear,
                )?;
                desc = desc.with_occlusion_texture(texture);
                if let Some(transform) = occlusion_texture_transform(&occlusion) {
                    desc = desc.with_occlusion_texture_transform(transform);
                }
            }
            if let Some(info) = material.emissive_texture() {
                let texture = texture_slot(
                    path,
                    "emissiveTexture",
                    info.texture().index(),
                    textures,
                    storage,
                    TextureColorSpace::Srgb,
                )?;
                desc = desc.with_emissive_texture(texture);
                if let Some(transform) = texture_transform(&info) {
                    desc = desc.with_emissive_texture_transform(transform);
                }
            }
            let emissive = material.emissive_factor();
            if emissive != [0.0, 0.0, 0.0] {
                desc = desc.with_emissive(Color::from_linear_rgb(
                    emissive[0],
                    emissive[1],
                    emissive[2],
                ));
            }
            if let Some(strength) = material.emissive_strength() {
                desc = desc.with_emissive_strength(strength);
            }
            desc = match material.alpha_mode() {
                ::gltf::material::AlphaMode::Opaque => desc,
                ::gltf::material::AlphaMode::Mask => desc.with_alpha_mode(AlphaMode::Mask {
                    cutoff: material.alpha_cutoff().unwrap_or(0.5),
                }),
                ::gltf::material::AlphaMode::Blend => desc.with_alpha_mode(AlphaMode::Blend),
            };
            if material.double_sided() {
                desc = desc.with_double_sided(true);
            }
            Ok(storage.materials.insert(desc))
        })
        .collect()
}

fn validate_material_texture_indices(
    path: &AssetPath,
    document: &Document,
    texture_count: usize,
) -> Result<(), AssetError> {
    let raw = document.as_json();
    for (material_index, material) in raw.materials.iter().enumerate() {
        let pbr = &material.pbr_metallic_roughness;
        validate_texture_info(
            path,
            material_index,
            "baseColorTexture",
            pbr.base_color_texture.as_ref().map(|info| info.index.value()),
            texture_count,
        )?;
        validate_texture_info(
            path,
            material_index,
            "metallicRoughnessTexture",
            pbr.metallic_roughness_texture
                .as_ref()
                .map(|info| info.index.value()),
            texture_count,
        )?;
        validate_texture_info(
            path,
            material_index,
            "normalTexture",
            material.normal_texture.as_ref().map(|info| info.index.value()),
            texture_count,
        )?;
        validate_texture_info(
            path,
            material_index,
            "occlusionTexture",
            material
                .occlusion_texture
                .as_ref()
                .map(|info| info.index.value()),
            texture_count,
        )?;
        validate_texture_info(
            path,
            material_index,
            "emissiveTexture",
            material.emissive_texture.as_ref().map(|info| info.index.value()),
            texture_count,
        )?;
    }
    Ok(())
}

fn validate_texture_info(
    path: &AssetPath,
    _material_index: usize,
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

fn texture_transform(info: &Info<'_>) -> Option<TextureTransform> {
    info.texture_transform().map(|transform| {
        TextureTransform::new(
            transform.offset(),
            transform.rotation(),
            transform.scale(),
            transform.tex_coord(),
        )
    })
}

fn extension_texture_transform(value: Option<&serde_json::Value>) -> Option<TextureTransform> {
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

fn normal_texture_transform(normal: &::gltf::material::NormalTexture<'_>) -> Option<TextureTransform> {
    extension_texture_transform(normal.extension_value("KHR_texture_transform"))
}

fn occlusion_texture_transform(
    occlusion: &::gltf::material::OcclusionTexture<'_>,
) -> Option<TextureTransform> {
    extension_texture_transform(occlusion.extension_value("KHR_texture_transform"))
}
