#![cfg(not(target_arch = "wasm32"))]

#[allow(dead_code)]
mod support;

use scena::{
    AntiAliasing, Backend, Capabilities, CapabilityStatus, Color, DepthRange, PerspectiveCamera,
    Primitive, Transform, Vec3, Vertex,
};
use support::parity::{
    PixelRegion, compare_frames_in_region, record_cpu_gpu_parity_pass,
    render_scene_cpu_gpu_pair_with_renderer, require_cpu_gpu_parity_adapter_or_skip,
};

const WIDTH: u32 = 128;
const HEIGHT: u32 = 96;

#[test]
fn close_camera_near_clip_matches_cpu_and_gpu_rendered_output() {
    if !require_cpu_gpu_parity_adapter_or_skip(
        "close_camera_near_clip_matches_cpu_and_gpu_rendered_output",
    ) {
        return;
    }
    assert_eq!(
        Capabilities::for_backend(Backend::HeadlessGpu).reversed_z_depth,
        CapabilityStatus::Supported,
        "the GPU side of this parity proof must exercise reversed-Z depth"
    );
    let pair = render_scene_cpu_gpu_pair_with_renderer(
        "c13-near-clip",
        WIDTH,
        HEIGHT,
        AntiAliasing::None,
        |renderer| renderer.set_background_color(Color::BLACK),
        |scene, _assets| {
            let camera = scene
                .add_perspective_camera(
                    scene.root(),
                    PerspectiveCamera::default().with_depth_range(DepthRange::new(0.5, 5.0)),
                    Transform::default(),
                )
                .expect("parity camera inserts");
            scene
                .add_renderable(
                    scene.root(),
                    vec![Primitive::triangle([
                        vertex(-0.5, -0.45, -1.0, Color::RED),
                        vertex(0.5, -0.45, -1.0, Color::GREEN),
                        vertex(0.0, 0.5, -0.25, Color::BLUE),
                    ])],
                    Transform::default(),
                )
                .expect("parity triangle inserts");
            camera
        },
    );
    let comparison = compare_frames_in_region(
        pair.cpu.borrowed(),
        pair.gpu.borrowed(),
        PixelRegion {
            x: 0,
            y: 0,
            width: WIDTH,
            height: HEIGHT,
        },
    );
    assert!(
        comparison.left_structure.foreground_fraction > 0.05,
        "CPU clipped output must contain the near-plane polygon"
    );
    assert!(
        comparison.right_structure.foreground_fraction > 0.05,
        "GPU hardware-clipped output must contain the near-plane polygon"
    );
    assert!(
        comparison.rmse <= 0.08,
        "near-plane CPU/GPU full-frame RMSE too high: {:.5}",
        comparison.rmse
    );
    assert!(
        comparison.channel_delta.mean_channel_delta <= 12.0,
        "near-plane CPU/GPU mean channel delta too high: {:.3}",
        comparison.channel_delta.mean_channel_delta
    );
    record_cpu_gpu_parity_pass(
        "close_camera_near_clip_matches_cpu_and_gpu_rendered_output",
        pair.gpu
            .gpu_adapter
            .as_ref()
            .expect("near-clip GPU adapter is recorded"),
        5,
    );
}

fn vertex(x: f32, y: f32, z: f32, color: Color) -> Vertex {
    Vertex {
        position: Vec3::new(x, y, z),
        color,
    }
}
