#[cfg(feature = "scene-host")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut host = scena::SceneHostCore::headless(128, 128)?;
    let source =
        pollster::block_on(host.instantiate_url("tests/assets/gltf/connector_debug_scene.gltf"))?;
    let targets = pollster::block_on(
        host.instantiate_url("tests/assets/gltf/connector_browser_targets.gltf"),
    )?;

    let report = host.connector_browser_json(source, &[targets])?;
    let pretty: serde_json::Value = serde_json::from_str(&report)?;
    assert_eq!(pretty["schema"], scena::CONNECTOR_BROWSER_SCHEMA_V1);
    println!("{}", serde_json::to_string_pretty(&pretty)?);
    Ok(())
}

#[cfg(not(feature = "scene-host"))]
fn main() {
    eprintln!("run with --features scene-host");
}
