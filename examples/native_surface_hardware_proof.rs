#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::PathBuf;

use scena::{
    AntiAliasing, Assets, Background, Color, GeometryDesc, MaterialDesc, PlatformSurface,
    PostBloomConfig, RenderReadbackMode, Renderer, Scene,
};
use serde_json::json;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

const WIDTH: u32 = 320;
const HEIGHT: u32 = 240;

struct PhaseProof {
    id: &'static str,
    rgba8: Vec<u8>,
    fnv1a64: String,
    nonblack: usize,
    resources: [u64; 6],
}

#[derive(Default)]
struct NativeSurfaceProof {
    attempted: bool,
    result: Option<Result<(), String>>,
}

impl ApplicationHandler for NativeSurfaceProof {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.attempted {
            return;
        }
        self.attempted = true;
        self.result = Some(run_proof(event_loop));
        event_loop.exit();
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        _event: WindowEvent,
    ) {
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("native surface proof creates an event loop");
    let mut proof = NativeSurfaceProof::default();
    event_loop
        .run_app(&mut proof)
        .expect("native surface proof event loop runs");
    match proof.result {
        Some(Ok(())) => {}
        Some(Err(error)) => panic!("native surface hardware proof failed: {error}"),
        None => panic!("native surface hardware proof never received a resumed event"),
    }
}

fn run_proof(event_loop: &ActiveEventLoop) -> Result<(), String> {
    eprintln!("native-surface-proof: create-window");
    let window = event_loop
        .create_window(
            Window::default_attributes()
                .with_title("scena native surface hardware proof")
                .with_inner_size(LogicalSize::new(f64::from(WIDTH), f64::from(HEIGHT)))
                .with_visible(true),
        )
        .map_err(|error| format!("native proof window creation failed: {error}"))?;
    eprintln!("native-surface-proof: create-platform-surface");
    let surface = PlatformSurface::native_window_handle(window, WIDTH, HEIGHT);
    eprintln!("native-surface-proof: create-renderer");
    let mut renderer = Renderer::from_surface(surface)
        .map_err(|error| format!("attached native renderer creation failed: {error:?}"))?;
    eprintln!("native-surface-proof: renderer-ready");
    let adapter = renderer
        .gpu_adapter_report()
        .ok_or_else(|| "attached native renderer did not report an adapter".to_owned())?;
    require_hardware_adapter(&adapter)?;
    renderer.set_background(Background::Black);
    renderer.set_anti_aliasing(AntiAliasing::None);
    renderer.set_bloom(None);

    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.8, 0.5, 0.35));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(70, 180, 230)));
    let mut scene = Scene::new();
    eprintln!("native-surface-proof: create-scene");
    scene
        .mesh(geometry, material)
        .add()
        .map_err(|error| format!("native proof mesh insertion failed: {error:?}"))?;
    let camera = scene
        .add_default_camera()
        .map_err(|error| format!("native proof camera insertion failed: {error:?}"))?;
    scene
        .frame_all_with_assets(camera, &assets)
        .map_err(|error| format!("native proof camera framing failed: {error:?}"))?;
    eprintln!("native-surface-proof: prepare");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .map_err(|error| format!("native proof prepare failed: {error:?}"))?;
    eprintln!("native-surface-proof: render-present-only");
    let before = renderer.stats();
    renderer
        .render_with_readback_mode(&scene, camera, RenderReadbackMode::PresentOnly)
        .map_err(|error| format!("native present-only render failed: {error:?}"))?;
    eprintln!("native-surface-proof: render-complete");
    let after = renderer.stats();
    let metrics = renderer.last_render_work_metrics();

    let failures = [
        ("readback_copies", metrics.readback_copies),
        ("map_requests", metrics.map_requests),
        ("blocking_polls", metrics.blocking_polls),
        ("blocking_waits", metrics.blocking_waits),
        ("cpu_frame_copy_bytes", metrics.cpu_frame_copy_bytes),
        ("gpu_buffer_creations", metrics.gpu_buffer_creations),
        ("gpu_texture_creations", metrics.gpu_texture_creations),
        ("gpu_pipeline_creations", metrics.gpu_pipeline_creations),
        ("gpu_bind_group_creations", metrics.gpu_bind_group_creations),
        (
            "gpu_shader_module_creations",
            metrics.gpu_shader_module_creations,
        ),
    ]
    .into_iter()
    .filter(|(_, value)| *value != 0)
    .collect::<Vec<_>>();
    if !failures.is_empty() {
        return Err(format!(
            "present-only render performed forbidden copy/sync/allocation work: {failures:?}"
        ));
    }
    let before_resources = resource_signature(before);
    let after_resources = resource_signature(after);
    if before_resources != after_resources {
        return Err(format!(
            "present-only render changed prepared GPU resource ownership: before={before_resources:?}, after={after_resources:?}"
        ));
    }

    let phases = [
        capture_phase(
            &mut renderer,
            &mut scene,
            &assets,
            camera,
            "off",
            AntiAliasing::None,
            None,
        )?,
        capture_phase(
            &mut renderer,
            &mut scene,
            &assets,
            camera,
            "bloom_only",
            AntiAliasing::None,
            Some(PostBloomConfig::new(96, 0.75, 4)),
        )?,
        capture_phase(
            &mut renderer,
            &mut scene,
            &assets,
            camera,
            "fxaa_only",
            AntiAliasing::Fxaa,
            None,
        )?,
        capture_phase(
            &mut renderer,
            &mut scene,
            &assets,
            camera,
            "on",
            AntiAliasing::Fxaa,
            Some(PostBloomConfig::new(96, 0.75, 4)),
        )?,
        capture_phase(
            &mut renderer,
            &mut scene,
            &assets,
            camera,
            "off_again",
            AntiAliasing::None,
            None,
        )?,
    ];
    validate_output_toggle(&phases)?;

    let phase_json = phases
        .iter()
        .map(|phase| {
            (
                phase.id.to_owned(),
                json!({
                    "id": phase.id,
                    "fnv1a64": phase.fnv1a64,
                    "nonblack": phase.nonblack,
                    "prepared_resources": phase.resources,
                }),
            )
        })
        .collect::<serde_json::Map<_, _>>();

    let artifact = json!({
        "schema": "scena.pf01_pf02.native_surface_hardware_proof.v1",
        "status": "passed",
        "release_evidence": true,
        "surface_attached": true,
        "backend": renderer.capabilities().backend,
        "adapter": adapter,
        "fixture": { "width": WIDTH, "height": HEIGHT, "primitive_count": 1 },
        "present_only": {
            "readback_copies": metrics.readback_copies,
            "map_requests": metrics.map_requests,
            "blocking_polls": metrics.blocking_polls,
            "blocking_waits": metrics.blocking_waits,
            "cpu_frame_copy_bytes": metrics.cpu_frame_copy_bytes,
            "gpu_buffer_creations": metrics.gpu_buffer_creations,
            "gpu_texture_creations": metrics.gpu_texture_creations,
            "gpu_pipeline_creations": metrics.gpu_pipeline_creations,
            "gpu_bind_group_creations": metrics.gpu_bind_group_creations,
            "gpu_shader_module_creations": metrics.gpu_shader_module_creations,
        },
        "prepared_resources_before": before_resources,
        "prepared_resources_after": after_resources,
        "output_toggle": {
            "status": "passed",
            "phases": phase_json,
            "acceptance": [
                "nonblank-all-phases",
                "bloom-only-pixel-delta",
                "fxaa-only-pixel-delta",
                "combined-effect-pixel-delta",
                "off-again-determinism",
                "zero-render-time-gpu-object-creation",
                "prepared-resource-toggle",
            ],
        },
        "command": std::env::var("SCENA_HARDWARE_PROOF_COMMAND")
            .unwrap_or_else(|_| "cargo run --example native_surface_hardware_proof".to_owned()),
    });
    let artifact_path = artifact_path();
    fs::create_dir_all(
        artifact_path
            .parent()
            .ok_or_else(|| "native proof artifact path has no parent".to_owned())?,
    )
    .map_err(|error| format!("native proof artifact directory failed: {error}"))?;
    let artifact_dir = artifact_path
        .parent()
        .ok_or_else(|| "native proof artifact path has no parent".to_owned())?;
    for phase in &phases {
        write_ppm(
            &artifact_dir.join(format!("{}.ppm", phase.id.replace('_', "-"))),
            &phase.rgba8,
        )?;
    }
    fs::write(
        &artifact_path,
        serde_json::to_vec_pretty(&artifact)
            .map_err(|error| format!("native proof artifact serialization failed: {error}"))?,
    )
    .map_err(|error| format!("native proof artifact write failed: {error}"))?;
    eprintln!("native-surface-proof: artifact-written");
    println!("{}", artifact);
    Ok(())
}

fn capture_phase(
    renderer: &mut Renderer,
    scene: &mut Scene,
    assets: &Assets,
    camera: scena::CameraKey,
    id: &'static str,
    anti_aliasing: AntiAliasing,
    bloom: Option<PostBloomConfig>,
) -> Result<PhaseProof, String> {
    eprintln!("native-surface-proof: {id}: configure-output");
    renderer.set_anti_aliasing(anti_aliasing);
    renderer.set_bloom(bloom);
    renderer
        .prepare_with_assets(scene, assets)
        .map_err(|error| format!("native {id} prepare failed: {error:?}"))?;
    let resources = resource_signature(renderer.stats());
    renderer
        .render_with_readback_mode(scene, camera, RenderReadbackMode::Synchronous)
        .map_err(|error| format!("native {id} synchronous capture failed: {error:?}"))?;
    let metrics = renderer.last_render_work_metrics();
    let forbidden = [
        ("gpu_buffer_creations", metrics.gpu_buffer_creations),
        ("gpu_texture_creations", metrics.gpu_texture_creations),
        ("gpu_pipeline_creations", metrics.gpu_pipeline_creations),
        ("gpu_bind_group_creations", metrics.gpu_bind_group_creations),
        (
            "gpu_shader_module_creations",
            metrics.gpu_shader_module_creations,
        ),
    ]
    .into_iter()
    .filter(|(_, value)| *value != 0)
    .collect::<Vec<_>>();
    if !forbidden.is_empty() {
        return Err(format!(
            "native {id} render created GPU resources after prepare: {forbidden:?}"
        ));
    }
    let resources_after = resource_signature(renderer.stats());
    if resources_after != resources {
        return Err(format!(
            "native {id} render changed prepared resource ownership: before={resources:?}, after={resources_after:?}"
        ));
    }
    let rgba8 = renderer.frame_rgba8().to_vec();
    let nonblack = rgba8
        .chunks_exact(4)
        .filter(|pixel| pixel[..3] != [0, 0, 0])
        .count();
    if nonblack == 0 {
        return Err(format!("native {id} output is blank"));
    }
    Ok(PhaseProof {
        id,
        fnv1a64: fnv1a64(&rgba8),
        rgba8,
        nonblack,
        resources,
    })
}

fn validate_output_toggle(phases: &[PhaseProof; 5]) -> Result<(), String> {
    let [off, bloom_only, fxaa_only, on, off_again] = phases;
    if bloom_only.fnv1a64 == off.fnv1a64 {
        return Err("native bloom-only output collapsed to baseline".to_owned());
    }
    if fxaa_only.fnv1a64 == off.fnv1a64 {
        return Err("native FXAA-only output collapsed to baseline".to_owned());
    }
    if on.fnv1a64 == off.fnv1a64 {
        return Err("native combined output collapsed to baseline".to_owned());
    }
    if on.fnv1a64 == bloom_only.fnv1a64 {
        return Err("native combined output collapsed to bloom-only".to_owned());
    }
    if on.fnv1a64 == fxaa_only.fnv1a64 {
        return Err("native combined output collapsed to FXAA-only".to_owned());
    }
    if off_again.fnv1a64 != off.fnv1a64 {
        return Err("native off-again output did not restore baseline pixels".to_owned());
    }
    for phase in [bloom_only, fxaa_only, on] {
        if phase.resources == off.resources {
            return Err(format!(
                "native {} did not prepare a distinct resource shape",
                phase.id
            ));
        }
    }
    if off_again.resources != off.resources {
        return Err("native off-again did not restore baseline resources".to_owned());
    }
    Ok(())
}

fn fnv1a64(bytes: &[u8]) -> String {
    let mut value = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        value ^= u64::from(*byte);
        value = value.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{value:016x}")
}

fn write_ppm(path: &std::path::Path, rgba8: &[u8]) -> Result<(), String> {
    let mut bytes = format!("P6\n{WIDTH} {HEIGHT}\n255\n").into_bytes();
    for pixel in rgba8.chunks_exact(4) {
        bytes.extend_from_slice(&pixel[..3]);
    }
    fs::write(path, bytes).map_err(|error| format!("native proof PPM write failed: {error}"))
}

fn require_hardware_adapter(adapter: &scena::GpuAdapterReport) -> Result<(), String> {
    if !matches!(
        adapter.device_type.as_str(),
        "DiscreteGpu" | "IntegratedGpu" | "VirtualGpu"
    ) {
        return Err(format!(
            "required native proof needs a hardware adapter device type: {adapter:?}"
        ));
    }
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
        if identity.contains(marker) {
            return Err(format!(
                "required native proof rejects software adapter marker {marker}: {adapter:?}"
            ));
        }
    }
    Ok(())
}

fn resource_signature(stats: scena::RendererStats) -> [u64; 6] {
    [
        stats.buffers,
        stats.gpu_textures,
        stats.render_targets,
        stats.pipelines,
        stats.bind_groups,
        stats.shader_modules,
    ]
}

fn artifact_path() -> PathBuf {
    std::env::var_os("SCENA_HARDWARE_PROOF_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("target/gate-artifacts/pf01-pf02-native-surface/native-present-only.json")
}
