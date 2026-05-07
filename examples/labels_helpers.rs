use scena::{
    Aabb, Assets, Color, GeometryDesc, LabelDesc, MaterialDesc, PerspectiveCamera, Renderer, Scene,
    Transform, Vec3,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new();
    let axes = assets.create_geometry(GeometryDesc::axes(1.0));
    let bounds = assets.create_geometry(GeometryDesc::bounding_box(Aabb::new(
        Vec3::new(-0.5, -0.5, -0.5),
        Vec3::new(0.5, 0.5, 0.5),
    )));
    let anchor = assets.create_geometry(GeometryDesc::anchor_marker(0.15));
    let material =
        assets.create_material(MaterialDesc::line(Color::from_srgb_u8(200, 220, 255), 1.0));

    let mut scene = Scene::new();
    scene.mesh(axes, material).add()?;
    scene.mesh(bounds, material).add()?;
    scene.mesh(anchor, material).add()?;
    let label = LabelDesc::msdf("origin")
        .with_color(Color::from_srgb_u8(255, 255, 255))
        .with_size(14.0);
    scene.add_label(
        scene.root(),
        label,
        Transform {
            translation: Vec3::new(0.0, 0.15, 0.0),
            ..Transform::default()
        },
    )?;
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::default(),
        Transform::default(),
    )?;
    scene.set_active_camera(camera)?;

    let mut renderer = Renderer::headless(200, 120)?;
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;
    println!("labels_helpers labels=1");
    Ok(())
}
