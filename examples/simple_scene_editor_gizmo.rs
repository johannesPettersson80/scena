use scena::{
    Assets, Color, GeometryDesc, GizmoAxis, GizmoConstraint, GizmoMode, GizmoRay, MaterialDesc,
    Renderer, Scene, TransformGizmo, Vec3,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.8, 0.5, 0.4));
    let material = assets.create_material(MaterialDesc::pbr_metallic_roughness(
        Color::from_srgb_u8(96, 146, 210),
        0.0,
        0.55,
    ));

    let mut scene = Scene::new();
    scene.add_studio_lighting()?;
    let selected = scene.mesh(geometry, material).add()?;

    let gizmo = TransformGizmo::new(GizmoMode::Translate)
        .with_constraint(GizmoConstraint::Axis(GizmoAxis::X))
        .with_size(0.75);
    let start_ray = GizmoRay::new(Vec3::new(-0.25, 0.0, 4.0), Vec3::new(0.0, 0.0, -1.0))
        .expect("example start ray is valid");
    let current_ray = GizmoRay::new(Vec3::new(0.55, 0.0, 4.0), Vec3::new(0.0, 0.0, -1.0))
        .expect("example current ray is valid");
    let moved = gizmo
        .drag_transform(
            scene
                .node(selected)
                .expect("selected node exists")
                .transform(),
            start_ray,
            current_ray,
        )
        .expect("constrained drag produces a transform");
    scene.set_transform(selected, moved)?;
    let helpers = gizmo.add_helpers(&mut scene, &assets, selected)?;

    let camera = scene.add_default_camera()?;
    scene.frame_all_with_assets(camera, &assets)?;
    let mut renderer = Renderer::headless(320, 200)?;
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;

    println!(
        "simple_scene_editor_gizmo selected={selected:?} translation={:?} helper_nodes={}",
        moved.translation,
        helpers.nodes().len()
    );
    Ok(())
}
