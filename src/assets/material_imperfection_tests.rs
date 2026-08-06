use crate::material::{Color, MaterialDesc};

use super::{
    Assets, MaterialImperfectionDesc, MaterialImperfectionProfileV1, TextureMemoryDesc,
    TextureMemoryId, TextureSlot,
};

#[test]
fn material_imperfection_is_deterministic_and_composited_into_existing_pbr_maps() {
    let assets = Assets::new();
    let width = 32;
    let height = 32;
    let mut flat_normal = Vec::with_capacity(width * height * 4);
    let mut flat_orm = Vec::with_capacity(width * height * 4);
    for _ in 0..width * height {
        flat_normal.extend_from_slice(&[128, 128, 255, 255]);
        flat_orm.extend_from_slice(&[255, 128, 0, 255]);
    }
    let normal = assets
        .create_texture_for_slot(
            TextureMemoryDesc::rgba8_for_slot(
                TextureMemoryId::new("test/imperfection/source-normal").unwrap(),
                width as u32,
                height as u32,
                flat_normal.clone(),
                TextureSlot::Normal,
            ),
            TextureSlot::Normal,
        )
        .unwrap();
    let orm = assets
        .create_texture_for_slot(
            TextureMemoryDesc::rgba8_for_slot(
                TextureMemoryId::new("test/imperfection/source-orm").unwrap(),
                width as u32,
                height as u32,
                flat_orm.clone(),
                TextureSlot::MetallicRoughness,
            ),
            TextureSlot::MetallicRoughness,
        )
        .unwrap();
    let source = MaterialDesc::pbr_metallic_roughness(Color::WHITE, 1.0, 1.0)
        .with_normal_texture(normal)
        .with_metallic_roughness_texture(orm)
        .with_occlusion_texture(orm)
        .with_photographic_surface_tile_size_m(0.10);
    let descriptor = MaterialImperfectionDesc::new(MaterialImperfectionProfileV1::Dust)
        .with_strength(0.18)
        .with_physical_scale_m(0.003)
        .with_seed(91);

    let first = assets
        .composite_material_imperfection(source.clone(), descriptor)
        .expect("imperfection composites into prepared PBR data");
    let second = assets
        .composite_material_imperfection(source, descriptor)
        .expect("identical imperfection input is deterministic");

    assert_eq!(first.base_color_texture(), None, "no shader layer is added");
    assert_ne!(first.normal_texture(), Some(normal));
    assert_ne!(first.metallic_roughness_texture(), Some(orm));
    assert_eq!(first.normal_texture(), second.normal_texture());
    assert_eq!(
        first.metallic_roughness_texture(),
        second.metallic_roughness_texture()
    );
    assert_eq!(
        first.occlusion_texture(),
        first.metallic_roughness_texture()
    );

    let (_, _, composed_normal) = assets
        .try_texture(first.normal_texture().unwrap())
        .unwrap()
        .decoded_rgba8()
        .map(|(width, height, pixels)| (width, height, pixels.to_vec()))
        .unwrap();
    let (_, _, composed_orm) = assets
        .try_texture(first.metallic_roughness_texture().unwrap())
        .unwrap()
        .decoded_rgba8()
        .map(|(width, height, pixels)| (width, height, pixels.to_vec()))
        .unwrap();
    assert_ne!(composed_normal, flat_normal);
    assert_ne!(composed_orm, flat_orm);
    assert!(
        composed_orm
            .chunks_exact(4)
            .all(|pixel| (105..=166).contains(&pixel[1])),
        "subtle imperfection must keep roughness variation bounded"
    );

    let oil_film = assets
        .composite_material_imperfection(
            MaterialDesc::pbr_metallic_roughness(Color::WHITE, 1.0, 1.0)
                .with_normal_texture(normal)
                .with_metallic_roughness_texture(orm)
                .with_occlusion_texture(orm)
                .with_photographic_surface_tile_size_m(0.10),
            MaterialImperfectionDesc::new(MaterialImperfectionProfileV1::OilFilm)
                .with_strength(0.12)
                .with_physical_scale_m(0.004)
                .with_seed(17),
        )
        .expect("oil film composites into prepared roughness data");
    assert_eq!(
        oil_film.normal_texture(),
        Some(normal),
        "roughness-dominated oil film must reuse the prepared normal map"
    );
    assert_ne!(oil_film.metallic_roughness_texture(), Some(orm));
}

#[test]
fn material_imperfection_defaults_escape_the_quantization_dead_zone() {
    let assets = Assets::new();
    let width = 64;
    let height = 64;
    let mut flat_normal = Vec::with_capacity(width * height * 4);
    let mut flat_orm = Vec::with_capacity(width * height * 4);
    for _ in 0..width * height {
        flat_normal.extend_from_slice(&[128, 128, 255, 255]);
        flat_orm.extend_from_slice(&[255, 128, 0, 255]);
    }
    let normal = assets
        .create_texture_for_slot(
            TextureMemoryDesc::rgba8_for_slot(
                TextureMemoryId::new("test/imperfection/default-source-normal").unwrap(),
                width as u32,
                height as u32,
                flat_normal,
                TextureSlot::Normal,
            ),
            TextureSlot::Normal,
        )
        .unwrap();
    let orm = assets
        .create_texture_for_slot(
            TextureMemoryDesc::rgba8_for_slot(
                TextureMemoryId::new("test/imperfection/default-source-orm").unwrap(),
                width as u32,
                height as u32,
                flat_orm,
                TextureSlot::MetallicRoughness,
            ),
            TextureSlot::MetallicRoughness,
        )
        .unwrap();

    for profile in [
        MaterialImperfectionProfileV1::Dust,
        MaterialImperfectionProfileV1::Smudge,
        MaterialImperfectionProfileV1::FineScratches,
        MaterialImperfectionProfileV1::OilFilm,
    ] {
        let source = MaterialDesc::pbr_metallic_roughness(Color::WHITE, 1.0, 1.0)
            .with_normal_texture(normal)
            .with_metallic_roughness_texture(orm)
            .with_occlusion_texture(orm)
            .with_photographic_surface_tile_size_m(0.10);
        let composed = assets
            .composite_material_imperfection(
                source,
                MaterialImperfectionDesc::new(profile).with_seed(73),
            )
            .expect("default imperfection composites into prepared PBR data");
        let (_, _, pixels) = assets
            .try_texture(composed.metallic_roughness_texture().unwrap())
            .unwrap()
            .decoded_rgba8()
            .map(|(width, height, pixels)| (width, height, pixels.to_vec()))
            .unwrap();
        let max_delta = pixels
            .chunks_exact(4)
            .map(|pixel| pixel[1].abs_diff(128))
            .max()
            .unwrap();

        assert!(
            max_delta >= 8,
            "{profile:?} default roughness delta must escape the 1-3 LSB quantization dead zone; got {max_delta}"
        );
        assert!(
            max_delta <= 52,
            "{profile:?} default roughness delta must remain bounded; got {max_delta}"
        );
    }
}
