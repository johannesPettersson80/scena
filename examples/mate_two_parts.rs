use scena::{Assets, Renderer, Scene};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new();
    let drive_part = pollster::block_on(assets.load_scene("tests/assets/gltf/drive_unit.glb"))?;
    let load_part = pollster::block_on(assets.load_scene("tests/assets/gltf/load_unit.glb"))?;

    let mut scene = Scene::new();
    let drive = scene.instantiate(&drive_part)?;
    let load = scene.instantiate(&load_part)?;

    scene.mate(&drive, "shaft", &load, "hub")?;

    let camera = scene.add_default_camera()?;
    scene.frame_import(camera, &drive)?;

    let mut renderer = Renderer::headless(640, 360)?;
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;

    Ok(())
}
