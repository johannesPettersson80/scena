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

use scena::{Assets, Color, DirectionalLight, Renderer, Scene, Transform};

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

    // Place a 3/4-view camera ~25 cm in front of the bottle, slightly raised
    // and offset, so the framing matches the Khronos reference screenshot
    // pose. `scene.frame` uses the default near/far which would clip an
    // asset whose bounding sphere is smaller than the near plane (0.1 m).
    let centre = scena::Vec3::new(
        (bounds.min.x + bounds.max.x) * 0.5,
        (bounds.min.y + bounds.max.y) * 0.5,
        (bounds.min.z + bounds.max.z) * 0.5,
    );
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            scena::PerspectiveCamera::default(),
            Transform::at(scena::Vec3::new(
                centre.x + 0.12,
                centre.y + 0.05,
                centre.z + 0.25,
            ))
            .rotate_y_deg(25.0)
            .rotate_x_deg(-10.0),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");

    // Key + fill directional rig so the body shading shows volume without
    // crushing the cap to black. The CPU rasterizer's degraded preview
    // does not do real specular highlights, but two lights at least keep
    // the rear-facing surfaces from going pure black.
    scene
        .directional_light(
            DirectionalLight::default()
                .with_color(Color::WHITE)
                .with_illuminance_lux(40_000.0),
        )
        .transform(Transform::default().rotate_x_deg(-30.0).rotate_y_deg(30.0))
        .add()
        .expect("key directional light inserts");
    scene
        .directional_light(
            DirectionalLight::default()
                .with_color(Color::from_srgb_u8(200, 215, 235))
                .with_illuminance_lux(15_000.0),
        )
        .transform(Transform::default().rotate_x_deg(-10.0).rotate_y_deg(-120.0))
        .add()
        .expect("fill directional light inserts");
    let environment = assets.default_environment();

    // Default to the CPU rasterizer so the test is deterministic on every
    // host. To render through the GPU pipeline (e.g. on a Linux box with a
    // working Vulkan adapter — desktop GPUs or `lavapipe` via Mesa), set
    // `SCENA_USE_GPU=1`; this is what produces the higher-fidelity render
    // matching the upstream Khronos reference (red cap, visible logo,
    // black label wrap, real metallic body).
    let mut renderer = if std::env::var("SCENA_USE_GPU").is_ok()
        && let Ok(r) = Renderer::headless_gpu(512, 512)
    {
        if let Some(report) = r.gpu_adapter_report() {
            eprintln!("scena GPU: {} ({})", report.name, report.backend);
        }
        r
    } else {
        Renderer::headless(512, 512).expect("CPU rasterizer")
    };
    renderer.set_environment(environment);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("WaterBottle prepares for the headless renderer");
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
        nonzero > 5_000,
        "framed WaterBottle silhouette must produce at least 5000 non-black pixels \
         (got {nonzero})"
    );

    // Sample known regions to pin the cream body + dark cap + dark label
    // colors that the IBL fix delivers. These bytes were the result of the
    // first green-flag render after the CPU IBL cubemap-irradiance fallback
    // landed; they catch regressions in either the texture sampling chain
    // or the environment irradiance derivation.
    let cap = pixel_at(frame, 256, 80);
    let body = pixel_at(frame, 256, 240);
    assert!(
        cap[0] < 200 && cap[1] < 100 && cap[2] < 100,
        "cap region should sample the texture's dark plastic (got {cap:?})"
    );
    assert!(
        body[0] > 80 && body[1] > 80 && body[2] < body[1],
        "body region should show warm cream/olive (got {body:?})"
    );

    write_png_artifact(frame, 512, 512);
}

fn pixel_at(frame: &[u8], x: usize, y: usize) -> [u8; 4] {
    let p = (y * 512 + x) * 4;
    [frame[p], frame[p + 1], frame[p + 2], frame[p + 3]]
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
