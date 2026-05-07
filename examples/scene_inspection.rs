use scena::{Assets, Color, GeometryDesc, MaterialDesc, Scene, Transform, Vec3};

fn main() -> scena::Result<()> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.4, 0.4, 0.4));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(84, 180, 140)));

    let mut scene = Scene::new();
    let part = scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(0.0, 0.0, 0.0)))
        .add()?;
    scene.add_tag(part, "inspectable")?;
    scene.set_render_group(part, 1)?;
    scene.add_default_camera()?;

    let report = scene.inspect();
    println!(
        "nodes={} visible_drawables={}",
        report.node_count(),
        report.visible_drawable_count()
    );
    for node in report.nodes() {
        println!(
            "{:?} kind={} visible={} tags={:?}",
            node.node(),
            node.kind(),
            node.visible(),
            node.tags()
        );
    }

    Ok(())
}
