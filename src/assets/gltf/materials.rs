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
use super::material_extensions::{
    anisotropy_extension, clearcoat_extension, extension_texture_transform, iridescence_extension,
    read_extension_texture_info, sheen_extension,
};
use super::textures::{GltfTexture, texture_slot};

#[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
fn material_now_ms() -> f64 {
    js_sys::Date::now()
}

#[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
fn log_material_step(label: &str, start_ms: f64) -> f64 {
    let now = material_now_ms();
    if crate::diagnostics::browser_timing_enabled() {
        web_sys::console::log_1(
            &format!("[scena-demo] material {label}: {:.1}ms", now - start_ms).into(),
        );
    }
    now
}

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
    #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
    let total_start = material_now_ms();
    validate_material_texture_indices(path, document, textures.len())?;
    let materials = document
        .materials()
        .filter_map(|material| material.index().map(|index| (index, material)))
        .map(|(material_index, material)| {
            #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
            let material_start = material_now_ms();
            let pbr = material.pbr_metallic_roughness();
            let base_color = pbr.base_color_factor();
            let base_color =
                Color::from_linear_rgba(base_color[0], base_color[1], base_color[2], base_color[3]);
            let metallic = pbr.metallic_factor();
            let roughness = pbr.roughness_factor();
            let mut desc = if material.unlit() {
                MaterialDesc::unlit(base_color)
            } else {
                MaterialDesc::pbr_metallic_roughness(base_color, metallic, roughness)
            };
            if let Some(info) = pbr.base_color_texture() {
                #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
                let slot_start = material_now_ms();
                let texture = texture_slot(
                    path,
                    "baseColorTexture",
                    info.texture().index(),
                    textures,
                    storage,
                    TextureColorSpace::Srgb,
                )?;
                #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
                {
                    log_material_step("baseColorTexture", slot_start);
                }
                desc = desc.with_base_color_texture(texture);
                if let Some(transform) = texture_transform(&info) {
                    desc = desc.with_base_color_texture_transform(transform);
                }
            }
            if let Some(info) = pbr.metallic_roughness_texture() {
                #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
                let slot_start = material_now_ms();
                let texture = texture_slot(
                    path,
                    "metallicRoughnessTexture",
                    info.texture().index(),
                    textures,
                    storage,
                    TextureColorSpace::Linear,
                )?;
                #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
                {
                    log_material_step("metallicRoughnessTexture", slot_start);
                }
                desc = desc.with_metallic_roughness_texture(texture);
                if let Some(transform) = texture_transform(&info) {
                    desc = desc.with_metallic_roughness_texture_transform(transform);
                }
            }
            if let Some(normal) = material.normal_texture() {
                #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
                let slot_start = material_now_ms();
                let texture = texture_slot(
                    path,
                    "normalTexture",
                    normal.texture().index(),
                    textures,
                    storage,
                    TextureColorSpace::Linear,
                )?;
                #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
                {
                    log_material_step("normalTexture", slot_start);
                }
                desc = desc
                    .with_normal_texture(texture)
                    // Phase 5.1: parse normalTexture.scale (glTF spec
                    // default 1.0). Previously dropped — assets that
                    // authored a custom scale rendered with strength
                    // always 1.0.
                    .with_normal_scale(normal.scale());
                if let Some(transform) = normal_texture_transform(&normal) {
                    desc = desc.with_normal_texture_transform(transform);
                }
            }
            if let Some(occlusion) = material.occlusion_texture() {
                #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
                let slot_start = material_now_ms();
                let texture = texture_slot(
                    path,
                    "occlusionTexture",
                    occlusion.texture().index(),
                    textures,
                    storage,
                    TextureColorSpace::Linear,
                )?;
                #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
                {
                    log_material_step("occlusionTexture", slot_start);
                }
                desc = desc
                    .with_occlusion_texture(texture)
                    // Phase 5.1: parse occlusionTexture.strength.
                    .with_occlusion_strength(occlusion.strength());
                if let Some(transform) = occlusion_texture_transform(&occlusion) {
                    desc = desc.with_occlusion_texture_transform(transform);
                }
            }
            if let Some(info) = material.emissive_texture() {
                #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
                let slot_start = material_now_ms();
                let texture = texture_slot(
                    path,
                    "emissiveTexture",
                    info.texture().index(),
                    textures,
                    storage,
                    TextureColorSpace::Srgb,
                )?;
                #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
                {
                    log_material_step("emissiveTexture", slot_start);
                }
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
            if let Some(clearcoat) = clearcoat_extension(document, material_index) {
                desc = desc
                    .with_clearcoat_factor(clearcoat.factor)
                    .with_clearcoat_roughness_factor(clearcoat.roughness_factor);
                if let Some(info) = clearcoat.texture {
                    let texture = texture_slot(
                        path,
                        "clearcoatTexture",
                        info.index,
                        textures,
                        storage,
                        TextureColorSpace::Linear,
                    )?;
                    desc = desc.with_clearcoat_texture(texture);
                    if let Some(transform) = info.transform {
                        desc = desc.with_clearcoat_texture_transform(transform);
                    }
                }
                if let Some(info) = clearcoat.roughness_texture {
                    let texture = texture_slot(
                        path,
                        "clearcoatRoughnessTexture",
                        info.index,
                        textures,
                        storage,
                        TextureColorSpace::Linear,
                    )?;
                    desc = desc.with_clearcoat_roughness_texture(texture);
                    if let Some(transform) = info.transform {
                        desc = desc.with_clearcoat_roughness_texture_transform(transform);
                    }
                }
                if let Some(info) = clearcoat.normal_texture {
                    let texture = texture_slot(
                        path,
                        "clearcoatNormalTexture",
                        info.index,
                        textures,
                        storage,
                        TextureColorSpace::Linear,
                    )?;
                    desc = desc
                        .with_clearcoat_normal_texture(texture)
                        .with_clearcoat_normal_scale(info.scale.unwrap_or(1.0));
                    if let Some(transform) = info.transform {
                        desc = desc.with_clearcoat_normal_texture_transform(transform);
                    }
                }
            }
            if let Some(sheen) = sheen_extension(document, material_index) {
                desc = desc
                    .with_sheen_color_factor(sheen.color_factor)
                    .with_sheen_roughness_factor(sheen.roughness_factor);
                if let Some(info) = sheen.color_texture {
                    let texture = texture_slot(
                        path,
                        "sheenColorTexture",
                        info.index,
                        textures,
                        storage,
                        TextureColorSpace::Srgb,
                    )?;
                    desc = desc.with_sheen_color_texture(texture);
                    if let Some(transform) = info.transform {
                        desc = desc.with_sheen_color_texture_transform(transform);
                    }
                }
                if let Some(info) = sheen.roughness_texture {
                    let texture = texture_slot(
                        path,
                        "sheenRoughnessTexture",
                        info.index,
                        textures,
                        storage,
                        TextureColorSpace::Linear,
                    )?;
                    desc = desc.with_sheen_roughness_texture(texture);
                    if let Some(transform) = info.transform {
                        desc = desc.with_sheen_roughness_texture_transform(transform);
                    }
                }
            }
            if let Some(anisotropy) = anisotropy_extension(document, material_index) {
                desc = desc
                    .with_anisotropy_strength_factor(anisotropy.strength)
                    .with_anisotropy_rotation_radians(anisotropy.rotation);
                if let Some(info) = anisotropy.texture {
                    let texture = texture_slot(
                        path,
                        "anisotropyTexture",
                        info.index,
                        textures,
                        storage,
                        TextureColorSpace::Linear,
                    )?;
                    desc = desc.with_anisotropy_texture(texture);
                    if let Some(transform) = info.transform {
                        desc = desc.with_anisotropy_texture_transform(transform);
                    }
                }
            }
            if let Some(iridescence) = iridescence_extension(document, material_index) {
                desc = desc
                    .with_iridescence_factor(iridescence.factor)
                    .with_iridescence_ior(iridescence.ior)
                    .with_iridescence_thickness_range_nm(
                        iridescence.thickness_minimum,
                        iridescence.thickness_maximum,
                    );
                if let Some(info) = iridescence.texture {
                    let texture = texture_slot(
                        path,
                        "iridescenceTexture",
                        info.index,
                        textures,
                        storage,
                        TextureColorSpace::Linear,
                    )?;
                    desc = desc.with_iridescence_texture(texture);
                    if let Some(transform) = info.transform {
                        desc = desc.with_iridescence_texture_transform(transform);
                    }
                }
                if let Some(info) = iridescence.thickness_texture {
                    let texture = texture_slot(
                        path,
                        "iridescenceThicknessTexture",
                        info.index,
                        textures,
                        storage,
                        TextureColorSpace::Linear,
                    )?;
                    desc = desc.with_iridescence_thickness_texture(texture);
                    if let Some(transform) = info.transform {
                        desc = desc.with_iridescence_thickness_texture_transform(transform);
                    }
                }
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
            #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
            {
                log_material_step("material total", material_start);
            }
            Ok(storage.materials.insert(desc))
        })
        .collect();
    #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
    {
        log_material_step("parse_materials total", total_start);
    }
    materials
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
            pbr.base_color_texture
                .as_ref()
                .map(|info| info.index.value()),
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
            material
                .normal_texture
                .as_ref()
                .map(|info| info.index.value()),
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
            material
                .emissive_texture
                .as_ref()
                .map(|info| info.index.value()),
            texture_count,
        )?;
        if let Some(clearcoat) = material
            .extensions
            .as_ref()
            .and_then(|extensions| extensions.others.get("KHR_materials_clearcoat"))
        {
            for (slot, key) in [
                ("clearcoatTexture", "clearcoatTexture"),
                ("clearcoatRoughnessTexture", "clearcoatRoughnessTexture"),
                ("clearcoatNormalTexture", "clearcoatNormalTexture"),
            ] {
                validate_texture_info(
                    path,
                    material_index,
                    slot,
                    read_extension_texture_info(clearcoat, key).map(|info| info.index),
                    texture_count,
                )?;
            }
        }
        if let Some(sheen) = material
            .extensions
            .as_ref()
            .and_then(|extensions| extensions.others.get("KHR_materials_sheen"))
        {
            for (slot, key) in [
                ("sheenColorTexture", "sheenColorTexture"),
                ("sheenRoughnessTexture", "sheenRoughnessTexture"),
            ] {
                validate_texture_info(
                    path,
                    material_index,
                    slot,
                    read_extension_texture_info(sheen, key).map(|info| info.index),
                    texture_count,
                )?;
            }
        }
        if let Some(anisotropy) = material
            .extensions
            .as_ref()
            .and_then(|extensions| extensions.others.get("KHR_materials_anisotropy"))
        {
            validate_texture_info(
                path,
                material_index,
                "anisotropyTexture",
                read_extension_texture_info(anisotropy, "anisotropyTexture").map(|info| info.index),
                texture_count,
            )?;
        }
        if let Some(iridescence) = material
            .extensions
            .as_ref()
            .and_then(|extensions| extensions.others.get("KHR_materials_iridescence"))
        {
            for (slot, key) in [
                ("iridescenceTexture", "iridescenceTexture"),
                ("iridescenceThicknessTexture", "iridescenceThicknessTexture"),
            ] {
                validate_texture_info(
                    path,
                    material_index,
                    slot,
                    read_extension_texture_info(iridescence, key).map(|info| info.index),
                    texture_count,
                )?;
            }
        }
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

fn normal_texture_transform(
    normal: &::gltf::material::NormalTexture<'_>,
) -> Option<TextureTransform> {
    extension_texture_transform(normal.extension_value("KHR_texture_transform"))
}

fn occlusion_texture_transform(
    occlusion: &::gltf::material::OcclusionTexture<'_>,
) -> Option<TextureTransform> {
    extension_texture_transform(occlusion.extension_value("KHR_texture_transform"))
}
