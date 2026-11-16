//! `Scene::pick` and the other picks that take no `Assets` must never silently
//! miss a visible asset-backed node.
//!
//! Without `Assets` the scene cannot intersect triangles, so a plain pick hits
//! mesh and instance-set nodes through the local bounds the scene recorded for
//! them (at glTF import or at the last prepare), carried through the node's
//! exact world matrix. When a visible asset-backed node has no recorded bounds,
//! the pick reports a structured error instead of pretending nothing is there.
#![cfg(not(target_arch = "wasm32"))]

use scena::{
    Assets, CameraKey, Color, CursorPosition, GeometryDesc, GeometryMorphTarget, HitTarget,
    LookupError, MaterialDesc, NodeKey, Quat, Renderer, Scene, Transform, Vec3, Viewport,
};

const SIZE: u32 = 96;

#[test]
fn plain_pick_reports_an_unprepared_mesh_node_instead_of_missing_it() {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let mesh = add_box(&mut scene, &assets, scene_root_transform());
    let camera = scene.add_default_camera().expect("camera inserts");

    let error = scene
        .pick(camera, center(), viewport())
        .expect_err("a visible mesh the scene knows nothing about must not read as a miss");

    assert_eq!(error, LookupError::PickRequiresAssets { node: mesh });
    assert!(
        error.to_string().contains("pick_with_assets"),
        "the message must name the asset-aware pick: {error}"
    );
    assert!(
        error.help().contains("pick_with_assets") && error.help().contains("prepare"),
        "the help must name both remedies: {}",
        error.help()
    );

    // The asset-aware pick was never affected and stays the triangle-accurate path.
    let hit = scene
        .pick_with_assets(camera, center(), viewport(), &assets)
        .expect("asset-aware pick runs")
        .expect("asset-aware pick hits the mesh");
    assert_eq!(hit.target(), HitTarget::Node(mesh));
}

#[test]
fn plain_pick_hits_a_prepared_mesh_through_its_recorded_bounds() {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let mesh = add_box(&mut scene, &assets, scene_root_transform());
    let camera = scene.add_default_camera().expect("camera inserts");
    prepare(&mut scene, &assets);

    let hit = scene
        .pick(camera, center(), viewport())
        .expect("plain pick runs once bounds are recorded")
        .expect("the cursor is dead centre on the mesh");

    assert_eq!(hit.target(), HitTarget::Node(mesh));
    // The box is its own bounds, so the bounds hit lands on its front face.
    assert!(
        hit.world_position
            .abs_diff_eq(Vec3::new(0.0, 0.0, 0.375), 1.0e-4),
        "expected the front face of the box, got {:?}",
        hit.world_position
    );
    assert!((hit.distance - 1.625).abs() <= 1.0e-4);
    assert_eq!(
        hit.normal, None,
        "a bounds hit has no surface to take a normal from"
    );

    let triangle_hit = scene
        .pick_with_assets(camera, center(), viewport(), &assets)
        .expect("asset-aware pick runs")
        .expect("asset-aware pick hits the mesh");
    assert_eq!(triangle_hit.target(), hit.target());
    assert!(triangle_hit.normal.is_some());

    assert_eq!(
        scene
            .pick(camera, CursorPosition::physical(2.0, 2.0), viewport())
            .expect("plain pick runs"),
        None,
        "a ray that passes the recorded bounds is a real miss"
    );
}

#[test]
fn plain_pick_convenience_entry_points_share_the_contract() {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let mesh = add_box(&mut scene, &assets, scene_root_transform());
    let camera = scene.add_default_camera().expect("camera inserts");
    let centre = SIZE as f32 * 0.5;

    for error in [
        scene
            .pick_pointer(camera, centre, centre, SIZE, SIZE, 1.0)
            .expect_err("pick_pointer must not miss silently"),
        scene
            .pick_and_select(camera, centre, centre, SIZE, SIZE, 1.0)
            .expect_err("pick_and_select must not miss silently"),
        scene
            .pick_and_hover(camera, centre, centre, SIZE, SIZE, 1.0)
            .expect_err("pick_and_hover must not miss silently"),
    ] {
        assert_eq!(error, LookupError::PickRequiresAssets { node: mesh });
    }
    assert_eq!(
        scene.interaction().primary_selection(),
        None,
        "a failed pick must not clear or set the selection"
    );

    prepare(&mut scene, &assets);
    let hit = scene
        .pick_and_select(camera, centre, centre, SIZE, SIZE, 1.0)
        .expect("pick_and_select runs once bounds are recorded")
        .expect("the cursor is on the mesh");
    assert_eq!(hit.target(), HitTarget::Node(mesh));
    assert_eq!(
        scene.interaction().primary_selection(),
        Some(HitTarget::Node(mesh))
    );
    scene
        .pick_and_hover(camera, centre, centre, SIZE, SIZE, 1.0)
        .expect("pick_and_hover runs once bounds are recorded");
    assert_eq!(scene.interaction().hover(), Some(HitTarget::Node(mesh)));
}

#[test]
fn plain_pick_hits_an_imported_gltf_mesh_without_a_prepare() {
    let assets = Assets::new();
    let scene_asset = pollster::block_on(
        assets.load_scene("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
    )
    .expect("mesh glTF loads");
    let mut scene = Scene::new();
    let import = scene.instantiate(&scene_asset).expect("glTF instantiates");
    let camera = scene.add_default_camera().expect("camera inserts");
    scene
        .frame_import(camera, &import)
        .expect("camera frames the import");

    let expected = scene
        .pick_with_assets(camera, center(), viewport(), &assets)
        .expect("asset-aware pick runs")
        .expect("asset-aware pick hits the imported mesh");
    let hit = scene
        .pick(camera, center(), viewport())
        .expect("the import recorded bounds for its mesh nodes")
        .expect("plain pick hits the imported mesh");

    assert_eq!(hit.target(), expected.target());
}

#[test]
fn plain_pick_carries_recorded_bounds_through_the_world_matrix() {
    // A rotated mesh under a non-uniformly scaled parent is sheared. The
    // recorded bounds must be carried through the exact world matrix: the
    // probe point below is inside the sheared box and outside the box any
    // translation/rotation/scale approximation of that matrix would give.
    let assets = Assets::new();
    let mut scene = Scene::new();
    let parent = scene
        .add_empty(
            scene.root(),
            Transform::IDENTITY.with_scale(Vec3::new(3.0, 1.0, 1.0)),
        )
        .expect("parent inserts");
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.6, 0.1, 0.1));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mesh = scene
        .mesh(geometry, material)
        .parent(parent)
        .transform(Transform {
            translation: Vec3::ZERO,
            rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_4),
            scale: Vec3::ONE,
        })
        .add()
        .expect("mesh inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    prepare(&mut scene, &assets);

    let world = scene.world_matrix(mesh).expect("mesh resolves");
    let on_the_bar = world.transform_point3(Vec3::new(0.27, 0.0, 0.0));
    let cursor = project(&scene, camera, on_the_bar);

    let expected = scene
        .pick_with_assets(camera, cursor, viewport(), &assets)
        .expect("asset-aware pick runs")
        .expect("the probe is on the sheared bar");
    let hit = scene
        .pick(camera, cursor, viewport())
        .expect("plain pick runs")
        .expect("the recorded bounds follow the sheared bar");

    assert_eq!(hit.target(), HitTarget::Node(mesh));
    assert_eq!(expected.target(), hit.target());
    assert!(
        (hit.distance - expected.distance).abs() <= 0.05,
        "the box is its own bounds, so both picks land on the same face: bounds={}, triangles={}",
        hit.distance,
        expected.distance
    );
}

#[test]
fn plain_pick_hits_instances_through_their_recorded_bounds() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.45, 0.45, 0.45));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    let (node, set) = scene
        .add_instance_set_node(scene.root(), geometry, material, Transform::IDENTITY)
        .expect("instance set inserts");
    let first = scene
        .push_instance(set, Transform::IDENTITY)
        .expect("instance inserts");
    scene
        .push_instance(set, Transform::at(Vec3::new(0.9, 0.0, 0.0)))
        .expect("second instance inserts");
    let camera = scene.add_default_camera().expect("camera inserts");

    assert_eq!(
        scene.pick(camera, center(), viewport()),
        Err(LookupError::PickRequiresAssets { node })
    );

    prepare(&mut scene, &assets);
    let hit = scene
        .pick(camera, center(), viewport())
        .expect("plain pick runs")
        .expect("the cursor is on the first instance");
    assert_eq!(
        hit.target(),
        HitTarget::Instance {
            node,
            instance: first
        }
    );
}

#[test]
fn plain_pick_ignores_stroke_and_hidden_nodes_like_the_asset_aware_pick() {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.75, 0.75, 0.75));
    let wireframe = assets.create_material(MaterialDesc::wireframe(Color::WHITE, 2.0));
    scene
        .mesh(geometry, wireframe)
        .add()
        .expect("wireframe helper inserts");
    let hidden = add_box(&mut scene, &assets, scene_root_transform());
    scene.set_visible(hidden, false).expect("mesh hides");
    let camera = scene.add_default_camera().expect("camera inserts");
    prepare(&mut scene, &assets);

    assert_eq!(
        scene
            .pick(camera, center(), viewport())
            .expect("stroke helpers and hidden nodes need no bounds"),
        None,
        "stroke materials are not pickable and hidden nodes are not visible"
    );
    assert_eq!(
        scene
            .pick_with_assets(camera, center(), viewport(), &assets)
            .expect("asset-aware pick runs"),
        None
    );
}

#[test]
fn plain_pick_does_not_trust_bounds_recorded_for_other_geometry_or_a_deformed_pose() {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let mesh = add_box(&mut scene, &assets, scene_root_transform());
    let camera = scene.add_default_camera().expect("camera inserts");
    prepare(&mut scene, &assets);
    assert!(scene.pick(camera, center(), viewport()).is_ok());

    // Swapping the material may turn the mesh into a stroke helper; the
    // recorded answer belongs to the old material until the next prepare.
    let other = assets.create_material(MaterialDesc::unlit(Color::RED));
    scene
        .set_mesh_material(mesh, other)
        .expect("material swaps");
    assert_eq!(
        scene.pick(camera, center(), viewport()),
        Err(LookupError::PickRequiresAssets { node: mesh })
    );
    prepare(&mut scene, &assets);
    assert!(scene.pick(camera, center(), viewport()).is_ok());

    // Recorded bounds describe the undeformed geometry. A morphed mesh can
    // leave them, so only the asset-aware pick can see it.
    let morphing = assets.create_geometry(
        GeometryDesc::box_xyz(0.2, 0.2, 0.2)
            .with_morph_targets(vec![GeometryMorphTarget::new(vec![
                Vec3::new(0.0, 0.5, 0.0);
                24
            ])])
            .expect("morph target matches the box"),
    );
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let morphed = scene
        .mesh(morphing, material)
        .transform(Transform::at(Vec3::new(0.0, -0.5, 0.0)))
        .add()
        .expect("morphing mesh inserts");
    scene
        .set_morph_weights(morphed, vec![1.0])
        .expect("morph weights apply");
    prepare(&mut scene, &assets);
    assert_eq!(
        scene.pick(camera, center(), viewport()),
        Err(LookupError::PickRequiresAssets { node: morphed })
    );
}

fn scene_root_transform() -> Transform {
    Transform::IDENTITY
}

fn add_box(scene: &mut Scene, assets: &Assets, transform: Transform) -> NodeKey {
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.75, 0.75, 0.75));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    scene
        .mesh(geometry, material)
        .transform(transform)
        .add()
        .expect("mesh inserts")
}

fn prepare(scene: &mut Scene, assets: &Assets) {
    Renderer::headless(SIZE, SIZE)
        .expect("CPU renderer builds")
        .prepare_with_assets(scene, assets)
        .expect("scene prepares");
}

fn viewport() -> Viewport {
    Viewport::new(SIZE, SIZE, 1.0).expect("viewport is valid")
}

fn center() -> CursorPosition {
    CursorPosition::physical(SIZE as f32 * 0.5, SIZE as f32 * 0.5)
}

fn project(scene: &Scene, camera: CameraKey, world: Vec3) -> CursorPosition {
    let point = scene
        .project_world_point(camera, world, SIZE, SIZE)
        .expect("camera resolves")
        .expect("the probe is in front of the camera");
    CursorPosition::physical(point.x, point.y)
}
