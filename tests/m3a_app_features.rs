#![cfg(not(target_arch = "wasm32"))]

use scena::{
    Aabb, AssetError, AssetFetcher, AssetPath, Assets, BuildError, Camera, ChangeKind, Color,
    CursorPosition, GeometryDesc, GeometryTopology, GeometryVertex, HitTarget, ImportOptions,
    InstanceCullingPolicy, InstantiateError, InteractionStyle, LabelBillboard, LabelDesc,
    LabelRasterization, LookupError, MaterialDesc, MaterialKind, NodeKind, NotPreparedReason,
    OffscreenTarget, PerspectiveCamera, Primitive, Quat, RenderError, Renderer, Scene,
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
fn glb_loader_reads_binary_chunk_mesh_materials_and_instantiates() {
    let assets = Assets::with_fetcher(BinaryFetcher::new(
        "memory://triangle.glb",
        minimal_glb_triangle_scene(),
    ));
    let scene_asset =
        pollster::block_on(assets.load_scene("memory://triangle.glb")).expect("GLB scene loads");

    let mesh = scene_asset.nodes()[0]
        .mesh()
        .expect("GLB node records mesh payload");
    let geometry = assets
        .geometry(mesh.geometry())
        .expect("GLB geometry is registered");
    let material = assets
        .material(mesh.material())
        .expect("GLB material is registered");

    assert_eq!(scene_asset.mesh_count(), 1);
    assert_eq!(geometry.vertices().len(), 3);
    assert_eq!(geometry.indices(), [0, 1, 2]);
    assert_eq!(
        material.base_color(),
        Color::from_linear_rgba(0.2, 0.8, 0.1, 1.0)
    );

    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("GLB scene instantiates");
    assert!(import.node("GlbTriangle").is_ok());
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
fn scene_import_anchor_lookups_parse_gltf_extras_and_stale() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(
        "memory://anchors.gltf",
        r#"{
            "asset": { "version": "2.0" },
            "nodes": [
                {
                    "name": "Root",
                    "children": [1],
                    "extras": {
                        "scena": {
                            "anchors": [
                                { "name": "mount", "translation": [100.0, 0.0, 0.0] }
                            ]
                        }
                    }
                },
                {
                    "name": "Child",
                    "extras": {
                        "scena": {
                            "anchors": [
                                { "name": "mount" },
                                { "name": "inspect", "translation": [0.0, 50.0, 0.0] }
                            ]
                        }
                    }
                }
            ]
        }"#,
    ));
    let scene_asset =
        pollster::block_on(assets.load_scene("memory://anchors.gltf")).expect("anchor glTF loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate_with(
            &scene_asset,
            ImportOptions::gltf_default().with_source_units(SourceUnits::Centimeters),
        )
        .expect("anchor scene instantiates");

    let root = import.node("Root").expect("root lookup succeeds");
    let child = import.node("Child").expect("child lookup succeeds");
    let inspect = import
        .anchor("inspect")
        .expect("unique anchor lookup succeeds");
    assert_eq!(inspect.node(), child);
    assert_eq!(inspect.name(), "inspect");
    assert_vec3_near(inspect.transform().translation, Vec3::new(0.0, 0.5, 0.0));
    assert_eq!(
        import
            .first_anchor("mount")
            .expect("first mount exists")
            .node(),
        root
    );
    assert_eq!(
        import
            .anchors_named("mount")
            .map(|anchor| anchor.node())
            .collect::<Vec<_>>(),
        vec![root, child]
    );
    assert!(matches!(
        import.anchor("mount"),
        Err(LookupError::AmbiguousAnchorName { ref name, ref hosts })
            if name == "mount" && hosts == &vec![root, child]
    ));

    let replacement = scene
        .replace_import(&import, &scene_asset)
        .expect("replacement succeeds");
    assert!(replacement.anchor("inspect").is_ok());
    assert!(matches!(
        import.anchor("inspect"),
        Err(LookupError::StaleImport)
    ));
}

#[test]
fn scene_import_rejects_duplicate_anchor_names_on_same_host() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(
        "memory://duplicate-anchor.gltf",
        r#"{
            "asset": { "version": "2.0" },
            "nodes": [
                {
                    "name": "Root",
                    "extras": {
                        "scena": {
                            "anchors": [
                                { "name": "mount" },
                                { "name": "mount", "translation": [1.0, 0.0, 0.0] }
                            ]
                        }
                    }
                }
            ]
        }"#,
    ));
    let scene_asset = pollster::block_on(assets.load_scene("memory://duplicate-anchor.gltf"))
        .expect("duplicate-anchor glTF parses before instantiate validation");
    let mut scene = Scene::new();

    let error = scene
        .instantiate(&scene_asset)
        .expect_err("duplicate same-host anchors are rejected");

    assert!(matches!(
        error,
        InstantiateError::InvalidAnchorExtras { ref node, ref reason }
            if node == "Root" && reason.contains("duplicate anchor 'mount'")
    ));
}

#[test]
fn scene_import_rejects_invalid_anchor_extras_data() {
    for (suffix, anchor_json, reason_fragment) in [
        ("blank-name", r#"{ "name": "   " }"#, "name"),
        (
            "bad-rotation",
            r#"{ "name": "bad-rotation", "rotation": [0.0, 0.0, 0.0, 2.0] }"#,
            "normalized",
        ),
        (
            "zero-scale",
            r#"{ "name": "zero-scale", "scale": [1.0, 0.0, 1.0] }"#,
            "scale",
        ),
    ] {
        let path = format!("memory://invalid-anchor-{suffix}.gltf");
        let source = format!(
            r#"{{
                "asset": {{ "version": "2.0" }},
                "nodes": [
                    {{
                        "name": "Root",
                        "extras": {{
                            "scena": {{
                                "anchors": [
                                    {anchor_json}
                                ]
                            }}
                        }}
                    }}
                ]
            }}"#
        );
        let assets = Assets::with_fetcher(MemoryFetcher::new(path.as_str(), source));
        let scene_asset = pollster::block_on(assets.load_scene(path.as_str()))
            .expect("invalid anchor glTF loads");
        let mut scene = Scene::new();

        let error = scene
            .instantiate(&scene_asset)
            .expect_err("invalid anchor extras are rejected during instantiation");

        assert!(matches!(
            error,
            InstantiateError::InvalidAnchorExtras { ref node, ref reason }
                if node == "Root" && reason.contains(reason_fragment)
        ));
    }
}

#[test]
fn scene_import_clip_lookups_are_import_local_and_stale() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(
        "memory://clips.gltf",
        r#"{
            "asset": { "version": "2.0" },
            "nodes": [{ "name": "Root" }],
            "animations": [
                { "name": "Spin" },
                { "name": "Pulse" },
                { "name": "Spin" }
            ]
        }"#,
    ));
    let scene_asset =
        pollster::block_on(assets.load_scene("memory://clips.gltf")).expect("clip glTF loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("clip scene instantiates");

    let pulse = import.clip("Pulse").expect("unique clip lookup succeeds");
    assert_eq!(pulse.name(), Some("Pulse"));
    assert_eq!(
        import.first_clip("Spin").expect("first spin exists").name(),
        Some("Spin")
    );
    assert_eq!(import.clips_named("Spin").count(), 2);
    assert!(matches!(
        import.clip("Spin"),
        Err(LookupError::AmbiguousClipName { ref name, ref matches })
            if name == "Spin" && matches.len() == 2
    ));

    let replacement = scene
        .replace_import(&import, &scene_asset)
        .expect("replacement succeeds");
    assert_ne!(
        replacement.clip("Pulse").expect("fresh clip exists").key(),
        pulse.key()
    );
    assert!(matches!(
        import.clip("Pulse"),
        Err(LookupError::StaleImport)
    ));
}

#[test]
fn gltf_required_punctual_lights_instantiate_as_scene_lights() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(
        "memory://lights.gltf",
        r#"{
            "asset": { "version": "2.0" },
            "extensionsUsed": ["KHR_lights_punctual"],
            "extensionsRequired": ["KHR_lights_punctual"],
            "extensions": {
                "KHR_lights_punctual": {
                    "lights": [
                        {
                            "name": "InspectionLamp",
                            "type": "point",
                            "color": [0.25, 0.5, 1.0],
                            "intensity": 42.0,
                            "range": 12.0
                        }
                    ]
                }
            },
            "nodes": [
                {
                    "name": "LampNode",
                    "extensions": {
                        "KHR_lights_punctual": { "light": 0 }
                    }
                }
            ]
        }"#,
    ));
    let scene_asset =
        pollster::block_on(assets.load_scene("memory://lights.gltf")).expect("light glTF loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("light scene instantiates");
    let node = import.node("LampNode").expect("light node lookup succeeds");

    let NodeKind::Light(light_key) = scene.node(node).expect("light node exists").kind() else {
        panic!("KHR_lights_punctual node should instantiate as a scene light");
    };
    let scena::Light::Point(point) = *scene.light(*light_key).expect("point light exists") else {
        panic!("test fixture declares a point light");
    };
    assert_eq!(point.color(), Color::from_linear_rgb(0.25, 0.5, 1.0));
    assert_eq!(point.intensity_candela(), 42.0);
    assert_eq!(point.range(), Some(12.0));
}

#[test]
fn gltf_required_texture_transform_and_mesh_quantization_are_realized() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(
        "memory://transform-quantized.gltf",
        r#"{
            "asset": { "version": "2.0" },
            "extensionsUsed": [
                "KHR_materials_unlit",
                "KHR_texture_transform",
                "KHR_mesh_quantization"
            ],
            "extensionsRequired": [
                "KHR_texture_transform",
                "KHR_mesh_quantization"
            ],
            "nodes": [
                { "name": "QuantizedTriangle", "mesh": 0 }
            ],
            "meshes": [
                {
                    "primitives": [
                        {
                            "attributes": { "POSITION": 0 },
                            "indices": 1,
                            "material": 0
                        }
                    ]
                }
            ],
            "materials": [
                {
                    "pbrMetallicRoughness": {
                        "baseColorTexture": {
                            "index": 0,
                            "extensions": {
                                "KHR_texture_transform": {
                                    "offset": [0.25, 0.5],
                                    "rotation": 1.5707964,
                                    "scale": [2.0, 3.0],
                                    "texCoord": 1
                                }
                            }
                        }
                    },
                    "extensions": { "KHR_materials_unlit": {} }
                }
            ],
            "textures": [
                { "source": 0 }
            ],
            "images": [
                { "uri": "textures/quantized.png" }
            ],
            "buffers": [
                {
                    "byteLength": 24,
                    "uri": "data:application/octet-stream;base64,AYABgAAA/38BgAAAAAD/fwAAAAABAAIA"
                }
            ],
            "bufferViews": [
                { "buffer": 0, "byteOffset": 0, "byteLength": 18 },
                { "buffer": 0, "byteOffset": 18, "byteLength": 6 }
            ],
            "accessors": [
                {
                    "bufferView": 0,
                    "componentType": 5122,
                    "count": 3,
                    "type": "VEC3",
                    "normalized": true
                },
                {
                    "bufferView": 1,
                    "componentType": 5123,
                    "count": 3,
                    "type": "SCALAR"
                }
            ]
        }"#,
    ));

    let scene_asset = pollster::block_on(assets.load_scene("memory://transform-quantized.gltf"))
        .expect("texture-transform and quantized glTF loads");
    let mesh = scene_asset.nodes()[0]
        .mesh()
        .expect("quantized mesh payload is registered");
    let geometry = assets
        .geometry(mesh.geometry())
        .expect("quantized geometry handle resolves");
    let material = assets
        .material(mesh.material())
        .expect("texture-transform material handle resolves");
    let transform = material
        .base_color_texture_transform()
        .expect("base-color texture transform is recorded");

    assert_eq!(transform.offset(), [0.25, 0.5]);
    assert_eq!(transform.scale(), [2.0, 3.0]);
    assert_eq!(transform.tex_coord(), Some(1));
    assert!((transform.rotation_radians() - 1.5707964).abs() <= 0.0001);
    assert_vec3_near(geometry.vertices()[0].position, Vec3::new(-1.0, -1.0, 0.0));
    assert_vec3_near(geometry.vertices()[1].position, Vec3::new(1.0, -1.0, 0.0));
    assert_vec3_near(geometry.vertices()[2].position, Vec3::new(0.0, 1.0, 0.0));
}

#[cfg(feature = "obj")]
#[test]
fn obj_feature_load_geometry_parses_triangle_faces() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(
        "memory://triangle.obj",
        r#"
            mtllib triangle.mtl
            v -0.5 -0.5 0.0
            v 0.5 -0.5 0.0
            v 0.0 0.5 0.0
            vn 0.0 0.0 1.0
            f 1//1 2//1 3//1
        "#,
    ));

    let geometry = pollster::block_on(assets.load_geometry("memory://triangle.obj"))
        .expect("OBJ geometry loads");
    let geometry = assets
        .geometry(geometry)
        .expect("OBJ geometry handle resolves");

    assert_eq!(geometry.topology(), GeometryTopology::Triangles);
    assert_eq!(geometry.vertices().len(), 3);
    assert_eq!(geometry.indices(), [0, 1, 2]);
    assert_vec3_near(geometry.bounds().min, Vec3::new(-0.5, -0.5, 0.0));
    assert_vec3_near(geometry.bounds().max, Vec3::new(0.5, 0.5, 0.0));
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
fn interaction_context_and_renderer_styles_are_explicit() {
    let mut scene = Scene::new();
    let node = scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::default(),
        )
        .expect("renderable node inserts");

    assert_eq!(scene.interaction().hover(), None);
    assert_eq!(scene.interaction().primary_selection(), None);
    scene
        .interaction_mut()
        .set_hover(Some(HitTarget::Node(node)));
    scene
        .interaction_mut()
        .set_primary_selection(Some(HitTarget::Node(node)));

    assert_eq!(scene.interaction().hover(), Some(HitTarget::Node(node)));
    assert_eq!(
        scene.interaction().primary_selection(),
        Some(HitTarget::Node(node))
    );

    let hover = InteractionStyle::outline(Color::from_linear_rgb(1.0, 0.8, 0.0), 3.0);
    let selection = InteractionStyle::outline(Color::from_linear_rgb(0.1, 0.4, 1.0), 4.0);
    let mut renderer = Renderer::headless(8, 8).expect("renderer builds");
    renderer.set_hover_style(hover);
    renderer.set_selection_style(selection);

    assert_eq!(renderer.hover_style(), hover);
    assert_eq!(renderer.selection_style(), selection);
}

#[test]
fn instance_sets_have_stable_ids_mutations_and_cpu_fallback() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(fullscreen_triangle_geometry());
    let material =
        assets.create_material(MaterialDesc::unlit(Color::from_linear_rgb(0.0, 1.0, 0.0)));
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::default(),
        )
        .expect("camera inserts");
    let set = scene
        .add_instance_set(scene.root(), geometry, material, Transform::default())
        .expect("instance set inserts");

    assert_eq!(
        scene
            .instance_set(set)
            .expect("instance set exists")
            .culling_policy(),
        InstanceCullingPolicy::CpuBoundingBoxFallback
    );

    scene
        .reserve_instances(set, 2)
        .expect("instance reserve succeeds");
    let first = scene
        .push_instance(
            set,
            Transform {
                translation: Vec3::new(-0.25, 0.0, 0.0),
                ..Transform::default()
            },
        )
        .expect("first instance inserts");
    let second = scene
        .push_instance(
            set,
            Transform {
                translation: Vec3::new(0.25, 0.0, 0.0),
                ..Transform::default()
            },
        )
        .expect("second instance inserts");

    assert_ne!(first, second);
    assert!(scene.instance_set(set).expect("set exists").contains(first));
    assert_eq!(
        scene
            .instance_set(set)
            .expect("set exists")
            .instances()
            .map(|instance| instance.id())
            .collect::<Vec<_>>(),
        vec![first, second]
    );

    let removed = scene
        .remove_instance(set, first)
        .expect("remove lookup succeeds")
        .expect("first instance is removed");
    assert_eq!(removed.id(), first);
    let third = scene
        .push_instance(set, Transform::default())
        .expect("third instance inserts");
    assert_ne!(third, first);
    assert_eq!(
        scene
            .instance_set(set)
            .expect("set exists")
            .instances()
            .map(|instance| instance.id())
            .collect::<Vec<_>>(),
        vec![second, third]
    );

    let mut renderer = Renderer::headless(8, 8).expect("renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("instanced scene prepares");
    let outcome = renderer
        .render(&scene, camera)
        .expect("instanced scene renders");
    assert_eq!(outcome.primitives, 2);
    assert_eq!(renderer.stats().draw_calls, 2);
    assert!(renderer.frame_rgba8().iter().any(|channel| *channel != 0));

    scene.clear_instances(set).expect("clear succeeds");
    assert!(scene.instance_set(set).expect("set exists").is_empty());
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

#[test]
fn offscreen_target_readback_is_explicit_and_owned() {
    let target = OffscreenTarget::new(4, 4).expect("offscreen target validates");
    let mut renderer = Renderer::offscreen(target).expect("offscreen renderer builds");
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::default(),
        )
        .expect("camera inserts");
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::default(),
        )
        .expect("renderable inserts");

    renderer.prepare(&mut scene).expect("scene prepares");
    renderer
        .render(&scene, camera)
        .expect("offscreen render succeeds");

    let readback = renderer.read_pixels();
    assert_eq!(readback.width(), 4);
    assert_eq!(readback.height(), 4);
    assert_eq!(readback.rgba8().len(), 4 * 4 * 4);
    assert_eq!(readback.rgba8(), renderer.frame_rgba8());
    assert!(readback.rgba8().iter().any(|channel| *channel != 0));
    assert!(matches!(
        OffscreenTarget::new(0, 4),
        Err(BuildError::InvalidTargetSize {
            width: 0,
            height: 4
        })
    ));
}

#[test]
fn labels_use_sdf_msdf_descriptors_and_billboard_render_path() {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::default(),
        )
        .expect("camera inserts");
    let label_desc = LabelDesc::sdf("Pump A")
        .with_color(Color::from_linear_rgb(0.0, 1.0, 0.0))
        .with_size(0.5)
        .with_billboard(LabelBillboard::ScreenAligned);
    let label = scene
        .add_label(scene.root(), label_desc.clone(), Transform::default())
        .expect("label inserts");

    assert_eq!(scene.label(label), Some(&label_desc));
    assert_eq!(label_desc.rasterization(), LabelRasterization::Sdf);
    assert_eq!(label_desc.billboard(), LabelBillboard::ScreenAligned);
    assert_eq!(
        LabelDesc::msdf("Pump B").rasterization(),
        LabelRasterization::Msdf
    );

    let mut renderer = Renderer::headless(8, 8).expect("renderer builds");
    renderer.prepare(&mut scene).expect("label scene prepares");
    let outcome = renderer.render(&scene, camera).expect("label renders");
    assert_eq!(outcome.primitives, 2);
    assert!(renderer.frame_rgba8().iter().any(|channel| *channel != 0));

    scene
        .set_label_text(label, "Pump A selected")
        .expect("label text mutates");
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

fn fullscreen_triangle_geometry() -> GeometryDesc {
    GeometryDesc::try_new(
        GeometryTopology::Triangles,
        vec![
            GeometryVertex {
                position: Vec3::new(-1.0, -1.0, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            GeometryVertex {
                position: Vec3::new(3.0, -1.0, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            GeometryVertex {
                position: Vec3::new(-1.0, 3.0, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
        ],
        vec![0, 1, 2],
    )
    .expect("fullscreen triangle geometry is valid")
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

#[derive(Clone)]
struct BinaryFetcher {
    path: AssetPath,
    bytes: Arc<Vec<u8>>,
    calls: Arc<AtomicUsize>,
}

impl BinaryFetcher {
    fn new(path: impl Into<AssetPath>, bytes: Vec<u8>) -> Self {
        Self {
            path: path.into(),
            bytes: Arc::new(bytes),
            calls: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl AssetFetcher for BinaryFetcher {
    type Future<'a> = Ready<Result<Vec<u8>, AssetError>>;

    fn fetch<'a>(&'a self, path: &'a AssetPath) -> Self::Future<'a> {
        if path == &self.path {
            self.calls.fetch_add(1, Ordering::SeqCst);
            ready(Ok((*self.bytes).clone()))
        } else {
            ready(Err(AssetError::NotFound {
                path: path.as_str().to_string(),
            }))
        }
    }
}

fn minimal_glb_triangle_scene() -> Vec<u8> {
    let mut bin = Vec::new();
    for value in [-0.5_f32, -0.5, 0.0, 0.5, -0.5, 0.0, 0.0, 0.5, 0.0] {
        bin.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0_u16, 1, 2] {
        bin.extend_from_slice(&value.to_le_bytes());
    }
    let buffer_byte_length = bin.len();
    pad_to_four(&mut bin, 0);

    let json = format!(
        r#"{{
            "asset": {{ "version": "2.0" }},
            "buffers": [{{ "byteLength": {buffer_byte_length} }}],
            "bufferViews": [
                {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
                {{ "buffer": 0, "byteOffset": 36, "byteLength": 6 }}
            ],
            "accessors": [
                {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
                {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }}
            ],
            "materials": [
                {{ "pbrMetallicRoughness": {{ "baseColorFactor": [0.2, 0.8, 0.1, 1.0] }} }}
            ],
            "meshes": [
                {{ "primitives": [{{ "attributes": {{ "POSITION": 0 }}, "indices": 1, "material": 0 }}] }}
            ],
            "nodes": [{{ "name": "GlbTriangle", "mesh": 0 }}]
        }}"#
    );
    let mut json = json.into_bytes();
    pad_to_four(&mut json, b' ');

    let length = 12 + 8 + json.len() + 8 + bin.len();
    let mut glb = Vec::with_capacity(length);
    glb.extend_from_slice(&0x4654_6C67_u32.to_le_bytes());
    glb.extend_from_slice(&2_u32.to_le_bytes());
    glb.extend_from_slice(&(length as u32).to_le_bytes());
    glb.extend_from_slice(&(json.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x4E4F_534A_u32.to_le_bytes());
    glb.extend_from_slice(&json);
    glb.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x004E_4942_u32.to_le_bytes());
    glb.extend_from_slice(&bin);
    glb
}

fn pad_to_four(bytes: &mut Vec<u8>, pad: u8) {
    while !bytes.len().is_multiple_of(4) {
        bytes.push(pad);
    }
}
