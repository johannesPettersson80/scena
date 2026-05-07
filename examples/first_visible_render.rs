use scena::{Primitive, Renderer, Scene, Transform};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut scene = Scene::new();
    scene.add_renderable(
        scene.root(),
        vec![Primitive::unlit_triangle()],
        Transform::default(),
    )?;
    scene.add_default_camera()?;

    let mut renderer = Renderer::headless(320, 240)?;
    renderer.prepare(&mut scene)?;
    renderer.render_active(&scene)?;
    Ok(())
}
