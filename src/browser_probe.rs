//! WASM-only browser proof hooks used by release-gate probes.

mod probes;
mod report;
mod workflows;

use report::{capabilities_json, diagnostics_json, stats_json};
use serde_json::json;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use workflows::{build_workflow_scene, scene_with_triangle};

use crate::{Assets, Backend, EnvironmentHandle, PlatformSurface, Renderer, Scene};

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
    render_scene(
        canvas,
        backend,
        workflow,
        &workflow_scene.assets,
        &mut workflow_scene.scene,
        workflow_scene.camera,
        workflow_scene.metadata,
        environment,
    )
    .await
}

async fn render_scene(
    canvas: HtmlCanvasElement,
    backend: Backend,
    workflow: &str,
    assets: &Assets,
    scene: &mut Scene,
    camera: crate::CameraKey,
    metadata: serde_json::Value,
    environment: Option<EnvironmentHandle>,
) -> Result<String, JsValue> {
    let width = canvas.width();
    let height = canvas.height();
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
    let mut renderer = Renderer::from_surface_async(surface)
        .await
        .map_err(|error| JsValue::from_str(&format!("build failed: {error:?}")))?;
    if let Some(environment) = environment {
        renderer.set_environment(environment);
    }

    renderer
        .prepare_with_assets(scene, assets)
        .map_err(|error| JsValue::from_str(&format!("prepare failed: {error:?}")))?;
    let outcome = renderer
        .render(scene, camera)
        .map_err(|error| JsValue::from_str(&format!("render failed: {error:?}")))?;
    let stats = renderer.stats();
    let capabilities = renderer.capabilities();

    Ok(json!({
        "schema": "scena.m6.browser_renderer_probe.v1",
        "status": "rendered",
        "workflow": workflow,
        "scene_api": "Scene",
        "assets_api": "Assets",
        "prepare_api": "Renderer::prepare_with_assets",
        "render_api": "Renderer::render",
        "metadata": metadata,
        "capabilities": capabilities_json(*capabilities),
        "diagnostics": diagnostics_json(renderer.diagnostics()),
        "backend": format!("{:?}", capabilities.backend),
        "gpu_device": capabilities.gpu_device,
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
    })
    .to_string())
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
