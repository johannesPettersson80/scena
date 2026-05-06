use scena::{
    Aabb, AlphaMode, AssetPath, Assets, Color, EnvironmentHandle, GeometryDesc, GeometryHandle,
    GeometryTopology, MaterialDesc, MaterialHandle, MaterialKind, ModelHandle, NodeKind, Scene,
    SceneAsset, TextureColorSpace, TextureDesc, TextureHandle, Transform, Vec3,
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

    assert_eq!(material.kind(), MaterialKind::PbrMetallicRoughness);
    assert_eq!(material.base_color(), Color::WHITE);
    assert_eq!(material.alpha_mode(), AlphaMode::Opaque);
    assert_eq!(material.emissive(), Color::BLACK);
    assert_eq!(material.emissive_strength(), 1.0);
    assert_eq!(material.metallic_factor(), 0.0);
    assert_eq!(material.roughness_factor(), 1.0);
    assert!(!material.double_sided());
    assert_eq!(material.base_color_texture(), None);
    assert_eq!(material.normal_texture(), None);
    assert_eq!(material.metallic_roughness_texture(), None);
    assert_eq!(material.occlusion_texture(), None);
    assert_eq!(material.emissive_texture(), None);
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
fn unlit_material_descriptor_sets_non_pbr_defaults() {
    const MATERIAL: MaterialDesc = MaterialDesc::unlit(Color::from_linear_rgb(0.25, 0.5, 0.75));

    assert_eq!(MATERIAL.kind(), MaterialKind::Unlit);
    assert_eq!(
        MATERIAL.base_color(),
        Color::from_linear_rgb(0.25, 0.5, 0.75)
    );
    assert_eq!(MATERIAL.metallic_factor(), 0.0);
    assert_eq!(MATERIAL.roughness_factor(), 1.0);
    assert_eq!(MATERIAL.alpha_mode(), AlphaMode::Opaque);
}

#[test]
fn pbr_material_factors_are_const_and_sanitized() {
    const CLAMPED: MaterialDesc = MaterialDesc::pbr_metallic_roughness(Color::WHITE, 2.0, -1.0);

    assert_eq!(CLAMPED.kind(), MaterialKind::PbrMetallicRoughness);
    assert_eq!(CLAMPED.metallic_factor(), 1.0);
    assert_eq!(CLAMPED.roughness_factor(), 0.0);

    let sanitized_nan = MaterialDesc::pbr_metallic_roughness(Color::WHITE, f32::NAN, f32::NAN);
    assert_eq!(sanitized_nan.metallic_factor(), 0.0);
    assert_eq!(sanitized_nan.roughness_factor(), 1.0);
    assert!(!sanitized_nan.metallic_factor().is_nan());
    assert!(!sanitized_nan.roughness_factor().is_nan());
}

#[test]
fn material_texture_slot_helpers_store_handles_without_color_space_duplication() {
    let assets = Assets::new();
    let albedo =
        pollster::block_on(assets.load_texture("paint_basecolor.png", TextureColorSpace::Srgb))
            .expect("albedo request is recorded");
    let normal =
        pollster::block_on(assets.load_texture("paint_normal.png", TextureColorSpace::Linear))
            .expect("normal request is recorded");
    let metallic_roughness = pollster::block_on(
        assets.load_texture("paint_metallic_roughness.png", TextureColorSpace::Linear),
    )
    .expect("metallic roughness request is recorded");
    let occlusion =
        pollster::block_on(assets.load_texture("paint_occlusion.png", TextureColorSpace::Linear))
            .expect("occlusion request is recorded");
    let emissive =
        pollster::block_on(assets.load_texture("paint_emissive.png", TextureColorSpace::Srgb))
            .expect("emissive request is recorded");

    let material = MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.2, 0.8)
        .with_base_color_texture(albedo)
        .with_normal_texture(normal)
        .with_metallic_roughness_texture(metallic_roughness)
        .with_occlusion_texture(occlusion)
        .with_emissive_texture(emissive);

    assert_eq!(material.base_color_texture(), Some(albedo));
    assert_eq!(material.normal_texture(), Some(normal));
    assert_eq!(
        material.metallic_roughness_texture(),
        Some(metallic_roughness)
    );
    assert_eq!(material.occlusion_texture(), Some(occlusion));
    assert_eq!(material.emissive_texture(), Some(emissive));
}

#[test]
fn alpha_and_emissive_helpers_sanitize_descriptor_values() {
    const MASKED: MaterialDesc = MaterialDesc::unlit(Color::WHITE)
        .with_alpha_mode(AlphaMode::Mask { cutoff: 0.4 })
        .with_emissive(Color::from_linear_rgb(0.1, 0.2, 0.3))
        .with_emissive_strength(2.5)
        .with_double_sided(true);

    assert_eq!(MASKED.alpha_mode(), AlphaMode::Mask { cutoff: 0.4 });
    assert_eq!(MASKED.emissive(), Color::from_linear_rgb(0.1, 0.2, 0.3));
    assert_eq!(MASKED.emissive_strength(), 2.5);
    assert!(MASKED.double_sided());

    const HIGH_CUTOFF: MaterialDesc =
        MaterialDesc::unlit(Color::WHITE).with_alpha_mode(AlphaMode::Mask { cutoff: 2.0 });
    const NAN_CUTOFF: MaterialDesc =
        MaterialDesc::unlit(Color::WHITE).with_alpha_mode(AlphaMode::Mask { cutoff: f32::NAN });
    const NEGATIVE_EMISSIVE: MaterialDesc =
        MaterialDesc::unlit(Color::WHITE).with_emissive_strength(-2.0);
    const NAN_EMISSIVE: MaterialDesc =
        MaterialDesc::unlit(Color::WHITE).with_emissive_strength(f32::NAN);

    assert_eq!(HIGH_CUTOFF.alpha_mode(), AlphaMode::Mask { cutoff: 1.0 });
    assert_eq!(NAN_CUTOFF.alpha_mode(), AlphaMode::Mask { cutoff: 0.5 });
    assert_eq!(NEGATIVE_EMISSIVE.emissive_strength(), 0.0);
    assert_eq!(NAN_EMISSIVE.emissive_strength(), 1.0);

    let transparent = MASKED.with_alpha_mode(AlphaMode::Blend);
    assert_eq!(transparent.alpha_mode(), AlphaMode::Blend);
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

fn assert_geometry_invariants(geometry: &GeometryDesc) {
    assert!(!geometry.vertices().is_empty());
    for index in geometry.indices() {
        assert!((*index as usize) < geometry.vertices().len());
    }
    for vertex in geometry.vertices() {
        assert!(geometry.bounds().contains(vertex.position));
    }
    match geometry.topology() {
        GeometryTopology::Triangles => {
            assert_eq!(geometry.indices().len() % 3, 0);
            for vertex in geometry.vertices() {
                let length = vector_length(vertex.normal);
                assert!(
                    (length - 1.0).abs() <= 1.0e-4,
                    "triangle geometry normals must be unit length, got {length}"
                );
            }
        }
        GeometryTopology::Lines => assert_eq!(geometry.indices().len() % 2, 0),
    }
}

fn vector_length(value: Vec3) -> f32 {
    (value.x * value.x + value.y * value.y + value.z * value.z).sqrt()
}

#[test]
fn builtin_geometry_generators_produce_valid_bounds_and_indices() {
    let geometries = [
        GeometryDesc::box_xyz(2.0, 4.0, 6.0),
        GeometryDesc::sphere(1.5, 12, 6),
        GeometryDesc::cylinder(1.0, 3.0, 12),
        GeometryDesc::plane(2.0, 3.0),
        GeometryDesc::line(Vec3::ZERO, Vec3::new(1.0, 2.0, 3.0)),
        GeometryDesc::polyline(&[
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
        ]),
        GeometryDesc::arrow(Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0)),
        GeometryDesc::grid(10.0, 10),
        GeometryDesc::axes(2.0),
    ];

    for geometry in geometries {
        assert_geometry_invariants(&geometry);
    }
    assert_eq!(
        GeometryDesc::box_xyz(2.0, 4.0, 6.0).bounds(),
        Aabb::new(Vec3::new(-1.0, -2.0, -3.0), Vec3::new(1.0, 2.0, 3.0))
    );
}

#[test]
fn geometry_construction_rejects_invalid_manual_buffers() {
    assert!(Aabb::from_vertices(&[]).is_none());
    assert!(GeometryDesc::try_new(GeometryTopology::Triangles, Vec::new(), Vec::new()).is_err());
    assert!(
        GeometryDesc::try_new(
            GeometryTopology::Triangles,
            GeometryDesc::plane(1.0, 1.0).vertices().to_vec(),
            vec![0, 1]
        )
        .is_err()
    );
    assert!(
        GeometryDesc::try_new(
            GeometryTopology::Lines,
            GeometryDesc::line(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0))
                .vertices()
                .to_vec(),
            vec![0, 2]
        )
        .is_err()
    );
}

#[test]
fn assets_create_geometry_stores_descriptor_by_typed_handle() {
    let assets = Assets::new();
    let geometry = GeometryDesc::plane(2.0, 2.0);

    let handle = assets.create_geometry(geometry.clone());

    assert_eq!(assets.geometry(handle), Some(geometry));
}

#[test]
fn scene_mesh_builder_inserts_typed_mesh_node() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::plane(1.0, 1.0));
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
