use scena::{Assets, Color, GeometryDesc, MaterialDesc, Renderer, Scene, Transform, Vec3};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.3, 0.3, 0.3));
    let visible_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(80, 170, 255)));
    let helper_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(255, 230, 80)));

    let mut scene = Scene::new();
    let machine = scene
        .mesh(geometry, visible_material)
        .transform(Transform::at(Vec3::new(-0.25, 0.0, 0.0)))
        .add()?;
    let helper = scene
        .mesh(geometry, helper_material)
        .transform(Transform::at(Vec3::new(0.25, 0.0, 0.0)).scale_by(0.5))
        .add()?;
    let hidden = scene
        .mesh(geometry, visible_material)
        .transform(Transform::at(Vec3::new(0.0, 0.4, 0.0)))
        .add()?;

    scene.add_tag(machine, "operational")?;
    scene.set_layer_mask(machine, 0b0001)?;
    scene.set_layer_mask(helper, 0b0001)?;
    scene.set_layer_mask(hidden, 0b0010)?;
    scene.set_visible(hidden, false)?;
    scene.set_render_group(helper, 10)?;
    scene.set_helper_on_top(helper, true)?;

    let camera = scene.add_default_camera()?;
    scene.set_camera_layer_mask(camera, 0b0001)?;

    let mut renderer = Renderer::headless(320, 180)?;
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    println!(
        "layers_visibility tagged={} helper_on_top={:?}",
        scene.tagged("operational").count(),
        scene.helper_on_top(helper)
    );
    Ok(())
}
