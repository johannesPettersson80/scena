use wasm_bindgen::prelude::*;

use super::{SceneHost, SceneHostCore};
use crate::scene_host::{SceneHostError, SceneHostErrorCode};
use crate::{Assets, PlatformSurface, RenderOutcome, Renderer, SurfaceViewport};

#[derive(Debug, Clone, Copy)]
pub(super) enum BrowserBackend {
    WebGpu,
    WebGl2,
}

pub(super) async fn build_from_canvas(
    backend: BrowserBackend,
    canvas: web_sys::HtmlCanvasElement,
    logical_width: f32,
    logical_height: f32,
    device_pixel_ratio: f32,
) -> Result<SceneHost, JsValue> {
    let browser_canvas = canvas.clone();
    let (surface, viewport) = surface_from_canvas(
        backend,
        canvas,
        logical_width,
        logical_height,
        device_pixel_ratio,
    )?;
    let renderer = Renderer::from_surface_async(surface)
        .await
        .map_err(js_error)?;
    let core = SceneHostCore::from_renderer(Assets::new(), renderer, viewport).map_err(js_error)?;
    Ok(SceneHost {
        core,
        browser_canvas: Some(browser_canvas),
    })
}

pub(super) fn surface_from_canvas(
    backend: BrowserBackend,
    canvas: web_sys::HtmlCanvasElement,
    logical_width: f32,
    logical_height: f32,
    device_pixel_ratio: f32,
) -> Result<(PlatformSurface, SurfaceViewport), JsValue> {
    let viewport = SurfaceViewport::new(logical_width, logical_height, device_pixel_ratio)
        .ok_or_else(|| {
            js_error(SceneHostError::new(
                SceneHostErrorCode::InvalidViewport,
                format!(
                    "invalid viewport {logical_width}x{logical_height} at DPR {device_pixel_ratio}"
                ),
            ))
        })?;
    let size = viewport.physical_size();
    let surface = match backend {
        BrowserBackend::WebGpu => {
            PlatformSurface::browser_webgpu_canvas_element(canvas, size.width, size.height)
        }
        BrowserBackend::WebGl2 => {
            PlatformSurface::browser_webgl2_canvas_element(canvas, size.width, size.height)
        }
    };
    Ok((surface, viewport))
}

pub(super) fn render_outcome_json(outcome: RenderOutcome) -> String {
    serde_json::json!({
        "width": outcome.width,
        "height": outcome.height,
        "draw_calls": outcome.draw_calls,
        "primitives": outcome.primitives,
        "skipped": outcome.skipped,
    })
    .to_string()
}

pub(super) fn render_outcome_js(outcome: RenderOutcome) -> JsValue {
    let object = js_sys::Object::new();
    for (field, value) in [
        ("width", f64::from(outcome.width)),
        ("height", f64::from(outcome.height)),
        ("draw_calls", outcome.draw_calls as f64),
        ("primitives", outcome.primitives as f64),
    ] {
        let _ = js_sys::Reflect::set(
            &object,
            &JsValue::from_str(field),
            &JsValue::from_f64(value),
        );
    }
    let _ = js_sys::Reflect::set(
        &object,
        &JsValue::from_str("skipped"),
        &JsValue::from_bool(outcome.skipped),
    );
    object.into()
}

pub(in crate::scene_host) fn js_error(error: impl Into<SceneHostError>) -> JsValue {
    let error = error.into();
    let object = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &object,
        &JsValue::from_str("code"),
        &JsValue::from_str(&format!("{:?}", error.code())),
    );
    let _ = js_sys::Reflect::set(
        &object,
        &JsValue::from_str("message"),
        &JsValue::from_str(error.message()),
    );
    object.into()
}
