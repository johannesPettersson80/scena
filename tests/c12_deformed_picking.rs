use scena::{
    Assets, Color, CursorPosition, GeometryDesc, GeometryMorphTarget, GeometrySkin,
    GeometryTopology, GeometryVertex, HitTarget, MaterialDesc, OrthographicCamera, Scene,
    SceneSkinBinding, SkinningMatrix, Transform, Vec3, Viewport,
};

const WIDTH: u32 = 200;
const HEIGHT: u32 = 200;

#[test]
fn morph_picking_uses_the_rendered_vertex_pose() {
    let assets = Assets::new();
    let geometry = triangle_geometry()
        .with_morph_targets(vec![GeometryMorphTarget::new(vec![Vec3::X; 3])])
        .expect("morph target validates");
    let geometry = assets.create_geometry(geometry);
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    let mesh = scene
        .mesh(geometry, material)
        .add()
        .expect("morph mesh inserts");
    let camera = orthographic_camera(&mut scene);

    scene
        .set_morph_weights(mesh, [0.0])
        .expect("base morph pose sets");
    assert!(
        pick_world_x(&scene, &assets, camera, 1.0).is_none(),
        "the ray aimed at the deformed pose must miss the base pose"
    );

    scene
        .set_morph_weights(mesh, [1.0])
        .expect("deformed morph pose sets");
    assert!(
        pick_world_x(&scene, &assets, camera, 0.0).is_none(),
        "the rendered surface moved away from its base pose"
    );
    let hit = pick_world_x(&scene, &assets, camera, 1.0)
        .expect("the ray aimed at the rendered morph pose must hit");
    assert_eq!(hit.target(), HitTarget::Node(mesh));
    assert_hit_geometry(hit, Vec3::new(1.0, 0.0, 0.0), Vec3::Z);
}

#[test]
fn profiled_picking_reports_intersection_and_deformation_work() {
    let assets = Assets::new();
    let geometry = triangle_geometry()
        .with_morph_targets(vec![GeometryMorphTarget::new(vec![Vec3::X; 3])])
        .expect("morph target validates");
    let geometry = assets.create_geometry(geometry);
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    let mesh = scene
        .mesh(geometry, material)
        .add()
        .expect("profiled morph mesh inserts");
    scene
        .set_morph_weights(mesh, [1.0])
        .expect("profiled morph pose sets");
    let camera = orthographic_camera(&mut scene);
    let cursor = CursorPosition::physical(WIDTH as f32 * 0.75, HEIGHT as f32 * 0.5);
    let viewport = Viewport::new(WIDTH, HEIGHT, 1.0).expect("viewport validates");

    let (hit, metrics) = scene
        .pick_with_assets_profiled(camera, cursor, viewport, &assets)
        .expect("profiled pick succeeds");

    assert_eq!(
        hit.expect("deformed triangle hits").target(),
        HitTarget::Node(mesh)
    );
    assert_eq!(metrics.mesh_nodes_considered, 1);
    assert_eq!(metrics.triangles_considered, 1);
    assert_eq!(metrics.triangle_bounds_tests, 1);
    assert_eq!(metrics.ray_triangle_intersection_tests, 1);
    assert_eq!(metrics.deformed_vertices_materialized, 3);
    assert_eq!(
        metrics.deformed_vertex_bytes_materialized,
        3 * std::mem::size_of::<GeometryVertex>() as u64
    );
}

#[test]
fn skin_picking_uses_the_rendered_joint_pose() {
    let assets = Assets::new();
    let geometry = triangle_geometry()
        .with_skin(GeometrySkin::new(
            vec![[0, 0, 0, 0]; 3],
            vec![[1.0, 0.0, 0.0, 0.0]; 3],
        ))
        .expect("skin validates");
    let geometry = assets.create_geometry(geometry);
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    let joint = scene
        .add_empty(scene.root(), Transform::at(Vec3::X))
        .expect("joint inserts");
    let mesh = scene
        .mesh(geometry, material)
        .add()
        .expect("skinned mesh inserts");
    scene
        .set_skin_binding(
            mesh,
            SceneSkinBinding::new(vec![joint], vec![SkinningMatrix::IDENTITY]),
        )
        .expect("skin binding sets");
    let camera = orthographic_camera(&mut scene);

    assert!(
        pick_world_x(&scene, &assets, camera, 0.0).is_none(),
        "the joint moved the rendered surface away from the base pose"
    );
    let hit = pick_world_x(&scene, &assets, camera, 1.0)
        .expect("the ray aimed at the rendered skin pose must hit");
    assert_eq!(hit.target(), HitTarget::Node(mesh));
    assert_hit_geometry(hit, Vec3::new(1.0, 0.0, 0.0), Vec3::Z);
}

#[test]
fn instance_picking_composes_distinct_root_and_instance_transforms() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(triangle_geometry());
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    let root_transform = Transform {
        translation: Vec3::new(-1.0, 0.0, 0.0),
        scale: Vec3::new(2.0, 1.5, 1.0),
        ..Transform::IDENTITY
    };
    let (node, set) = scene
        .add_instance_set_node(scene.root(), geometry, material, root_transform)
        .expect("instance-set root inserts");
    let first = scene
        .push_instance(set, Transform::at(Vec3::new(0.5, 0.0, 0.0)))
        .expect("first instance inserts");
    let second = scene
        .push_instance(set, Transform::at(Vec3::new(1.0, 0.0, 0.0)))
        .expect("second instance inserts");
    let camera = orthographic_camera(&mut scene);

    let first_hit = pick_world_x(&scene, &assets, camera, 0.0).expect("first instance hits");
    assert_eq!(
        first_hit.target(),
        HitTarget::Instance {
            node,
            instance: first
        }
    );
    let second_hit = pick_world_x(&scene, &assets, camera, 1.0).expect("second instance hits");
    assert_eq!(
        second_hit.target(),
        HitTarget::Instance {
            node,
            instance: second
        }
    );
}

#[test]
fn picking_reports_world_distance_and_winding_normal_for_scaled_geometry() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(triangle_geometry());
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    let mesh = scene
        .mesh(geometry, material)
        .transform(Transform {
            translation: Vec3::new(0.0, 0.0, -2.0),
            scale: Vec3::new(-2.0, 0.5, 3.0),
            ..Transform::IDENTITY
        })
        .add()
        .expect("negative nonuniform mesh inserts");
    let camera = orthographic_camera(&mut scene);

    let hit = pick_world_x(&scene, &assets, camera, 0.0).expect("scaled triangle hits");
    assert_eq!(hit.target(), HitTarget::Node(mesh));
    assert_hit_geometry(hit, Vec3::new(0.0, 0.0, -2.0), -Vec3::Z);
    assert!((hit.distance - 5.0).abs() < 1.0e-5);

    scene
        .set_transform(
            mesh,
            Transform {
                scale: Vec3::new(0.0, 1.0, 1.0),
                ..Transform::IDENTITY
            },
        )
        .expect("singular transform is a valid scene state");
    assert!(
        pick_world_x(&scene, &assets, camera, 0.0).is_none(),
        "a singular transform that collapses a triangle has no hittable surface"
    );
}

fn triangle_geometry() -> GeometryDesc {
    GeometryDesc::try_new(
        GeometryTopology::Triangles,
        vec![vertex(-0.3, -0.3), vertex(0.3, -0.3), vertex(0.0, 0.3)],
        vec![0, 1, 2],
    )
    .expect("triangle validates")
}

fn vertex(x: f32, y: f32) -> GeometryVertex {
    GeometryVertex {
        position: Vec3::new(x, y, 0.0),
        normal: Vec3::Z,
    }
}

fn orthographic_camera(scene: &mut Scene) -> scena::CameraKey {
    scene
        .add_orthographic_camera(
            scene.root(),
            OrthographicCamera {
                left: -2.0,
                right: 2.0,
                bottom: -2.0,
                top: 2.0,
                near: 0.01,
                far: 20.0,
            },
            Transform::at(Vec3::new(0.0, 0.0, 3.0)),
        )
        .expect("orthographic camera inserts")
}

fn pick_world_x(
    scene: &Scene,
    assets: &Assets,
    camera: scena::CameraKey,
    world_x: f32,
) -> Option<scena::Hit> {
    let physical_x = (world_x + 2.0) / 4.0 * WIDTH as f32;
    scene
        .pick_with_assets(
            camera,
            CursorPosition::physical(physical_x, HEIGHT as f32 * 0.5),
            Viewport::new(WIDTH, HEIGHT, 1.0).expect("viewport validates"),
            assets,
        )
        .expect("asset-aware picking succeeds")
}

fn assert_hit_geometry(hit: scena::Hit, position: Vec3, normal: Vec3) {
    assert!(hit.world_position.abs_diff_eq(position, 1.0e-5));
    assert!(
        hit.normal
            .expect("nondegenerate triangle has a normal")
            .abs_diff_eq(normal, 1.0e-5)
    );
}
