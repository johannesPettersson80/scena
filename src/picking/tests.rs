use super::*;

#[test]
fn asset_picking_ignores_stroke_material_geometry() {
    let assets = crate::Assets::new();
    let geometry = assets.create_geometry(crate::GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material = assets.create_material(crate::MaterialDesc::wireframe(crate::Color::WHITE, 2.0));
    let mut scene = crate::Scene::new();
    let camera = scene.add_default_camera().expect("camera inserts");
    scene.mesh(geometry, material).add().expect("mesh inserts");
    let viewport = crate::Viewport::new(64, 64, 1.0).expect("viewport is valid");

    let hit = pick_scene_with_assets(
        &scene,
        &assets,
        camera,
        CursorPosition::physical(32.0, 32.0),
        viewport,
    )
    .expect("pick runs");

    assert_eq!(
        hit, None,
        "stroke materials are visible helpers in 1.7 but are not pickable geometry"
    );
}

#[test]
fn asset_picking_skips_stroke_overlay_and_returns_underlying_mesh() {
    let assets = crate::Assets::new();
    let mesh_geometry = assets.create_geometry(crate::GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let mesh_material = assets.create_material(crate::MaterialDesc::unlit(crate::Color::WHITE));
    let stroke_geometry = assets.create_geometry(crate::GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let stroke_material =
        assets.create_material(crate::MaterialDesc::wireframe(crate::Color::WHITE, 2.0));
    let mut scene = crate::Scene::new();
    let camera = scene.add_default_camera().expect("camera inserts");
    let mesh = scene
        .mesh(mesh_geometry, mesh_material)
        .add()
        .expect("mesh inserts");
    scene
        .mesh(stroke_geometry, stroke_material)
        .add()
        .expect("stroke inserts");
    let viewport = crate::Viewport::new(64, 64, 1.0).expect("viewport is valid");

    let hit = pick_scene_with_assets(
        &scene,
        &assets,
        camera,
        CursorPosition::physical(32.0, 32.0),
        viewport,
    )
    .expect("pick runs")
    .expect("underlying mesh is pickable through stroke overlay");

    assert_eq!(hit.target(), HitTarget::Node(mesh));
}

#[test]
fn primitive_bounds_rejects_ray_before_triangle_intersection() {
    let ray = Ray {
        origin: Vec3::ZERO,
        direction: Vec3::new(0.0, 0.0, -1.0),
    };
    let min = Vec3::new(10.0, 10.0, -4.0);
    let max = Vec3::new(11.0, 11.0, -3.0);

    assert!(!ray_hits_bounds(ray, min, max));
}

#[test]
fn primitive_bounds_accepts_ray_through_triangle_bounds() {
    let ray = Ray {
        origin: Vec3::ZERO,
        direction: Vec3::new(0.0, 0.0, -1.0),
    };
    let min = Vec3::new(-1.0, -1.0, -4.0);
    let max = Vec3::new(1.0, 1.0, -3.0);

    assert!(ray_hits_bounds(ray, min, max));
}
