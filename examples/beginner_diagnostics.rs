use scena::{DiagnosticSeverity, Primitive, Renderer, Scene, Transform};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut scene = Scene::new();
    let mut renderer = Renderer::headless(160, 120)?;

    let diagnostics = renderer.diagnose_scene(&scene);
    for diagnostic in diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
    {
        println!(
            "diagnostic code={:?} help={}",
            diagnostic.code,
            diagnostic.help.as_deref().unwrap_or("none")
        );
    }

    scene.add_renderable(
        scene.root(),
        vec![Primitive::unlit_triangle()],
        Transform::default(),
    )?;
    scene.add_default_camera()?;
    renderer.prepare(&mut scene)?;
    renderer.render_active(&scene)?;
    println!("beginner_diagnostics recovered=true");
    Ok(())
}
