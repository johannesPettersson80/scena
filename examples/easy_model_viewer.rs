use scena::{
    Assets, AutoExposureConfig, FramingOptions, GridFloorOptions, OrbitControls, PerspectiveCamera,
    Renderer, Scene, Transform,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new();
    let model = pollster::block_on(
        assets.load_scene("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
    )?;

    let mut scene = Scene::new();
    let import = scene.instantiate(&model)?;
    scene.add_studio_lighting()?;

    let bounds = import
        .bounds_world(&scene)
        .ok_or("import has no renderable bounds")?;
    scene.add_grid_floor(&assets, GridFloorOptions::new().under_bounds(bounds))?;

    let width = 640;
    let height = 360;
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::default().with_aspect(width as f32 / height as f32),
        Transform::default(),
    )?;
    let framing = scene.frame_bounds(
        camera,
        bounds,
        FramingOptions::new()
            .three_quarter_front_right()
            .fill(0.72)
            .margin_px(32.0)
            .viewport(width, height),
    )?;

    let orbit = OrbitControls::from_framing(framing);
    let mut renderer = Renderer::headless(width, height)?;
    renderer.set_auto_exposure(AutoExposureConfig::default());
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render(&scene, camera)?;

    println!(
        "easy_model_viewer roots={} fill={:.2} orbit_target={:?}",
        import.roots().len(),
        framing.projected_rect.fill_fraction(width, height),
        orbit.target()
    );
    Ok(())
}
