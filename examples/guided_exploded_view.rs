use std::path::{Path, PathBuf};

use scena::{Assets, ExplodedView, Renderer, Scene, Vec3};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/gate-artifacts/guided-exploded-view"));
    std::fs::create_dir_all(&out_dir)?;

    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/exploded_view_assembly.gltf"))?;
    let (mut scene, camera) = Scene::with_default_camera()?;
    let import = scene.instantiate(&scene_asset)?;
    scene.frame_import(camera, &import)?;

    let mut renderer = Renderer::headless(320, 220)?;
    render_ppm(
        &mut renderer,
        &mut scene,
        &assets,
        &out_dir.join("assembly-assembled.ppm"),
    )?;

    let assembly = import.node("Assembly")?;
    let exploded = ExplodedView::from_node(assembly)
        .along_axis(Vec3::X)
        .factor(1.0)
        .distance(0.35)
        .transforms(&scene, &assets)?;
    scene.set_transforms(&exploded.as_transform_updates())?;
    render_ppm(
        &mut renderer,
        &mut scene,
        &assets,
        &out_dir.join("assembly-exploded.ppm"),
    )?;

    scene.set_transforms(
        &exploded
            .updates()
            .iter()
            .map(|update| (update.node, update.original))
            .collect::<Vec<_>>(),
    )?;
    render_ppm(
        &mut renderer,
        &mut scene,
        &assets,
        &out_dir.join("assembly-restored.ppm"),
    )?;

    println!("{}", out_dir.display());
    Ok(())
}

fn render_ppm(
    renderer: &mut Renderer,
    scene: &mut Scene,
    assets: &Assets,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    renderer.prepare_with_assets(scene, assets)?;
    renderer.render_active(scene)?;
    write_ppm(path, 320, 220, renderer.frame_rgba8())?;
    Ok(())
}

fn write_ppm(path: &Path, width: u32, height: u32, rgba: &[u8]) -> std::io::Result<()> {
    let mut ppm = format!("P6\n{} {}\n255\n", width, height).into_bytes();
    for pixel in rgba.chunks_exact(4) {
        ppm.extend_from_slice(&pixel[..3]);
    }
    std::fs::write(path, ppm)
}
