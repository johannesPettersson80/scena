#![cfg(not(target_arch = "wasm32"))]

use scena::{
    Aabb, AssetError, AssetFetcher, AssetPath, Assets, Camera, ChangeKind, Color, CursorPosition,
    GeometryTopology, HitTarget, ImportOptions, LookupError, MaterialKind, NodeKind,
    NotPreparedReason, PerspectiveCamera, Primitive, Quat, RenderError, Renderer, Scene,
    SourceCoordinateSystem, SourceUnits, Transform, Vec3, Viewport,
};
use std::future::{Ready, ready};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
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
fn assets_load_scene_uses_fetcher_trait_and_deduplicates_by_asset_path() {
    let fetcher = MemoryFetcher::new(
        "memory://scene.gltf",
        r#"{
            "asset": { "version": "2.0" },
            "nodes": [
                { "name": "FetchedRoot" },
                { "name": "FetchedChild" }
            ]
        }"#,
    );
    let assets = Assets::with_fetcher(fetcher.clone());

    let scene = pollster::block_on(assets.load_scene("memory://scene.gltf"))
        .expect("scene loads from custom fetcher");
    let duplicate = pollster::block_on(assets.load_scene("memory://scene.gltf"))
        .expect("scene cache hit does not refetch");

    assert_eq!(scene, duplicate);
    assert_eq!(scene.path().as_str(), "memory://scene.gltf");
    assert_eq!(
        scene
            .nodes()
            .iter()
            .filter_map(scena::SceneAssetNode::name)
            .collect::<Vec<_>>(),
        vec!["FetchedRoot", "FetchedChild"]
    );
    assert_eq!(fetcher.calls(), 1);

    let missing = pollster::block_on(assets.load_scene("memory://missing.gltf"))
        .expect_err("custom fetcher reports structured missing asset");
    assert_eq!(
        missing,
        AssetError::NotFound {
            path: "memory://missing.gltf".to_string()
        }
    );
}

#[test]
fn gltf_loader_creates_geometry_material_texture_and_vertex_color_contracts() {
    let assets = Assets::new();
    let scene_asset = pollster::block_on(
        assets.load_scene("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
    )
    .expect("mesh glTF scene loads");

    let mesh = scene_asset.nodes()[0]
        .mesh()
        .expect("glTF node records mesh payload");
    let geometry = assets
        .geometry(mesh.geometry())
        .expect("glTF geometry is registered in Assets");
    let material = assets
        .material(mesh.material())
        .expect("glTF material is registered in Assets");
    let base_color_texture = material
        .base_color_texture()
        .expect("glTF base color texture is registered");
    let texture = assets
        .texture(base_color_texture)
        .expect("glTF texture handle resolves");

    assert_eq!(scene_asset.mesh_count(), 1);
    assert!(mesh.uses_vertex_colors());
    assert_eq!(geometry.topology(), GeometryTopology::Triangles);
    assert_eq!(geometry.vertices().len(), 3);
    assert_eq!(geometry.indices(), [0, 1, 2]);
    assert_eq!(
        geometry.vertex_colors(),
        [
            Color::from_linear_rgba(1.0, 0.0, 0.0, 1.0),
            Color::from_linear_rgba(0.0, 1.0, 0.0, 1.0),
            Color::from_linear_rgba(0.0, 0.0, 1.0, 1.0),
        ]
    );
    assert_eq!(material.kind(), MaterialKind::Unlit);
    assert_eq!(
        material.base_color(),
        Color::from_linear_rgba(0.25, 0.5, 0.75, 1.0)
    );
    assert_eq!(
        texture.path().as_str(),
        "tests/assets/gltf/textures/albedo.png"
    );

    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("mesh scene instantiates");
    let node = import
        .node("ColoredTriangle")
        .expect("mesh node is import-queryable");
    let NodeKind::Mesh(mesh_node) = scene.node(node).expect("mesh node exists").kind() else {
        panic!("glTF mesh node should instantiate as Scene::mesh");
    };
    assert_eq!(mesh_node.geometry(), mesh.geometry());
    assert_eq!(mesh_node.material(), mesh.material());
}

#[test]
fn scene_import_reports_local_and_world_bounds_for_imported_meshes() {
    let assets = Assets::new();
    let scene_asset = pollster::block_on(
        assets.load_scene("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
    )
    .expect("mesh glTF scene loads");
    let mut scene = Scene::new();

    let import = scene
        .instantiate(&scene_asset)
        .expect("mesh scene instantiates");
    let node = import
        .node("ColoredTriangle")
        .expect("mesh node lookup succeeds");
    scene
        .set_transform(
            node,
            Transform {
                translation: Vec3::new(2.0, 3.0, 4.0),
                scale: Vec3::new(2.0, 2.0, 2.0),
                ..Transform::default()
            },
        )
        .expect("mesh transform updates");

    let local = import.bounds_local().expect("import has local bounds");
    let world = import
        .bounds_world(&scene)
        .expect("import has world bounds");

    assert_vec3_near(local.min, Vec3::new(-0.5, -0.5, 0.0));
    assert_vec3_near(local.max, Vec3::new(0.5, 0.5, 0.0));
    assert_vec3_near(world.min, Vec3::new(1.0, 2.0, 4.0));
    assert_vec3_near(world.max, Vec3::new(3.0, 4.0, 4.0));
}

#[test]
fn scene_pick_returns_typed_hit_target_for_renderable_triangle() {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::default(),
        )
        .expect("camera inserts");
    let target = scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::default(),
        )
        .expect("renderable triangle inserts");
    let viewport = Viewport::new(8, 8, 1.0).expect("viewport is valid");

    let hit = scene
        .pick(camera, CursorPosition::physical(4.0, 4.0), viewport)
        .expect("pick succeeds")
        .expect("center cursor hits triangle");

    assert_eq!(hit.target(), HitTarget::Node(target));
    assert_vec3_near(hit.world_position, Vec3::new(0.0, 0.0, 0.0));
    assert!(hit.distance >= 0.0);
    assert_eq!(
        scene
            .pick(camera, CursorPosition::logical(0.0, 0.0), viewport)
            .expect("corner pick succeeds"),
        None
    );
}

#[test]
fn import_options_apply_gltf_node_transforms_and_source_units() {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/transform_options_scene.gltf"))
            .expect("transform glTF scene loads");
    let mut scene = Scene::new();

    let import = scene
        .instantiate_with(
            &scene_asset,
            ImportOptions::gltf_default().with_source_units(SourceUnits::Centimeters),
        )
        .expect("centimeter source scene instantiates");
    let root = import.node("RootCm").expect("root lookup succeeds");
    let child = import.node("ChildCm").expect("child lookup succeeds");

    assert_eq!(
        scene_asset.nodes()[0].transform().translation,
        Vec3::new(100.0, 0.0, 0.0)
    );
    assert_vec3_near(
        scene
            .node(root)
            .expect("root exists")
            .transform()
            .translation,
        Vec3::new(1.0, 0.0, 0.0),
    );
    assert_vec3_near(
        scene.node(root).expect("root exists").transform().scale,
        Vec3::new(0.02, 0.02, 0.02),
    );
    assert_vec3_near(
        scene
            .node(child)
            .expect("child exists")
            .transform()
            .translation,
        Vec3::new(0.0, 0.5, 0.25),
    );

    let mut z_up_scene = Scene::new();
    let z_up_import = z_up_scene
        .instantiate_with(
            &scene_asset,
            ImportOptions::gltf_default()
                .with_source_coordinate_system(SourceCoordinateSystem::ZUpRightHanded),
        )
        .expect("Z-up source scene instantiates");
    let z_up_child = z_up_import
        .node("ChildCm")
        .expect("Z-up child lookup succeeds");
    assert_vec3_near(
        z_up_scene
            .node(z_up_child)
            .expect("Z-up child exists")
            .transform()
            .translation,
        Vec3::new(0.0, 25.0, -50.0),
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

#[derive(Clone)]
struct MemoryFetcher {
    path: AssetPath,
    source: Arc<str>,
    calls: Arc<AtomicUsize>,
}

impl MemoryFetcher {
    fn new(path: impl Into<AssetPath>, source: impl Into<Arc<str>>) -> Self {
        Self {
            path: path.into(),
            source: source.into(),
            calls: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl AssetFetcher for MemoryFetcher {
    type Future<'a> = Ready<Result<Vec<u8>, AssetError>>;

    fn fetch<'a>(&'a self, path: &'a AssetPath) -> Self::Future<'a> {
        if path == &self.path {
            self.calls.fetch_add(1, Ordering::SeqCst);
            ready(Ok(self.source.as_bytes().to_vec()))
        } else {
            ready(Err(AssetError::NotFound {
                path: path.as_str().to_string(),
            }))
        }
    }
}
