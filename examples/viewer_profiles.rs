use std::path::PathBuf;

use scena::{ViewerProfile, headless_gltf_viewer};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/gate-artifacts/viewer-profiles"));
    std::fs::create_dir_all(&out_dir)?;

    for profile in [
        ViewerProfile::model_viewer(),
        ViewerProfile::cad_inspection(),
        ViewerProfile::product(),
        ViewerProfile::industrial(),
        ViewerProfile::documentation(),
    ] {
        let path = out_dir.join(format!("{}.png", profile.name()));
        pollster::block_on(
            headless_gltf_viewer("tests/assets/gltf/khronos/UnlitTest/UnlitTest.gltf")
                .size(320, 200)
                .with_viewer_profile(profile)
                .render_png(&path),
        )?;
        println!("{} {}", profile.name(), path.display());
    }

    Ok(())
}
