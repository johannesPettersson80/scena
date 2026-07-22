#![cfg(not(target_arch = "wasm32"))]

#[allow(dead_code)]
mod support;

use scena::{AntiAliasing, Color, DirectionalLight, Transform};
use support::parity::{
    PixelRegion, compare_frames_in_region, configure_lavapipe_adapter,
    render_scene_cpu_gpu_pair_with_renderer,
};

const WIDTH: u32 = 128;
const HEIGHT: u32 = 128;

#[test]
fn khronos_missing_normal_skin_matches_cpu_and_gpu_rendered_output() {
    if std::env::var_os("SCENA_RUN_UNSTABLE_HEADLESS_GPU_RELEASE_TESTS").is_none() {
        let artifact_dir = std::path::Path::new("target/gate-artifacts/c14-gltf-semantics");
        std::fs::create_dir_all(artifact_dir).expect("C14 diagnostic artifact directory creates");
        std::fs::write(
            artifact_dir.join("gpu-parity-unavailable.json"),
            concat!(
                "{\n",
                "  \"schema\": \"scena.c14_gltf_semantic_gpu.v1\",\n",
                "  \"release_evidence\": false,\n",
                "  \"reason\": \"explicit_gpu_parity_lane_not_requested\"\n",
                "}\n"
            ),
        )
        .expect("C14 fail-closed diagnostic artifact writes");
        return;
    }
    configure_lavapipe_adapter();
    let pair = render_scene_cpu_gpu_pair_with_renderer(
        "c14-khronos-simple-skin",
        WIDTH,
        HEIGHT,
        AntiAliasing::None,
        |renderer| renderer.set_background_color(Color::BLACK),
        |scene, assets| {
            let scene_asset = pollster::block_on(
                assets.load_scene("tests/assets/gltf/khronos/SimpleSkin/SimpleSkin.gltf"),
            )
            .expect("real Khronos SimpleSkin loads for parity");
            let import = scene
                .instantiate(&scene_asset)
                .expect("SimpleSkin instantiates for parity");
            let camera = scene.add_default_camera().expect("camera inserts");
            scene.frame_import(camera, &import).expect("asset frames");
            scene
                .directional_light(DirectionalLight::key_light())
                .transform(Transform::default().rotate_x_deg(-35.0).rotate_y_deg(25.0))
                .add()
                .expect("key light inserts");
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
    assert!(comparison.left_structure.foreground_fraction > 0.01);
    assert!(comparison.right_structure.foreground_fraction > 0.01);
    assert!(
        comparison.rmse <= 0.16,
        "computed-normal skin CPU/GPU RMSE too high: {:.5}",
        comparison.rmse,
    );
    assert!(
        comparison.channel_delta.mean_channel_delta <= 24.0,
        "computed-normal skin CPU/GPU mean channel delta too high: {:.3}",
        comparison.channel_delta.mean_channel_delta,
    );
}
