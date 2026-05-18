use scena::{
    Aabb, Assets, Color, FramingOptions, GeometryDesc, MaterialDesc, OrbitControls,
    PerspectiveCamera, Renderer, Scene, Transform, Vec3,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.2, 0.4, 0.4));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(70, 160, 240)));

    let mut scene = Scene::new();
    let inspected_part = scene.mesh(geometry, material).add()?;
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::default().with_aspect(16.0 / 9.0),
        Transform::default(),
    )?;

    let bounds = Aabb::new(Vec3::new(-0.6, -0.2, -0.2), Vec3::new(0.6, 0.2, 0.2));
    let framing = scene.frame_bounds(
        camera,
        bounds,
        FramingOptions::new()
            .right()
            .fill(0.72)
            .margin_px(24.0)
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
