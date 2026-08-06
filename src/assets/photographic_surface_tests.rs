use super::{Assets, PhotographicSurfaceDesc, PhotographicSurfaceKind};
use crate::material::Color;

#[test]
fn photographic_surface_generation_is_deterministic_and_non_uniform() {
    let assets = Assets::new();
    let descriptor = PhotographicSurfaceDesc::new(
        PhotographicSurfaceKind::BrushedMetal,
        Color::from_srgb_u8(174, 179, 184),
    )
    .with_feature_scale_m(0.000_35)
    .with_variation(0.72)
    .with_wear(0.18)
    .with_seed(0x5ce9_a123)
    .with_resolution(32);

    let first = assets
        .create_photographic_surface(descriptor)
        .expect("surface generation succeeds");
    let second = assets
        .create_photographic_surface(descriptor)
        .expect("identical surface generation succeeds");

    assert_eq!(first, second, "the complete descriptor is the cache key");
    assert_eq!(
        assets
            .material(first.material())
            .expect("generated material")
            .normal_texture(),
        Some(first.normal_texture())
    );
    assert_eq!(
        assets
            .material(first.material())
            .expect("generated material")
            .metallic_roughness_texture(),
        Some(first.metallic_roughness_texture())
    );
    assert_eq!(
        assets
            .material(first.material())
            .expect("generated material")
            .photographic_surface_tile_size_m(),
        Some(descriptor.tile_size_m()),
        "generated materials must retain physical mapping metadata"
    );

    let sample_uvs = [
        [0.07, 0.11],
        [0.19, 0.73],
        [0.41, 0.29],
        [0.68, 0.87],
        [0.91, 0.47],
    ];
    let roughness = sample_uvs.map(|uv| {
        assets
            .sample_texture(first.metallic_roughness_texture(), uv)
            .expect("roughness sample")
            .g
    });
    let normal_x = sample_uvs.map(|uv| {
        assets
            .sample_texture(first.normal_texture(), uv)
            .expect("normal sample")
            .r
    });

    assert!(
        range(roughness) >= 0.025,
        "roughness must contain visible but bounded surface variation"
    );
    assert!(
        range(normal_x) >= 0.015,
        "normal map must contain real directional microdetail"
    );
}

#[test]
fn photographic_surface_seed_and_finish_change_generated_assets() {
    let assets = Assets::new();
    let base = PhotographicSurfaceDesc::new(
        PhotographicSurfaceKind::MoldedPlastic,
        Color::from_srgb_u8(38, 88, 196),
    )
    .with_feature_scale_m(0.000_6)
    .with_variation(0.55)
    .with_resolution(32);

    let first = assets
        .create_photographic_surface(base.with_seed(7))
        .expect("first surface");
    let different_seed = assets
        .create_photographic_surface(base.with_seed(8))
        .expect("different seed surface");
    let different_finish = assets
        .create_photographic_surface(
            PhotographicSurfaceDesc::new(
                PhotographicSurfaceKind::PowderCoatedMetal,
                Color::from_srgb_u8(38, 88, 196),
            )
            .with_feature_scale_m(0.000_6)
            .with_variation(0.55)
            .with_seed(7)
            .with_resolution(32),
        )
        .expect("different finish surface");

    assert_ne!(first.material(), different_seed.material());
    assert_ne!(first.normal_texture(), different_seed.normal_texture());
    assert_ne!(first.material(), different_finish.material());
    assert_ne!(
        first.metallic_roughness_texture(),
        different_finish.metallic_roughness_texture()
    );
}

#[test]
fn photographic_surface_normal_energy_is_visible_but_subtle() {
    let assets = Assets::new();
    let surface = assets
        .create_photographic_surface(
            PhotographicSurfaceDesc::new(
                PhotographicSurfaceKind::CastMetal,
                Color::from_srgb_u8(28, 29, 31),
            )
            .with_variation(1.0)
            .with_seed(0x7ca5_71a9)
            .with_resolution(64),
        )
        .expect("cast surface");
    let material = assets
        .material(surface.material())
        .expect("generated material");
    let mut maximum_effective_slope = 0.0_f32;
    for y in 0..64 {
        for x in 0..64 {
            let sample = assets
                .sample_texture(
                    surface.normal_texture(),
                    [(x as f32 + 0.5) / 64.0, (y as f32 + 0.5) / 64.0],
                )
                .expect("normal sample");
            let x = (sample.r * 2.0 - 1.0) * material.normal_scale();
            let y = (sample.g * 2.0 - 1.0) * material.normal_scale();
            maximum_effective_slope = maximum_effective_slope.max(x.hypot(y));
        }
    }

    assert!(
        (0.015..=0.16).contains(&maximum_effective_slope),
        "cast detail must remain visible without reading as large pitting: {maximum_effective_slope}"
    );
}

fn range(values: [f32; 5]) -> f32 {
    let minimum = values.iter().copied().fold(f32::INFINITY, f32::min);
    let maximum = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    maximum - minimum
}
