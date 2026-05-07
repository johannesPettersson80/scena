use scena::{
    Assets, Color, GeometryDesc, MaterialDesc, PerspectiveCamera, Renderer, Scene, Transform, Vec3,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.2, 0.2, 0.2));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(70, 220, 160)));

    let mut scene = Scene::new();
    let set = scene.add_instance_set(scene.root(), geometry, material, Transform::default())?;
    scene.reserve_instances(set, 16)?;
    for index in 0..10 {
        scene.push_instance(
            set,
            Transform {
                translation: Vec3::new(index as f32 * 0.24 - 1.0, 0.0, 0.0),
                ..Transform::default()
            },
        )?;
    }
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::default(),
        Transform::default(),
    )?;
    scene.set_active_camera(camera)?;

    let mut renderer = Renderer::headless(320, 120)?;
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    println!(
        "instances={}",
        scene.instance_set(set).expect("set exists").len()
    );
    Ok(())
}
