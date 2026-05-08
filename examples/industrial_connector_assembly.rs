use scena::{
    Assets, ConnectOptions, ConnectionAlignment, ConnectorFrame, Renderer, Scene, Transform, Vec3,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new();
    let part_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/connector_debug_scene.gltf"))?;

    let mut scene = Scene::new();
    let base = scene.instantiate(&part_asset)?;
    let pump = scene.instantiate(&part_asset)?;
    let sensor = scene.instantiate(&part_asset)?;

    scene.set_transform(base.roots()[0], Transform::at(Vec3::new(0.0, 0.0, 0.0)))?;
    scene.set_transform(pump.roots()[0], Transform::at(Vec3::new(1.0, 0.0, 0.0)))?;
    scene.set_transform(sensor.roots()[0], Transform::at(Vec3::new(2.0, 0.0, 0.0)))?;
    scene.lock_node_for_connections(base.roots()[0])?;

    let base_mount = ConnectorFrame::from_import_connector(base.connector("mount")?);
    let pump_mount = ConnectorFrame::from_import_connector(pump.connector("mount")?);
    let sensor_mount = ConnectorFrame::from_import_connector(sensor.connector("mount")?);

    let options = ConnectOptions::default().with_alignment(ConnectionAlignment::ForwardToBack);
    scene.connect(pump_mount.clone(), base_mount, options)?;
    scene.connect(
        sensor_mount,
        pump_mount,
        options.with_mate_offset(Transform::at(Vec3::new(0.4, 0.0, 0.0))),
    )?;

    let camera = scene.add_default_camera()?;
    scene.frame_import(camera, &base)?;

    let mut renderer = Renderer::headless(360, 200)?;
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;

    println!("assembled three imported parts with typed connector metadata");
    Ok(())
}
