use crate::assets::Assets;
use crate::geometry::{Primitive, Vertex};
use crate::material::Color;
use crate::scene::{PerspectiveCamera, Scene, Transform, Vec3};

use super::{
    AntiAliasing, DepthOfFieldConfig, PostBloomConfig, Renderer, ScreenSpaceAmbientOcclusionConfig,
};

const CAMERA_DISTANCE_FOR_NDC_FIXTURES: f32 = 1.732_050_8;

#[test]
fn gpu_post_chain_preserves_bloom_when_fxaa_follows() {
    let Some((fxaa_only, fxaa_stats)) =
        render_gpu_post_frame(32, 32, bloom_reference_scene, |renderer| {
            renderer.set_anti_aliasing(AntiAliasing::Fxaa);
            renderer.clear_bloom();
            renderer.clear_screen_space_ambient_occlusion();
        })
    else {
        return;
    };
    let Some((bloom_and_fxaa, combined_stats)) =
        render_gpu_post_frame(32, 32, bloom_reference_scene, |renderer| {
            renderer.set_anti_aliasing(AntiAliasing::Fxaa);
            renderer.set_bloom(Some(PostBloomConfig::new(96, 0.65, 3)));
            renderer.clear_screen_space_ambient_occlusion();
        })
    else {
        return;
    };

    assert_eq!(fxaa_stats.fxaa_passes, 1);
    assert_eq!(fxaa_stats.bloom_passes, 0);
    assert_eq!(combined_stats.fxaa_passes, 1);
    assert_eq!(combined_stats.bloom_passes, 1);
    assert!(
        bloom_and_fxaa != fxaa_only,
        "bloom parameters must survive until the bloom pass executes when FXAA follows it"
    );
    let fxaa_halo = region_luma_sum_outside(&fxaa_only, 32, 32, 12..20, 12..20);
    let combined_halo = region_luma_sum_outside(&bloom_and_fxaa, 32, 32, 12..20, 12..20);
    assert!(
        combined_halo > fxaa_halo + 1_000,
        "bloom+FXAA must preserve bloom energy outside the emitter; fxaa={fxaa_halo} combined={combined_halo}"
    );
}

#[test]
fn gpu_post_passes_have_independent_quality_measurements() {
    let Some((baseline_edge, baseline_edge_stats)) =
        render_gpu_post_frame(32, 32, fxaa_edge_scene, |renderer| {
            renderer.set_anti_aliasing(AntiAliasing::None);
            renderer.clear_bloom();
            renderer.clear_screen_space_ambient_occlusion();
        })
    else {
        return;
    };
    let Some((fxaa, fxaa_stats)) = render_gpu_post_frame(32, 32, fxaa_edge_scene, |renderer| {
        renderer.set_anti_aliasing(AntiAliasing::Fxaa);
        renderer.clear_bloom();
        renderer.clear_screen_space_ambient_occlusion();
    }) else {
        return;
    };
    assert_eq!(baseline_edge_stats.fxaa_passes, 0);
    assert_eq!(fxaa_stats.fxaa_passes, 1);
    assert_eq!(fxaa_stats.bloom_passes, 0);
    assert_eq!(fxaa_stats.ambient_occlusion_passes, 0);
    let baseline_edges = hard_edge_transition_count(&baseline_edge, 32, 32, 200);
    let fxaa_edges = hard_edge_transition_count(&fxaa, 32, 32, 200);
    assert!(
        fxaa_edges < baseline_edges,
        "FXAA alone should reduce high-contrast edge transitions; baseline={baseline_edges} fxaa={fxaa_edges}"
    );

    let Some((baseline_bloom, baseline_bloom_stats)) =
        render_gpu_post_frame(32, 32, bloom_reference_scene, |renderer| {
            renderer.set_anti_aliasing(AntiAliasing::None);
            renderer.clear_bloom();
            renderer.clear_screen_space_ambient_occlusion();
        })
    else {
        return;
    };
    let Some((bloom, bloom_stats)) =
        render_gpu_post_frame(32, 32, bloom_reference_scene, |renderer| {
            renderer.set_anti_aliasing(AntiAliasing::None);
            renderer.set_bloom(Some(PostBloomConfig::new(96, 0.65, 3)));
            renderer.clear_screen_space_ambient_occlusion();
        })
    else {
        return;
    };
    assert_eq!(baseline_bloom_stats.bloom_passes, 0);
    assert_eq!(bloom_stats.bloom_passes, 1);
    assert_eq!(bloom_stats.fxaa_passes, 0);
    assert_eq!(bloom_stats.ambient_occlusion_passes, 0);
    let baseline_halo = region_luma_sum_outside(&baseline_bloom, 32, 32, 12..20, 12..20);
    let bloom_halo = region_luma_sum_outside(&bloom, 32, 32, 12..20, 12..20);
    assert!(
        bloom_halo > baseline_halo + 1_000,
        "bloom alone should add energy outside the emitter silhouette; baseline={baseline_halo} bloom={bloom_halo}"
    );

    let Some((baseline_ssao, baseline_ssao_stats)) =
        render_gpu_post_frame(48, 48, ssao_depth_contact_scene, |renderer| {
            renderer.set_anti_aliasing(AntiAliasing::None);
            renderer.clear_bloom();
            renderer.clear_screen_space_ambient_occlusion();
        })
    else {
        return;
    };
    let Some((ssao, ssao_stats)) =
        render_gpu_post_frame(48, 48, ssao_depth_contact_scene, |renderer| {
            renderer.set_anti_aliasing(AntiAliasing::None);
            renderer.clear_bloom();
            renderer.set_screen_space_ambient_occlusion(Some(
                ScreenSpaceAmbientOcclusionConfig::new(4, 0.8, 0.0),
            ));
        })
    else {
        return;
    };
    assert_eq!(baseline_ssao_stats.ambient_occlusion_passes, 0);
    assert_eq!(ssao_stats.ambient_occlusion_passes, 1);
    assert_eq!(ssao_stats.bloom_passes, 0);
    assert_eq!(ssao_stats.fxaa_passes, 0);
    let contact_drop = max_luma_drop_ring(
        &baseline_ssao,
        &ssao,
        48,
        (14..28, 18..30),
        (18..24, 20..28),
    );
    let baseline_open = average_luma_region(&baseline_ssao, 48, 8..14, 20..28);
    let ssao_open = average_luma_region(&ssao, 48, 8..14, 20..28);
    assert!(
        contact_drop >= 4,
        "SSAO alone should darken at least one contact/corner pixel around the foreground depth edge; contact_drop={contact_drop}"
    );
    assert!(
        (baseline_open - ssao_open).abs() <= 2.0,
        "SSAO alone should leave open floor within tolerance; baseline={baseline_open:.2} ssao={ssao_open:.2}"
    );

    let Some((baseline_dof, baseline_dof_stats)) =
        render_gpu_post_frame(48, 32, depth_of_field_reference_scene, |renderer| {
            renderer.set_anti_aliasing(AntiAliasing::None);
            renderer.clear_bloom();
            renderer.clear_screen_space_ambient_occlusion();
            renderer.clear_depth_of_field();
        })
    else {
        return;
    };
    let Some((dof, dof_stats)) =
        render_gpu_post_frame(48, 32, depth_of_field_reference_scene, |renderer| {
            renderer.set_anti_aliasing(AntiAliasing::None);
            renderer.clear_bloom();
            renderer.clear_screen_space_ambient_occlusion();
            renderer.set_depth_of_field(Some(DepthOfFieldConfig::new(
                CAMERA_DISTANCE_FOR_NDC_FIXTURES,
                0.7,
                12,
            )));
        })
    else {
        return;
    };
    assert_eq!(baseline_dof_stats.depth_of_field_passes, 0);
    assert_eq!(dof_stats.depth_of_field_passes, 1);
    let baseline_background = region_luma_range(&baseline_dof, 48, 30..44, 8..24);
    let dof_background = region_luma_range(&dof, 48, 30..44, 8..24);
    let baseline_focus = average_luma_region(&baseline_dof, 48, 22..26, 14..18);
    let dof_focus = average_luma_region(&dof, 48, 22..26, 14..18);
    assert!(
        dof_background * 4 < baseline_background * 3,
        "DoF alone should blur high-frequency background contrast; baseline={baseline_background} dof={dof_background}"
    );
    assert!(
        (baseline_focus - dof_focus).abs() <= 8.0,
        "DoF should preserve the focal subject luma; baseline={baseline_focus:.2} dof={dof_focus:.2}"
    );
}

fn render_gpu_post_frame<F, S>(
    width: u32,
    height: u32,
    scene_factory: S,
    configure: F,
) -> Option<(Vec<u8>, crate::RendererStats)>
where
    F: FnOnce(&mut Renderer),
    S: FnOnce() -> (Assets, Scene, crate::CameraKey),
{
    let Ok(mut renderer) = Renderer::headless_gpu(width, height) else {
        return None;
    };
    let (assets, mut scene, camera) = scene_factory();
    renderer.set_background_color(Color::BLACK);
    configure(&mut renderer);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("GPU post quality scene prepares");
    renderer
        .render(&scene, camera)
        .expect("GPU post quality scene renders");
    Some((renderer.frame_rgba8().to_vec(), renderer.stats()))
}

fn fxaa_edge_scene() -> (Assets, Scene, crate::CameraKey) {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let camera = scene.add_default_camera().expect("camera inserts");
    scene
        .add_renderable(
            scene.root(),
            quad_primitives(
                0.0,
                -1.1,
                1.1,
                1.1,
                0.0,
                Color::from_linear_rgb(2.0, 2.0, 2.0),
            )
            .to_vec(),
            Transform::default(),
        )
        .expect("FXAA edge primitive inserts");
    (assets, scene, camera)
}

fn bloom_reference_scene() -> (Assets, Scene, crate::CameraKey) {
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
        .expect("bloom reference primitives insert");
    (assets, scene, camera)
}

fn ssao_depth_contact_scene() -> (Assets, Scene, crate::CameraKey) {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, CAMERA_DISTANCE_FOR_NDC_FIXTURES)),
        )
        .expect("camera inserts");
    scene
        .set_active_camera(camera)
        .expect("camera becomes active");
    scene
        .add_renderable(
            scene.root(),
            quad_primitives(
                -0.75,
                -0.55,
                0.75,
                0.35,
                0.0,
                Color::from_linear_rgb(1.0, 1.0, 1.0),
            )
            .to_vec(),
            Transform::default(),
        )
        .expect("contact floor inserts");
    scene
        .add_renderable(
            scene.root(),
            quad_primitives(
                -0.14,
                -0.18,
                0.14,
                0.18,
                0.16,
                Color::from_linear_rgb(0.72, 0.72, 0.72),
            )
            .to_vec(),
            Transform::default(),
        )
        .expect("contact block inserts");
    (assets, scene, camera)
}

fn depth_of_field_reference_scene() -> (Assets, Scene, crate::CameraKey) {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, CAMERA_DISTANCE_FOR_NDC_FIXTURES)),
        )
        .expect("camera inserts");
    scene
        .set_active_camera(camera)
        .expect("camera becomes active");
    let mut background = Vec::new();
    for row in 0..8 {
        for col in 0..12 {
            let x0 = -1.2 + col as f32 * 0.2;
            let x1 = x0 + 0.2;
            let y0 = -0.8 + row as f32 * 0.2;
            let y1 = y0 + 0.2;
            let value = if (row + col) % 2 == 0 { 0.08 } else { 1.2 };
            background.extend(quad_primitives(
                x0,
                y0,
                x1,
                y1,
                -1.6,
                Color::from_linear_rgb(value, value, value),
            ));
        }
    }
    scene
        .add_renderable(scene.root(), background, Transform::default())
        .expect("background checker inserts");
    scene
        .add_renderable(
            scene.root(),
            quad_primitives(
                -0.42,
                -0.42,
                0.42,
                0.42,
                0.0,
                Color::from_linear_rgb(0.45, 0.45, 0.45),
            )
            .to_vec(),
            Transform::default(),
        )
        .expect("focal subject inserts");
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

fn hard_edge_transition_count(frame: &[u8], width: u32, height: u32, threshold: u8) -> u64 {
    let mut count = 0;
    for y in 0..height {
        for x in 0..width.saturating_sub(1) {
            if luma_at(frame, width, x, y).abs_diff(luma_at(frame, width, x + 1, y)) >= threshold {
                count += 1;
            }
        }
    }
    count
}

fn region_luma_sum_outside(
    frame: &[u8],
    width: u32,
    height: u32,
    silhouette_x: std::ops::Range<u32>,
    silhouette_y: std::ops::Range<u32>,
) -> u64 {
    let mut total = 0;
    for y in 0..height {
        for x in 0..width {
            if silhouette_x.contains(&x) && silhouette_y.contains(&y) {
                continue;
            }
            total += u64::from(luma_at(frame, width, x, y));
        }
    }
    total
}

fn average_luma_region(
    frame: &[u8],
    width: u32,
    region_x: std::ops::Range<u32>,
    region_y: std::ops::Range<u32>,
) -> f32 {
    let mut total = 0u64;
    let mut count = 0u64;
    for y in region_y {
        for x in region_x.clone() {
            total += u64::from(luma_at(frame, width, x, y));
            count += 1;
        }
    }
    total as f32 / count.max(1) as f32
}

fn region_luma_range(
    frame: &[u8],
    width: u32,
    region_x: std::ops::Range<u32>,
    region_y: std::ops::Range<u32>,
) -> u32 {
    let mut min_luma = u8::MAX;
    let mut max_luma = 0_u8;
    for y in region_y {
        for x in region_x.clone() {
            let value = luma_at(frame, width, x, y);
            min_luma = min_luma.min(value);
            max_luma = max_luma.max(value);
        }
    }
    u32::from(max_luma.saturating_sub(min_luma))
}

fn max_luma_drop_ring(
    before: &[u8],
    after: &[u8],
    width: u32,
    outer: (std::ops::Range<u32>, std::ops::Range<u32>),
    inner: (std::ops::Range<u32>, std::ops::Range<u32>),
) -> u8 {
    let mut max_drop = 0u8;
    for y in outer.1 {
        for x in outer.0.clone() {
            if inner.0.contains(&x) && inner.1.contains(&y) {
                continue;
            }
            let drop = luma_at(before, width, x, y).saturating_sub(luma_at(after, width, x, y));
            max_drop = max_drop.max(drop);
        }
    }
    max_drop
}

fn luma_at(frame: &[u8], width: u32, x: u32, y: u32) -> u8 {
    let index = ((y * width + x) * 4) as usize;
    let red = frame[index] as f32;
    let green = frame[index + 1] as f32;
    let blue = frame[index + 2] as f32;
    (0.299 * red + 0.587 * green + 0.114 * blue).round() as u8
}
