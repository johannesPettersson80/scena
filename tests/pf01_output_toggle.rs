#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::{Path, PathBuf};

use scena::{
    AntiAliasing, Background, GpuAdapterReport, Primitive, RenderReadbackMode, Renderer,
    RendererStats, Scene, Transform,
};
use serde_json::json;

const WIDTH: u32 = 320;
const HEIGHT: u32 = 240;

#[test]
fn pf01_native_gpu_output_toggle_renders_off_on_off_without_lazy_resources() {
    let Ok(mut renderer) = Renderer::headless_gpu(WIDTH, HEIGHT) else {
        if std::env::var_os("SCENA_REQUIRE_HARDWARE_GPU").is_some() {
            panic!("PF01 required native hardware lane did not expose a GPU adapter");
        }
        return;
    };
    let adapter = renderer
        .gpu_adapter_report()
        .expect("headless GPU renderer reports its adapter");
    if std::env::var_os("SCENA_REQUIRE_HARDWARE_GPU").is_some() {
        require_hardware_adapter(&adapter);
    }

    renderer.set_background(Background::Black);
    let mut scene = Scene::new();
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::IDENTITY,
        )
        .expect("PF01 triangle inserts");
    let camera = scene.add_default_camera().expect("PF01 camera inserts");

    let off = render_phase(&mut renderer, &mut scene, camera, AntiAliasing::None);
    let on = render_phase(&mut renderer, &mut scene, camera, AntiAliasing::Fxaa);
    let off_again = render_phase(&mut renderer, &mut scene, camera, AntiAliasing::None);

    assert_eq!(
        off.rgba8, off_again.rgba8,
        "returning to the disabled output state must reproduce the original pixels"
    );
    let changed_channels = off
        .rgba8
        .iter()
        .zip(&on.rgba8)
        .filter(|(left, right)| left != right)
        .count();
    assert!(
        changed_channels > 0,
        "FXAA on/off must produce a rendered pixel difference"
    );
    assert!(
        off.rgba8
            .chunks_exact(4)
            .any(|pixel| pixel[..3] != [0, 0, 0]),
        "disabled output render must contain visible geometry"
    );
    assert_ne!(
        off.resources, on.resources,
        "enabling FXAA must prepare a distinct resource shape"
    );
    assert_eq!(
        off.resources, off_again.resources,
        "disabling FXAA again must restore the baseline resource shape"
    );

    let artifacts = PathBuf::from("target/gate-artifacts/pf01-output-toggle/native");
    fs::create_dir_all(&artifacts).expect("PF01 native artifact directory creates");
    write_ppm(&artifacts.join("off.ppm"), &off.rgba8);
    write_ppm(&artifacts.join("on.ppm"), &on.rgba8);
    write_ppm(&artifacts.join("off-again.ppm"), &off_again.rgba8);
    let report = json!({
        "schema": "scena.pf01.native_output_toggle.v1",
        "status": "passed",
        "release_evidence": std::env::var_os("SCENA_REQUIRE_HARDWARE_GPU").is_some(),
        "backend": "native-headless-gpu",
        "adapter": adapter,
        "fixture": { "width": WIDTH, "height": HEIGHT, "primitive_count": 1 },
        "changed_channels_off_to_on": changed_channels,
        "phases": {
            "off": off.resources,
            "on": on.resources,
            "off_again": off_again.resources,
        },
        "acceptance": [
            "nonblank-render",
            "off-on-pixel-delta",
            "off-again-determinism",
            "zero-render-time-gpu-object-creation",
            "prepared-resource-toggle",
        ],
        "command": std::env::var("SCENA_HARDWARE_PROOF_COMMAND").unwrap_or_else(|_| {
            "cargo test --test pf01_output_toggle -- --exact --nocapture".to_owned()
        }),
    });
    fs::write(
        artifacts.join("native-output-toggle.json"),
        serde_json::to_vec_pretty(&report).expect("PF01 native report serializes"),
    )
    .expect("PF01 native report writes");
}

struct Phase {
    rgba8: Vec<u8>,
    resources: [u64; 6],
}

fn render_phase(
    renderer: &mut Renderer,
    scene: &mut Scene,
    camera: scena::CameraKey,
    anti_aliasing: AntiAliasing,
) -> Phase {
    renderer.set_anti_aliasing(anti_aliasing);
    renderer.prepare(scene).expect("PF01 output phase prepares");
    let resources = resource_signature(renderer.stats());
    renderer
        .render_with_readback_mode(scene, camera, RenderReadbackMode::Synchronous)
        .expect("PF01 output phase renders and captures");
    let metrics = renderer.last_render_work_metrics();
    assert_eq!(metrics.gpu_buffer_creations, 0);
    assert_eq!(metrics.gpu_texture_creations, 0);
    assert_eq!(metrics.gpu_pipeline_creations, 0);
    assert_eq!(metrics.gpu_bind_group_creations, 0);
    assert_eq!(metrics.gpu_shader_module_creations, 0);
    assert_eq!(
        resource_signature(renderer.stats()),
        resources,
        "render must preserve the prepared resource shape"
    );
    Phase {
        rgba8: renderer.frame_rgba8().to_vec(),
        resources,
    }
}

fn resource_signature(stats: RendererStats) -> [u64; 6] {
    [
        stats.buffers,
        stats.gpu_textures,
        stats.render_targets,
        stats.pipelines,
        stats.bind_groups,
        stats.shader_modules,
    ]
}

fn require_hardware_adapter(adapter: &GpuAdapterReport) {
    assert!(
        matches!(
            adapter.device_type.as_str(),
            "DiscreteGpu" | "IntegratedGpu" | "VirtualGpu"
        ),
        "PF01 required native proof needs a hardware adapter: {adapter:?}"
    );
    let identity = format!(
        "{} {} {} {}",
        adapter.name, adapter.device_type, adapter.driver, adapter.driver_info
    )
    .to_ascii_lowercase();
    for marker in [
        "swiftshader",
        "llvmpipe",
        "lavapipe",
        "software rasterizer",
        "microsoft basic render",
    ] {
        assert!(
            !identity.contains(marker),
            "PF01 required native proof rejects software adapter marker {marker}: {adapter:?}"
        );
    }
}

fn write_ppm(path: &Path, rgba8: &[u8]) {
    let mut bytes = format!("P6\n{WIDTH} {HEIGHT}\n255\n").into_bytes();
    for pixel in rgba8.chunks_exact(4) {
        bytes.extend_from_slice(&pixel[..3]);
    }
    fs::write(path, bytes).expect("PF01 PPM writes");
}
