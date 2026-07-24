//! Stage C2: material parsing now uses the `gltf` crate's typed
//! `Material` accessors. `KHR_materials_unlit` and
//! `KHR_materials_emissive_strength` are surfaced via typed methods on
//! `Material`; KHR_texture_transform is read from each `texture::Info`'s
//! `texture_transform()` accessor.

use ::gltf::Document;
use ::gltf::texture::Info;

use crate::diagnostics::AssetError;
use crate::material::{AlphaMode, Color, MaterialDesc, TextureColorSpace, TextureTransform};

use super::super::{
    AssetMaterialFallback, AssetMaterialSource, AssetPath, AssetStorage, MaterialHandle,
};
use super::material_extensions::{
    anisotropy_extension, clearcoat_extension, dispersion_extension, extension_texture_transform,
    ior_extension, iridescence_extension, sheen_extension, transmission_extension,
    validate_material_texture_indices, volume_extension,
};
use super::material_fallbacks::{
    MaterialFallbackSinks, TextureSlotFallbackRequest, texture_slot_with_fallback,
};
use super::textures::IndexedGltfTextures;

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
    textures: &IndexedGltfTextures,
    material_fallbacks: &mut Vec<AssetMaterialFallback>,
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
            let material_name = material.name().map(str::to_owned);
            let result: Result<MaterialHandle, AssetError> = (|| {
                #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
                let material_start = material_now_ms();
                let mut source_fallbacks = Vec::new();
                let mut fallback_sinks = MaterialFallbackSinks {
                    all: material_fallbacks,
                    source: &mut source_fallbacks,
                };
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
                    #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
                    let slot_start = material_now_ms();
                    let texture = texture_slot_with_fallback(
                        TextureSlotFallbackRequest::new(
                            path,
                            "baseColorTexture",
                            material_index,
                            info.texture().index(),
                            TextureColorSpace::Srgb,
                        ),
                        textures,
                        storage,
                        &mut fallback_sinks,
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
                    let texture = texture_slot_with_fallback(
                        TextureSlotFallbackRequest::new(
                            path,
                            "metallicRoughnessTexture",
                            material_index,
                            info.texture().index(),
                            TextureColorSpace::Linear,
                        ),
                        textures,
                        storage,
                        &mut fallback_sinks,
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
                    let texture = texture_slot_with_fallback(
                        TextureSlotFallbackRequest::new(
                            path,
                            "normalTexture",
                            material_index,
                            normal.texture().index(),
                            TextureColorSpace::Linear,
                        ),
                        textures,
                        storage,
                        &mut fallback_sinks,
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
                    let texture = texture_slot_with_fallback(
                        TextureSlotFallbackRequest::new(
                            path,
                            "occlusionTexture",
                            material_index,
                            occlusion.texture().index(),
                            TextureColorSpace::Linear,
                        ),
                        textures,
                        storage,
                        &mut fallback_sinks,
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
                    let texture = texture_slot_with_fallback(
                        TextureSlotFallbackRequest::new(
                            path,
                            "emissiveTexture",
                            material_index,
                            info.texture().index(),
                            TextureColorSpace::Srgb,
                        ),
                        textures,
                        storage,
                        &mut fallback_sinks,
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
                        let texture = texture_slot_with_fallback(
                            TextureSlotFallbackRequest::new(
                                path,
                                "clearcoatTexture",
                                material_index,
                                info.index,
                                TextureColorSpace::Linear,
                            ),
                            textures,
                            storage,
                            &mut fallback_sinks,
                        )?;
                        desc = desc.with_clearcoat_texture(texture);
                        if let Some(transform) = info.transform {
                            desc = desc.with_clearcoat_texture_transform(transform);
                        }
                    }
                    if let Some(info) = clearcoat.roughness_texture {
                        let texture = texture_slot_with_fallback(
                            TextureSlotFallbackRequest::new(
                                path,
                                "clearcoatRoughnessTexture",
                                material_index,
                                info.index,
                                TextureColorSpace::Linear,
                            ),
                            textures,
                            storage,
                            &mut fallback_sinks,
                        )?;
                        desc = desc.with_clearcoat_roughness_texture(texture);
                        if let Some(transform) = info.transform {
                            desc = desc.with_clearcoat_roughness_texture_transform(transform);
                        }
                    }
                    if let Some(info) = clearcoat.normal_texture {
                        let texture = texture_slot_with_fallback(
                            TextureSlotFallbackRequest::new(
                                path,
                                "clearcoatNormalTexture",
                                material_index,
                                info.index,
                                TextureColorSpace::Linear,
                            ),
                            textures,
                            storage,
                            &mut fallback_sinks,
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
                        let texture = texture_slot_with_fallback(
                            TextureSlotFallbackRequest::new(
                                path,
                                "sheenColorTexture",
                                material_index,
                                info.index,
                                TextureColorSpace::Srgb,
                            ),
                            textures,
                            storage,
                            &mut fallback_sinks,
                        )?;
                        desc = desc.with_sheen_color_texture(texture);
                        if let Some(transform) = info.transform {
                            desc = desc.with_sheen_color_texture_transform(transform);
                        }
                    }
                    if let Some(info) = sheen.roughness_texture {
                        let texture = texture_slot_with_fallback(
                            TextureSlotFallbackRequest::new(
                                path,
                                "sheenRoughnessTexture",
                                material_index,
                                info.index,
                                TextureColorSpace::Linear,
                            ),
                            textures,
                            storage,
                            &mut fallback_sinks,
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
                        let texture = texture_slot_with_fallback(
                            TextureSlotFallbackRequest::new(
                                path,
                                "anisotropyTexture",
                                material_index,
                                info.index,
                                TextureColorSpace::Linear,
                            ),
                            textures,
                            storage,
                            &mut fallback_sinks,
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
                        let texture = texture_slot_with_fallback(
                            TextureSlotFallbackRequest::new(
                                path,
                                "iridescenceTexture",
                                material_index,
                                info.index,
                                TextureColorSpace::Linear,
                            ),
                            textures,
                            storage,
                            &mut fallback_sinks,
                        )?;
                        desc = desc.with_iridescence_texture(texture);
                        if let Some(transform) = info.transform {
                            desc = desc.with_iridescence_texture_transform(transform);
                        }
                    }
                    if let Some(info) = iridescence.thickness_texture {
                        let texture = texture_slot_with_fallback(
                            TextureSlotFallbackRequest::new(
                                path,
                                "iridescenceThicknessTexture",
                                material_index,
                                info.index,
                                TextureColorSpace::Linear,
                            ),
                            textures,
                            storage,
                            &mut fallback_sinks,
                        )?;
                        desc = desc.with_iridescence_thickness_texture(texture);
                        if let Some(transform) = info.transform {
                            desc = desc.with_iridescence_thickness_texture_transform(transform);
                        }
                    }
                }
                if let Some(dispersion) = dispersion_extension(document, material_index) {
                    desc = desc.with_dispersion_factor(dispersion.factor);
                }
                if let Some(transmission) = transmission_extension(document, material_index) {
                    desc = desc.with_transmission_factor(transmission.factor);
                    if let Some(info) = transmission.texture {
                        let texture = texture_slot_with_fallback(
                            TextureSlotFallbackRequest::new(
                                path,
                                "transmissionTexture",
                                material_index,
                                info.index,
                                TextureColorSpace::Linear,
                            ),
                            textures,
                            storage,
                            &mut fallback_sinks,
                        )?;
                        desc = desc.with_transmission_texture(texture);
                        if let Some(transform) = info.transform {
                            desc = desc.with_transmission_texture_transform(transform);
                        }
                    }
                }
                if let Some(ior) = ior_extension(document, material_index) {
                    desc = desc.with_ior(ior.ior);
                }
                if let Some(volume) = volume_extension(document, material_index) {
                    desc = desc
                        .with_thickness_factor(volume.thickness_factor)
                        .with_attenuation_distance(volume.attenuation_distance)
                        .with_attenuation_color(volume.attenuation_color);
                    if let Some(info) = volume.thickness_texture {
                        let texture = texture_slot_with_fallback(
                            TextureSlotFallbackRequest::new(
                                path,
                                "thicknessTexture",
                                material_index,
                                info.index,
                                TextureColorSpace::Linear,
                            ),
                            textures,
                            storage,
                            &mut fallback_sinks,
                        )?;
                        desc = desc.with_thickness_texture(texture);
                        if let Some(transform) = info.transform {
                            desc = desc.with_thickness_texture_transform(transform);
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
                let handle = storage.materials.insert(std::sync::Arc::new(desc));
                storage.material_sources.insert(
                    handle,
                    AssetMaterialSource::source_material(
                        path.clone(),
                        material_index,
                        source_fallbacks,
                    ),
                );
                Ok(handle)
            })();
            result.map_err(|error| {
                add_missing_texture_material_identity(error, material_index, material_name.clone())
            })
        })
        .collect();
    #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
    {
        log_material_step("parse_materials total", total_start);
    }
    materials
}

fn add_missing_texture_material_identity(
    mut error: AssetError,
    index: usize,
    name: Option<String>,
) -> AssetError {
    if let AssetError::MissingTexture { context, .. } = &mut error {
        context.material_index = Some(index);
        context.material_name = name;
    }
    error
}

fn texture_transform(info: &Info<'_>) -> Option<TextureTransform> {
    info.texture_transform().map(|transform| {
        TextureTransform::new(transform.offset(), transform.rotation(), transform.scale())
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
