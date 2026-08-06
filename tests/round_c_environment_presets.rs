use std::path::{Path, PathBuf};

use scena::{
    Aabb, Assets, Color, EnvironmentPreset, GeometryDesc, MaterialDesc, Renderer, Scene, Vec3,
};

#[test]
fn environment_preset_catalog_exposes_metadata_and_package_budget() {
    assert_eq!(
        EnvironmentPreset::ALL,
        &[EnvironmentPreset::NeutralStudio, EnvironmentPreset::Studio]
    );

    let studio = EnvironmentPreset::Studio.metadata();
    assert_eq!(studio.name(), "Studio");
    assert_eq!(
        studio.source_path(),
        "tests/assets/environment/polyhaven/studio_small_03_1k.hdr"
    );
    assert_eq!(
        studio.runtime_uri(),
        "scena://bundled/environment/studio_small_03_128x64.hdr"
    );
    assert_eq!(
        studio.source_sha256(),
        "30933d55e45f0795daf49f3cbefbe0e5ebcb821ee04fb0a2818c02ffc3938817"
    );
    assert_eq!(studio.license(), "CC0-1.0");
    assert!(
        studio
            .source_url()
            .contains("polyhaven.com/a/studio_small_03")
    );
    assert!(studio.contract().contains("studio HDR"));
    assert_eq!(
        studio.files(),
        &[
            studio.source_path(),
            "tests/assets/environment/generated/studio_small_03_128x64.hdr",
            "tests/assets/environment/polyhaven/studio_small_08_2k.hdr",
            "tests/assets/environment/polyhaven/studio_small_08_2k.provenance.json",
        ]
    );
    assert!(
        studio.source_size_bytes() < 50_000,
        "the package-embedded runtime derivative must not ship the full 1K HDR into every WASM bundle"
    );

    let total_bytes = EnvironmentPreset::ALL
        .iter()
        .flat_map(|preset| preset.metadata().files())
        .map(|path| std::fs::metadata(path).expect("preset file exists").len())
        .sum::<u64>();
    assert!(total_bytes <= EnvironmentPreset::PACKAGE_SIZE_BUDGET_BYTES);
}

#[test]
fn environment_presets_load_without_user_supplied_paths() {
    let assets = Assets::new();

    for preset in EnvironmentPreset::ALL {
        let handle = pollster::block_on(assets.load_environment_preset(*preset))
            .unwrap_or_else(|error| panic!("{} loads: {error}", preset.metadata().name()));
        let desc = assets
            .try_environment(handle)
            .expect("preset handle resolves in the source asset store");
        assert_eq!(desc.source_path().as_str(), preset.metadata().runtime_uri());
    }
}

#[test]
fn environment_presets_render_reference_contact_sheet() {
    let assets = Assets::new();
    let mut contact_sheet = Vec::new();

    for preset in EnvironmentPreset::ALL {
        let handle = pollster::block_on(assets.load_environment_preset(*preset))
            .unwrap_or_else(|error| panic!("{} loads: {error}", preset.metadata().name()));
        let frame = render_environment_preview(&assets, handle);
        let visible_pixels = frame
            .chunks_exact(4)
            .filter(|rgba| rgba[0] > 10 || rgba[1] > 10 || rgba[2] > 10)
            .count();
        assert!(
            visible_pixels > 32,
            "{} preset render must be nonblank",
            preset.metadata().name()
        );
        contact_sheet.extend(frame);
    }

    let path = artifact_dir().join("environment-preset-reference-docs-image.ppm");
    write_horizontal_ppm(
        &path,
        64,
        64,
        EnvironmentPreset::ALL.len() as u32,
        &contact_sheet,
    );
    assert!(path.exists());
}

fn render_environment_preview(assets: &Assets, environment: scena::EnvironmentHandle) -> Vec<u8> {
    let mut scene = Scene::new();
    let geometry = assets.create_geometry(GeometryDesc::sphere(0.55, 24, 12));
    let material = assets.create_material(MaterialDesc::metal(Color::LIGHT_GRAY));
    scene
        .mesh(geometry, material)
        .add()
        .expect("preview sphere inserts");
    let camera = scene.add_default_camera().expect("default camera inserts");
    scene
        .frame_bounds(
            camera,
            Aabb::new(Vec3::splat(-0.55), Vec3::splat(0.55)),
            scena::FramingOptions::new().three_quarter_front_right(),
        )
        .expect("preview sphere frames");

    let mut renderer = Renderer::headless(64, 64).expect("headless renderer builds");
    renderer.set_environment(environment);
    renderer
        .prepare_with_assets(&mut scene, assets)
        .expect("environment preset scene prepares");
    renderer.render_active(&scene).expect("preview renders");
    renderer.frame_rgba8().to_vec()
}

fn artifact_dir() -> PathBuf {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/environment-presets");
    std::fs::create_dir_all(&path).expect("artifact dir exists");
    path
}

fn write_horizontal_ppm(path: &Path, tile_width: u32, height: u32, tiles: u32, rgba: &[u8]) {
    let width = tile_width * tiles;
    let mut bytes = format!("P6\n{width} {height}\n255\n").into_bytes();
    for row in 0..height as usize {
        for tile in 0..tiles as usize {
            let tile_stride = tile_width as usize * height as usize * 4;
            let row_start = tile * tile_stride + row * tile_width as usize * 4;
            for pixel in rgba[row_start..row_start + tile_width as usize * 4].chunks_exact(4) {
                bytes.extend_from_slice(&pixel[..3]);
            }
        }
    }
    std::fs::write(path, bytes).expect("reference artifact writes");
}
