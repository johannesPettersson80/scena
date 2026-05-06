use scena::{
    AlphaMode, AssetPath, Assets, Color, EnvironmentHandle, GeometryHandle, MaterialDesc,
    MaterialHandle, MaterialKind, ModelHandle, NodeKind, Scene, SceneAsset, TextureColorSpace,
    TextureDesc, TextureHandle, Transform, Vec3,
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

#[test]
fn scene_mesh_builder_inserts_typed_mesh_node() {
    let assets = Assets::new();
    // Geometry creation lands with built-in geometry; this sentinel only proves the
    // scene-side typed-handle insertion path.
    let geometry = GeometryHandle::default();
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();

    let node = scene
        .mesh(geometry, material)
        .transform(Transform {
            translation: Vec3::new(1.0, 2.0, 3.0),
            ..Transform::default()
        })
        .add()
        .expect("mesh node inserts under root");

    let node = scene.node(node).expect("mesh node exists");
    assert_eq!(node.transform().translation, Vec3::new(1.0, 2.0, 3.0));
    match node.kind() {
        NodeKind::Mesh(mesh) => {
            assert_eq!(mesh.geometry(), geometry);
            assert_eq!(mesh.material(), material);
        }
        other => panic!("expected mesh node, got {other:?}"),
    }
}

#[test]
fn scene_model_builder_inserts_typed_model_node_under_parent() {
    let mut scene = Scene::new();
    let parent = scene
        .add_empty(scene.root(), Transform::default())
        .expect("parent inserts");
    // Model loading lands with glTF/model assets; this sentinel only proves the
    // scene-side typed-handle insertion path.
    let model = ModelHandle::default();

    let node = scene
        .model(model)
        .parent(parent)
        .add()
        .expect("model node inserts under parent");

    let node = scene.node(node).expect("model node exists");
    assert_eq!(node.parent(), Some(parent));
    match node.kind() {
        NodeKind::Model(model_node) => assert_eq!(model_node.model(), model),
        other => panic!("expected model node, got {other:?}"),
    }
}
