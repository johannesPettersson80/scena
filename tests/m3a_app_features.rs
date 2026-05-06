#![cfg(not(target_arch = "wasm32"))]

use scena::{
    Aabb, AssetError, Assets, Camera, ChangeKind, ImportOptions, LookupError, NodeKind,
    NotPreparedReason, PerspectiveCamera, Quat, RenderError, Renderer, Scene, Transform, Vec3,
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

#[test]
fn camera_frame_and_look_at_helpers_update_view_and_require_prepare() {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::default(),
        )
        .expect("camera inserts");
    let target = scene
        .add_empty(
            scene.root(),
            Transform {
                translation: Vec3::new(3.0, 2.0, -5.0),
                ..Transform::default()
            },
        )
        .expect("target node inserts");
    let bounds = Aabb::new(Vec3::new(-2.0, -1.0, -3.0), Vec3::new(4.0, 5.0, 1.0));
    let mut renderer = Renderer::headless(8, 8).expect("renderer builds");
    renderer.prepare(&mut scene).expect("scene prepares");

    scene
        .frame(camera, bounds)
        .expect("camera frames imported bounds");

    let camera_node = scene.camera_node(camera).expect("camera node is queryable");
    let framed_transform = scene
        .node(camera_node)
        .expect("camera node exists")
        .transform();
    let framed_camera = match scene.camera(camera).expect("camera descriptor exists") {
        Camera::Perspective(camera) => *camera,
        Camera::Orthographic(_) => panic!("test inserted a perspective camera"),
    };
    let center = Vec3::new(1.0, 2.0, -1.0);
    let radius = (3.0_f32 * 3.0 + 3.0 * 3.0 + 2.0 * 2.0).sqrt();
    let distance = framed_transform.translation.z - center.z;

    assert_vec3_near(
        framed_transform.translation,
        Vec3::new(1.0, 2.0, center.z + distance),
    );
    assert!(distance > radius);
    assert!(framed_camera.near <= distance - radius);
    assert!(framed_camera.far >= distance + radius);

    scene
        .look_at(camera, target)
        .expect("camera looks at target node");
    let looked_transform = scene
        .node(camera_node)
        .expect("camera node exists")
        .transform();
    let forward = rotate_vec3(looked_transform.rotation, Vec3::new(0.0, 0.0, -1.0));
    let expected_forward = normalize(sub_vec3(
        scene
            .node(target)
            .expect("target exists")
            .transform()
            .translation,
        looked_transform.translation,
    ));

    assert_vec3_near(forward, expected_forward);
    assert!(matches!(
        renderer.render(&scene, camera),
        Err(RenderError::NotPrepared {
            reason: NotPreparedReason::SceneChanged {
                change: ChangeKind::SceneStructure,
                ..
            },
        })
    ));
}

fn assert_vec3_near(actual: Vec3, expected: Vec3) {
    const EPSILON: f32 = 0.0001;
    assert!(
        (actual.x - expected.x).abs() <= EPSILON
            && (actual.y - expected.y).abs() <= EPSILON
            && (actual.z - expected.z).abs() <= EPSILON,
        "expected {actual:?} to be within {EPSILON} of {expected:?}"
    );
}

fn rotate_vec3(rotation: Quat, vector: Vec3) -> Vec3 {
    let tx = 2.0 * (rotation.y * vector.z - rotation.z * vector.y);
    let ty = 2.0 * (rotation.z * vector.x - rotation.x * vector.z);
    let tz = 2.0 * (rotation.x * vector.y - rotation.y * vector.x);
    Vec3::new(
        vector.x + rotation.w * tx + (rotation.y * tz - rotation.z * ty),
        vector.y + rotation.w * ty + (rotation.z * tx - rotation.x * tz),
        vector.z + rotation.w * tz + (rotation.x * ty - rotation.y * tx),
    )
}

fn normalize(value: Vec3) -> Vec3 {
    let length = (value.x * value.x + value.y * value.y + value.z * value.z).sqrt();
    Vec3::new(value.x / length, value.y / length, value.z / length)
}

fn sub_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x - right.x, left.y - right.y, left.z - right.z)
}
