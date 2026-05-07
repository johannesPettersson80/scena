use scena::{
    Assets, Color, GeometryDesc, MaterialDesc, PerspectiveCamera, Renderer, Scene, Transform,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new();
    let cube = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material = assets.create_material(MaterialDesc::pbr_metallic_roughness(
        Color::from_srgb_u8(90, 148, 255),
        0.0,
        0.55,
    ));

    let mut scene = Scene::new();
    scene.mesh(cube, material).add()?;
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::default(),
        Transform::default(),
    )?;
    scene.set_active_camera(camera)?;

    let mut renderer = Renderer::headless(320, 240)?;
    renderer.prepare_with_assets(&mut scene, &assets)?;
    let outcome = renderer.render_active(&scene)?;
    println!("primitive_shapes draw_calls={}", outcome.draw_calls);
    Ok(())
}
