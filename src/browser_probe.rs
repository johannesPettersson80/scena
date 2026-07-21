//! WASM-only browser proof hooks used by release-gate probes.

mod dropped_file;
mod material_variant;
mod parity;
mod probes;
mod report;
mod workflows;

use base64::Engine;
use report::{capabilities_json, diagnostics_json, stats_json};
use serde_json::json;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use workflows::{build_workflow_scene, scene_with_triangle};

use crate::{
    ASSET_DOCTOR_REPORT_SCHEMA_V1, AntiAliasing, AssetFetcher, Assets, Backend, Background,
    CameraKey, EnvironmentHandle, FlyControls, FollowControls, OrbitControlAction, OrbitControls,
    OutputColorSpace, PerspectiveCamera, PixelReadback, PlatformSurface, PointerEvent, Profile,
    Quality, Renderer, RendererOptions, Scene, Tonemapper, Transform, Vec3, fnv1a64_hex,
    sample_rgba8,
};

#[wasm_bindgen(js_name = m6RenderWebgl2Probe)]
pub async fn m6_render_webgl2_probe(canvas: HtmlCanvasElement) -> Result<String, JsValue> {
    render_probe(canvas, Backend::WebGl2).await
}

#[wasm_bindgen(js_name = m6RenderWebgpuProbe)]
pub async fn m6_render_webgpu_probe(canvas: HtmlCanvasElement) -> Result<String, JsValue> {
    render_probe(canvas, Backend::WebGpu).await
}

#[wasm_bindgen(js_name = m6RenderWorkflowProbe)]
pub async fn m6_render_workflow_probe(
    canvas: HtmlCanvasElement,
    backend: String,
    workflow: String,
) -> Result<String, JsValue> {
    let backend = parse_browser_backend(&backend)?;
    render_workflow_probe(canvas, backend, &workflow).await
}

#[wasm_bindgen(js_name = m6AssetDoctorBrowserProbe)]
pub async fn m6_asset_doctor_browser_probe(url: String) -> Result<String, JsValue> {
    let report = Assets::new().doctor_asset_path(url.as_str()).await;
    let has_required_extension_finding = report
        .findings
        .iter()
        .any(|finding| finding.code == "unsupported_required_extension");
    Ok(json!({
        "schema": "scena.m6.asset_doctor_browser_proof.v1",
        "status": if report.schema == ASSET_DOCTOR_REPORT_SCHEMA_V1 && !report.ok && has_required_extension_finding {
            "passed"
        } else {
            "failed"
        },
        "proof_class": "asset-doctor-browser",
        "source": url,
        "doctor": report,
    })
    .to_string())
}

#[wasm_bindgen(js_name = m6RenderDisplayP3Probe)]
pub async fn m6_render_display_p3_probe(
    canvas: HtmlCanvasElement,
    backend: String,
) -> Result<String, JsValue> {
    let backend = parse_browser_backend(&backend)?;
    let assets = Assets::new();
    let (mut scene, camera) = scene_with_triangle();
    render_scene_with_options(
        canvas,
        backend,
        "display-p3-output",
        &assets,
        &mut scene,
        camera,
        json!({ "requested_output_color_space": "DisplayP3" }),
        None,
        RendererOptions::default().with_output_color_space(OutputColorSpace::DisplayP3),
    )
    .await
}

#[wasm_bindgen(js_name = m6RenderDroppedFileProbe)]
pub async fn m6_render_dropped_file_probe(
    canvas: HtmlCanvasElement,
    backend: String,
    bytes: Box<[u8]>,
    file_name: String,
) -> Result<String, JsValue> {
    let backend = parse_browser_backend(&backend)?;
    dropped_file::render_dropped_file_probe(canvas, backend, &bytes, &file_name).await
}

#[wasm_bindgen(js_name = m6RenderMaterialVariantProbe)]
pub async fn m6_render_material_variant_probe(
    canvas: HtmlCanvasElement,
    backend: String,
    variant_name: String,
) -> Result<String, JsValue> {
    let backend = parse_browser_backend(&backend)?;
    material_variant::render_material_variant_probe(canvas, backend, &variant_name).await
}

#[wasm_bindgen(js_name = m6RenderSurfaceLifecycleProbe)]
pub async fn m6_render_surface_lifecycle_probe(
    canvas: HtmlCanvasElement,
    backend: String,
) -> Result<String, JsValue> {
    let backend = parse_browser_backend(&backend)?;
    probes::render_surface_lifecycle_probe(canvas, backend).await
}

#[wasm_bindgen(js_name = m6RenderBenchmarkProbe)]
pub async fn m6_render_benchmark_probe(
    canvas: HtmlCanvasElement,
    backend: String,
) -> Result<String, JsValue> {
    let backend = parse_browser_backend(&backend)?;
    probes::render_benchmark_probe(canvas, backend).await
}

#[wasm_bindgen(js_name = m6RenderStateLifecycleProbe)]
pub async fn m6_render_state_lifecycle_probe(
    canvas: HtmlCanvasElement,
    backend: String,
) -> Result<String, JsValue> {
    let backend = parse_browser_backend(&backend)?;
    probes::render_state_lifecycle_probe(canvas, backend).await
}

#[wasm_bindgen(js_name = m6CameraControlKitProbe)]
pub fn m6_camera_control_kit_probe() -> Result<String, JsValue> {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::standard(),
            Transform::at(Vec3::new(0.0, 0.0, 5.0)),
        )
        .map_err(|error| JsValue::from_str(&format!("camera insert failed: {error:?}")))?;
    scene
        .set_active_camera(camera)
        .map_err(|error| JsValue::from_str(&format!("active camera failed: {error:?}")))?;

    let mut orbit = OrbitControls::new(Vec3::ZERO, 4.0)
        .presentation()
        .zoom_limits_bounds_relative(0.5, 2.0);
    let initial_distance = orbit.distance();
    let orbit_actions = vec![
        orbit.handle_pointer(PointerEvent::primary_pressed(160.0, 120.0)),
        orbit.handle_pointer(PointerEvent::moved(184.0, 108.0, 24.0, -12.0)),
        orbit.handle_pointer(PointerEvent::wheel(184.0, 108.0, -1.0)),
        orbit.handle_pointer(PointerEvent::released(184.0, 108.0)),
        orbit.advance(1.0 / 30.0),
    ];
    orbit
        .apply_to_scene(&mut scene, camera)
        .map_err(|error| JsValue::from_str(&format!("orbit apply failed: {error:?}")))?;
    let orbit_camera = camera_translation(&scene, camera)?;

    let target = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(2.0, 0.5, -1.0)))
        .map_err(|error| JsValue::from_str(&format!("target insert failed: {error:?}")))?;
    FollowControls::behind_and_above(3.0, 1.25)
        .apply_to_scene(&mut scene, camera, target)
        .map_err(|error| JsValue::from_str(&format!("follow apply failed: {error:?}")))?;
    let follow_camera = camera_translation(&scene, camera)?;

    let mut fly = FlyControls::new(Vec3::ZERO)
        .with_yaw_pitch_degrees(90.0, 0.0)
        .with_move_speed(2.0);
    let fly_move = fly.move_local(1.0, 0.5, 0.25, 2.0);
    let fly_look = fly.look_delta(8.0, -4.0);
    fly.apply_to_scene(&mut scene, camera)
        .map_err(|error| JsValue::from_str(&format!("fly apply failed: {error:?}")))?;
    let fly_camera = camera_translation(&scene, camera)?;

    let passed = orbit_actions.contains(&OrbitControlAction::BeginOrbit)
        && orbit_actions.contains(&OrbitControlAction::Orbit)
        && orbit_actions.contains(&OrbitControlAction::Zoom)
        && orbit_actions.contains(&OrbitControlAction::End)
        && orbit.distance() < initial_distance
        && follow_camera.y > 0.5
        && fly_camera.x > 0.0
        && fly_camera.z < 0.0
        && matches!(fly_move, OrbitControlAction::Pan)
        && matches!(fly_look, OrbitControlAction::Orbit);

    Ok(json!({
        "schema": "scena.m6.camera_control_kit_browser_proof.v1",
        "status": if passed { "passed" } else { "failed" },
        "proof_class": "browser-demo",
        "visual_proof": "browser-demo",
        "orbit": {
            "initial_distance": initial_distance,
            "distance_after_zoom": orbit.distance(),
            "yaw_radians": orbit.yaw_radians(),
            "pitch_radians": orbit.pitch_radians(),
            "actions": orbit_actions.iter().map(|action| action_name(*action)).collect::<Vec<_>>(),
            "camera_translation": vec3_json(orbit_camera),
        },
        "follow": {
            "target_translation": [2.0, 0.5, -1.0],
            "camera_translation": vec3_json(follow_camera),
        },
        "fly": {
            "move_action": action_name(fly_move),
            "look_action": action_name(fly_look),
            "camera_translation": vec3_json(fly_camera),
            "yaw_radians": fly.yaw_radians(),
            "pitch_radians": fly.pitch_radians(),
        },
    })
    .to_string())
}

async fn render_probe(canvas: HtmlCanvasElement, backend: Backend) -> Result<String, JsValue> {
    let assets = Assets::new();
    let (mut scene, camera) = scene_with_triangle();
    render_scene(
        canvas,
        backend,
        "triangle",
        &assets,
        &mut scene,
        camera,
        json!({}),
        None,
    )
    .await
}

async fn render_workflow_probe(
    canvas: HtmlCanvasElement,
    backend: Backend,
    workflow: &str,
) -> Result<String, JsValue> {
    let mut workflow_scene = build_workflow_scene(workflow).await?;
    let environment = if let Some(path) = workflow_scene
        .metadata
        .get("environment_path")
        .and_then(|value| value.as_str())
    {
        Some(
            workflow_scene
                .assets
                .load_environment(path)
                .await
                .map_err(|error| {
                    JsValue::from_str(&format!("environment load failed: {error:?}"))
                })?,
        )
    } else {
        None
    };
    let renderer_options = match workflow {
        "color-transfer-no-post" => RendererOptions::default().with_quality(Quality::Low),
        "color-transfer-post" => RendererOptions::default().with_quality(Quality::Medium),
        "msaa-capability" => RendererOptions::default().with_profile(Profile::Quality),
        _ => RendererOptions::default(),
    };
    render_scene_with_options(
        canvas,
        backend,
        workflow,
        &workflow_scene.assets,
        &mut workflow_scene.scene,
        workflow_scene.camera,
        workflow_scene.metadata,
        environment,
        renderer_options,
    )
    .await
}

async fn render_scene<F: AssetFetcher>(
    canvas: HtmlCanvasElement,
    backend: Backend,
    workflow: &str,
    assets: &Assets<F>,
    scene: &mut Scene,
    camera: crate::CameraKey,
    metadata: serde_json::Value,
    environment: Option<EnvironmentHandle>,
) -> Result<String, JsValue> {
    render_scene_with_options(
        canvas,
        backend,
        workflow,
        assets,
        scene,
        camera,
        metadata,
        environment,
        RendererOptions::default(),
    )
    .await
}

async fn render_scene_with_options<F: AssetFetcher>(
    canvas: HtmlCanvasElement,
    backend: Backend,
    workflow: &str,
    assets: &Assets<F>,
    scene: &mut Scene,
    camera: crate::CameraKey,
    metadata: serde_json::Value,
    environment: Option<EnvironmentHandle>,
    renderer_options: RendererOptions,
) -> Result<String, JsValue> {
    let width = canvas.width();
    let height = canvas.height();
    let cpu_parity_required = match backend {
        Backend::WebGl2 => matches!(
            workflow,
            "triangle" | "color-transfer-no-post" | "color-transfer-post"
        ),
        Backend::WebGpu => workflow == "triangle",
        _ => false,
    };
    let cpu_parity_frame = if cpu_parity_required {
        let mut cpu_renderer = Renderer::headless_with_options(width, height, renderer_options)
            .map_err(|error| JsValue::from_str(&format!("CPU parity build failed: {error:?}")))?;
        configure_probe_renderer(&mut cpu_renderer, &metadata, environment);
        cpu_renderer
            .prepare_with_assets(scene, assets)
            .map_err(|error| JsValue::from_str(&format!("CPU parity prepare failed: {error:?}")))?;
        cpu_renderer
            .render(scene, camera)
            .map_err(|error| JsValue::from_str(&format!("CPU parity render failed: {error:?}")))?;
        Some(cpu_renderer.read_pixels())
    } else {
        None
    };
    let surface = match backend {
        Backend::WebGl2 => PlatformSurface::browser_webgl2_canvas_element(canvas, width, height),
        Backend::WebGpu => PlatformSurface::browser_webgpu_canvas_element(canvas, width, height),
        Backend::Headless
        | Backend::HeadlessGpu
        | Backend::SurfaceDescriptor
        | Backend::NativeSurface => {
            return Err(JsValue::from_str("browser probe requires WebGL2 or WebGPU"));
        }
    };
    let mut renderer = Renderer::from_surface_async_with_options(surface, renderer_options)
        .await
        .map_err(|error| JsValue::from_str(&format!("build failed: {error:?}")))?;
    configure_probe_renderer(&mut renderer, &metadata, environment);

    renderer
        .prepare_with_assets(scene, assets)
        .map_err(|error| JsValue::from_str(&format!("prepare failed: {error:?}")))?;
    let outcome = renderer
        .render(scene, camera)
        .map_err(|error| JsValue::from_str(&format!("render failed: {error:?}")))?;
    let renderer_readback = renderer.browser_readback_rgba8().await?;
    let parity = cpu_parity_frame
        .as_ref()
        .map(|cpu| parity::cpu_browser_gpu_report(cpu, renderer_readback.as_ref(), backend));
    let renderer_readback = renderer_readback.as_ref().map(renderer_readback_json);
    let stats = renderer.stats();
    let capabilities = *renderer.capabilities();
    let adapter = renderer.gpu_adapter_report();
    let selected_anti_aliasing = format!("{:?}", renderer.anti_aliasing());
    let diagnostics = diagnostics_json(renderer.diagnostics());
    let explicit_msaa = if workflow == "msaa-capability" {
        renderer.set_anti_aliasing(AntiAliasing::Msaa4);
        match renderer.prepare_with_assets(scene, assets) {
            Err(crate::PrepareError::UnsupportedSampleCount {
                requested, maximum, ..
            }) => Some(json!({
                "status": "rejected",
                "requested": requested,
                "maximum": maximum,
            })),
            Ok(()) => Some(json!({ "status": "accepted" })),
            Err(error) => Some(json!({
                "status": "wrong_error",
                "error": format!("{error:?}"),
            })),
        }
    } else {
        None
    };

    Ok(json!({
        "schema": "scena.m6.browser_renderer_probe.v1",
        "status": "rendered",
        "workflow": workflow,
        "metadata": metadata,
        "capabilities": capabilities_json(capabilities),
        "diagnostics": diagnostics,
        "anti_aliasing": selected_anti_aliasing,
        "explicit_msaa": explicit_msaa,
        "requested_output_color_space": format!("{:?}", renderer.output_color_space()),
        "backend": format!("{:?}", capabilities.backend),
        "gpu_device": capabilities.gpu_device,
        "adapter": adapter,
        "surface_attached": capabilities.surface_attached,
        "width": outcome.width,
        "height": outcome.height,
        "draw_calls": outcome.draw_calls,
        "primitives": outcome.primitives,
        "stats": stats_json(stats),
        "gpu_submissions": stats.gpu_submissions,
        "prepared_buffers": stats.buffers,
        "prepared_pipelines": stats.pipelines,
        "prepared_bind_groups": stats.bind_groups,
        "renderer_readback": renderer_readback,
        "parity": parity,
    })
    .to_string())
}

fn configure_probe_renderer(
    renderer: &mut Renderer,
    metadata: &serde_json::Value,
    environment: Option<EnvironmentHandle>,
) {
    if let Some(environment) = environment {
        renderer.set_environment(environment);
    }
    if metadata.get("background").and_then(|value| value.as_str()) == Some("neutral_gray") {
        renderer.set_background(Background::NeutralGray);
    }
    if let Some(exposure_ev) = metadata.get("exposure_ev").and_then(|value| value.as_f64()) {
        renderer.set_exposure_ev(exposure_ev as f32);
    }
    if metadata.get("tonemapper").and_then(|value| value.as_str()) == Some("standard") {
        renderer.set_tonemapper(Tonemapper::Standard);
    }
}

fn camera_translation(scene: &Scene, camera: CameraKey) -> Result<Vec3, JsValue> {
    let camera_node = scene
        .camera_node(camera)
        .ok_or_else(|| JsValue::from_str("camera node missing"))?;
    let transform = scene
        .world_transform(camera_node)
        .ok_or_else(|| JsValue::from_str("camera world transform missing"))?;
    Ok(transform.translation)
}

fn action_name(action: OrbitControlAction) -> &'static str {
    match action {
        OrbitControlAction::None => "None",
        OrbitControlAction::BeginOrbit => "BeginOrbit",
        OrbitControlAction::Orbit => "Orbit",
        OrbitControlAction::Pan => "Pan",
        OrbitControlAction::Zoom => "Zoom",
        OrbitControlAction::End => "End",
    }
}

fn vec3_json(value: Vec3) -> [f32; 3] {
    [value.x, value.y, value.z]
}

pub(super) fn renderer_readback_json(readback: &PixelReadback) -> serde_json::Value {
    json!({
        "source": "renderer-owned-gpu-copy",
        "width": readback.width(),
        "height": readback.height(),
        "rgba8_fnv1a64": fnv1a64_hex(readback.rgba8()),
        "rgba8_base64": base64::engine::general_purpose::STANDARD.encode(readback.rgba8()),
        "pixel_statistics": summarize_pixel_readback(readback),
    })
}

fn summarize_pixel_readback(readback: &PixelReadback) -> serde_json::Value {
    let width = readback.width();
    let height = readback.height();
    let rgba = readback.rgba8();
    let mut nonblack = 0_u64;
    let mut max = [0_u8; 4];
    for pixel in rgba.chunks_exact(4) {
        if pixel[0] > 0 || pixel[1] > 0 || pixel[2] > 0 {
            nonblack = nonblack.saturating_add(1);
        }
        max[0] = max[0].max(pixel[0]);
        max[1] = max[1].max(pixel[1]);
        max[2] = max[2].max(pixel[2]);
        max[3] = max[3].max(pixel[3]);
    }
    json!({
        "center": sample_rgba8(rgba, width, height, width as f32 / 2.0, height as f32 / 2.0),
        "left": sample_rgba8(rgba, width, height, width as f32 * 0.25, height as f32 / 2.0),
        "right": sample_rgba8(rgba, width, height, width as f32 * 0.75, height as f32 / 2.0),
        "flat": sample_rgba8(rgba, width, height, width as f32 * 0.38, height as f32 / 2.0),
        "inverted": sample_rgba8(rgba, width, height, width as f32 * 0.62, height as f32 / 2.0),
        "nonblack": nonblack,
        "max": max,
    })
}

fn parse_browser_backend(value: &str) -> Result<Backend, JsValue> {
    match value {
        "webgl2" | "WebGl2" => Ok(Backend::WebGl2),
        "webgpu" | "WebGpu" => Ok(Backend::WebGpu),
        other => Err(JsValue::from_str(&format!(
            "browser probe backend must be webgl2 or webgpu, got {other}"
        ))),
    }
}
