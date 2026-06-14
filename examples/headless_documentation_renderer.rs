use std::path::PathBuf;

use scena::headless_gltf_viewer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/gate-artifacts/headless-documentation-renderer"));
    std::fs::create_dir_all(&out_dir)?;

    let first = pollster::block_on(
        headless_gltf_viewer("tests/assets/gltf/mesh_material_vertex_color_scene.gltf")
            .size(640, 360)
            .with_default_light()
            .render(),
    )?;
    let capture = first.capture()?;

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
