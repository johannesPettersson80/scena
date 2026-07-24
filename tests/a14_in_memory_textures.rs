use scena::{
    AssetError, Assets, TextureColorSpace, TextureFilter, TextureMemoryDesc, TextureMemoryId,
    TextureMipPolicy, TexturePixelFormat, TextureSamplerDesc, TextureSlot, TextureWrap,
};

fn checker_rgba8() -> Vec<u8> {
    vec![
        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
    ]
}

#[test]
fn generated_rgba8_texture_has_typed_identity_and_deduplicates_exact_content() {
    let assets = Assets::new();
    let descriptor = TextureMemoryDesc::rgba8_for_slot(
        TextureMemoryId::new("generated/checker").expect("identity is valid"),
        2,
        2,
        checker_rgba8(),
        TextureSlot::BaseColor,
    )
    .with_sampler(TextureSamplerDesc::default())
    .with_mip_policy(TextureMipPolicy::Generate);

    let first = assets
        .create_texture(descriptor.clone())
        .expect("generated texture creates");
    let second = assets
        .create_texture(descriptor)
        .expect("identical identity and content deduplicate");
    assert_eq!(first, second);

    let texture = assets.texture(first).expect("texture remains available");
    assert_eq!(
        texture.memory_identity().unwrap().as_str(),
        "generated/checker"
    );
    assert_eq!(texture.decoded_dimensions(), Some((2, 2)));
    assert_eq!(texture.pixel_format(), TexturePixelFormat::Rgba8UnormSrgb);
    assert_eq!(texture.mip_policy(), TextureMipPolicy::Generate);
    assert_eq!(texture.decoded_rgba8().unwrap().2, checker_rgba8());
}

#[test]
fn generated_linear_float_texture_preserves_linear_float_contract() {
    let assets = Assets::new();
    let descriptor = TextureMemoryDesc::linear_rgba32f(
        TextureMemoryId::new("generated/linear-meter").unwrap(),
        2,
        1,
        vec![[0.25, 0.5, 1.0, 1.0], [2.0, 0.0, 0.125, 1.0]],
    )
    .with_mip_policy(TextureMipPolicy::None);

    let handle = assets.create_texture(descriptor).unwrap();
    let texture = assets.texture(handle).unwrap();
    assert_eq!(texture.color_space(), TextureColorSpace::Linear);
    assert_eq!(texture.pixel_format(), TexturePixelFormat::Rgba16Float);
    assert_eq!(texture.decoded_dimensions(), Some((2, 1)));
    assert!(texture.decoded_rgba8().is_none());
}

#[test]
fn generated_texture_rejects_identity_collision_and_slot_color_mismatch() {
    let assets = Assets::new();
    let identity = TextureMemoryId::new("generated/material").unwrap();
    let base = TextureMemoryDesc::rgba8(
        identity.clone(),
        1,
        1,
        vec![255, 255, 255, 255],
        TextureColorSpace::Srgb,
    );
    assets.create_texture(base).unwrap();

    let changed =
        TextureMemoryDesc::rgba8(identity, 1, 1, vec![0, 0, 0, 255], TextureColorSpace::Srgb);
    assert!(matches!(
        assets.create_texture(changed),
        Err(AssetError::TextureIdentityCollision { .. })
    ));

    let wrong_space = TextureMemoryDesc::rgba8(
        TextureMemoryId::new("generated/normal").unwrap(),
        1,
        1,
        vec![128, 128, 255, 255],
        TextureColorSpace::Srgb,
    );
    assert!(matches!(
        assets.create_texture_for_slot(wrong_space, TextureSlot::Normal),
        Err(AssetError::TextureColorSpaceMismatch { .. })
    ));
    assert_eq!(
        TextureSlot::BaseColor.color_space(),
        TextureColorSpace::Srgb
    );
    assert_eq!(TextureSlot::Normal.color_space(), TextureColorSpace::Linear);
}

#[test]
fn generated_texture_rejects_malformed_zero_overflow_and_non_finite_inputs() {
    assert!(matches!(
        TextureMemoryId::new("  "),
        Err(AssetError::InvalidTextureIdentity { .. })
    ));

    let malformed = TextureMemoryDesc::rgba8(
        TextureMemoryId::new("generated/short").unwrap(),
        2,
        2,
        vec![0; 15],
        TextureColorSpace::Linear,
    );
    assert!(matches!(
        Assets::new().create_texture(malformed),
        Err(AssetError::InvalidTextureData {
            expected_elements: 16,
            actual_elements: 15,
            ..
        })
    ));

    let zero = TextureMemoryDesc::rgba8(
        TextureMemoryId::new("generated/zero").unwrap(),
        0,
        1,
        Vec::new(),
        TextureColorSpace::Linear,
    );
    assert!(matches!(
        Assets::new().create_texture(zero),
        Err(AssetError::TextureSizeLimit {
            width: 0,
            height: 1,
            ..
        })
    ));

    let overflow = TextureMemoryDesc::rgba8(
        TextureMemoryId::new("generated/overflow").unwrap(),
        u32::MAX,
        u32::MAX,
        Vec::new(),
        TextureColorSpace::Linear,
    );
    assert!(matches!(
        Assets::new().create_texture(overflow),
        Err(AssetError::TextureSizeLimit { .. })
    ));

    let non_finite = TextureMemoryDesc::linear_rgba32f(
        TextureMemoryId::new("generated/nan").unwrap(),
        1,
        1,
        vec![[f32::NAN, 0.0, 0.0, 1.0]],
    );
    assert!(matches!(
        Assets::new().create_texture(non_finite),
        Err(AssetError::InvalidTextureData { .. })
    ));
}

#[test]
fn generated_texture_rejects_mipmap_sampler_without_generated_mips() {
    let descriptor = TextureMemoryDesc::rgba8(
        TextureMemoryId::new("generated/mip-contract").unwrap(),
        1,
        1,
        vec![255; 4],
        TextureColorSpace::Linear,
    )
    .with_sampler(TextureSamplerDesc::new(
        Some(TextureFilter::Linear),
        Some(TextureFilter::LinearMipmapLinear),
        TextureWrap::Repeat,
        TextureWrap::Repeat,
    ));

    assert!(matches!(
        Assets::new().create_texture(descriptor),
        Err(AssetError::InvalidTextureData { .. })
    ));
}
