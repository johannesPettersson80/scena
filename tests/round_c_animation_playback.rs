#![cfg(not(target_arch = "wasm32"))]

use scena::{AnimationPlaybackState, Assets, Scene, Vec3, headless_gltf_viewer};

fn assert_vec3_changed(value: Vec3) {
    assert!(
        value.x.abs() > 0.0001 || value.y.abs() > 0.0001 || value.z.abs() > 0.0001,
        "expected animation to move the node, got {value:?}"
    );
}

#[test]
fn scene_play_animation_by_name_creates_and_starts_mixer() {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/animated_connector_scene.gltf"))
            .expect("animated fixture loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("animated fixture instantiates");
    let animated = import
        .node("AnimatedMount")
        .expect("animated node resolves");

    let mixer = scene
        .play_animation_by_name(&import, "MoveMount")
        .expect("named clip starts");
    assert_eq!(
        scene.animation_mixer(mixer).expect("mixer exists").state(),
        AnimationPlaybackState::Playing
    );

    scene
        .update_animation(mixer, 0.5)
        .expect("started mixer updates");
    assert_vec3_changed(
        scene
            .node(animated)
            .expect("animated node remains live")
            .transform()
            .translation,
    );
}

#[test]
fn headless_viewer_play_clip_starts_loaded_import_animation() {
    let mut viewer = pollster::block_on(
        headless_gltf_viewer("tests/assets/gltf/animated_connector_scene.gltf")
            .without_framing()
            .build(),
    )
    .expect("headless viewer builds from animated fixture");
    let animated = viewer
        .import()
        .node("AnimatedMount")
        .expect("animated node resolves");

    let mixer = viewer
        .play_clip("MoveMount")
        .expect("viewer starts named clip");
    assert_eq!(
        viewer
            .scene()
            .animation_mixer(mixer)
            .expect("mixer exists")
            .state(),
        AnimationPlaybackState::Playing
    );

    viewer
        .scene_mut()
        .update_animation(mixer, 0.5)
        .expect("viewer-owned animation updates");
    assert_vec3_changed(
        viewer
            .scene()
            .node(animated)
            .expect("animated node remains live")
            .transform()
            .translation,
    );
}
