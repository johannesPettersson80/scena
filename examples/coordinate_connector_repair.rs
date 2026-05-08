use scena::{
    Assets, ConnectOptions, ConnectionError, ConnectorFrame, Scene, SourceCoordinateSystem,
    Transform,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/connector_zup_scene.gltf"))?;

    let mut scene = Scene::new();
    let source = scene.add_empty(scene.root(), Transform::IDENTITY)?;
    let wrong_import = scene.instantiate_with(
        &scene_asset,
        scena::ImportOptions::gltf_default()
            .with_source_coordinate_system(SourceCoordinateSystem::YUpLeftHanded),
    )?;

    let error = scene
        .connect(
            ConnectorFrame::new(source, Transform::IDENTITY).named("source"),
            ConnectorFrame::from_import_connector(wrong_import.connector("z-up-mount")?),
            ConnectOptions::default(),
        )
        .expect_err("left-handed import must be repaired before connecting");
    match error {
        ConnectionError::HandednessMismatch {
            coordinate_system, ..
        } => {
            println!("rejected connector from {coordinate_system:?}; re-importing as right-handed")
        }
        other => return Err(Box::new(other)),
    }

    let repaired_import = scene.instantiate_with(
        &scene_asset,
        scena::ImportOptions::gltf_default()
            .with_source_coordinate_system(SourceCoordinateSystem::ZUpRightHanded),
    )?;
    scene.connect(
        ConnectorFrame::new(source, Transform::IDENTITY).named("source"),
        ConnectorFrame::from_import_connector(repaired_import.connector("z-up-mount")?),
        ConnectOptions::default(),
    )?;

    println!("connector repaired with explicit source coordinate metadata");
    Ok(())
}
