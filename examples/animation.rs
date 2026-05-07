use scena::{AnimationPlaybackState, Assets, PerspectiveCamera, Renderer, Scene, Transform};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new();
    let scene_asset = pollster::block_on(
        assets.load_scene("tests/assets/gltf/khronos/MorphCube/AnimatedMorphCube.gltf"),
    )?;

    let mut scene = Scene::new();
    let import = scene.instantiate(&scene_asset)?;
    let mixer = scene.create_animation_mixer(&import, "Square")?;
    scene.play_animation(mixer)?;
    scene.update_animation(mixer, 1.0 / 60.0)?;

    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::default(),
        Transform::default(),
    )?;
    scene.set_active_camera(camera)?;
    let mut renderer = Renderer::headless(256, 256)?;
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    let state = scene.animation_mixer(mixer)?.state();
    assert_eq!(state, AnimationPlaybackState::Playing);
    println!("animation state={state:?}");
    Ok(())
}
