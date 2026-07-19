use scena::{
    Assets, Color, CursorPosition, DirectionalLight, GeometryDesc, GeometryMorphTarget,
    GeometryTopology, GeometryVertex, MaterialDesc, OrthographicCamera, Renderer, Scene, Transform,
    Vec3, Viewport,
};

const GRID_EDGE: usize = 64;
const TRIANGLE_COUNT: u64 = (GRID_EDGE * GRID_EDGE * 2) as u64;

#[test]
fn pf06_static_geometry_reuses_bvh_and_reduces_triangle_tests_sublinearly() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(grid_geometry(false));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let (scene, camera) = pick_scene(geometry, material, false);
    let cursor = CursorPosition::physical(128.0, 128.0);
    let viewport = Viewport::new(256, 256, 1.0).expect("viewport");

    let (first_hit, first) = scene
        .pick_with_assets_profiled(camera, cursor, viewport, &assets)
        .expect("first profiled pick");
    let (second_hit, second) = scene
        .pick_with_assets_profiled(camera, cursor, viewport, &assets)
        .expect("second profiled pick");

    assert_eq!(first_hit, second_hit);
    assert!(first_hit.is_some());
    assert_eq!(first.static_bvh_cache_misses, 1);
    assert_eq!(first.static_bvh_cache_hits, 0);
    assert_eq!(second.static_bvh_cache_hits, 1);
    assert_eq!(second.static_bvh_cache_misses, 0);
    assert_eq!(first.deformed_bvh_builds, 0);
    assert!(first.mesh_bounds_tests > 0);
    assert!(first.bvh_node_bounds_tests > 0);
    assert!(
        first.ray_triangle_intersection_tests < TRIANGLE_COUNT / 8,
        "BVH should reduce {} triangles to far fewer exact tests: {first:?}",
        TRIANGLE_COUNT
    );
    assert_eq!(
        first.ray_triangle_intersection_tests,
        second.ray_triangle_intersection_tests
    );
}

#[test]
fn pf06_deformed_geometry_rebuilds_from_current_pose_without_using_static_bvh() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(grid_geometry(true));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let (scene, camera) = pick_scene(geometry, material, true);

    let (hit, metrics) = scene
        .pick_with_assets_profiled(
            camera,
            CursorPosition::physical(128.0, 128.0),
            Viewport::new(256, 256, 1.0).expect("viewport"),
            &assets,
        )
        .expect("deformed profiled pick");

    assert!(hit.is_some());
    assert_eq!(metrics.static_bvh_cache_hits, 0);
    assert_eq!(metrics.static_bvh_cache_misses, 0);
    assert_eq!(metrics.deformed_bvh_builds, 1);
    assert!(metrics.ray_triangle_intersection_tests < TRIANGLE_COUNT / 8);
}

#[test]
fn pf06_bvh_preserves_original_triangle_order_for_equal_distance_ties() {
    let assets = Assets::new();
    let vertices = vec![
        GeometryVertex {
            position: Vec3::new(-0.5, -0.5, 0.0),
            normal: Vec3::Z,
        },
        GeometryVertex {
            position: Vec3::new(0.5, -0.5, 0.0),
            normal: Vec3::Z,
        },
        GeometryVertex {
            position: Vec3::new(0.0, 0.5, 0.0),
            normal: Vec3::Z,
        },
    ];
    let geometry = assets.create_geometry(
        GeometryDesc::try_new(
            GeometryTopology::Triangles,
            vertices,
            vec![0, 1, 2, 0, 2, 1],
        )
        .expect("coincident opposite-winding triangles"),
    );
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let (scene, camera) = pick_scene(geometry, material, false);

    let hit = scene
        .pick_with_assets(
            camera,
            CursorPosition::physical(128.0, 128.0),
            Viewport::new(256, 256, 1.0).expect("viewport"),
            &assets,
        )
        .expect("pick")
        .expect("coincident triangles hit");

    assert_eq!(hit.normal, Some(Vec3::Z));
}

#[test]
fn pf06_prepare_scoped_shadow_cache_reuses_shared_deformed_world_positions() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(grid_geometry(false));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    scene.mesh(geometry, material).add().expect("grid mesh");
    scene
        .directional_light(DirectionalLight::default().with_shadows(true))
        .add()
        .expect("shadowed light");
    let mut renderer = Renderer::headless(32, 32).expect("renderer");

    let metrics = renderer
        .prepare_with_assets_profiled(&mut scene, &assets)
        .expect("profiled prepare");

    assert!(metrics.shadow_visibility_cache_misses > 0, "{metrics:?}");
    assert!(
        metrics.shadow_visibility_cache_hits > metrics.shadow_visibility_cache_misses,
        "shared indexed vertices should reuse prepare-scoped shadow results: {metrics:?}"
    );
    assert_eq!(
        metrics.shadow_rays, metrics.shadow_visibility_cache_misses,
        "only first use of each world position should cast a directional shadow ray"
    );
}

fn pick_scene(
    geometry: scena::GeometryHandle,
    material: scena::MaterialHandle,
    deformed: bool,
) -> (Scene, scena::CameraKey) {
    let mut scene = Scene::new();
    let mesh = scene.mesh(geometry, material).add().expect("mesh");
    if deformed {
        scene.set_morph_weights(mesh, [1.0]).expect("morph pose");
    }
    let camera = scene
        .add_orthographic_camera(
            scene.root(),
            OrthographicCamera {
                left: -4.0,
                right: 4.0,
                bottom: -4.0,
                top: 4.0,
                near: 0.01,
                far: 20.0,
            },
            Transform::at(Vec3::new(0.013, 0.017, 3.0)),
        )
        .expect("camera");
    (scene, camera)
}

fn grid_geometry(with_morph: bool) -> GeometryDesc {
    let mut vertices = Vec::with_capacity((GRID_EDGE + 1) * (GRID_EDGE + 1));
    for y in 0..=GRID_EDGE {
        for x in 0..=GRID_EDGE {
            vertices.push(GeometryVertex {
                position: Vec3::new(
                    x as f32 / GRID_EDGE as f32 * 8.0 - 4.0,
                    y as f32 / GRID_EDGE as f32 * 8.0 - 4.0,
                    0.0,
                ),
                normal: Vec3::Z,
            });
        }
    }
    let mut indices = Vec::with_capacity(GRID_EDGE * GRID_EDGE * 6);
    for y in 0..GRID_EDGE {
        for x in 0..GRID_EDGE {
            let row = GRID_EDGE + 1;
            let a = (y * row + x) as u32;
            let b = a + 1;
            let c = a + row as u32;
            let d = c + 1;
            indices.extend_from_slice(&[a, b, d, a, d, c]);
        }
    }
    let geometry = GeometryDesc::try_new(GeometryTopology::Triangles, vertices, indices)
        .expect("grid geometry");
    if with_morph {
        geometry
            .with_morph_targets(vec![GeometryMorphTarget::new(vec![
                Vec3::new(0.0, 0.0, 0.1);
                (GRID_EDGE + 1)
                    * (GRID_EDGE + 1)
            ])])
            .expect("grid morph")
    } else {
        geometry
    }
}
