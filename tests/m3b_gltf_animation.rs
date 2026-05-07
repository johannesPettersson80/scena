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
