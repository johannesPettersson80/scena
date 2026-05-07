use scena::{
    Assets, Color, GeometryDesc, MaterialDesc, Renderer, Scene, SourceCoordinateSystem,
    SourceUnits, Transform, Vec3,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.12, 0.12, 0.12));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(120, 230, 90)));

    let cad_position_mm = Vec3::new(250.0, 0.0, 100.0);
    let meters_per_unit = SourceUnits::Millimeters.meters_per_unit();
    let y_up_position = SourceCoordinateSystem::ZUpRightHanded.convert_position(cad_position_mm);
    let render_position = Vec3::new(
        y_up_position.x * meters_per_unit,
        y_up_position.y * meters_per_unit,
        y_up_position.z * meters_per_unit,
    );

    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .transform(Transform::at(render_position))
        .add()?;
    let camera = scene.add_default_camera()?;
    scene.look_at_point(camera, render_position)?;

    let mut renderer = Renderer::headless(320, 180)?;
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    println!("coordinate_units meters_per_unit={meters_per_unit}");
    Ok(())
}
