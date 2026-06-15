use std::path::{Path, PathBuf};

use scena::{
    Assets, Color, GeometryDesc, MaterialDesc, MeasurementOverlay, Renderer, Scene, Transform,
    UnitFormat, Vec3,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/gate-artifacts/cad-inspection-viewer"));
    std::fs::create_dir_all(&out_dir)?;

    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.5, 0.5, 0.5));
    let selected_material =
        assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::BLUE, 0.0, 0.45));
    let context_material =
        assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::GRAY, 0.0, 0.6));
    let cover_material =
        assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::RED, 0.0, 0.5));

    let mut scene = Scene::new();
    let selected = scene
        .mesh(geometry, selected_material)
        .transform(Transform::at(Vec3::new(-0.45, 0.0, 0.0)))
        .add()?;
    let context = scene
        .mesh(geometry, context_material)
        .transform(Transform::at(Vec3::new(0.45, 0.0, 0.0)))
        .add()?;
    let cover = scene
        .mesh(geometry, cover_material)
        .transform(Transform::at(Vec3::new(0.0, 0.0, -0.5)))
        .add()?;

    scene.add_tag(selected, "selected")?;
    scene.show_only([selected, context])?;
    scene.ghost(context, 0.35)?;
    assert!(!scene.visible(cover).expect("cover exists"));

    scene.add_measurement_overlay(
        &assets,
        MeasurementOverlay::distance(
            "center-spacing",
            Vec3::new(-0.45, 0.0, 0.0),
            Vec3::new(0.45, 0.0, 0.0),
        )
        .with_label("center spacing")
        .with_units(UnitFormat::millimeters()),
    )?;

    let camera = scene.add_default_camera()?;
    scene.frame_all_with_assets(camera, &assets)?;
    let mut renderer = Renderer::headless(320, 220)?;
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;

    let artifact = out_dir.join("cad-inspection-viewer.ppm");
    write_ppm(&artifact, 320, 220, renderer.frame_rgba8())?;
    println!("{}", artifact.display());
    Ok(())
}

fn write_ppm(path: &Path, width: u32, height: u32, rgba: &[u8]) -> std::io::Result<()> {
    let mut ppm = format!("P6\n{} {}\n255\n", width, height).into_bytes();
    for pixel in rgba.chunks_exact(4) {
        ppm.extend_from_slice(&pixel[..3]);
    }
    std::fs::write(path, ppm)
}
