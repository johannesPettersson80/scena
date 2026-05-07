use scena::{Assets, Color, GeometryDesc, MaterialDesc, Renderer, Scene, Transform, Vec3};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new();
    let source = GeometryDesc::box_xyz(0.12, 0.12, 0.12);
    let transforms = (0..12).map(|index| {
        Transform::at(Vec3::new(
            (index % 6) as f32 * 0.18 - 0.45,
            (index / 6) as f32 * 0.18 - 0.09,
            0.0,
        ))
    });
    let (batch, report) = assets.create_static_batch_with_report(&source, transforms);
    assert!(report.requires_prepare_after_rebuild());

    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(240, 200, 60)));
    let mut scene = Scene::new();
    scene.mesh(batch, material).add()?;
    scene.add_default_camera()?;

    let mut renderer = Renderer::headless(320, 180)?;
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    println!(
        "static_batching instances={} vertices={} picking_debug_instances={}",
        report.instance_count(),
        report.output_vertices(),
        report.picking_debug_instances()
    );
    Ok(())
}
