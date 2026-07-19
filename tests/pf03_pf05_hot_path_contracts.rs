#![cfg(not(target_arch = "wasm32"))]

use std::sync::Arc;

use scena::{
    Assets, Color, GeometryDesc, GeometryTopology, GeometryVertex, MaterialDesc, PerspectiveCamera,
    Primitive, Renderer, Scene, TextureColorSpace, Transform, Vec3,
};

const INLINE_RED_PNG: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";

#[test]
fn warm_cpu_render_borrows_prepared_lists_without_cloning_them() {
    let mut scene = Scene::new();
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::IDENTITY,
        )
        .expect("triangle inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 1.732_050_8)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");
    let mut renderer = Renderer::headless(32, 32).expect("renderer creates");
    renderer.prepare(&mut scene).expect("scene prepares");

    renderer
        .render(&scene, camera)
        .expect("first render succeeds");
    let first = renderer.frame_rgba8().to_vec();
    renderer
        .render(&scene, camera)
        .expect("warm render succeeds");
    let metrics = renderer.last_render_work_metrics();

    assert_eq!(renderer.frame_rgba8(), first);
    assert_eq!(metrics.prepared_primitive_list_clones, 0);
    assert_eq!(metrics.prepared_stroke_list_clones, 0);
    assert_eq!(metrics.prepared_label_list_clones, 0);
    assert_eq!(metrics.prepared_list_clone_bytes, 0);
}

#[test]
fn prepared_geometry_shares_model_vertices_and_draw_transforms() {
    const TRIANGLE_COUNT: usize = 4_096;
    let assets = Assets::new();
    let mut vertices = Vec::with_capacity(TRIANGLE_COUNT * 3);
    let mut indices = Vec::with_capacity(TRIANGLE_COUNT * 3);
    for triangle in 0..TRIANGLE_COUNT {
        let base = u32::try_from(triangle * 3).expect("contract fixture index fits u32");
        let x = (triangle % 64) as f32 * 0.01;
        let y = (triangle / 64) as f32 * 0.01;
        vertices.extend([
            GeometryVertex {
                position: Vec3::new(x, y, 0.0),
                normal: Vec3::Z,
            },
            GeometryVertex {
                position: Vec3::new(x + 0.005, y, 0.0),
                normal: Vec3::Z,
            },
            GeometryVertex {
                position: Vec3::new(x, y + 0.005, 0.0),
                normal: Vec3::Z,
            },
        ]);
        indices.extend([base, base + 1, base + 2]);
    }
    let geometry = assets.create_geometry(
        GeometryDesc::try_new(GeometryTopology::Triangles, vertices, indices)
            .expect("contract geometry validates"),
    );
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .add()
        .expect("contract mesh inserts");
    let mut renderer = Renderer::headless(1, 1).expect("renderer creates");

    let metrics = renderer
        .prepare_with_assets_profiled(&mut scene, &assets)
        .expect("contract scene prepares");

    assert_eq!(metrics.prepared_triangle_count, TRIANGLE_COUNT as u64);
    assert_eq!(metrics.prepared_model_vertex_buffer_count, 1);
    assert_eq!(metrics.prepared_unique_draw_transforms, 1);
    assert!(metrics.prepared_model_vertex_bytes > 0);
    assert!(
        metrics.prepared_draw_transform_bytes < 8 * 16 * size_of::<f32>() as u64,
        "one node must own one compact forward/inverse transform set, not four matrices per triangle"
    );
    assert!(
        metrics.prepared_triangle_reference_bytes
            < TRIANGLE_COUNT as u64 * 4 * 16 * size_of::<f32>() as u64,
        "triangle records must contain shared references rather than duplicated matrices"
    );
    assert_eq!(
        metrics.prepared_list_copy_bytes, 0,
        "a full prepare must retain and draw through shared primitive-list ownership"
    );
    assert!(
        metrics.bytes_cloned_or_copied() >= metrics.prepared_model_vertex_bytes,
        "profiled prepare must include model-buffer materialization in its byte accounting"
    );
}

#[test]
fn asset_snapshots_are_shared_and_texture_bake_has_no_inner_loop_locks() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(triangle_geometry());
    let texture = pollster::block_on(assets.load_texture(INLINE_RED_PNG, TextureColorSpace::Srgb))
        .expect("inline texture loads");
    let material =
        assets.create_material(MaterialDesc::unlit(Color::WHITE).with_base_color_texture(texture));

    let geometry_a = assets
        .geometry_snapshot(geometry)
        .expect("geometry snapshot");
    let geometry_b = assets
        .geometry_snapshot(geometry)
        .expect("geometry snapshot repeats");
    let material_a = assets
        .material_snapshot(material)
        .expect("material snapshot");
    let material_b = assets
        .material_snapshot(material)
        .expect("material snapshot repeats");
    let texture_a = assets.texture_snapshot(texture).expect("texture snapshot");
    let texture_b = assets
        .texture_snapshot(texture)
        .expect("texture snapshot repeats");
    assert!(Arc::ptr_eq(&geometry_a, &geometry_b));
    assert!(Arc::ptr_eq(&material_a, &material_b));
    assert!(Arc::ptr_eq(&texture_a, &texture_b));

    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .add()
        .expect("textured mesh inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 1.732_050_8)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");
    let mut renderer = Renderer::headless(256, 256).expect("renderer creates");
    let before = assets.storage_lock_acquisitions();
    let metrics = renderer
        .prepare_with_assets_profiled(&mut scene, &assets)
        .expect("textured scene prepares");
    let actual_delta = assets.storage_lock_acquisitions().saturating_sub(before);

    assert_eq!(metrics.asset_storage_lock_acquisitions, actual_delta);
    assert!(
        metrics.texture_samples >= 500,
        "adaptive texture preparation must exercise hundreds of lock-free samples; samples={}",
        metrics.texture_samples
    );
    assert!(
        actual_delta < 64,
        "texture sampling must use prepared snapshots instead of one mutex acquisition per sample; locks={actual_delta}"
    );
}

fn triangle_geometry() -> GeometryDesc {
    GeometryDesc::try_new_with_vertex_colors_and_tex_coords(
        GeometryTopology::Triangles,
        vec![
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
        ],
        vec![0, 1, 2],
        vec![Color::WHITE; 3],
        vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
    )
    .expect("triangle geometry validates")
}

#[test]
fn resolved_scene_cache_rebuilds_once_per_relevant_revision() {
    let mut scene = Scene::new();
    let mut parent = scene.root();
    let mut midpoint = parent;
    for depth in 0..256 {
        parent = scene
            .add_empty(parent, Transform::at(Vec3::new(1.0, 0.0, 0.0)))
            .expect("deep node inserts");
        if depth == 127 {
            midpoint = parent;
        }
    }

    assert_eq!(scene.resolved_scene_cache_stats().rebuilds, 0);
    assert_eq!(
        scene
            .world_transform(parent)
            .expect("world transform")
            .translation
            .x,
        256.0
    );
    let built = scene.resolved_scene_cache_stats();
    assert_eq!(built.rebuilds, 1);
    assert_eq!(built.ancestor_vec_allocations, 0);
    for _ in 0..100 {
        assert!(scene.world_transform(parent).is_some());
        assert_eq!(scene.resolved_visibility(parent), Some(true));
    }
    assert_eq!(scene.resolved_scene_cache_stats().rebuilds, 1);

    scene
        .set_transform(midpoint, Transform::at(Vec3::new(2.0, 0.0, 0.0)))
        .expect("middle transform changes");
    assert_eq!(
        scene
            .world_transform(parent)
            .expect("world transform")
            .translation
            .x,
        257.0
    );
    assert_eq!(scene.resolved_scene_cache_stats().rebuilds, 2);
    scene
        .set_visible(midpoint, false)
        .expect("middle node hides");
    assert_eq!(scene.resolved_visibility(parent), Some(false));
    assert_eq!(scene.resolved_scene_cache_stats().rebuilds, 3);
}
