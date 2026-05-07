use scena::{PerspectiveCamera, PlatformSurface, Renderer, Scene, Transform};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let surface = PlatformSurface::native_window(800, 600);
    let mut renderer = Renderer::from_surface(surface)?;

    let mut scene = Scene::new();
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::default(),
        Transform::default(),
    )?;
    scene.set_active_camera(camera)?;
    renderer.prepare(&mut scene)?;
    println!(
        "native_window backend={:?}",
        renderer.capabilities().backend
    );
    Ok(())
}
