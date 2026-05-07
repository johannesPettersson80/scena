use scena::{OrbitControls, PointerEvent, Primitive, Renderer, Scene, Transform, Vec3};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeMouseButton {
    Left,
    Right,
}

fn native_press(button: NativeMouseButton, x: f32, y: f32) -> PointerEvent {
    match button {
        NativeMouseButton::Left => PointerEvent::primary_pressed(x, y),
        NativeMouseButton::Right => PointerEvent::secondary_pressed(x, y),
    }
}

fn native_drag(x: f32, y: f32, delta_x: f32, delta_y: f32) -> PointerEvent {
    PointerEvent::moved(x, y, delta_x, delta_y)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut scene = Scene::new();
    scene.add_renderable(
        scene.root(),
        vec![Primitive::unlit_triangle()],
        Transform::default(),
    )?;
    let camera = scene.add_default_camera()?;

    let mut controls = OrbitControls::new(Vec3::ZERO, 2.0).with_damping(0.15);
    controls.handle_pointer(native_press(NativeMouseButton::Left, 160.0, 120.0));
    controls.handle_pointer(native_drag(172.0, 112.0, 12.0, -8.0));
    controls.handle_pointer(PointerEvent::released(172.0, 112.0));
    controls.handle_pointer(native_press(NativeMouseButton::Right, 172.0, 112.0));
    controls.handle_pointer(native_drag(170.0, 118.0, -2.0, 6.0));
    controls.apply_to_scene(&mut scene, camera)?;

    let mut renderer = Renderer::headless(320, 240)?;
    renderer.prepare(&mut scene)?;
    renderer.render_active(&scene)?;
    Ok(())
}
