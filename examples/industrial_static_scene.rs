use scena::{
    Assets, Color, GeometryDesc, LabelDesc, MaterialDesc, Profile, Renderer, RendererOptions,
    Scene, Transform, Vec3,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new();
    let floor = assets.create_geometry(GeometryDesc::grid(3.0, 12));
    let body = assets.create_geometry(GeometryDesc::box_xyz(0.36, 0.2, 0.18));
    let pipe = assets.create_geometry(GeometryDesc::box_xyz(0.08, 0.08, 0.7));
    let floor_material =
        assets.create_material(MaterialDesc::line(Color::from_srgb_u8(90, 110, 130), 1.0));
    let body_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(55, 150, 220)));
    let pipe_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(205, 210, 220)));

    let mut scene = Scene::new();
    scene
        .mesh(floor, floor_material)
        .transform(Transform::at(Vec3::new(0.0, -0.35, 0.0)))
        .add()?;
    for x in [-0.45, 0.0, 0.45] {
        scene
            .mesh(body, body_material)
            .transform(Transform::at(Vec3::new(x, 0.0, 0.0)))
            .add()?;
        scene
            .mesh(pipe, pipe_material)
            .transform(Transform::at(Vec3::new(x, -0.18, 0.0)))
            .add()?;
    }
    scene.add_label(
        scene.root(),
        LabelDesc::sdf("Line A"),
        Transform::at(Vec3::new(0.0, 0.34, 0.0)),
    )?;
    let camera = scene.add_default_camera()?;
    scene.frame_all_with_assets(camera, &assets)?;

    let options = RendererOptions::default().with_profile(Profile::Industrial);
    let mut renderer = Renderer::headless_with_options(512, 256, options)?;
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    let skipped = renderer.render_active(&scene)?;
    println!("industrial_static_scene skipped={}", skipped.skipped);
    Ok(())
}
