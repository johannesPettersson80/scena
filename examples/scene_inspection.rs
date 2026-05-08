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

    let report = scene.inspect_with_assets(&assets);
    println!(
        "nodes={} cameras={} lights={} anchors={} connectors={} visible_drawables={}",
        report.node_count(),
        report.camera_count(),
        report.light_count(),
        report.anchor_count(),
        report.connector_count(),
        report.visible_drawable_count()
    );
    for node in report.nodes() {
        println!(
            "{:?} kind={} visible={} local={:?} world={:?} mesh_geometry={:?} mesh_material={:?} material_preview={:?} camera={:?} tags={:?}",
            node.node(),
            node.kind(),
            node.visible(),
            node.transform(),
            node.world_transform(),
            node.mesh_geometry(),
            node.mesh_material(),
            node.material_preview(),
            node.camera(),
            node.tags()
        );
    }
    for draw in report.draw_list() {
        println!(
            "draw node={:?} instance={:?} geometry={:?} material={:?} topology={:?} primitives={} vertices={} indices={} world={:?}",
            draw.node(),
            draw.instance(),
            draw.geometry(),
            draw.material(),
            draw.topology(),
            draw.primitive_count(),
            draw.vertex_count(),
            draw.index_count(),
            draw.world_transform()
        );
    }
    for frustum in report.camera_frustums() {
        println!(
            "frustum camera={:?} node={:?} near={} far={} first_corner={:?}",
            frustum.camera(),
            frustum.node(),
            frustum.near(),
            frustum.far(),
            frustum.corners()[0]
        );
    }
    for normals in report.normal_overlays() {
        println!(
            "normals node={:?} instance={:?} geometry={:?} segments={} length={}",
            normals.node(),
            normals.instance(),
            normals.geometry(),
            normals.segments().len(),
            normals.length()
        );
    }

    Ok(())
}
