use scena::{OrbitControls, PointerEvent, Primitive, Renderer, Scene, TouchEvent, Transform, Vec3};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut scene = Scene::new();
    scene.add_renderable(
        scene.root(),
        vec![Primitive::unlit_triangle()],
        Transform::default(),
    )?;
    let camera = scene.add_default_camera()?;

    let mut controls = OrbitControls::new(Vec3::ZERO, 2.0).with_damping(0.15);
    controls.handle_pointer(PointerEvent::primary_pressed(160.0, 120.0));
    controls.handle_pointer(PointerEvent::moved(168.0, 116.0, 8.0, -4.0));
    controls.handle_touch(TouchEvent::pinch(168.0, 116.0, -0.1));
    controls.handle_pointer(PointerEvent::wheel(168.0, 116.0, -0.25));
    controls.apply_to_scene(&mut scene, camera)?;

    let mut renderer = Renderer::headless(320, 240)?;
    renderer.prepare(&mut scene)?;
    renderer.render_active(&scene)?;
    Ok(())
}
