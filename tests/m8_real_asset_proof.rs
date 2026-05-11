//! Plan line 783: real-asset glTF import proof. Imports the Khronos
//! `WaterBottle` real-product PBR fixture from `tests/assets/gltf/khronos/`
//! and verifies the importer produces a renderable scene with the expected
//! mesh + material + texture-role topology, real-world dimensions, and
//! non-black framed pixels through the headless CPU rasterizer.
//!
//! The test fixture is bundled under `tests/assets/gltf/khronos/WaterBottle/`
//! and pinned by SHA-256 in `tests/assets/gltf/khronos/manifest.toml`. The
//! .gltf and .bin are upstream-faithful; the four PNG textures were
//! downsampled from the upstream 2048² to 256² with Pillow LANCZOS so the
//! bundled fixture stays under 300 KB while preserving every material role
//! the importer + renderer must handle.
#![cfg(not(target_arch = "wasm32"))]

use std::fs::File;
use std::io::BufWriter;

use scena::{Assets, Renderer, Scene};

const WATERBOTTLE_PATH: &str = "tests/assets/gltf/khronos/WaterBottle/WaterBottle.gltf";
const ARTIFACT_PNG: &str = "target/gate-artifacts/m8-real-asset/waterbottle.png";

#[test]
fn m8_real_asset_waterbottle_imports_and_renders() {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene(WATERBOTTLE_PATH)).expect("WaterBottle .gltf loads");

    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("WaterBottle instantiates into a scene");
    let bounds = import
        .bounds_world(&scene)
        .expect("imported WaterBottle has world bounds");

    // The Khronos WaterBottle is authored at real-world millimeter scale: a
    // ~10.9 cm wide × ~26 cm tall × ~10.9 cm deep bottle (the upstream is
    // exported in metres, so the bounds extents are roughly 0.054 × 0.13
    // × 0.054 metres). Asserting a real-world-shaped extent rather than NDC
    // unit-cube extents proves the importer preserves the upstream metric
    // scale instead of normalizing every asset to ±1.
    let extents = (
        bounds.max.x - bounds.min.x,
        bounds.max.y - bounds.min.y,
        bounds.max.z - bounds.min.z,
    );
    assert!(
        extents.0 > 0.05 && extents.0 < 0.20,
        "WaterBottle X extent must be on the order of metres-scale millimetres (got {})",
        extents.0
    );
    assert!(
        extents.1 > 0.10 && extents.1 < 0.30,
        "WaterBottle Y extent must be on the order of metres-scale millimetres (got {})",
        extents.1
    );

    let camera = scene.add_default_camera().expect("default camera inserts");
    scene
        .frame(camera, bounds)
        .expect("camera frames the WaterBottle");

    let mut renderer = Renderer::headless(256, 256).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("WaterBottle prepares for the headless rasterizer");
    renderer
        .render_active(&scene)
        .expect("WaterBottle renders");

    let stats = renderer.stats();
    assert_eq!(
        stats.materials, 1,
        "WaterBottle's `BottleMat` must surface as one prepared material"
    );
    assert_eq!(
        stats.textures, 4,
        "WaterBottle must surface the four upstream PBR texture roles \
         (baseColor, normal, occlusionRoughnessMetallic, emissive)"
    );
    assert!(
        stats.triangles > 1000,
        "real product mesh must have a non-trivial triangle count, got {}",
        stats.triangles
    );

    let frame = renderer.frame_rgba8();
    let nonzero = frame
        .chunks_exact(4)
        .filter(|p| p[..3] != [0, 0, 0])
        .count();
    assert!(
        nonzero > 100,
        "framed WaterBottle silhouette must produce at least 100 non-black pixels through the \
         CPU rasterizer (got {nonzero})"
    );

    write_png_artifact(frame, 256, 256);
}

fn write_png_artifact(rgba8: &[u8], width: u32, height: u32) {
    if let Some(parent) = std::path::Path::new(ARTIFACT_PNG).parent() {
        std::fs::create_dir_all(parent).expect("artifact dir");
    }
    let file = File::create(ARTIFACT_PNG).expect("create artifact PNG");
    let writer = BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("PNG header writes");
    writer.write_image_data(rgba8).expect("PNG payload writes");
}
