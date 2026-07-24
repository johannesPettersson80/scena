use crate::assets::Assets;
use crate::geometry::{GeometryDesc, Primitive, Vertex};
use crate::material::{Color, MaterialDesc};
use crate::platform::SurfaceEvent;
use crate::reference_image::{ReferenceImage, ReferenceImageTolerance, regress_with_tolerance};
use crate::scene::{DirectionalLight, NodeKey, Scene, Transform, Vec3};

use super::{PrepareTelemetry, Renderer, gpu::GpuDeviceState};

impl Renderer {
    fn prepare_telemetry_for_test(&self) -> PrepareTelemetry {
        self.prepare_telemetry
    }

    fn gpu_draw_vertex_ranges_for_test(&self) -> Vec<(u32, u32)> {
        self.gpu
            .as_ref()
            .map(GpuDeviceState::draw_vertex_ranges_for_test)
            .unwrap_or_default()
    }
}

#[test]
fn headless_gpu_line_geometry_renders_retained_strokes() {
    let Ok(mut renderer) = Renderer::headless_gpu(64, 64) else {
        return;
    };
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::line(
        Vec3::new(-0.6, 0.0, 0.0),
        Vec3::new(0.6, 0.0, 0.0),
    ));
    let material = assets.create_material(MaterialDesc::line(Color::WHITE, 3.0));
    let mut scene = Scene::new();
    let camera = scene.add_default_camera().expect("camera inserts");
    scene.mesh(geometry, material).add().expect("line inserts");

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("GPU line prepare succeeds");
    renderer.render(&scene, camera).expect("GPU line renders");

    assert!(
        renderer
            .frame_rgba8()
            .chunks_exact(4)
            .any(|pixel| pixel[0] > 16 || pixel[1] > 16 || pixel[2] > 16),
        "retained GPU line path must draw visible stroke pixels"
    );
}

#[test]
fn opaque_tint_gpu_prepare_updates_draw_uniforms_without_recollecting_primitives() {
    let Ok(mut renderer) = Renderer::headless_gpu(32, 32) else {
        return;
    };
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.7, 0.7, 0.7));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    let camera = scene.add_default_camera().expect("camera inserts");
    let mesh = scene.mesh(geometry, material).add().expect("mesh inserts");

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("initial GPU prepare succeeds");
    renderer
        .render(&scene, camera)
        .expect("initial GPU render succeeds");
    let first_frame = renderer.frame_rgba8().to_vec();
    let first_telemetry = renderer.prepare_telemetry_for_test();
    let first_dirty = scene.dirty_state();

    scene
        .set_node_tint(mesh, Some(Color::from_linear_rgba(1.0, 0.0, 0.0, 1.0)))
        .expect("opaque tint updates");
    let tinted_dirty = scene.dirty_state();
    assert_eq!(
        tinted_dirty.structure_revision, first_dirty.structure_revision,
        "opaque tint must not bump structure_revision"
    );
    assert!(
        tinted_dirty.appearance_revision > first_dirty.appearance_revision,
        "opaque tint must bump appearance_revision"
    );

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("opaque-tint GPU prepare succeeds");
    renderer
        .render(&scene, camera)
        .expect("tinted GPU render succeeds");
    let second_telemetry = renderer.prepare_telemetry_for_test();

    assert_eq!(
        second_telemetry.prepared_primitive_collections,
        first_telemetry.prepared_primitive_collections,
        "opaque tint prepares must skip canonical primitive collection"
    );
    assert_eq!(
        second_telemetry.static_gpu_resource_rebuilds, first_telemetry.static_gpu_resource_rebuilds,
        "opaque tint prepares must not rebuild vertex buffers or static GPU draw resources"
    );
    assert_eq!(
        second_telemetry.draw_uniform_only_updates,
        first_telemetry.draw_uniform_only_updates + 1,
        "opaque tint prepares update the retained draw uniforms"
    );
    assert_ne!(
        renderer.frame_rgba8(),
        first_frame.as_slice(),
        "opaque tint must change rendered pixels"
    );
}

#[test]
fn translucent_tint_stays_structural() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.4, 0.4, 0.4));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    let mesh = scene.mesh(geometry, material).add().expect("mesh inserts");
    let before = scene.dirty_state();

    scene
        .set_node_tint(mesh, Some(Color::from_linear_rgba(1.0, 0.0, 0.0, 0.5)))
        .expect("translucent tint updates");
    let after = scene.dirty_state();

    assert!(
        after.structure_revision > before.structure_revision,
        "translucent tint flips material pass and must remain structural"
    );
    assert_eq!(
        after.appearance_revision, before.appearance_revision,
        "translucent tint is not an appearance-only fast path"
    );
}

#[test]
fn cpu_and_gpu_opaque_tint_render_within_reference_tolerance() {
    let Ok(mut gpu_renderer) = Renderer::headless_gpu(32, 32) else {
        return;
    };
    let (assets, mut cpu_scene, camera, mesh) = tinted_parity_scene();
    let (gpu_assets, mut gpu_scene, gpu_camera, gpu_mesh) = tinted_parity_scene();

    let mut cpu_renderer = Renderer::headless(32, 32).expect("CPU renderer builds");
    cpu_renderer
        .prepare_with_assets(&mut cpu_scene, &assets)
        .expect("CPU baseline prepares");
    cpu_renderer
        .render(&cpu_scene, camera)
        .expect("CPU baseline renders");
    let cpu_baseline = cpu_renderer.frame_rgba8().to_vec();

    gpu_renderer
        .prepare_with_assets(&mut gpu_scene, &gpu_assets)
        .expect("GPU baseline prepares");
    gpu_renderer
        .render(&gpu_scene, gpu_camera)
        .expect("GPU baseline renders");
    let gpu_baseline = gpu_renderer.frame_rgba8().to_vec();

    let tint = Color::from_linear_rgba(0.25, 1.0, 0.35, 1.0);
    cpu_scene
        .set_node_tint(mesh, Some(tint))
        .expect("CPU tint sets");
    gpu_scene
        .set_node_tint(gpu_mesh, Some(tint))
        .expect("GPU tint sets");

    cpu_renderer
        .prepare_with_assets(&mut cpu_scene, &assets)
        .expect("CPU tinted prepare succeeds");
    cpu_renderer
        .render(&cpu_scene, camera)
        .expect("CPU tinted render succeeds");
    let cpu_tinted = cpu_renderer.frame_rgba8().to_vec();

    gpu_renderer
        .prepare_with_assets(&mut gpu_scene, &gpu_assets)
        .expect("GPU tinted prepare succeeds");
    gpu_renderer
        .render(&gpu_scene, gpu_camera)
        .expect("GPU tinted render succeeds");
    let gpu_tinted = gpu_renderer.frame_rgba8().to_vec();

    let cpu_delta = frame_abs_diff(&cpu_baseline, &cpu_tinted);
    let gpu_delta = frame_abs_diff(&gpu_baseline, &gpu_tinted);
    assert!(cpu_delta > 0, "CPU tint path must change rendered pixels");
    assert!(gpu_delta > 0, "GPU tint path must change rendered pixels");
    assert_delta_within_ratio(cpu_delta, gpu_delta, 0.25);

    assert_reference_images_close("baseline", &cpu_baseline, &gpu_baseline);
    assert_reference_images_close("tinted", &cpu_tinted, &gpu_tinted);
    assert_reference_images_close(
        "opaque tint delta",
        &frame_abs_diff_rgba8(&cpu_baseline, &cpu_tinted),
        &frame_abs_diff_rgba8(&gpu_baseline, &gpu_tinted),
    );
}

#[test]
fn visibility_middle_primitive_reencodes_batches_without_vertex_reupload() {
    let Ok(mut renderer) = Renderer::headless_gpu(48, 24) else {
        return;
    };
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.22, 0.22, 0.22));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    scene.add_default_camera().expect("camera inserts");
    scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(-0.45, 0.0, 0.0)))
        .add()
        .expect("left mesh inserts");
    let middle = scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(0.0, 0.0, 0.0)))
        .add()
        .expect("middle mesh inserts");
    scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(0.45, 0.0, 0.0)))
        .add()
        .expect("right mesh inserts");

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("initial GPU prepare succeeds");
    renderer
        .render(&scene, scene.active_camera().expect("camera is active"))
        .expect("initial visibility render succeeds");
    let visible_center_pixels = bright_pixels_in_region(renderer.frame_rgba8(), 48, 20, 8, 8, 8);
    let visible_pixels = bright_pixels_in_region(renderer.frame_rgba8(), 48, 0, 0, 48, 24);
    let first = renderer.prepare_telemetry_for_test();
    let initial_ranges = renderer.gpu_draw_vertex_ranges_for_test();
    assert!(
        initial_ranges.contains(&(36, 36)),
        "the middle cube must occupy a non-prefix/non-suffix retained vertex range"
    );
    assert!(
        visible_center_pixels > 4,
        "initial render must show the middle primitive in the center region, got {visible_center_pixels} bright pixels"
    );

    scene.set_visible(middle, false).expect("middle hides");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("visibility-only GPU prepare succeeds");
    renderer
        .render(&scene, scene.active_camera().expect("camera is active"))
        .expect("hidden visibility render succeeds");
    let hidden_pixels = bright_pixels_in_region(renderer.frame_rgba8(), 48, 0, 0, 48, 24);
    let second = renderer.prepare_telemetry_for_test();
    let hidden_ranges = renderer.gpu_draw_vertex_ranges_for_test();

    assert_eq!(
        second.prepared_primitive_collections, first.prepared_primitive_collections,
        "visibility-only prepares must not re-collect primitives"
    );
    assert_eq!(
        second.static_gpu_resource_rebuilds, first.static_gpu_resource_rebuilds,
        "visibility-only prepares must not re-upload vertex buffers"
    );
    assert_eq!(
        hidden_ranges,
        vec![(0, 36), (72, 36)],
        "hiding the middle primitive must preserve original vertex offsets and break batch contiguity"
    );
    assert!(
        visible_pixels.saturating_sub(hidden_pixels) >= 4,
        "visibility-only dynamic prepare must remove the hidden primitive from rendered pixels, not only update counters: visible_pixels={visible_pixels}, hidden_pixels={hidden_pixels}"
    );
}

#[test]
fn target_change_rejects_transform_only_gpu_template_reuse() {
    let Ok(mut renderer) = Renderer::headless_gpu(16, 16) else {
        return;
    };
    let (assets, mut scene, moving) = gpu_template_scene();

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("initial GPU prepare succeeds");
    let first = renderer.prepare_telemetry_for_test();

    renderer
        .handle_surface_event(SurfaceEvent::Resize {
            width: 24,
            height: 16,
        })
        .expect("target resizes");
    scene
        .set_transform(moving, Transform::at(Vec3::new(-0.15, 0.0, 0.0)))
        .expect("mesh transform updates");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("target-changed prepare succeeds");
    let second = renderer.prepare_telemetry_for_test();

    assert!(
        second.prepared_primitive_collections > first.prepared_primitive_collections,
        "target changes must force a full prepare instead of a dynamic draw-template update"
    );
}

#[test]
fn minimized_zero_resize_preserves_last_valid_target_until_resume() {
    let mut renderer = Renderer::headless(16, 12).expect("renderer builds");
    renderer
        .handle_surface_event(SurfaceEvent::Resize {
            width: 0,
            height: 0,
        })
        .expect("minimized zero extent is accepted");
    assert_eq!(renderer.stats().target_width, 16);
    assert_eq!(renderer.stats().target_height, 12);

    renderer
        .handle_surface_event(SurfaceEvent::Resize {
            width: 24,
            height: 18,
        })
        .expect("non-zero resume extent applies");
    assert_eq!(renderer.stats().target_width, 24);
    assert_eq!(renderer.stats().target_height, 18);
}

#[test]
fn environment_changes_reject_transform_only_gpu_template_reuse() {
    let Ok(mut renderer) = Renderer::headless_gpu(16, 16) else {
        return;
    };
    let (assets, mut scene, moving) = gpu_template_scene();

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("initial GPU prepare succeeds");
    let first = renderer.prepare_telemetry_for_test();

    renderer.set_environment(assets.default_environment());
    scene
        .set_transform(moving, Transform::at(Vec3::new(-0.15, 0.0, 0.0)))
        .expect("mesh transform updates");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("environment-changed prepare succeeds");
    let second = renderer.prepare_telemetry_for_test();

    assert!(
        second.prepared_primitive_collections > first.prepared_primitive_collections,
        "environment changes must force a full prepare"
    );
    renderer.clear_environment();
    scene
        .set_transform(moving, Transform::at(Vec3::new(0.0, 0.0, 0.0)))
        .expect("mesh transform updates again");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("environment-clear prepare succeeds");
    let third = renderer.prepare_telemetry_for_test();

    assert!(
        third.prepared_primitive_collections > second.prepared_primitive_collections,
        "environment changes must keep forcing full prepare"
    );
}

#[test]
fn shadow_state_change_rejects_transform_only_gpu_template_reuse() {
    let Ok(mut renderer) = Renderer::headless_gpu(16, 16) else {
        return;
    };
    let (assets, mut scene, _moving) = gpu_template_scene();

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("initial GPU prepare succeeds");
    let first = renderer.prepare_telemetry_for_test();

    scene
        .directional_light(DirectionalLight::default().with_shadows(true))
        .add()
        .expect("shadowed light inserts");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("shadow-state-changed prepare succeeds");
    let second = renderer.prepare_telemetry_for_test();

    assert!(
        second.prepared_primitive_collections > first.prepared_primitive_collections,
        "shadow pass eligibility changes must force a full prepare"
    );
    assert_eq!(
        renderer.stats().shadow_maps,
        1,
        "shadow pass must stay enabled after the fallback full prepare"
    );
}

fn gpu_template_scene() -> (Assets, Scene, NodeKey) {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.4, 0.4, 0.4));
    let material =
        assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 0.8));
    let mut scene = Scene::new();
    scene.add_default_camera().expect("camera inserts");
    let moving = scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(-0.4, 0.0, 0.0)))
        .add()
        .expect("first mesh inserts");
    scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(0.4, 0.0, 0.0)))
        .add()
        .expect("second mesh inserts");
    (assets, scene, moving)
}

fn tinted_parity_scene() -> (Assets, Scene, crate::CameraKey, NodeKey) {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let camera = scene.add_default_camera().expect("camera inserts");
    let mesh = scene
        .add_renderable(
            scene.root(),
            vec![Primitive::triangle([
                Vertex {
                    position: Vec3::new(-2.0, -2.0, 0.0),
                    color: Color::WHITE,
                },
                Vertex {
                    position: Vec3::new(4.0, -2.0, 0.0),
                    color: Color::WHITE,
                },
                Vertex {
                    position: Vec3::new(-2.0, 4.0, 0.0),
                    color: Color::WHITE,
                },
            ])],
            Transform::default(),
        )
        .expect("fullscreen reference primitive inserts");
    (assets, scene, camera, mesh)
}

fn assert_reference_images_close(label: &str, cpu_frame: &[u8], gpu_frame: &[u8]) {
    let cpu = ReferenceImage::from_rgba8(32, 32, cpu_frame.to_vec())
        .expect("CPU reference image is valid");
    let gpu = ReferenceImage::from_rgba8(32, 32, gpu_frame.to_vec())
        .expect("GPU reference image is valid");
    regress_with_tolerance(
        &gpu,
        &cpu,
        ReferenceImageTolerance::new()
            .with_max_abs_diff(24)
            .with_max_mismatched_pixels(96),
    )
    .unwrap_or_else(|error| panic!("{label} CPU/GPU tint reference diff exceeded: {error}"));
}

fn frame_abs_diff(before: &[u8], after: &[u8]) -> u64 {
    before
        .iter()
        .zip(after)
        .map(|(before, after)| u64::from(before.abs_diff(*after)))
        .sum()
}

fn frame_abs_diff_rgba8(before: &[u8], after: &[u8]) -> Vec<u8> {
    before
        .iter()
        .zip(after)
        .map(|(before, after)| before.abs_diff(*after))
        .collect()
}

fn bright_pixels_in_region(
    frame: &[u8],
    width: usize,
    x: usize,
    y: usize,
    region_width: usize,
    region_height: usize,
) -> u32 {
    let mut count = 0_u32;
    for py in y..y.saturating_add(region_height) {
        for px in x..x.saturating_add(region_width) {
            let offset = (py * width + px) * 4;
            if frame.get(offset..offset + 4).is_some_and(|pixel| {
                pixel[3] > 16 && (pixel[0] > 24 || pixel[1] > 24 || pixel[2] > 24)
            }) {
                count = count.saturating_add(1);
            }
        }
    }
    count
}

fn assert_delta_within_ratio(cpu_delta: u64, gpu_delta: u64, max_ratio: f64) {
    let smaller = cpu_delta.min(gpu_delta) as f64;
    let larger = cpu_delta.max(gpu_delta) as f64;
    let ratio = (larger - smaller) / larger;
    assert!(
        ratio <= max_ratio,
        "CPU/GPU tint pixel deltas should match within {:.0}%: cpu_delta={cpu_delta}, gpu_delta={gpu_delta}, ratio={ratio:.3}",
        max_ratio * 100.0,
    );
}
