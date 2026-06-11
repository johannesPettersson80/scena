use crate::assets::Assets;
use crate::geometry::GeometryDesc;
use crate::material::{AlphaMode, Color, MaterialDesc};
use crate::scene::{Scene, Transform, Vec3};

use super::{PrepareTelemetry, Renderer};

impl Renderer {
    fn phase4_prepare_telemetry_for_test(&self) -> PrepareTelemetry {
        self.prepare_telemetry
    }

    fn phase4_gpu_vertex_buffer_bytes_for_test(&self) -> Option<u64> {
        self.gpu
            .as_ref()
            .and_then(|gpu| gpu.vertex_buffer_bytes_for_test())
    }
}

#[test]
fn instance_transform_gpu_prepare_updates_instance_buffer_without_recollecting_primitives() {
    let Ok(mut renderer) = Renderer::headless_gpu(48, 24) else {
        return;
    };
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.25, 0.25, 0.25));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    scene.add_default_camera().expect("camera inserts");
    let set = scene
        .add_instance_set(scene.root(), geometry, material, Transform::IDENTITY)
        .expect("instance set inserts");
    let left = scene
        .push_instance(set, Transform::at(Vec3::new(-0.5, 0.0, 0.0)))
        .expect("left instance inserts");
    let middle = scene
        .push_instance(set, Transform::IDENTITY)
        .expect("middle instance inserts");
    scene
        .push_instance(set, Transform::at(Vec3::new(0.5, 0.0, 0.0)))
        .expect("right instance inserts");

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("initial GPU instance prepare succeeds");
    let first = renderer.phase4_prepare_telemetry_for_test();

    scene
        .set_instance_transform(set, left, Transform::at(Vec3::new(-0.35, 0.0, 0.0)))
        .expect("instance transform updates");
    scene
        .set_instance_visible(set, middle, false)
        .expect("middle instance hides");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("instance-only GPU prepare succeeds");
    let second = renderer.phase4_prepare_telemetry_for_test();

    assert_eq!(
        second.prepared_primitive_collections, first.prepared_primitive_collections,
        "transform/visibility-only instance prepares must skip canonical primitive collection"
    );
    assert_eq!(
        second.static_gpu_resource_rebuilds, first.static_gpu_resource_rebuilds,
        "transform/visibility-only instance prepares must not rebuild static GPU resources"
    );
    assert_eq!(
        second.draw_uniform_only_updates,
        first.draw_uniform_only_updates + 1,
        "instance-only prepares must update dynamic GPU draw state"
    );
    assert_eq!(
        renderer.stats().instances,
        2,
        "hidden middle instance must be filtered out of the drawn instance records"
    );
}

#[test]
fn gpu_stats_report_submission_and_instance_counts_without_renaming_legacy_aliases() {
    let Ok(mut renderer) = Renderer::headless_gpu(48, 48) else {
        return;
    };
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.25, 0.25, 0.25));
    let opaque = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let transparent = assets.create_material(
        MaterialDesc::unlit(Color::from_linear_rgba(0.2, 0.6, 1.0, 0.5))
            .with_alpha_mode(AlphaMode::Blend),
    );
    let mut scene = Scene::new();
    let camera = scene.add_default_camera().expect("camera inserts");
    scene
        .mesh(geometry, opaque)
        .transform(Transform::at(Vec3::new(-0.55, 0.0, 0.0)))
        .add()
        .expect("opaque mesh inserts");
    scene
        .mesh(geometry, transparent)
        .transform(Transform::at(Vec3::new(0.0, 0.0, 0.0)))
        .add()
        .expect("transparent mesh inserts");
    let set = scene
        .add_instance_set(
            scene.root(),
            geometry,
            opaque,
            Transform::at(Vec3::new(0.55, 0.0, 0.0)),
        )
        .expect("instance set inserts");
    for offset in [-0.2_f32, 0.0, 0.2] {
        scene
            .push_instance(set, Transform::at(Vec3::new(offset, 0.0, 0.0)))
            .expect("instance inserts");
    }

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("GPU stats scene prepares");
    renderer
        .render(&scene, camera)
        .expect("GPU stats scene renders");
    let stats = renderer.stats();

    assert_eq!(stats.draw_calls, stats.triangles);
    assert_eq!(stats.primitives, stats.triangles);
    assert_eq!(stats.instances, 3);
    assert!(
        stats.gpu_draw_submissions > 0 && stats.gpu_draw_submissions < stats.triangles,
        "gpu_draw_submissions must be counted at GPU submission sites, not copied from triangle count: {stats:?}"
    );
}

#[test]
fn instanced_sets_keep_vertex_buffer_bytes_identical_excluding_instance_buffer() {
    let Some(single) = prepared_instanced_vertex_bytes(1) else {
        return;
    };
    let Some(many) = prepared_instanced_vertex_bytes(32) else {
        return;
    };

    assert_eq!(single.instances, 1);
    assert_eq!(many.instances, 32);
    assert_eq!(
        single.vertex_buffer_bytes, many.vertex_buffer_bytes,
        "instancing must share one retained vertex template; per-instance growth belongs only in the instance buffer"
    );
    assert!(
        many.approximate_gpu_memory_bytes > single.approximate_gpu_memory_bytes,
        "overall GPU memory should still account for the larger instance buffer"
    );
}

#[derive(Debug, Clone, Copy)]
struct PreparedInstancedBytes {
    vertex_buffer_bytes: u64,
    approximate_gpu_memory_bytes: u64,
    instances: u64,
}

fn prepared_instanced_vertex_bytes(instance_count: usize) -> Option<PreparedInstancedBytes> {
    let Ok(mut renderer) = Renderer::headless_gpu(48, 48) else {
        return None;
    };
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.25, 0.25, 0.25));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    scene.add_default_camera().expect("camera inserts");
    let set = scene
        .add_instance_set(scene.root(), geometry, material, Transform::IDENTITY)
        .expect("instance set inserts");
    for index in 0..instance_count {
        scene
            .push_instance(set, Transform::at(Vec3::new(index as f32 * 0.01, 0.0, 0.0)))
            .expect("instance inserts");
    }

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("instanced scene prepares");
    let stats = renderer.stats();
    Some(PreparedInstancedBytes {
        vertex_buffer_bytes: renderer
            .phase4_gpu_vertex_buffer_bytes_for_test()
            .expect("GPU resources expose vertex bytes after prepare"),
        approximate_gpu_memory_bytes: stats.approximate_gpu_memory_bytes.unwrap_or_default(),
        instances: stats.instances,
    })
}
