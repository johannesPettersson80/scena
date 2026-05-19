#![cfg(not(target_arch = "wasm32"))]

use scena::{AnimationPlaybackState, Assets, Scene, Vec3};

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
