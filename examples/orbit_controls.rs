use scena::{
    Assets, Color, GeometryDesc, MaterialDesc, OrbitControls, PointerEvent, Renderer, Scene,
    TouchEvent, Vec3,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.8, 0.45, 0.35));
    let material = assets.create_material(MaterialDesc::unlit(Color::CYAN));

    let mut scene = Scene::new();
    scene.mesh(geometry, material).add()?;
    let camera = scene.add_default_camera()?;

    // Named damping preset is the first-path default; .with_damping(<f32>)
    // remains available as the escape hatch when a project needs a specific
    // damping factor outside the preset table.
    let mut controls = OrbitControls::new(Vec3::ZERO, 2.0)
        .snappy()
        .with_damping(0.15);
    controls.handle_pointer(PointerEvent::primary_pressed(160.0, 120.0));
    controls.handle_pointer(PointerEvent::moved(168.0, 116.0, 8.0, -4.0));
    controls.handle_touch(TouchEvent::pinch(168.0, 116.0, -0.1));
    controls.handle_pointer(PointerEvent::wheel(168.0, 116.0, -0.25));
    controls.apply_to_scene(&mut scene, camera)?;

    let mut renderer = Renderer::headless(320, 240)?;
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    Ok(())
}
