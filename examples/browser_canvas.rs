use scena::{PerspectiveCamera, PlatformSurface, Renderer, Scene, Transform};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let surface = PlatformSurface::browser_webgpu_canvas(640, 480);
    let mut renderer = pollster::block_on(Renderer::from_surface_async(surface))?;

    let mut scene = Scene::new();
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::default(),
        Transform::default(),
    )?;
    scene.set_active_camera(camera)?;
    renderer.prepare(&mut scene)?;
    println!(
        "browser_canvas backend={:?}",
        renderer.capabilities().backend
    );
    Ok(())
}
