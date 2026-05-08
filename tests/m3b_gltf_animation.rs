#![cfg(not(target_arch = "wasm32"))]

use scena::{
    AnimationError, AnimationLoopMode, AnimationPlaybackState, AnimationTarget, AssetError,
    AssetFetcher, AssetPath, Assets, ChangeKind, NotPreparedReason, PerspectiveCamera, RenderError,
    Renderer, Scene, SourceUnits, Transform, Vec3,
};
use std::collections::BTreeMap;
use std::future::{Ready, ready};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[test]
fn mixer_controls_rebind_translation_channels_to_import_local_nodes() {
    let assets = Assets::with_fetcher(MultiMemoryFetcher::new(vec![
        (
            AssetPath::from("memory://models/animated-translation.gltf"),
            animated_translation_gltf().into_bytes(),
        ),
        (
            AssetPath::from("memory://models/animated.bin"),
            animated_translation_buffer(),
        ),
    ]));
    let scene_asset =
        pollster::block_on(assets.load_scene("memory://models/animated-translation.gltf"))
            .expect("animated glTF loads");
    let mut scene = Scene::new();
    scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::default(),
        )
        .expect("camera inserts");

    let first = scene
        .instantiate_with(
            &scene_asset,
            scena::ImportOptions::gltf_default().with_source_units(SourceUnits::Centimeters),
        )
        .expect("first animated import instantiates");
    let second = scene
        .instantiate_with(&scene_asset, scena::ImportOptions::gltf_default())
        .expect("second animated import instantiates");
    let first_target = first.node("Animated").expect("first target resolves");
    let second_target = second.node("Animated").expect("second target resolves");
    let first_clip = first.clip("MoveX").expect("first clip resolves");

    assert_eq!(first_clip.channels().len(), 1);
    assert_eq!(first_clip.channels()[0].target_node(), first_target);
    assert_eq!(
        first_clip.channels()[0].target(),
        AnimationTarget::Translation
    );
    assert_eq!(first_clip.duration_seconds(), 1.0);

    let first_mixer = scene
        .create_animation_mixer(&first, "MoveX")
        .expect("first mixer creates");
    let second_mixer = scene
        .create_animation_mixer(&second, "MoveX")
        .expect("second mixer creates");

    scene
        .play_animation(first_mixer)
        .expect("first mixer starts");
    scene
        .set_animation_speed(first_mixer, 2.0)
        .expect("speed updates");
    scene
        .set_animation_loop_mode(first_mixer, AnimationLoopMode::Repeat)
        .expect("loop mode updates");
    scene
        .update_animation(first_mixer, 0.25)
        .expect("first mixer updates by delta");

    assert_eq!(
        scene
            .animation_mixer(first_mixer)
            .expect("mixer exists")
            .state(),
        AnimationPlaybackState::Playing
    );
    assert_vec3_near(
        scene
            .node(first_target)
            .expect("first animated node exists")
            .transform()
            .translation,
        Vec3::new(0.005, 0.0, 0.0),
    );
    assert_vec3_near(
        scene
            .node(second_target)
            .expect("second animated node exists")
            .transform()
            .translation,
        Vec3::ZERO,
    );

    scene
        .pause_animation(first_mixer)
        .expect("first mixer pauses");
    scene
        .update_animation(first_mixer, 0.25)
        .expect("paused update is accepted");
    assert_vec3_near(
        scene
            .node(first_target)
            .expect("first animated node exists")
            .transform()
            .translation,
        Vec3::new(0.005, 0.0, 0.0),
    );

    scene
        .set_animation_loop_mode(first_mixer, AnimationLoopMode::Once)
        .expect("loop mode switches back to clamped playback");
    scene
        .seek_animation(first_mixer, 1.0)
        .expect("seek samples while paused");
    assert_vec3_near(
        scene
            .node(first_target)
            .expect("first animated node exists")
            .transform()
            .translation,
        Vec3::new(0.01, 0.0, 0.0),
    );

    scene.stop_animation(first_mixer).expect("stop resets pose");
    assert_eq!(
        scene
            .animation_mixer(first_mixer)
            .expect("mixer exists")
            .state(),
        AnimationPlaybackState::Stopped
    );
    assert_vec3_near(
        scene
            .node(first_target)
            .expect("first animated node exists")
            .transform()
            .translation,
        Vec3::ZERO,
    );

    assert_ne!(first_mixer, second_mixer);
}

#[test]
fn playing_paused_and_seek_animation_dirty_prepared_render_state() {
    let assets = animated_translation_assets();
    let scene_asset =
        pollster::block_on(assets.load_scene("memory://models/animated-translation.gltf"))
            .expect("animated glTF loads");
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
        .expect("animated import instantiates");
    let mixer = scene
        .create_animation_mixer(&import, "MoveX")
        .expect("mixer creates");
    let mut renderer = Renderer::headless(8, 8).expect("renderer builds");

    renderer.prepare(&mut scene).expect("scene prepares");
    scene.play_animation(mixer).expect("mixer starts");
    scene
        .update_animation(mixer, 0.5)
        .expect("playing mixer updates");
    assert_scene_changed(&mut renderer, &scene, camera);

    renderer.prepare(&mut scene).expect("scene re-prepares");
    scene.pause_animation(mixer).expect("mixer pauses");
    scene
        .update_animation(mixer, 0.25)
        .expect("paused update is accepted");
    renderer
        .render(&scene, camera)
        .expect("paused update does not dirty a prepared frame");

    scene
        .seek_animation(mixer, 1.0)
        .expect("paused seek samples once");
    assert_scene_changed(&mut renderer, &scene, camera);

    renderer
        .prepare(&mut scene)
        .expect("scene re-prepares after seek");
    scene.stop_animation(mixer).expect("stop resets pose once");
    assert_scene_changed(&mut renderer, &scene, camera);
}

#[test]
fn replace_import_invalidates_animation_mixers_with_stale_error() {
    let assets = animated_translation_assets();
    let scene_asset =
        pollster::block_on(assets.load_scene("memory://models/animated-translation.gltf"))
            .expect("animated glTF loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("animated import instantiates");
    let mixer = scene
        .create_animation_mixer(&import, "MoveX")
        .expect("mixer creates");

    let replacement = scene
        .replace_import(&import, &scene_asset)
        .expect("replacement succeeds");
    assert!(replacement.clip("MoveX").is_ok());
    assert_eq!(
        scene.play_animation(mixer),
        Err(AnimationError::StaleMixer(mixer))
    );
}

#[test]
fn replacement_import_rebinds_stable_animation_clip_names() {
    let assets = animated_translation_assets();
    let scene_asset =
        pollster::block_on(assets.load_scene("memory://models/animated-translation.gltf"))
            .expect("animated glTF loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("animated import instantiates");
    let previous_clip = import.clip("MoveX").expect("clip resolves").clone();
    let previous_target = previous_clip.channels()[0].target_node();

    let replacement = scene
        .replace_import(&import, &scene_asset)
        .expect("replacement succeeds");
    let replacement_clip = replacement
        .replacement_clip(&previous_clip)
        .expect("stable clip name rebinds after replacement");

    assert_eq!(replacement_clip.name(), Some("MoveX"));
    assert_ne!(replacement_clip.key(), previous_clip.key());
    assert_ne!(
        replacement_clip.channels()[0].target_node(),
        previous_target,
        "replacement clip must target replacement import-local nodes, not stale nodes"
    );
}

#[test]
fn gltf_animation_supports_rotation_scale_weights_and_normalizes_quaternions() {
    let assets = Assets::with_fetcher(MultiMemoryFetcher::new(vec![
        (
            AssetPath::from("memory://models/animated-targets.gltf"),
            animated_targets_gltf().into_bytes(),
        ),
        (
            AssetPath::from("memory://models/targets.bin"),
            animated_targets_buffer(),
        ),
    ]));
    let scene_asset =
        pollster::block_on(assets.load_scene("memory://models/animated-targets.gltf"))
            .expect("animated-targets glTF loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("animated-targets import instantiates");
    let rotating = import.node("Rotating").expect("rotating node resolves");
    let scaling = import.node("Scaling").expect("scaling node resolves");
    let weighted = import.node("Weighted").expect("weighted node resolves");
    let clip = import.clip("Targets").expect("targets clip resolves");

    assert_eq!(
        clip.channels()
            .iter()
            .map(|channel| (channel.target_node(), channel.target()))
            .collect::<Vec<_>>(),
        vec![
            (rotating, AnimationTarget::Rotation),
            (scaling, AnimationTarget::Scale),
            (weighted, AnimationTarget::Weights),
        ]
    );

    let mixer = scene
        .create_animation_mixer(&import, "Targets")
        .expect("targets mixer creates");
    scene
        .seek_animation(mixer, 1.0)
        .expect("seek samples target channels");

    let rotation = scene
        .node(rotating)
        .expect("rotating node exists")
        .transform()
        .rotation;
    let rotation_len = (rotation.x * rotation.x
        + rotation.y * rotation.y
        + rotation.z * rotation.z
        + rotation.w * rotation.w)
        .sqrt();
    assert!(
        (rotation_len - 1.0).abs() <= 0.0001,
        "sampled quaternion output must be normalized"
    );
    assert_vec3_near(
        scene
            .node(scaling)
            .expect("scaling node exists")
            .transform()
            .scale,
        Vec3::new(2.0, 3.0, 4.0),
    );
}

#[test]
fn morph_target_weights_channel_updates_scene_morph_weights() {
    let assets = Assets::with_fetcher(MultiMemoryFetcher::new(vec![
        (
            AssetPath::from("memory://models/morph-weight.gltf"),
            morph_weight_gltf().into_bytes(),
        ),
        (
            AssetPath::from("memory://models/morph.bin"),
            morph_weight_buffer(),
        ),
    ]));
    let scene_asset = pollster::block_on(assets.load_scene("memory://models/morph-weight.gltf"))
        .expect("morph-weight glTF loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("morph-weight import instantiates");
    let morphing = import.node("Morphing").expect("morphing node resolves");
    let mixer = scene
        .create_animation_mixer(&import, "MorphWeight")
        .expect("morph mixer creates");

    assert_eq!(
        scene.morph_weights(morphing).expect("morph weights exist"),
        &[0.0]
    );
    scene
        .seek_animation(mixer, 1.0)
        .expect("morph weight seek samples");
    assert_eq!(
        scene.morph_weights(morphing).expect("morph weights update"),
        &[1.0]
    );

    let mesh = scene_asset.nodes()[0]
        .mesh()
        .expect("morph fixture has mesh");
    let geometry = assets
        .geometry(mesh.geometry())
        .expect("morph geometry resolves");
    let morphed = geometry
        .morphed_vertices(scene.morph_weights(morphing).expect("weights exist"))
        .expect("morph target applies");
    assert_vec3_near(morphed[2].position, Vec3::new(0.0, 1.0, 0.5));
}

#[test]
fn skinning_rebinds_joints_and_deforms_vertices_from_skeleton_hierarchy() {
    let assets = Assets::with_fetcher(MultiMemoryFetcher::new(vec![
        (
            AssetPath::from("memory://models/skinned-hierarchy.gltf"),
            skinned_hierarchy_gltf().into_bytes(),
        ),
        (
            AssetPath::from("memory://models/skinned.bin"),
            skinned_hierarchy_buffer(),
        ),
    ]));
    let scene_asset =
        pollster::block_on(assets.load_scene("memory://models/skinned-hierarchy.gltf"))
            .expect("skinned glTF loads");
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
        .expect("skinned import instantiates");
    let mesh_node = import.node("SkinnedMesh").expect("mesh node resolves");
    let joint_node = import.node("Joint").expect("joint node resolves");
    let mixer = scene
        .create_animation_mixer(&import, "JointLift")
        .expect("joint animation mixer creates");

    assert_eq!(
        scene
            .skin_binding(mesh_node)
            .expect("skin binding exists")
            .joints(),
        &[joint_node]
    );

    let mesh = scene_asset.nodes()[0]
        .mesh()
        .expect("skinned fixture has mesh");
    let geometry = assets
        .geometry(mesh.geometry())
        .expect("skinned geometry resolves");
    let initial_skin_matrices = scene
        .skin_matrices(mesh_node)
        .expect("skin matrices resolve");
    let initially_skinned = geometry
        .skinned_vertices(geometry.vertices(), &initial_skin_matrices)
        .expect("skinning succeeds")
        .expect("skin influences deform vertices");
    assert_vec3_near(initially_skinned[2].position, Vec3::new(0.0, 0.5, 0.0));

    scene
        .seek_animation(mixer, 1.0)
        .expect("joint animation samples");
    let lifted_skin_matrices = scene
        .skin_matrices(mesh_node)
        .expect("updated skin matrices resolve");
    let lifted_vertices = geometry
        .skinned_vertices(geometry.vertices(), &lifted_skin_matrices)
        .expect("updated skinning succeeds")
        .expect("updated joint transforms deform vertices");
    assert_vec3_near(lifted_vertices[2].position, Vec3::new(0.0, 1.0, 0.0));

    let mut renderer = Renderer::headless(16, 16).expect("renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("skinned mesh prepares through render path");
    renderer
        .render(&scene, camera)
        .expect("skinned mesh renders");
    assert!(renderer.frame_rgba8().iter().any(|channel| *channel != 0));
}

#[test]
fn combined_morph_and_skinning_deforms_morphed_vertices_through_joint_matrices() {
    let geometry = scena::GeometryDesc::try_new(
        scena::GeometryTopology::Triangles,
        vec![
            scena::GeometryVertex {
                position: Vec3::new(-0.5, -0.5, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            scena::GeometryVertex {
                position: Vec3::new(0.5, -0.5, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            scena::GeometryVertex {
                position: Vec3::ZERO,
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
        ],
        vec![0, 1, 2],
    )
    .expect("base geometry validates")
    .with_morph_targets(vec![scena::GeometryMorphTarget::new(vec![
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::new(0.0, 0.0, 0.5),
    ])])
    .expect("morph target validates")
    .with_skin(scena::GeometrySkin::new(
        vec![[0, 0, 0, 0]; 3],
        vec![[1.0, 0.0, 0.0, 0.0]; 3],
    ))
    .expect("skin influences validate");
    let morphed = geometry
        .morphed_vertices(&[1.0])
        .expect("morph target applies before skinning");
    let skinned = geometry
        .skinned_vertices(
            &morphed,
            &[scena::SkinningMatrix::from_transform(Transform {
                translation: Vec3::new(0.0, 0.5, 0.0),
                ..Transform::default()
            })],
        )
        .expect("combined skinning succeeds")
        .expect("skin influences apply after morphing");

    assert_vec3_near(skinned[2].position, Vec3::new(0.0, 0.5, 0.5));
}

#[test]
fn interpolation_handles_step_cubic_spline_and_quaternion_slerp() {
    let assets = Assets::with_fetcher(MultiMemoryFetcher::new(vec![
        (
            AssetPath::from("memory://models/interpolation.gltf"),
            interpolation_gltf().into_bytes(),
        ),
        (
            AssetPath::from("memory://models/interpolation.bin"),
            interpolation_buffer(),
        ),
    ]));
    let scene_asset = pollster::block_on(assets.load_scene("memory://models/interpolation.gltf"))
        .expect("interpolation glTF loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("interpolation import instantiates");
    let cubic = import.node("Cubic").expect("cubic node resolves");
    let slerp = import.node("Slerp").expect("slerp node resolves");
    let stepped = import.node("Stepped").expect("stepped node resolves");
    let mixer = scene
        .create_animation_mixer(&import, "Interpolation")
        .expect("interpolation mixer creates");

    scene
        .seek_animation(mixer, 0.5)
        .expect("interpolation seek samples");

    assert_vec3_near(
        scene
            .node(cubic)
            .expect("cubic node exists")
            .transform()
            .translation,
        Vec3::new(1.0, 0.0, 0.0),
    );
    let rotation = scene
        .node(slerp)
        .expect("slerp node exists")
        .transform()
        .rotation;
    assert!(
        (rotation.z - std::f32::consts::FRAC_1_SQRT_2).abs() <= 0.0001
            && (rotation.w - std::f32::consts::FRAC_1_SQRT_2).abs() <= 0.0001,
        "linear quaternion interpolation must slerp and normalize: {rotation:?}"
    );
    assert_vec3_near(
        scene
            .node(stepped)
            .expect("stepped node exists")
            .transform()
            .scale,
        Vec3::ONE,
    );
}

#[test]
fn khronos_sample_assets_load_instantiate_and_cover_animation_contracts() {
    let assets = Assets::new();
    for sample in [
        KhronosSample {
            name: "RiggedSimple",
            path: "tests/assets/gltf/khronos/RiggedSimple/RiggedSimple.gltf",
            requires_skin: true,
            requires_morph: false,
        },
        KhronosSample {
            name: "SimpleSkin",
            path: "tests/assets/gltf/khronos/SimpleSkin/SimpleSkin.gltf",
            requires_skin: true,
            requires_morph: false,
        },
        KhronosSample {
            name: "SimpleMorph",
            path: "tests/assets/gltf/khronos/SimpleMorph/SimpleMorph.gltf",
            requires_skin: false,
            requires_morph: true,
        },
        KhronosSample {
            name: "MorphCube",
            path: "tests/assets/gltf/khronos/MorphCube/AnimatedMorphCube.gltf",
            requires_skin: false,
            requires_morph: true,
        },
        KhronosSample {
            name: "RiggedFigure",
            path: "tests/assets/gltf/khronos/RiggedFigure/RiggedFigure.gltf",
            requires_skin: true,
            requires_morph: false,
        },
        KhronosSample {
            name: "BrainStem",
            path: "tests/assets/gltf/khronos/BrainStem/BrainStem.gltf",
            requires_skin: true,
            requires_morph: false,
        },
    ] {
        let scene_asset = pollster::block_on(assets.load_scene(sample.path))
            .unwrap_or_else(|error| panic!("{} must load: {error}", sample.name));
        assert!(
            scene_asset.node_count() > 0,
            "{} should contain nodes",
            sample.name
        );
        assert!(
            scene_asset.mesh_count() > 0,
            "{} should contain meshes",
            sample.name
        );
        if sample.requires_skin {
            assert!(
                !scene_asset.skins().is_empty(),
                "{} should expose glTF skins",
                sample.name
            );
        }
        if sample.requires_morph {
            assert!(
                scene_asset
                    .nodes()
                    .iter()
                    .flat_map(|node| node.meshes())
                    .any(|mesh| {
                        !assets
                            .geometry(mesh.geometry())
                            .expect("sample geometry resolves")
                            .morph_targets()
                            .is_empty()
                    }),
                "{} should expose morph targets",
                sample.name
            );
        }
        let mut scene = Scene::new();
        let import = scene
            .instantiate(&scene_asset)
            .unwrap_or_else(|error| panic!("{} must instantiate: {error}", sample.name));
        if let Some(named_clip) = scene_asset.clips().iter().find_map(|clip| clip.name()) {
            let mixer = scene
                .create_animation_mixer(&import, named_clip)
                .unwrap_or_else(|error| panic!("{} mixer must create: {error}", sample.name));
            scene
                .seek_animation(mixer, scene_asset.clips()[0].duration_seconds() * 0.5)
                .unwrap_or_else(|error| panic!("{} mixer must seek: {error}", sample.name));
        }
    }

    let rigged_figure = pollster::block_on(
        assets.load_scene("tests/assets/gltf/khronos/RiggedFigure/RiggedFigure.gltf"),
    )
    .expect("RiggedFigure reloads from cache");
    let z_up = rigged_figure
        .nodes()
        .iter()
        .find(|node| node.name() == Some("Z_UP"))
        .expect("RiggedFigure exposes Z_UP root");
    assert_ne!(
        z_up.transform(),
        Transform::IDENTITY,
        "glTF node matrix transforms must not be silently ignored"
    );
}

#[test]
fn steady_animation_update_reprepare_keeps_resource_counts_stable() {
    let assets = Assets::with_fetcher(MultiMemoryFetcher::new(vec![
        (
            AssetPath::from("memory://models/skinned-hierarchy.gltf"),
            skinned_hierarchy_gltf().into_bytes(),
        ),
        (
            AssetPath::from("memory://models/skinned.bin"),
            skinned_hierarchy_buffer(),
        ),
    ]));
    let scene_asset =
        pollster::block_on(assets.load_scene("memory://models/skinned-hierarchy.gltf"))
            .expect("skinned glTF loads");
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
        .expect("skinned import instantiates");
    let mixer = scene
        .create_animation_mixer(&import, "JointLift")
        .expect("mixer creates");
    let mut renderer = Renderer::headless(16, 16).expect("renderer builds");

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("initial prepare succeeds");
    renderer
        .render(&scene, camera)
        .expect("initial render succeeds");
    let prepared = renderer.stats();

    scene.play_animation(mixer).expect("mixer starts");
    scene.update_animation(mixer, 0.25).expect("mixer updates");
    assert_scene_changed(&mut renderer, &scene, camera);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("animated reprepare succeeds");
    let animated = renderer.stats();

    assert_eq!(animated.buffers, prepared.buffers);
    assert_eq!(animated.textures, prepared.textures);
    assert_eq!(animated.materials, prepared.materials);
    assert_eq!(animated.live_logical_handles, prepared.live_logical_handles);
    assert_eq!(animated.pending_destructions, prepared.pending_destructions);
}

fn animated_translation_assets() -> Assets<MultiMemoryFetcher> {
    Assets::with_fetcher(MultiMemoryFetcher::new(vec![
        (
            AssetPath::from("memory://models/animated-translation.gltf"),
            animated_translation_gltf().into_bytes(),
        ),
        (
            AssetPath::from("memory://models/animated.bin"),
            animated_translation_buffer(),
        ),
    ]))
}

fn morph_weight_gltf() -> String {
    r#"{
        "asset": { "version": "2.0" },
        "nodes": [
            { "name": "Morphing", "mesh": 0 }
        ],
        "meshes": [
            {
                "weights": [0.0],
                "primitives": [
                    {
                        "attributes": { "POSITION": 0 },
                        "indices": 1,
                        "targets": [
                            { "POSITION": 2 }
                        ]
                    }
                ]
            }
        ],
        "animations": [
            {
                "name": "MorphWeight",
                "samplers": [
                    { "input": 3, "output": 4, "interpolation": "LINEAR" }
                ],
                "channels": [
                    { "sampler": 0, "target": { "node": 0, "path": "weights" } }
                ]
            }
        ],
        "buffers": [
            { "byteLength": 94, "uri": "morph.bin" }
        ],
        "bufferViews": [
            { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
            { "buffer": 0, "byteOffset": 36, "byteLength": 6 },
            { "buffer": 0, "byteOffset": 42, "byteLength": 36 },
            { "buffer": 0, "byteOffset": 78, "byteLength": 8 },
            { "buffer": 0, "byteOffset": 86, "byteLength": 8 }
        ],
        "accessors": [
            { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" },
            { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" },
            { "bufferView": 2, "componentType": 5126, "count": 3, "type": "VEC3" },
            { "bufferView": 3, "componentType": 5126, "count": 2, "type": "SCALAR" },
            { "bufferView": 4, "componentType": 5126, "count": 2, "type": "SCALAR" }
        ]
    }"#
    .to_string()
}

fn morph_weight_buffer() -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [
        -0.5_f32, -0.5, 0.0, 0.5, -0.5, 0.0, 0.0, 0.5, 0.0, // base positions
    ] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0_u16, 1, 2] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in [
        0.0_f32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.5, 0.5, // morph deltas
        0.0, 1.0, // input times
        0.0, 1.0, // output weights
    ] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

#[derive(Debug, Clone, Copy)]
struct KhronosSample {
    name: &'static str,
    path: &'static str,
    requires_skin: bool,
    requires_morph: bool,
}

fn interpolation_gltf() -> String {
    r#"{
        "asset": { "version": "2.0" },
        "nodes": [
            { "name": "Cubic" },
            { "name": "Slerp" },
            { "name": "Stepped" }
        ],
        "animations": [
            {
                "name": "Interpolation",
                "samplers": [
                    { "input": 0, "output": 1, "interpolation": "CUBICSPLINE" },
                    { "input": 0, "output": 2, "interpolation": "LINEAR" },
                    { "input": 0, "output": 3, "interpolation": "STEP" }
                ],
                "channels": [
                    { "sampler": 0, "target": { "node": 0, "path": "translation" } },
                    { "sampler": 1, "target": { "node": 1, "path": "rotation" } },
                    { "sampler": 2, "target": { "node": 2, "path": "scale" } }
                ]
            }
        ],
        "buffers": [
            { "byteLength": 136, "uri": "interpolation.bin" }
        ],
        "bufferViews": [
            { "buffer": 0, "byteOffset": 0, "byteLength": 8 },
            { "buffer": 0, "byteOffset": 8, "byteLength": 72 },
            { "buffer": 0, "byteOffset": 80, "byteLength": 32 },
            { "buffer": 0, "byteOffset": 112, "byteLength": 24 }
        ],
        "accessors": [
            { "bufferView": 0, "componentType": 5126, "count": 2, "type": "SCALAR" },
            { "bufferView": 1, "componentType": 5126, "count": 6, "type": "VEC3" },
            { "bufferView": 2, "componentType": 5126, "count": 2, "type": "VEC4" },
            { "bufferView": 3, "componentType": 5126, "count": 2, "type": "VEC3" }
        ]
    }"#
    .to_string()
}

fn interpolation_buffer() -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [
        0.0_f32, 1.0, // input times
        0.0, 0.0, 0.0, // key 0 in tangent
        0.0, 0.0, 0.0, // key 0 value
        4.0, 0.0, 0.0, // key 0 out tangent
        0.0, 0.0, 0.0, // key 1 in tangent
        1.0, 0.0, 0.0, // key 1 value
        0.0, 0.0, 0.0, // key 1 out tangent
        0.0, 0.0, 0.0, 1.0, // first rotation
        0.0, 0.0, 1.0, 0.0, // second rotation
        1.0, 1.0, 1.0, // first scale
        2.0, 2.0, 2.0, // second scale
    ] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn skinned_hierarchy_gltf() -> String {
    r#"{
        "asset": { "version": "2.0" },
        "extensionsUsed": ["KHR_materials_unlit"],
        "nodes": [
            { "name": "SkinnedMesh", "mesh": 0, "skin": 0 },
            { "name": "SkeletonRoot", "children": [2], "translation": [0.0, 0.25, 0.0] },
            { "name": "Joint", "translation": [0.0, 0.25, 0.0] }
        ],
        "skins": [
            { "joints": [2], "inverseBindMatrices": 4 }
        ],
        "materials": [
            {
                "extensions": { "KHR_materials_unlit": {} },
                "pbrMetallicRoughness": {
                    "baseColorFactor": [1.0, 1.0, 1.0, 1.0]
                }
            }
        ],
        "meshes": [
            {
                "primitives": [
                    {
                        "attributes": {
                            "POSITION": 0,
                            "JOINTS_0": 2,
                            "WEIGHTS_0": 3
                        },
                        "indices": 1,
                        "material": 0
                    }
                ]
            }
        ],
        "animations": [
            {
                "name": "JointLift",
                "samplers": [
                    { "input": 5, "output": 6, "interpolation": "LINEAR" }
                ],
                "channels": [
                    { "sampler": 0, "target": { "node": 2, "path": "translation" } }
                ]
            }
        ],
        "buffers": [
            { "byteLength": 210, "uri": "skinned.bin" }
        ],
        "bufferViews": [
            { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
            { "buffer": 0, "byteOffset": 36, "byteLength": 6 },
            { "buffer": 0, "byteOffset": 42, "byteLength": 24 },
            { "buffer": 0, "byteOffset": 66, "byteLength": 48 },
            { "buffer": 0, "byteOffset": 114, "byteLength": 64 },
            { "buffer": 0, "byteOffset": 178, "byteLength": 8 },
            { "buffer": 0, "byteOffset": 186, "byteLength": 24 }
        ],
        "accessors": [
            { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" },
            { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" },
            { "bufferView": 2, "componentType": 5123, "count": 3, "type": "VEC4" },
            { "bufferView": 3, "componentType": 5126, "count": 3, "type": "VEC4" },
            { "bufferView": 4, "componentType": 5126, "count": 1, "type": "MAT4" },
            { "bufferView": 5, "componentType": 5126, "count": 2, "type": "SCALAR" },
            { "bufferView": 6, "componentType": 5126, "count": 2, "type": "VEC3" }
        ]
    }"#
    .to_string()
}

fn skinned_hierarchy_buffer() -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [
        -0.5_f32, -0.5, 0.0, 0.5, -0.5, 0.0, 0.0, 0.0, 0.0, // positions
    ] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0_u16, 1, 2] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for joints in [[0_u16, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]] {
        for value in joints {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    for weights in [
        [1.0_f32, 0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0, 0.0],
    ] {
        for value in weights {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    for value in [
        1.0_f32, 0.0, 0.0, 0.0, // inverse bind matrix, column 0
        0.0, 1.0, 0.0, 0.0, // column 1
        0.0, 0.0, 1.0, 0.0, // column 2
        0.0, 0.0, 0.0, 1.0, // column 3
        0.0, 1.0, // input times
        0.0, 0.25, 0.0, // first joint translation
        0.0, 0.75, 0.0, // second joint translation
    ] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn animated_targets_gltf() -> String {
    r#"{
        "asset": { "version": "2.0" },
        "nodes": [
            { "name": "Root", "children": [1, 2, 3] },
            { "name": "Rotating" },
            { "name": "Scaling" },
            { "name": "Weighted" }
        ],
        "animations": [
            {
                "name": "Targets",
                "samplers": [
                    { "input": 0, "output": 1, "interpolation": "CUBICSPLINE" },
                    { "input": 0, "output": 2, "interpolation": "STEP" },
                    { "input": 0, "output": 3, "interpolation": "LINEAR" }
                ],
                "channels": [
                    { "sampler": 0, "target": { "node": 1, "path": "rotation" } },
                    { "sampler": 1, "target": { "node": 2, "path": "scale" } },
                    { "sampler": 2, "target": { "node": 3, "path": "weights" } }
                ]
            }
        ],
        "buffers": [
            { "byteLength": 72, "uri": "targets.bin" }
        ],
        "bufferViews": [
            { "buffer": 0, "byteOffset": 0, "byteLength": 8 },
            { "buffer": 0, "byteOffset": 8, "byteLength": 32 },
            { "buffer": 0, "byteOffset": 40, "byteLength": 24 },
            { "buffer": 0, "byteOffset": 64, "byteLength": 8 }
        ],
        "accessors": [
            { "bufferView": 0, "componentType": 5126, "count": 2, "type": "SCALAR" },
            { "bufferView": 1, "componentType": 5126, "count": 2, "type": "VEC4" },
            { "bufferView": 2, "componentType": 5126, "count": 2, "type": "VEC3" },
            { "bufferView": 3, "componentType": 5126, "count": 2, "type": "SCALAR" }
        ]
    }"#
    .to_string()
}

fn animated_targets_buffer() -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [
        0.0_f32, 1.0, // input times
        0.0, 0.0, 0.0, 1.0, // first rotation
        0.0, 0.0, 2.0, 0.0, // second rotation deliberately non-normalized
        1.0, 1.0, 1.0, // first scale
        2.0, 3.0, 4.0, // second scale
        0.0, 1.0, // weights
    ] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn animated_translation_gltf() -> String {
    r#"{
        "asset": { "version": "2.0" },
        "nodes": [
            { "name": "Root", "children": [1] },
            { "name": "Animated" }
        ],
        "animations": [
            {
                "name": "MoveX",
                "samplers": [
                    {
                        "input": 0,
                        "output": 1,
                        "interpolation": "LINEAR"
                    }
                ],
                "channels": [
                    {
                        "sampler": 0,
                        "target": { "node": 1, "path": "translation" }
                    }
                ]
            }
        ],
        "buffers": [
            { "byteLength": 32, "uri": "animated.bin" }
        ],
        "bufferViews": [
            { "buffer": 0, "byteOffset": 0, "byteLength": 8 },
            { "buffer": 0, "byteOffset": 8, "byteLength": 24 }
        ],
        "accessors": [
            { "bufferView": 0, "componentType": 5126, "count": 2, "type": "SCALAR" },
            { "bufferView": 1, "componentType": 5126, "count": 2, "type": "VEC3" }
        ]
    }"#
    .to_string()
}

fn animated_translation_buffer() -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [0.0_f32, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
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

fn assert_scene_changed(renderer: &mut Renderer, scene: &Scene, camera: scena::CameraKey) {
    assert!(matches!(
        renderer.render(scene, camera),
        Err(RenderError::NotPrepared {
            reason: NotPreparedReason::SceneChanged {
                change: ChangeKind::SceneStructure,
                ..
            },
        })
    ));
}

#[derive(Clone)]
struct MultiMemoryFetcher {
    sources: Arc<BTreeMap<AssetPath, Vec<u8>>>,
    calls: Arc<AtomicUsize>,
}

impl MultiMemoryFetcher {
    fn new(entries: Vec<(AssetPath, Vec<u8>)>) -> Self {
        Self {
            sources: Arc::new(entries.into_iter().collect()),
            calls: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl AssetFetcher for MultiMemoryFetcher {
    type Future<'a> = Ready<Result<Vec<u8>, AssetError>>;

    fn fetch<'a>(&'a self, path: &'a AssetPath) -> Self::Future<'a> {
        if let Some(bytes) = self.sources.get(path) {
            self.calls.fetch_add(1, Ordering::SeqCst);
            ready(Ok(bytes.clone()))
        } else {
            ready(Err(AssetError::NotFound {
                path: path.as_str().to_string(),
            }))
        }
    }
}
