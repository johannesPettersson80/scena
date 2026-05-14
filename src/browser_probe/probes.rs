use serde_json::json;
use wasm_bindgen::prelude::JsValue;
use web_sys::HtmlCanvasElement;

use super::renderer_readback_json;
use super::report::{capabilities_json, diagnostics_json, stats_json};
use super::workflows::{
    animation_scene, build_workflow_scene, instancing_scene_with_count, picking_selection_scene,
};
use crate::{
    Assets, Backend, NotPreparedReason, PlatformSurface, RenderError, RenderMode, Renderer,
    RendererOptions, RetainPolicy, SurfaceEvent,
};

mod state_lifecycle;

pub(super) use state_lifecycle::render_state_lifecycle_probe;

pub(super) async fn render_surface_lifecycle_probe(
    canvas: HtmlCanvasElement,
    backend: Backend,
) -> Result<String, JsValue> {
    let workflow_scene = build_workflow_scene("material-textures").await?;
    let mut assets = workflow_scene.assets;
    assets.set_retain_policy(RetainPolicy::OnContextLossOnly);
    let mut scene = workflow_scene.scene;
    let camera = workflow_scene.camera;
    let mut renderer = Renderer::from_surface_async(browser_surface(
        &canvas,
        backend,
        "browser lifecycle probe",
    )?)
    .await
    .map_err(|error| JsValue::from_str(&format!("build failed: {error:?}")))?;
    let mut events = Vec::new();

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .map_err(|error| JsValue::from_str(&format!("initial prepare failed: {error:?}")))?;
    let initial = renderer
        .render(&scene, camera)
        .map_err(|error| JsValue::from_str(&format!("initial render failed: {error:?}")))?;
    events.extend(["prepare", "render"]);

    canvas.set_width(80);
    canvas.set_height(48);
    renderer
        .handle_surface_event(SurfaceEvent::Resize {
            width: 80,
            height: 48,
        })
        .map_err(|error| JsValue::from_str(&format!("resize event failed: {error:?}")))?;
    renderer
        .handle_surface_event(SurfaceEvent::ScaleFactorChanged { scale_factor: 2.0 })
        .map_err(|error| JsValue::from_str(&format!("DPR event failed: {error:?}")))?;
    events.extend(["resize", "scale-factor"]);

    let target_changed = matches!(
        renderer.render(&scene, camera),
        Err(RenderError::NotPrepared {
            reason: NotPreparedReason::TargetChanged { .. }
        })
    );
    if !target_changed {
        return Err(JsValue::from_str(
            "render after resize/DPR should require explicit prepare",
        ));
    }
    events.push("not-prepared-target-changed");

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .map_err(|error| JsValue::from_str(&format!("resize prepare failed: {error:?}")))?;
    let resized = renderer
        .render(&scene, camera)
        .map_err(|error| JsValue::from_str(&format!("resize render failed: {error:?}")))?;
    events.extend(["reprepare-after-resize", "render-after-resize"]);

    verify_context_recovery(&mut renderer, &assets, &mut scene, camera, &mut events)?;
    renderer
        .handle_surface_event(SurfaceEvent::Lost)
        .map_err(|error| JsValue::from_str(&format!("surface lost event failed: {error:?}")))?;
    let surface_lost = matches!(
        renderer.render(&scene, camera),
        Err(RenderError::SurfaceLost { recoverable: true })
    );
    if !surface_lost {
        return Err(JsValue::from_str(
            "render while surface lost should return structured SurfaceLost",
        ));
    }
    let final_outcome = rebuild_after_surface_loss(canvas, backend, &assets, &mut scene, camera)
        .await
        .map_err(|error| JsValue::from_str(&format!("surface rebuild failed: {error:?}")))?;
    events.extend([
        "surface-lost",
        "rebuild-surface-renderer",
        "final-prepare",
        "final-render",
    ]);

    let stats = final_outcome.renderer_stats;
    let capabilities = final_outcome.capabilities;
    Ok(json!({
        "schema": "scena.m6.browser_surface_lifecycle_probe.v1",
        "status": "rendered",
        "workflow": "surface-context-lifecycle",
        "scene_api": "Scene",
        "assets_api": "Assets",
        "prepare_api": "Renderer::prepare_with_assets",
        "render_api": "Renderer::render",
        "event_sequence": events,
        "retain_policy": format!("{:?}", assets.retain_policy()),
        "initial": { "width": initial.width, "height": initial.height, "draw_calls": initial.draw_calls },
        "resized": { "width": resized.width, "height": resized.height, "draw_calls": resized.draw_calls },
        "context_recovered": final_outcome.context_recovered,
        "device_recovered": final_outcome.device_recovered,
        "final_prepare": "ok",
        "final": {
            "width": final_outcome.render.width,
            "height": final_outcome.render.height,
            "draw_calls": final_outcome.render.draw_calls,
            "primitives": final_outcome.render.primitives,
        },
        "capabilities": capabilities_json(capabilities),
        "diagnostics": final_outcome.diagnostics,
        "backend": format!("{:?}", capabilities.backend),
        "gpu_device": capabilities.gpu_device,
        "surface_attached": capabilities.surface_attached,
        "width": final_outcome.render.width,
        "height": final_outcome.render.height,
        "draw_calls": final_outcome.render.draw_calls,
        "primitives": final_outcome.render.primitives,
        "stats": stats_json(stats),
        "material_texture_bindings": stats.material_texture_bindings,
        "material_sampler_bindings": stats.material_sampler_bindings,
        "gpu_submissions": stats.gpu_submissions,
        "prepared_buffers": stats.buffers,
        "prepared_pipelines": stats.pipelines,
        "prepared_bind_groups": stats.bind_groups,
        "renderer_readback": final_outcome.renderer_readback,
    })
    .to_string())
}

pub(super) async fn render_benchmark_probe(
    canvas: HtmlCanvasElement,
    backend: Backend,
) -> Result<String, JsValue> {
    let mut workflow_scene = instancing_scene_with_count(128);
    let mut renderer = Renderer::from_surface_async_with_options(
        browser_surface(&canvas, backend, "browser benchmark probe")?,
        RendererOptions::default().with_render_mode(RenderMode::OnChange),
    )
    .await
    .map_err(|error| JsValue::from_str(&format!("benchmark build failed: {error:?}")))?;

    let start = js_sys::Date::now();
    renderer
        .prepare_with_assets(&mut workflow_scene.scene, &workflow_scene.assets)
        .map_err(|error| JsValue::from_str(&format!("benchmark prepare failed: {error:?}")))?;
    let first_prepare_ms = js_sys::Date::now() - start;

    let start = js_sys::Date::now();
    let first_render = renderer
        .render(&workflow_scene.scene, workflow_scene.camera)
        .map_err(|error| JsValue::from_str(&format!("benchmark render failed: {error:?}")))?;
    let first_render_ms = js_sys::Date::now() - start;

    let start = js_sys::Date::now();
    let idle_render = renderer
        .render(&workflow_scene.scene, workflow_scene.camera)
        .map_err(|error| JsValue::from_str(&format!("benchmark idle render failed: {error:?}")))?;
    let steady_state_render_ms = js_sys::Date::now() - start;
    let (resized_render, resize_dpr_ms) =
        resize_and_render(&canvas, &mut renderer, &mut workflow_scene).map_err(|error| {
            JsValue::from_str(&format!("benchmark resize/render failed: {error:?}"))
        })?;

    let picking_start = js_sys::Date::now();
    let picking_scene = picking_selection_scene()?;
    let picking_ms = js_sys::Date::now() - picking_start;
    let animation_start = js_sys::Date::now();
    let animation_scene = animation_scene().await?;
    let animation_tick_ms = js_sys::Date::now() - animation_start;

    let stats = renderer.stats();
    let capabilities = renderer.capabilities();
    Ok(json!({
        "schema": "scena.m6.browser_benchmark_probe.v1",
        "status": "rendered",
        "workflow": "benchmark-idle",
        "scene_api": "Scene",
        "assets_api": "Assets",
        "prepare_api": "Renderer::prepare_with_assets",
        "render_api": "Renderer::render",
        "render_mode": "OnChange",
        "benchmark_metrics": {
            "first_prepare_ms": first_prepare_ms,
            "first_render_ms": first_render_ms,
            "steady_state_render_ms": steady_state_render_ms,
            "resize_dpr_ms": resize_dpr_ms,
            "picking_ms": picking_ms,
            "animation_tick_ms": animation_tick_ms,
            "high_instance_count": 128,
            "high_instance_primitives": first_render.primitives,
            "idle_render_skipped": idle_render.skipped,
            "resized_width": resized_render.width,
            "resized_height": resized_render.height,
        },
        "metadata": { "picking": picking_scene.metadata, "animation": animation_scene.metadata },
        "capabilities": capabilities_json(*capabilities),
        "diagnostics": diagnostics_json(renderer.diagnostics()),
        "backend": format!("{:?}", capabilities.backend),
        "gpu_device": capabilities.gpu_device,
        "surface_attached": capabilities.surface_attached,
        "width": resized_render.width,
        "height": resized_render.height,
        "draw_calls": first_render.draw_calls,
        "primitives": first_render.primitives,
        "stats": stats_json(stats),
        "gpu_submissions": stats.gpu_submissions,
        "prepared_buffers": stats.buffers,
        "prepared_pipelines": stats.pipelines,
        "prepared_bind_groups": stats.bind_groups,
    })
    .to_string())
}

fn verify_context_recovery(
    renderer: &mut Renderer,
    assets: &Assets,
    scene: &mut crate::Scene,
    camera: crate::CameraKey,
    events: &mut Vec<&'static str>,
) -> Result<(), JsValue> {
    renderer
        .handle_surface_event(SurfaceEvent::Hidden)
        .map_err(|error| JsValue::from_str(&format!("hidden event failed: {error:?}")))?;
    renderer
        .handle_surface_event(SurfaceEvent::Shown)
        .map_err(|error| JsValue::from_str(&format!("shown event failed: {error:?}")))?;
    renderer
        .handle_surface_event(SurfaceEvent::Occluded { occluded: false })
        .map_err(|error| JsValue::from_str(&format!("occluded event failed: {error:?}")))?;
    events.extend(["hidden", "shown", "occluded"]);
    verify_loss_and_recovery(
        renderer,
        assets,
        scene,
        camera,
        SurfaceEvent::ContextLost { recoverable: true },
        "context",
    )?;
    events.extend([
        "context-lost",
        "context-restored",
        "recover-context",
        "render-after-context-recovery",
    ]);
    verify_loss_and_recovery(
        renderer,
        assets,
        scene,
        camera,
        SurfaceEvent::DeviceLost { recoverable: true },
        "device",
    )?;
    events.extend([
        "device-lost",
        "recover-device",
        "render-after-device-recovery",
    ]);
    Ok(())
}

fn verify_loss_and_recovery(
    renderer: &mut Renderer,
    assets: &Assets,
    scene: &mut crate::Scene,
    camera: crate::CameraKey,
    event: SurfaceEvent,
    label: &str,
) -> Result<(), JsValue> {
    renderer
        .handle_surface_event(event)
        .map_err(|error| JsValue::from_str(&format!("{label} lost event failed: {error:?}")))?;
    let lost_is_structured = match label {
        "context" => matches!(
            renderer.render(scene, camera),
            Err(RenderError::ContextLost { recoverable: true })
        ),
        "device" => matches!(
            renderer.render(scene, camera),
            Err(RenderError::GpuDeviceLost { recoverable: true })
        ),
        _ => false,
    };
    if !lost_is_structured {
        return Err(JsValue::from_str(&format!(
            "render while {label} lost should return a structured loss error"
        )));
    }
    if label == "context" {
        renderer
            .handle_surface_event(SurfaceEvent::ContextRestored)
            .map_err(|error| {
                JsValue::from_str(&format!("{label} restored event failed: {error:?}"))
            })?;
    }
    renderer
        .recover_context(assets, scene)
        .map_err(|error| JsValue::from_str(&format!("{label} recovery failed: {error:?}")))?;
    renderer
        .prepare_with_assets(scene, assets)
        .map_err(|error| JsValue::from_str(&format!("{label} prepare failed: {error:?}")))?;
    renderer
        .render(scene, camera)
        .map_err(|error| JsValue::from_str(&format!("{label} render failed: {error:?}")))?;
    Ok(())
}

async fn rebuild_after_surface_loss(
    canvas: HtmlCanvasElement,
    backend: Backend,
    assets: &Assets,
    scene: &mut crate::Scene,
    camera: crate::CameraKey,
) -> Result<LifecycleFinal, JsValue> {
    let mut renderer = Renderer::from_surface_async(browser_surface(
        &canvas,
        backend,
        "browser lifecycle probe",
    )?)
    .await
    .map_err(|error| JsValue::from_str(&format!("surface rebuild failed: {error:?}")))?;
    renderer
        .prepare_with_assets(scene, assets)
        .map_err(|error| JsValue::from_str(&format!("final prepare failed: {error:?}")))?;
    let render = renderer
        .render(scene, camera)
        .map_err(|error| JsValue::from_str(&format!("final render failed: {error:?}")))?;
    let renderer_readback = renderer
        .browser_probe_readback_rgba8()
        .await?
        .map(|readback| renderer_readback_json(&readback));
    Ok(LifecycleFinal {
        render,
        renderer_stats: renderer.stats(),
        capabilities: *renderer.capabilities(),
        diagnostics: diagnostics_json(renderer.diagnostics()),
        context_recovered: json!({ "draw_calls": render.draw_calls }),
        device_recovered: json!({ "draw_calls": render.draw_calls }),
        renderer_readback,
    })
}

fn resize_and_render(
    canvas: &HtmlCanvasElement,
    renderer: &mut Renderer,
    workflow_scene: &mut super::workflows::WorkflowScene,
) -> Result<(crate::RenderOutcome, f64), JsValue> {
    canvas.set_width(96);
    canvas.set_height(64);
    let start = js_sys::Date::now();
    renderer
        .handle_surface_event(SurfaceEvent::Resize {
            width: 96,
            height: 64,
        })
        .map_err(|error| JsValue::from_str(&format!("resize event failed: {error:?}")))?;
    renderer
        .handle_surface_event(SurfaceEvent::ScaleFactorChanged { scale_factor: 2.0 })
        .map_err(|error| JsValue::from_str(&format!("DPR event failed: {error:?}")))?;
    renderer
        .prepare_with_assets(&mut workflow_scene.scene, &workflow_scene.assets)
        .map_err(|error| JsValue::from_str(&format!("resize prepare failed: {error:?}")))?;
    let render = renderer
        .render(&workflow_scene.scene, workflow_scene.camera)
        .map_err(|error| JsValue::from_str(&format!("resize render failed: {error:?}")))?;
    Ok((render, js_sys::Date::now() - start))
}

fn browser_surface(
    canvas: &HtmlCanvasElement,
    backend: Backend,
    label: &str,
) -> Result<PlatformSurface, JsValue> {
    match backend {
        Backend::WebGl2 => Ok(PlatformSurface::browser_webgl2_canvas_element(
            canvas.clone(),
            canvas.width(),
            canvas.height(),
        )),
        Backend::WebGpu => Ok(PlatformSurface::browser_webgpu_canvas_element(
            canvas.clone(),
            canvas.width(),
            canvas.height(),
        )),
        Backend::Headless
        | Backend::HeadlessGpu
        | Backend::SurfaceDescriptor
        | Backend::NativeSurface => Err(JsValue::from_str(&format!(
            "{label} requires WebGL2 or WebGPU"
        ))),
    }
}

struct LifecycleFinal {
    render: crate::RenderOutcome,
    renderer_stats: crate::RendererStats,
    capabilities: crate::Capabilities,
    diagnostics: serde_json::Value,
    context_recovered: serde_json::Value,
    device_recovered: serde_json::Value,
    renderer_readback: Option<serde_json::Value>,
}
