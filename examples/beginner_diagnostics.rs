use scena::{Assets, Color, DiagnosticSeverity, GeometryDesc, MaterialDesc, Renderer, Scene};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new();
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

    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.6, 0.4, 0.3));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(80, 180, 220)));
    scene.mesh(geometry, material).add()?;
    let camera = scene.add_default_camera()?;
    scene.frame_all_with_assets(camera, &assets)?;
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    println!("beginner_diagnostics recovered=true");
    Ok(())
}
