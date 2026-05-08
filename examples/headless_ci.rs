use scena::{Assets, Color, GeometryDesc, MaterialDesc, Renderer, Scene};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.7, 0.45, 0.35));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(90, 148, 255)));

    let mut scene = Scene::new();
    scene.mesh(geometry, material).add()?;
    let camera = scene.add_default_camera()?;
    scene.frame_all_with_assets(camera, &assets)?;
    scene.set_active_camera(camera)?;

    let mut renderer = Renderer::headless(64, 64)?;
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    let center = &renderer.frame_rgba8()[((32 * 64 + 32) * 4)..((32 * 64 + 33) * 4)];
    println!("headless_ci center={center:?}");
    Ok(())
}
