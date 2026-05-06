#![cfg(not(target_arch = "wasm32"))]

use scena::{
    AssetError, Assets, ChangeKind, ImportOptions, LookupError, NodeKind, NotPreparedReason,
    PerspectiveCamera, RenderError, Renderer, Scene, Transform,
};

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

#[test]
fn scene_import_convenience_uses_gltf_default_options() {
    let assets = Assets::new();
    let scene_asset = pollster::block_on(assets.load_scene("tests/assets/gltf/minimal_scene.gltf"))
        .expect("minimal glTF scene loads");

    let mut from_asset = Scene::new();
    let import = from_asset
        .instantiate_with(&scene_asset, ImportOptions::gltf_default())
        .expect("scene asset instantiates with explicit options");
    assert!(import.node("Root").is_ok());

    let mut from_path = Scene::new();
    let import = pollster::block_on(from_path.import_with(
        &assets,
        "tests/assets/gltf/minimal_scene.gltf",
        ImportOptions::gltf_default(),
    ))
    .expect("scene imports with explicit options");
    assert!(import.path("Root/Child").is_ok());

    let mut sugar = Scene::new();
    let import = pollster::block_on(sugar.import(&assets, "tests/assets/gltf/minimal_scene.gltf"))
        .expect("scene import convenience uses glTF defaults");
    assert!(import.first_node("Child").is_some());
}

#[test]
fn replace_import_returns_fresh_import_and_stales_old_lookups() {
    let assets = Assets::new();
    let scene_asset = pollster::block_on(assets.load_scene("tests/assets/gltf/minimal_scene.gltf"))
        .expect("minimal glTF scene loads");
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::default(),
        )
        .expect("camera inserts");
    let import = scene
        .instantiate(&scene_asset)
        .expect("scene asset instantiates");
    let old_root = import.node("Root").expect("old root lookup succeeds");
    let mut renderer = Renderer::headless(8, 8).expect("renderer builds");
    renderer.prepare(&mut scene).expect("scene prepares");

    let replacement = scene
        .replace_import(&import, &scene_asset)
        .expect("import replacement succeeds");
    let new_root = replacement.node("Root").expect("new root lookup succeeds");

    assert_ne!(new_root, old_root);
    assert!(matches!(import.node("Root"), Err(LookupError::StaleImport)));
    let error = renderer
        .render(&scene, camera)
        .expect_err("replacement marks renderer state as needing prepare");
    assert!(matches!(
        error,
        RenderError::NotPrepared {
            reason: NotPreparedReason::SceneChanged {
                change: ChangeKind::SceneStructure,
                ..
            },
        }
    ));
}

#[test]
fn scene_import_reports_duplicate_names_and_escaped_paths() {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/name_lookup_scene.gltf"))
            .expect("name lookup glTF scene loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("name lookup scene instantiates");

    let duplicate = import
        .node("Dup")
        .expect_err("unique lookup rejects duplicate node names");
    assert!(matches!(
        duplicate,
        LookupError::AmbiguousNodeName { ref name, ref matches }
            if name == "Dup" && matches.len() == 2
    ));

    let slash_node = import
        .path("Root/A\\/B")
        .expect("escaped slash path lookup succeeds");
    assert_eq!(
        import.node("A/B").expect("unique slash name lookup"),
        slash_node
    );
}
