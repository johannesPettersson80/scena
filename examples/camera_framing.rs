use scena::{
    Assets, Color, FramingOptions, GeometryDesc, MaterialDesc, OrbitControls, PerspectiveCamera,
    Renderer, Scene, Transform, Vec3,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.2, 0.4, 0.4));
    let material = assets.create_material(MaterialDesc::unlit(Color::BLUE));

    let mut scene = Scene::new();
    let group = scene.add_empty(scene.root(), Transform::at(Vec3::new(100.0, 0.0, 0.0)))?;
    let inspected_part = scene
        .mesh(geometry, material)
        .parent(group)
        .transform(Transform::at(Vec3::new(8.0, 0.0, 0.0)))
        .add()?;
    scene.center_visible_bounds_on(group, &assets, Vec3::ZERO)?;
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::standard(),
        Transform::default(),
    )?;

    let bounds = scene
        .node_world_bounds(group, &assets)?
        .ok_or("part has no visible bounds")?;
    let framing = scene.frame_bounds(
        camera,
        bounds,
        FramingOptions::new()
            .three_quarter_front_right()
            .fill(0.72)
            .margin_px(24.0)
            .tighten_depth_range(true)
            .viewport(320, 180),
    )?;
    let controls = OrbitControls::from_framing(framing);

    let mut renderer = Renderer::headless(320, 180)?;
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render(&scene, camera)?;

    let camera_node = scene.camera_node(camera).expect("camera node exists");
    let camera_z = scene
        .node(camera_node)
        .expect("camera node resolves")
        .transform()
        .translation
        .z;
    println!(
        "camera_framing node={inspected_part:?} camera_z={camera_z:.2} orbit_target={:?}",
        controls.target()
    );
    Ok(())
}
