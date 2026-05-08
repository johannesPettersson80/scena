use scena::{
    Assets, Color, GeometryDesc, MaterialDesc, OrbitControls, PointerEvent, Renderer, Scene,
    TouchEvent, Vec3,
};

fn browser_pointer_drag(css_x: f32, css_y: f32, movement_x: f32, movement_y: f32) -> PointerEvent {
    PointerEvent::moved(css_x, css_y, movement_x, movement_y)
}

fn browser_wheel(css_x: f32, css_y: f32, delta_y: f32) -> PointerEvent {
    PointerEvent::wheel(css_x, css_y, delta_y.signum())
}

fn browser_pinch(css_x: f32, css_y: f32, scale_delta: f32) -> TouchEvent {
    TouchEvent::pinch(css_x, css_y, -scale_delta)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.8, 0.45, 0.35));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(90, 180, 220)));

    let mut scene = Scene::new();
    scene.mesh(geometry, material).add()?;
    let camera = scene.add_default_camera()?;

    let mut controls = OrbitControls::new(Vec3::ZERO, 2.0)
        .focus(Vec3::ZERO, 2.5)
        .with_damping(0.12);
    controls.handle_pointer(PointerEvent::primary_pressed(160.0, 120.0));
    controls.handle_pointer(browser_pointer_drag(166.0, 118.0, 6.0, -2.0));
    controls.handle_pointer(browser_wheel(166.0, 118.0, -120.0));
    controls.handle_touch(browser_pinch(166.0, 118.0, 0.25));
    controls.apply_to_scene(&mut scene, camera)?;

    let mut renderer = Renderer::headless(320, 240)?;
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    Ok(())
}
