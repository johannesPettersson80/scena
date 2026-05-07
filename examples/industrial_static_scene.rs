use scena::{
    Assets, Color, GeometryDesc, MaterialDesc, PerspectiveCamera, Profile, Renderer,
    RendererOptions, Scene, Transform,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new();
    let floor = assets.create_geometry(GeometryDesc::grid(10.0, 20));
    let material =
        assets.create_material(MaterialDesc::line(Color::from_srgb_u8(110, 130, 150), 1.0));

    let mut scene = Scene::new();
    scene.mesh(floor, material).add()?;
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::default(),
        Transform::default(),
    )?;
    scene.set_active_camera(camera)?;

    let options = RendererOptions::default().with_profile(Profile::Industrial);
    let mut renderer = Renderer::headless_with_options(512, 256, options)?;
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    let skipped = renderer.render_active(&scene)?;
    println!("industrial_static_scene skipped={}", skipped.skipped);
    Ok(())
}
