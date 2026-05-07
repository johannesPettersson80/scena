use scena::{Assets, Color, GeometryDesc, MaterialDesc, Renderer, Scene};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/anchor_debug_scene.gltf"))?;
    let marker_geometry = assets.create_geometry(GeometryDesc::anchor_marker(0.2));
    let marker_material =
        assets.create_material(MaterialDesc::line(Color::from_srgb_u8(255, 220, 70), 1.0));

    let mut scene = Scene::new();
    let import = scene.instantiate(&scene_asset)?;
    let marker = scene.mesh(marker_geometry, marker_material).add()?;
    scene.snap_anchor(marker, import.anchor("inspection")?)?;

    let anchor_debug = import.anchor_debug_metadata()?;
    let camera = scene.add_default_camera()?;
    scene.frame_import(camera, &import)?;

    let mut renderer = Renderer::headless(320, 180)?;
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    println!("anchor_alignment anchors={}", anchor_debug.len());
    Ok(())
}
