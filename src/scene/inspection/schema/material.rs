use crate::assets::{
    AssetMaterialFallbackV1, AssetMaterialSource, AssetMaterialSourceKind, TextureSourceFormat,
};
use crate::material::{AlphaMode, MaterialKind, TextureColorSpace};

use super::{
    SceneMaterialInspectionV1, SceneMaterialSlotInspectionV1, SceneMaterialSourceInspectionV1,
};
use crate::scene::inspection::{SceneMaterialInspection, SceneTextureInspection};

pub(super) fn material_inspection_v1(
    material: &SceneMaterialInspection,
) -> SceneMaterialInspectionV1 {
    let fallbacks = material
        .fallbacks()
        .iter()
        .map(AssetMaterialFallbackV1::from)
        .collect::<Vec<_>>();
    SceneMaterialInspectionV1 {
        kind: material_kind_name(material.kind()).to_string(),
        source: material
            .source()
            .map(material_source_inspection_v1)
            .unwrap_or_else(unknown_material_source),
        base_color: material.base_color(),
        alpha_mode: alpha_mode_name(material.alpha_mode()).to_string(),
        textures: material_texture_slot_rows(material, &fallbacks),
        fallbacks,
    }
}

fn material_source_inspection_v1(source: &AssetMaterialSource) -> SceneMaterialSourceInspectionV1 {
    SceneMaterialSourceInspectionV1 {
        kind: material_source_kind_name(source.kind()).to_string(),
        asset_path: source.asset_path().map(|path| path.as_str().to_owned()),
        material_index: source.material_index(),
        reason: source.reason().map(str::to_owned),
    }
}

fn unknown_material_source() -> SceneMaterialSourceInspectionV1 {
    SceneMaterialSourceInspectionV1 {
        kind: "unknown".to_string(),
        asset_path: None,
        material_index: None,
        reason: Some("material source metadata was not recorded".to_string()),
    }
}

fn material_texture_slot_rows(
    material: &SceneMaterialInspection,
    fallbacks: &[AssetMaterialFallbackV1],
) -> Vec<SceneMaterialSlotInspectionV1> {
    [
        ("baseColorTexture", material.base_color_texture()),
        ("normalTexture", material.normal_texture()),
        (
            "metallicRoughnessTexture",
            material.metallic_roughness_texture(),
        ),
        ("occlusionTexture", material.occlusion_texture()),
        ("emissiveTexture", material.emissive_texture()),
        ("clearcoatTexture", material.clearcoat_texture()),
        (
            "clearcoatRoughnessTexture",
            material.clearcoat_roughness_texture(),
        ),
        (
            "clearcoatNormalTexture",
            material.clearcoat_normal_texture(),
        ),
        ("sheenColorTexture", material.sheen_color_texture()),
        ("sheenRoughnessTexture", material.sheen_roughness_texture()),
        ("anisotropyTexture", material.anisotropy_texture()),
        ("iridescenceTexture", material.iridescence_texture()),
        (
            "iridescenceThicknessTexture",
            material.iridescence_thickness_texture(),
        ),
        ("transmissionTexture", material.transmission_texture()),
        ("thicknessTexture", material.thickness_texture()),
    ]
    .into_iter()
    .filter_map(|(slot, texture)| {
        let texture = texture?;
        Some(SceneMaterialSlotInspectionV1 {
            slot: slot.to_string(),
            source_path: texture.source_path().as_str().to_owned(),
            source_format: texture_source_format_name(texture.source_format()).to_string(),
            color_space: texture_color_space_name(texture.color_space()).to_string(),
            decoded_dimensions: texture
                .decoded_dimensions()
                .map(|(width, height)| [width, height]),
            has_decoded_pixels: texture.has_decoded_pixels(),
            provenance: texture.provenance().clone(),
            fallback: matching_fallback(slot, &texture, fallbacks),
        })
    })
    .collect()
}

fn matching_fallback(
    slot: &str,
    texture: &SceneTextureInspection,
    fallbacks: &[AssetMaterialFallbackV1],
) -> Option<AssetMaterialFallbackV1> {
    fallbacks
        .iter()
        .find(|fallback| {
            fallback.material_slot == slot
                && fallback.fallback_path == texture.source_path().as_str()
        })
        .cloned()
}

fn material_source_kind_name(kind: AssetMaterialSourceKind) -> &'static str {
    match kind {
        AssetMaterialSourceKind::UserCreated => "user_created",
        AssetMaterialSourceKind::SourceMaterial => "source_material",
        AssetMaterialSourceKind::GeneratedDefault => "generated_default",
    }
}

fn material_kind_name(kind: MaterialKind) -> &'static str {
    match kind {
        MaterialKind::Unlit => "unlit",
        MaterialKind::PbrMetallicRoughness => "pbr_metallic_roughness",
        MaterialKind::Line => "line",
        MaterialKind::Wireframe => "wireframe",
        MaterialKind::Edge => "edge",
    }
}

fn alpha_mode_name(alpha_mode: AlphaMode) -> &'static str {
    match alpha_mode {
        AlphaMode::Opaque => "opaque",
        AlphaMode::Mask { .. } => "mask",
        AlphaMode::Blend => "blend",
    }
}

fn texture_source_format_name(format: TextureSourceFormat) -> &'static str {
    match format {
        TextureSourceFormat::Png => "png",
        TextureSourceFormat::Jpeg => "jpeg",
        TextureSourceFormat::Webp => "webp",
        TextureSourceFormat::Ktx2Basisu => "ktx2_basisu",
        TextureSourceFormat::MemoryRgba8 => "memory_rgba8",
        TextureSourceFormat::MemoryRgba16Float => "memory_rgba16_float",
    }
}

fn texture_color_space_name(color_space: TextureColorSpace) -> &'static str {
    match color_space {
        TextureColorSpace::Linear => "linear",
        TextureColorSpace::Srgb => "srgb",
    }
}
