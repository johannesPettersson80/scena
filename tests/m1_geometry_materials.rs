use scena::{
    AlphaMode, AssetPath, Assets, Color, EnvironmentHandle, GeometryHandle, MaterialDesc,
    MaterialHandle, MaterialKind, ModelHandle, SceneAsset, TextureColorSpace, TextureDesc,
    TextureHandle,
};

fn assert_handle<T: Copy + Eq + std::fmt::Debug>() {}

#[test]
fn asset_taxonomy_reserves_distinct_typed_handles() {
    assert_handle::<ModelHandle>();
    assert_handle::<GeometryHandle>();
    assert_handle::<MaterialHandle>();
    assert_handle::<TextureHandle>();
    assert_handle::<EnvironmentHandle>();

    let scene_asset = SceneAsset::empty();
    assert!(format!("{scene_asset:?}").contains("SceneAsset"));
}

#[test]
fn material_descriptor_defaults_are_explicit() {
    let material = MaterialDesc::default();

    assert_eq!(material.kind, MaterialKind::PbrMetallicRoughness);
    assert_eq!(material.base_color, Color::WHITE);
    assert_eq!(material.alpha_mode, AlphaMode::Opaque);
    assert_eq!(material.emissive, Color::BLACK);
    assert_eq!(material.metallic_factor, 0.0);
    assert_eq!(material.roughness_factor, 1.0);
}

#[test]
fn color_constructors_make_source_color_space_explicit() {
    assert_eq!(
        Color::from_srgb_u8(255, 128, 0),
        Color::from_srgb(1.0, 128.0 / 255.0, 0.0)
    );
    assert_eq!(
        Color::from_hex_srgb("#ff8000").expect("valid hex color"),
        Color::from_srgb_u8(255, 128, 0)
    );
    assert!(Color::from_hex_srgb("ff8000").is_err());
}

#[test]
fn assets_create_material_stores_descriptor_by_typed_handle() {
    let assets = Assets::new();
    let material = MaterialDesc::unlit(Color::from_linear_rgb(0.25, 0.5, 0.75));

    let handle = assets.create_material(material.clone());
    let retrieved: Option<MaterialDesc> = assets.material(handle);

    assert_eq!(retrieved, Some(material));
}

#[test]
fn assets_load_texture_records_color_space_and_deduplicates_cache_key() {
    let assets = Assets::new();

    let first =
        pollster::block_on(assets.load_texture("textures/albedo.png", TextureColorSpace::Srgb))
            .expect("texture request is recorded");
    let duplicate =
        pollster::block_on(assets.load_texture("textures/albedo.png", TextureColorSpace::Srgb))
            .expect("duplicate texture request is recorded");
    let linear =
        pollster::block_on(assets.load_texture("textures/albedo.png", TextureColorSpace::Linear))
            .expect("linear texture request is recorded");
    let first_texture: Option<TextureDesc> = assets.texture(first);

    assert_eq!(first, duplicate);
    assert_ne!(first, linear);
    assert_eq!(
        first_texture.as_ref().map(|texture| texture.path()),
        Some(&AssetPath::from("textures/albedo.png"))
    );
    assert_eq!(
        first_texture.map(|texture| texture.color_space()),
        Some(TextureColorSpace::Srgb)
    );
    assert_eq!(
        assets.texture(linear).map(|texture| texture.color_space()),
        Some(TextureColorSpace::Linear)
    );
}
