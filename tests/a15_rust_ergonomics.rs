use scena::prelude::*;

#[test]
fn prelude_covers_everyday_scene_asset_and_render_work_without_schema_bulk() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let (mut scene, camera) = Scene::with_default_camera().unwrap();
    let node = scene.mesh(geometry, material).add().unwrap();
    let outcome = scene
        .frame_node_with_assets_and_options(
            camera,
            node,
            &assets,
            FramingOptions::new().viewport(320, 240),
        )
        .unwrap();
    assert!(outcome.distance.is_finite());

    let prelude_source = include_str!("../src/prelude.rs");
    assert!(!prelude_source.contains("V1"));
    assert!(!prelude_source.contains("SCHEMA"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_missing_file_is_curated_not_found() {
    let unique = format!(
        "target/a15-missing-{}-{}.glb",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    );
    let path = AssetPath::from(unique.clone());
    let error = pollster::block_on(FileAssetFetcher.fetch(&path))
        .expect_err("fixture path is intentionally absent");
    assert!(matches!(error, AssetError::NotFound { path } if path == unique));
}

#[test]
fn controls_features_remain_documented_metadata_only_aliases() {
    let cargo = include_str!("../Cargo.toml");
    assert!(cargo.contains("controls = []"));
    assert!(cargo.contains("controls-winit = [\"controls\"]"));
    assert!(cargo.contains("controls-web = [\"controls\"]"));

    let controls = include_str!("../src/controls.rs");
    assert!(!controls.contains("cfg(feature = \"controls\")"));
    let ownership = include_str!("../docs/specs/feature-ownership.json");
    assert!(ownership.contains("\"kind\": \"compatibility-alias\""));
    let docs = include_str!("../docs/feature-flags.md");
    assert!(docs.contains("compatibility alias enabling `controls`"));
}
