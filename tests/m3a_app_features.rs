#![cfg(not(target_arch = "wasm32"))]

use scena::{AssetError, Assets, NodeKind, Scene};

#[test]
fn assets_load_scene_caches_gltf_asset_and_rejects_required_extensions() {
    let assets = Assets::new();

    let scene = pollster::block_on(assets.load_scene("tests/assets/gltf/minimal_scene.gltf"))
        .expect("minimal glTF scene loads");
    let duplicate = pollster::block_on(assets.load_scene("tests/assets/gltf/minimal_scene.gltf"))
        .expect("minimal glTF scene cache hit loads");

    assert_eq!(scene, duplicate);
    assert_eq!(
        scene.path().as_str(),
        "tests/assets/gltf/minimal_scene.gltf"
    );
    assert_eq!(scene.node_count(), 2);
    assert_eq!(scene.extensions_used(), ["KHR_materials_unlit"]);
    assert!(scene.extensions_required().is_empty());

    let error = pollster::block_on(
        assets.load_scene("tests/assets/gltf/unsupported_required_extension.gltf"),
    )
    .expect_err("unsupported required glTF extension is rejected");
    assert_eq!(
        error,
        AssetError::UnsupportedRequiredExtension {
            path: "tests/assets/gltf/unsupported_required_extension.gltf".to_string(),
            extension: "KHR_materials_clearcoat".to_string(),
        }
    );
}

#[test]
fn scene_instantiate_creates_import_hierarchy_and_name_lookups() {
    let assets = Assets::new();
    let scene_asset = pollster::block_on(assets.load_scene("tests/assets/gltf/minimal_scene.gltf"))
        .expect("minimal glTF scene loads");
    let mut scene = Scene::new();

    let import = scene
        .instantiate(&scene_asset)
        .expect("scene asset instantiates");
    let root = import.node("Root").expect("unique root lookup succeeds");
    let child = import.node("Child").expect("unique child lookup succeeds");

    assert_eq!(import.first_node("Root"), Some(root));
    assert_eq!(import.nodes_named("Child").collect::<Vec<_>>(), vec![child]);
    assert_eq!(
        import.path("Root/Child").expect("path lookup succeeds"),
        child
    );
    assert_eq!(
        scene.node(root).expect("root node exists").parent(),
        Some(scene.root())
    );
    assert_eq!(
        scene.node(child).expect("child node exists").parent(),
        Some(root)
    );
    assert_eq!(
        scene.node(root).expect("root node exists").kind(),
        &NodeKind::Empty
    );
    assert_eq!(
        scene.node(child).expect("child node exists").kind(),
        &NodeKind::Empty
    );
}
