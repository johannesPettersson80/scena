use scena::{Aabb, Assets, Color, GeometryDesc, MaterialDesc, Renderer, Scene, Vec3};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.2, 0.4, 0.4));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(70, 160, 240)));

    let mut scene = Scene::new();
    let inspected_part = scene.mesh(geometry, material).add()?;
    let camera = scene.add_default_camera()?;

    let bounds = Aabb::new(Vec3::new(-0.6, -0.2, -0.2), Vec3::new(0.6, 0.2, 0.2));
    scene.frame(camera, bounds)?;
    scene.look_at(camera, inspected_part)?;

    let mut renderer = Renderer::headless(320, 180)?;
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;

    let camera_node = scene.camera_node(camera).expect("camera node exists");
    let camera_z = scene
        .node(camera_node)
        .expect("camera node resolves")
        .transform()
        .translation
        .z;
    println!("camera_framing camera_z={camera_z:.2}");
    Ok(())
}
