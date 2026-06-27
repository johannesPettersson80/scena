#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::{Path, PathBuf};

use scena::{Assets, MeasurementOverlay, Renderer, Scene, UnitFormat, Vec3};

#[test]
fn measurement_overlay_renders_line_and_label_pixels() {
    let assets = Assets::new();
    let mut scene = Scene::new();
    scene.add_default_camera().expect("camera inserts");
    let report = scene
        .add_measurement_overlay(
            &assets,
            MeasurementOverlay::distance(
                "width",
                Vec3::new(-0.6, 0.0, 0.0),
                Vec3::new(0.6, 0.0, 0.0),
            )
            .with_label("width")
            .with_units(UnitFormat::meters().with_precision(2)),
        )
        .expect("measurement overlay inserts");
    assert!(
        report.label.is_some(),
        "visual proof includes label geometry"
    );

    let mut renderer = Renderer::headless(128, 96).expect("renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("measurement scene prepares");
    renderer
        .render_active(&scene)
        .expect("measurement scene renders");
    let frame = renderer.frame_rgba8().to_vec();
    assert!(
        nonblack_pixel_count(&frame) > 40,
        "measurement overlay should render visible line/label pixels"
    );

    let artifact_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/measurement");
    fs::create_dir_all(&artifact_dir).expect("artifact dir exists");
    write_ppm_artifact(
        &artifact_dir,
        "measurement-distance-line-label",
        128,
        96,
        &frame,
    );
}

fn nonblack_pixel_count(frame: &[u8]) -> usize {
    frame
        .chunks_exact(4)
        .filter(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)
        .count()
}

fn write_ppm_artifact(dir: &Path, name: &str, width: u32, height: u32, rgba: &[u8]) {
    let mut ppm = format!("P6\n{} {}\n255\n", width, height).into_bytes();
    for pixel in rgba.chunks_exact(4) {
        ppm.extend_from_slice(&pixel[..3]);
    }
    fs::write(dir.join(format!("{name}.ppm")), ppm).expect("PPM artifact writes");
}
