use scena::{ConnectOptions, ConnectorFrame, Scene, Transform, Vec3};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut scene = Scene::new();

    let motor = scene.add_empty(scene.root(), Transform::IDENTITY)?;
    let pump = scene.add_empty(scene.root(), Transform::at(Vec3::new(2.0, 0.0, 0.0)))?;

    let motor_shaft = scene.add_connector(
        ConnectorFrame::new(motor, Transform::at(Vec3::new(0.5, 0.0, 0.0))).named("shaft"),
    )?;
    let pump_drive = scene.add_connector(
        ConnectorFrame::new(pump, Transform::at(Vec3::new(-0.25, 0.0, 0.0))).named("drive"),
    )?;

    scene.connect_by_key(motor_shaft, pump_drive, ConnectOptions::default())?;

    let motor_position = scene
        .world_transform(motor)
        .expect("connected node remains in the scene")
        .translation;
    println!("motor connected at {motor_position:?}");
    Ok(())
}
