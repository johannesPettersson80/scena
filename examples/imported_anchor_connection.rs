use scena::{Assets, ConnectOptions, ConnectorFrame, Renderer, Scene, Transform, Vec3};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/anchor_debug_scene.gltf"))?;

    let mut scene = Scene::new();
    let source = scene.instantiate(&scene_asset)?;
    let target = scene.instantiate(&scene_asset)?;
    scene.set_transform(target.roots()[0], Transform::at(Vec3::new(1.0, 0.0, 0.0)))?;

    let source_anchor = scene.add_connector(
        ConnectorFrame::from_import_anchor(source.anchor("inspection")?).with_kind("mount"),
    )?;
    let target_anchor = scene.add_connector(
        ConnectorFrame::from_import_anchor(target.anchor("inspection")?).with_kind("mount"),
    )?;

    scene.connect_by_key(source_anchor, target_anchor, ConnectOptions::default())?;

    let camera = scene.add_default_camera()?;
    scene.frame_import(camera, &target)?;
    let mut renderer = Renderer::headless(320, 180)?;
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;

    println!("two imports connected by named inspection anchors");
    Ok(())
}
