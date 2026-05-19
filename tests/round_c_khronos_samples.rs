#![cfg(feature = "khronos-samples")]

use std::path::PathBuf;

use scena::{Assets, DirectionalLight, KhronosSample, Renderer, Scene};

#[test]
fn khronos_sample_catalog_exposes_manifest_metadata_and_package_budget() {
    assert!(KhronosSample::ALL.contains(&KhronosSample::WaterBottle));
    assert!(KhronosSample::ALL.contains(&KhronosSample::TransmissionTest));

    let water_bottle = KhronosSample::WaterBottle.metadata();
    assert_eq!(water_bottle.name(), "WaterBottle");
    assert_eq!(
        water_bottle.primary_path(),
        "tests/assets/gltf/khronos/WaterBottle/WaterBottle.gltf"
    );
    assert_eq!(
        water_bottle.primary_sha256(),
        "0596f4e61dc781439d254fdfb5e3462daf1762c18715e3e3ac13001aa8f3f547"
    );
    assert!(water_bottle.contract().contains("real product PBR"));
    assert!(
        water_bottle
            .license_reference()
            .contains("glTF-Sample-Assets")
    );

    let total_bytes = KhronosSample::ALL
        .iter()
        .flat_map(|sample| sample.metadata().files())
        .map(|path| std::fs::metadata(path).expect("sample file exists").len())
        .sum::<u64>();
    assert!(total_bytes <= KhronosSample::PACKAGE_SIZE_BUDGET_BYTES);
}

#[test]
fn khronos_sample_loader_loads_every_catalog_entry_without_user_paths() {
    let assets = Assets::new();
    let samples = assets.khronos();

    for sample in KhronosSample::ALL {
        let loaded = pollster::block_on(samples.load(*sample))
            .unwrap_or_else(|error| panic!("{} loads: {error}", sample.metadata().name()));
        assert_eq!(loaded.path().as_str(), sample.metadata().primary_path());
    }
}

#[test]
fn khronos_sample_loader_has_named_shortcuts_for_headline_assets() {
    let assets = Assets::new();
    let samples = assets.khronos();

    let water_bottle = pollster::block_on(samples.water_bottle()).expect("WaterBottle loads");
    assert_eq!(
        water_bottle.path().as_str(),
        KhronosSample::WaterBottle.metadata().primary_path()
    );

    let transmission =
        pollster::block_on(samples.transmission_test()).expect("TransmissionTest loads");
    assert_eq!(
        transmission.path().as_str(),
        KhronosSample::TransmissionTest.metadata().primary_path()
    );

    let rigged_simple = pollster::block_on(samples.rigged_simple()).expect("RiggedSimple loads");
    assert_eq!(
        rigged_simple.path().as_str(),
        KhronosSample::RiggedSimple.metadata().primary_path()
    );
}

#[test]
fn khronos_sample_loader_renders_rigged_sample_reference_artifact() {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.khronos().rigged_simple()).expect("RiggedSimple loads");
    let mut scene = Scene::new();
    let import = scene.instantiate(&scene_asset).expect("scene instantiates");
    let camera = scene.add_default_camera().expect("camera inserts");
    scene
        .frame_import(camera, &import)
        .expect("RiggedSimple frames");
    scene
        .directional_light(DirectionalLight::key_light())
        .add()
        .expect("key light inserts");

    let mut renderer = Renderer::headless(48, 48).expect("renderer builds");
    renderer.set_environment(assets.default_environment());
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("scene prepares");
    renderer.render_active(&scene).expect("scene renders");

    let frame = renderer.frame_rgba8();
    let visible_pixels = frame
        .chunks_exact(4)
        .filter(|rgba| rgba[0] > 12 || rgba[1] > 12 || rgba[2] > 12)
        .count();
    assert!(visible_pixels > 16, "RiggedSimple render must be nonblank");

    let path = artifact_dir().join("rigged-simple-sample-loader-reference.ppm");
    write_ppm(&path, 48, 48, frame);
    assert!(path.exists());
}

fn artifact_dir() -> PathBuf {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/khronos-samples");
    std::fs::create_dir_all(&path).expect("artifact dir exists");
    path
}

fn write_ppm(path: &std::path::Path, width: u32, height: u32, rgba: &[u8]) {
    let mut bytes = format!("P6\n{width} {height}\n255\n").into_bytes();
    for pixel in rgba.chunks_exact(4) {
        bytes.extend_from_slice(&pixel[..3]);
    }
    std::fs::write(path, bytes).expect("reference artifact writes");
}
