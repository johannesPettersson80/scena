use scena::{Assets, Renderer, Scene};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let which = std::env::args().nth(1).unwrap_or_else(|| "assembly".into());
    let path = match which.as_str() {
        "drive" => "demo/samples/connector-snap/drive_unit.glb",
        _ => "demo/samples/connector-snap/connector_snap_assembly.glb",
    };
    let assets = Assets::new();
    eprintln!("loading {path}");
    let model = pollster::block_on(assets.load_scene(path))?;
    eprintln!("loaded");
    let mut scene = Scene::new();
    let import = scene.instantiate(&model)?;
    eprintln!("instantiated; bounds = {:?}", import.bounds_world(&scene));
    if std::env::var("NO_LIGHTS").is_err() {
        eprintln!("-> add_studio_lighting");
        scene.add_studio_lighting()?;
        eprintln!("   ok");
    } else {
        eprintln!("-> studio lighting SKIPPED");
    }
    let bounds = import.bounds_world(&scene).ok_or("no bounds")?;
    eprintln!("-> add_perspective_camera_default_for");
    let camera = scene.add_perspective_camera_default_for(bounds, (400, 300))?;
    eprintln!("   ok");
    scene.set_active_camera(camera)?;
    eprintln!("-> Renderer::headless");
    let mut renderer = Renderer::headless_gpu(400, 300)?;
    eprintln!("   ok");
    eprintln!("-> prepare_with_assets");
    renderer.prepare_with_assets(&mut scene, &assets)?;
    eprintln!("prepared");
    renderer.render_active(&scene)?;
    eprintln!("rendered OK");
    Ok(())
}
