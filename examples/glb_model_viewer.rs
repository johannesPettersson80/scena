use scena::{Assets, Renderer, Scene};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/minimal_scene.gltf"))?;

    let mut scene = Scene::new();
    let import = scene.instantiate(&scene_asset)?;
    let camera = scene.add_default_camera()?;
    scene.frame_import(camera, &import)?;

    let mut renderer = Renderer::headless(320, 240)?;
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    println!("glb_model_viewer roots={}", import.roots().len());
    Ok(())
}
