use crate::assets::Assets;
use crate::geometry::GeometryDesc;
use crate::material::{Color, MaterialDesc};
use crate::scene::{Scene, Transform, Vec3};

use super::{PrepareTelemetry, Renderer};

impl Renderer {
    fn movement_prepare_telemetry_for_test(&self) -> PrepareTelemetry {
        self.prepare_telemetry
    }
}

#[test]
fn transform_only_gpu_prepare_updates_draw_uniforms_without_recollecting_primitives() {
    let Some(mut renderer) = headless_gpu_for_movement_test(48, 48) else {
        return;
    };
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.4, 0.4, 0.4));
    let material =
        assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 0.8));
    let mut scene = Scene::new();
    let camera = scene.add_default_camera().expect("camera inserts");
    let moving = scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(-0.4, 0.0, 0.0)))
        .add()
        .expect("first mesh inserts");

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("initial GPU prepare succeeds");
    renderer
        .render(&scene, camera)
        .expect("initial GPU render succeeds");
    let initial_x = visible_centroid_x(renderer.frame_rgba8(), 48)
        .expect("initial render must contain moving mesh pixels");
    let first = renderer.movement_prepare_telemetry_for_test();

    scene
        .set_transform(moving, Transform::at(Vec3::new(0.25, 0.0, 0.0)))
        .expect("mesh transform updates");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("transform-only GPU prepare succeeds");
    renderer
        .render(&scene, camera)
        .expect("moved GPU render succeeds");
    let moved_x = visible_centroid_x(renderer.frame_rgba8(), 48)
        .expect("moved render must contain mesh pixels");
    let second = renderer.movement_prepare_telemetry_for_test();

    assert_eq!(
        second.prepared_primitive_collections, first.prepared_primitive_collections,
        "transform-only GPU prepares must skip canonical primitive collection"
    );
    assert_eq!(
        second.static_gpu_resource_rebuilds, first.static_gpu_resource_rebuilds,
        "transform-only GPU prepares must reuse static GPU draw resources"
    );
    assert_eq!(
        second.dynamic_template_prepares,
        first.dynamic_template_prepares + 1,
        "transform-only GPU prepares must take the dynamic template path"
    );
    assert_eq!(
        second.draw_uniform_only_updates,
        first.draw_uniform_only_updates + 1,
        "transform-only GPU prepares must update per-draw uniforms"
    );
    assert!(
        moved_x > initial_x + 4.0,
        "transform-only dynamic prepare must move the rendered mesh pixels, not only update counters: initial_x={initial_x:.2}, moved_x={moved_x:.2}"
    );
}

#[test]
fn transform_only_cpu_prepare_moves_rendered_mesh_pixels() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.4, 0.4, 0.4));
    let material =
        assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 0.8));
    let mut scene = Scene::new();
    let camera = scene.add_default_camera().expect("camera inserts");
    let moving = scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(-0.4, 0.0, 0.0)))
        .add()
        .expect("moving mesh inserts");
    let mut renderer = Renderer::headless(48, 48).expect("CPU renderer builds");

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("initial CPU prepare succeeds");
    renderer
        .render(&scene, camera)
        .expect("initial CPU render succeeds");
    let initial_x = visible_centroid_x(renderer.frame_rgba8(), 48)
        .expect("initial CPU render must contain moving mesh pixels");

    scene
        .set_transform(moving, Transform::at(Vec3::new(0.25, 0.0, 0.0)))
        .expect("mesh transform updates");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("moved CPU prepare succeeds");
    renderer
        .render(&scene, camera)
        .expect("moved CPU render succeeds");
    let moved_x = visible_centroid_x(renderer.frame_rgba8(), 48)
        .expect("moved CPU render must contain moving mesh pixels");

    assert!(
        moved_x > initial_x + 4.0,
        "CPU prepare/render must move rendered mesh pixels: initial_x={initial_x:.2}, moved_x={moved_x:.2}"
    );
}

#[test]
fn line_geometry_gpu_prepare_updates_draw_uniforms_without_recollecting_strokes() {
    let Some(mut renderer) = headless_gpu_for_movement_test(48, 48) else {
        return;
    };
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::line(
        Vec3::new(0.0, -0.6, 0.0),
        Vec3::new(0.0, 0.6, 0.0),
    ));
    let material = assets.create_material(MaterialDesc::line(Color::WHITE, 3.0));
    let mut scene = Scene::new();
    let camera = scene.add_default_camera().expect("camera inserts");
    let line = scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(-0.35, 0.0, 0.0)))
        .add()
        .expect("line mesh inserts");

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("initial GPU prepare succeeds");
    renderer
        .render(&scene, camera)
        .expect("initial line render succeeds");
    let initial_x = visible_centroid_x(renderer.frame_rgba8(), 48)
        .expect("initial render must contain line pixels");
    let first = renderer.movement_prepare_telemetry_for_test();

    scene
        .set_transform(line, Transform::at(Vec3::new(0.35, 0.0, 0.0)))
        .expect("line transform updates");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("transform-only line prepare succeeds");
    renderer
        .render(&scene, camera)
        .expect("moved line render succeeds");
    let moved_x = visible_centroid_x(renderer.frame_rgba8(), 48)
        .expect("moved render must contain line pixels");
    let second = renderer.movement_prepare_telemetry_for_test();

    assert_eq!(
        second.prepared_primitive_collections, first.prepared_primitive_collections,
        "transform-only line prepares must reuse retained stroke segments"
    );
    assert_eq!(
        second.static_gpu_resource_rebuilds, first.static_gpu_resource_rebuilds,
        "transform-only line prepares must not re-upload stroke vertex buffers"
    );
    assert_eq!(
        second.draw_uniform_only_updates,
        first.draw_uniform_only_updates + 1,
        "transform-only line prepares must update stroke draw uniforms"
    );
    assert!(
        moved_x > initial_x + 8.0,
        "transform-only retained strokes must move rendered line pixels, not only update counters: initial_x={initial_x:.2}, moved_x={moved_x:.2}"
    );
}

fn visible_centroid_x(frame: &[u8], width: usize) -> Option<f32> {
    let mut weighted_x = 0.0_f32;
    let mut count = 0_u32;
    for (index, pixel) in frame.chunks_exact(4).enumerate() {
        if pixel[0] > 16 || pixel[1] > 16 || pixel[2] > 16 {
            weighted_x += (index % width) as f32;
            count += 1;
        }
    }
    (count > 0).then_some(weighted_x / count as f32)
}

fn headless_gpu_for_movement_test(width: u32, height: u32) -> Option<Renderer> {
    match Renderer::headless_gpu(width, height) {
        Ok(renderer) => Some(renderer),
        Err(error) if std::env::var_os("SCENA_USE_GPU").is_none() => {
            eprintln!("skipping GPU movement test without available HeadlessGpu: {error}");
            None
        }
        Err(error) => panic!(
            "SCENA_USE_GPU is set, so GPU movement proof must run instead of skipping: {error}"
        ),
    }
}
