use std::path::PathBuf;

use scena::{
    Assets, Callout, CaptureOptions, Color, GeometryDesc, MaterialDesc, Renderer, Scene, Transform,
    Vec3,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/gate-artifacts/headless-documentation-renderer"));
    std::fs::create_dir_all(&out_dir)?;

    let assets = Assets::new();
    let body_geometry = assets.create_geometry(GeometryDesc::box_xyz(0.8, 0.42, 0.28));
    let body_material =
        assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::BLUE, 0.0, 0.5));

    let mut scene = Scene::new();
    let body = scene
        .mesh(body_geometry, body_material)
        .transform(Transform::IDENTITY)
        .add()?;
    scene.add_callout(
        &assets,
        Callout::node(
            "primary-callout",
            body,
            Vec3::new(0.32, 0.16, 0.0),
            "Service panel",
        )
        .with_label_offset(Vec3::new(0.28, 0.24, 0.0))
        .with_color(Color::YELLOW),
    )?;

    let camera = scene.add_default_camera()?;
    let bounds = scene
        .node_world_bounds(body, &assets)?
        .ok_or_else(|| std::io::Error::other("body bounds should be available"))?;
    scene.frame_bounds(camera, bounds, Default::default())?;

    let mut renderer = Renderer::headless(640, 360)?;
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    let capture = renderer.capture_rgba8(&scene, CaptureOptions::default())?;

    let png_path = out_dir.join("documentation-render.png");
    let descriptor_path = out_dir.join("documentation-render.capture.json");
    capture.write_png(&png_path)?;
    std::fs::write(
        &descriptor_path,
        serde_json::to_string_pretty(&capture.descriptor)?,
    )?;

    println!("{}", png_path.display());
    println!("{}", descriptor_path.display());
    Ok(())
}
