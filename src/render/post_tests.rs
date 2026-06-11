use crate::assets::Assets;
use crate::geometry::{Primitive, Vertex};
use crate::material::Color;
use crate::reference_image::{ReferenceImage, ReferenceImageTolerance, regress_with_tolerance};
use crate::scene::{Scene, Transform, Vec3};

use super::{AntiAliasing, PostBloomConfig, Renderer, ScreenSpaceAmbientOcclusionConfig};

#[test]
fn headless_gpu_post_chain_runs_enabled_passes_and_changes_pixels() {
    let Ok(mut baseline_renderer) = Renderer::headless_gpu(32, 32) else {
        return;
    };
    let (assets, mut baseline_scene, camera) = post_chain_reference_scene();
    baseline_renderer.set_anti_aliasing(AntiAliasing::None);
    baseline_renderer.clear_bloom();
    baseline_renderer.clear_screen_space_ambient_occlusion();
    baseline_renderer.set_background_color(Color::BLACK);
    baseline_renderer
        .prepare_with_assets(&mut baseline_scene, &assets)
        .expect("baseline GPU post reference prepares");
    baseline_renderer
        .render(&baseline_scene, camera)
        .expect("baseline GPU post reference renders");
    assert_eq!(baseline_renderer.stats().fxaa_passes, 0);
    assert_eq!(baseline_renderer.stats().bloom_passes, 0);
    assert_eq!(baseline_renderer.stats().ambient_occlusion_passes, 0);
    let baseline = baseline_renderer.frame_rgba8().to_vec();

    let Ok(mut post_renderer) = Renderer::headless_gpu(32, 32) else {
        return;
    };
    let (post_assets, mut post_scene, post_camera) = post_chain_reference_scene();
    post_renderer.set_anti_aliasing(AntiAliasing::Fxaa);
    post_renderer.set_bloom(Some(PostBloomConfig::new(96, 0.65, 3)));
    post_renderer.set_screen_space_ambient_occlusion(Some(ScreenSpaceAmbientOcclusionConfig::new(
        3, 0.5, 0.015,
    )));
    post_renderer.set_background_color(Color::BLACK);
    post_renderer
        .prepare_with_assets(&mut post_scene, &post_assets)
        .expect("enabled GPU post reference prepares");
    post_renderer
        .render(&post_scene, post_camera)
        .expect("enabled GPU post reference renders");

    assert_eq!(post_renderer.stats().ambient_occlusion_passes, 1);
    assert_eq!(post_renderer.stats().bloom_passes, 1);
    assert_eq!(post_renderer.stats().fxaa_passes, 1);
    assert!(
        frame_abs_diff(&baseline, post_renderer.frame_rgba8()) > 0,
        "enabled GPU post chain must alter rendered pixels versus all-off"
    );
}

#[test]
fn depth_color_target_is_allocated_only_for_ssao() {
    let Ok(mut renderer) = Renderer::headless_gpu(32, 32) else {
        return;
    };
    let (assets, mut scene, camera) = post_chain_reference_scene();
    renderer.set_anti_aliasing(AntiAliasing::None);
    renderer.clear_bloom();
    renderer.clear_screen_space_ambient_occlusion();
    renderer.set_background_color(Color::BLACK);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("depth-prepass reference scene prepares");

    assert_eq!(
        gpu_depth_prepass_has_color_target(&renderer),
        Some(false),
        "preparing a depth-prepass scene with all post disabled must not allocate the SSAO depth-color target"
    );
    renderer
        .render(&scene, camera)
        .expect("all-off depth-prepass scene renders");
    assert_eq!(
        gpu_depth_prepass_has_color_target(&renderer),
        Some(false),
        "all-off render must keep the depth prepass depth-only"
    );

    renderer.set_anti_aliasing(AntiAliasing::Fxaa);
    renderer.set_bloom(Some(PostBloomConfig::new(96, 0.65, 3)));
    renderer
        .render(&scene, camera)
        .expect("bloom+fxaa render without SSAO renders");
    assert_eq!(renderer.stats().ambient_occlusion_passes, 0);
    assert_eq!(
        gpu_depth_prepass_has_color_target(&renderer),
        Some(false),
        "non-SSAO post passes must not allocate or write the depth-color target"
    );

    renderer.set_screen_space_ambient_occlusion(Some(ScreenSpaceAmbientOcclusionConfig::new(
        3, 0.5, 0.015,
    )));
    renderer
        .render(&scene, camera)
        .expect("SSAO render allocates depth-color target");
    assert_eq!(renderer.stats().ambient_occlusion_passes, 1);
    assert_eq!(
        gpu_depth_prepass_has_color_target(&renderer),
        Some(true),
        "SSAO render must allocate the depth-color target"
    );

    renderer.clear_screen_space_ambient_occlusion();
    renderer.clear_bloom();
    renderer.set_anti_aliasing(AntiAliasing::None);
    renderer
        .render(&scene, camera)
        .expect("all-off render after SSAO drops depth-color target");
    assert_eq!(renderer.stats().ambient_occlusion_passes, 0);
    assert_eq!(
        gpu_depth_prepass_has_color_target(&renderer),
        Some(false),
        "clearing SSAO must restore a depth-only prepass"
    );
}

#[test]
fn cpu_and_gpu_bloom_threshold_delta_match_with_reference_tolerance() {
    let Ok(mut gpu_renderer) = Renderer::headless_gpu(32, 32) else {
        return;
    };
    let config = PostBloomConfig::new(128, 0.5, 2);
    let (assets, mut cpu_scene, camera) = bloom_threshold_reference_scene();
    let (gpu_assets, mut gpu_scene, gpu_camera) = bloom_threshold_reference_scene();

    let mut cpu_renderer = Renderer::headless(32, 32).expect("CPU renderer builds");
    cpu_renderer.set_anti_aliasing(AntiAliasing::None);
    cpu_renderer.clear_bloom();
    cpu_renderer
        .prepare_with_assets(&mut cpu_scene, &assets)
        .expect("CPU bloom baseline prepares");
    cpu_renderer
        .render(&cpu_scene, camera)
        .expect("CPU bloom baseline renders");
    let cpu_baseline = cpu_renderer.frame_rgba8().to_vec();
    cpu_renderer.set_bloom(Some(config));
    cpu_renderer
        .render(&cpu_scene, camera)
        .expect("CPU bloom threshold renders");
    let cpu_bloom = cpu_renderer.frame_rgba8().to_vec();

    gpu_renderer.set_anti_aliasing(AntiAliasing::None);
    gpu_renderer.clear_bloom();
    gpu_renderer
        .prepare_with_assets(&mut gpu_scene, &gpu_assets)
        .expect("GPU bloom baseline prepares");
    gpu_renderer
        .render(&gpu_scene, gpu_camera)
        .expect("GPU bloom baseline renders");
    let gpu_baseline = gpu_renderer.frame_rgba8().to_vec();
    gpu_renderer.set_bloom(Some(config));
    gpu_renderer
        .render(&gpu_scene, gpu_camera)
        .expect("GPU bloom threshold renders");
    let gpu_bloom = gpu_renderer.frame_rgba8().to_vec();

    let cpu_delta = frame_abs_diff_rgba8(&cpu_baseline, &cpu_bloom);
    let gpu_delta = frame_abs_diff_rgba8(&gpu_baseline, &gpu_bloom);
    assert!(
        frame_abs_diff(&cpu_baseline, &cpu_bloom) > 0,
        "CPU bloom threshold fixture must alter pixels"
    );
    assert!(
        frame_abs_diff(&gpu_baseline, &gpu_bloom) > 0,
        "GPU bloom threshold fixture must alter pixels"
    );
    assert_reference_images_close("bloom threshold delta", &cpu_delta, &gpu_delta);
}

fn gpu_depth_prepass_has_color_target(renderer: &Renderer) -> Option<bool> {
    renderer
        .gpu
        .as_ref()
        .and_then(|gpu| gpu.depth_prepass_has_color_target())
}

fn post_chain_reference_scene() -> (Assets, Scene, crate::CameraKey) {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut primitives = Vec::new();
    primitives.extend(quad_primitives(
        -1.2,
        -1.2,
        1.2,
        1.2,
        -0.12,
        Color::from_linear_rgb(0.08, 0.08, 0.08),
    ));
    primitives.extend(quad_primitives(
        -0.32,
        -0.32,
        0.32,
        0.32,
        0.0,
        Color::from_linear_rgb(2.0, 2.0, 2.0),
    ));
    scene
        .add_renderable(scene.root(), primitives, Transform::default())
        .expect("post reference primitives insert");
    (assets, scene, camera)
}

fn bloom_threshold_reference_scene() -> (Assets, Scene, crate::CameraKey) {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut primitives = Vec::new();
    primitives.extend(quad_primitives(
        -0.72,
        -0.24,
        -0.28,
        0.24,
        0.0,
        Color::from_linear_rgb(0.2, 0.2, 0.2),
    ));
    primitives.extend(quad_primitives(
        0.28,
        -0.24,
        0.72,
        0.24,
        0.0,
        Color::from_linear_rgb(1.8, 1.8, 1.8),
    ));
    scene
        .add_renderable(scene.root(), primitives, Transform::default())
        .expect("bloom threshold primitives insert");
    (assets, scene, camera)
}

fn quad_primitives(x0: f32, y0: f32, x1: f32, y1: f32, z: f32, color: Color) -> [Primitive; 2] {
    [
        Primitive::triangle([
            Vertex {
                position: Vec3::new(x0, y0, z),
                color,
            },
            Vertex {
                position: Vec3::new(x1, y0, z),
                color,
            },
            Vertex {
                position: Vec3::new(x1, y1, z),
                color,
            },
        ]),
        Primitive::triangle([
            Vertex {
                position: Vec3::new(x0, y0, z),
                color,
            },
            Vertex {
                position: Vec3::new(x1, y1, z),
                color,
            },
            Vertex {
                position: Vec3::new(x0, y1, z),
                color,
            },
        ]),
    ]
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
    .unwrap_or_else(|error| panic!("{label} CPU/GPU post reference diff exceeded: {error}"));
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
