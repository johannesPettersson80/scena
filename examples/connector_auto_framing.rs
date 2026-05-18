use scena::{
    Assets, AutoExposureConfig, FramingOptions, GridFloorOptions, PerspectiveCamera, Renderer,
    Scene, Transform, Vec3,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new();
    let drive_part = pollster::block_on(assets.load_scene("tests/assets/gltf/drive_unit.glb"))?;
    let load_part = pollster::block_on(assets.load_scene("tests/assets/gltf/load_unit.glb"))?;

    let mut scene = Scene::new();
    let load = scene.instantiate(&load_part)?;
    let drive = scene.instantiate(&drive_part)?;
    let drive_root = *drive.roots().first().ok_or("drive import has no root")?;

    let before = Transform::at(Vec3::new(-0.48, 0.11, 0.0));
    scene.set_transform(drive_root, before)?;
    scene.mate(&drive, "shaft", &load, "hub")?;
    let after = scene
        .world_transform(drive_root)
        .ok_or("drive root missing")?;
    scene.set_transform(drive_root, before)?;

    let replay_bounds = scene.bounds_for_transforms(drive_root, &[before, after], &assets)?;
    let load_bounds = load
        .bounds_world(&scene)
        .ok_or("load import has no bounds")?;
    let framing_bounds = replay_bounds.union(load_bounds);

    scene.add_studio_lighting()?;
    scene.add_grid_floor(
        &assets,
        GridFloorOptions::new().under_bounds(framing_bounds),
    )?;

    let width = 960;
    let height = 540;
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::default().with_aspect(width as f32 / height as f32),
        Transform::default(),
    )?;
    let framing = scene.frame_bounds(
        camera,
        framing_bounds,
        FramingOptions::new()
            .azimuth_elevation(-27.5, 17.8)
            .fill(0.72)
            .margin_px(32.0)
            .viewport(width, height),
    )?;

    let mut renderer = Renderer::headless(width, height)?;
    renderer.set_auto_exposure(AutoExposureConfig::default());
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render(&scene, camera)?;

    println!(
        "connector_auto_framing before_x={:.2} after_x={:.2} fill={:.2}",
        before.translation.x,
        after.translation.x,
        framing.projected_rect.fill_fraction(width, height)
    );
    Ok(())
}
