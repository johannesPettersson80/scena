use scena::{PerspectiveCamera, Primitive, Renderer, Scene, Transform};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut scene = Scene::new();
    scene.add_renderable(
        scene.root(),
        vec![Primitive::unlit_triangle()],
        Transform::default(),
    )?;
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::default(),
        Transform::default(),
    )?;
    scene.set_active_camera(camera)?;

    let mut renderer = Renderer::headless(64, 64)?;
    renderer.prepare(&mut scene)?;
    renderer.render_active(&scene)?;
    let center = &renderer.frame_rgba8()[((32 * 64 + 32) * 4)..((32 * 64 + 33) * 4)];
    println!("headless_ci center={center:?}");
    Ok(())
}
