use wasm_bindgen::prelude::*;

use super::inputs::vec3_array_from_slice;
use super::wasm_readback::browser_canvas_rgba8;
use super::{SceneHostCore, SceneHostError};
use crate::{Assets, CaptureRgba8, PlatformSurface, RenderOutcome, Renderer, SurfaceViewport};

#[wasm_bindgen]
pub struct SceneHost {
    pub(super) core: SceneHostCore,
    browser_canvas: Option<web_sys::HtmlCanvasElement>,
}

#[wasm_bindgen]
impl SceneHost {
    #[wasm_bindgen(js_name = newWebgl2)]
    pub async fn new_webgl2(
        canvas: web_sys::HtmlCanvasElement,
        logical_width: f32,
        logical_height: f32,
        device_pixel_ratio: f32,
    ) -> Result<SceneHost, JsValue> {
        build_from_canvas(
            BrowserBackend::WebGl2,
            canvas,
            logical_width,
            logical_height,
            device_pixel_ratio,
        )
        .await
    }

    #[wasm_bindgen(js_name = newWebgpu)]
    pub async fn new_webgpu(
        canvas: web_sys::HtmlCanvasElement,
        logical_width: f32,
        logical_height: f32,
        device_pixel_ratio: f32,
    ) -> Result<SceneHost, JsValue> {
        build_from_canvas(
            BrowserBackend::WebGpu,
            canvas,
            logical_width,
            logical_height,
            device_pixel_ratio,
        )
        .await
    }

    #[wasm_bindgen(js_name = rootHandle)]
    pub fn root_handle(&self) -> u64 {
        self.core.root_handle()
    }

    pub fn backend(&self) -> String {
        serde_json::to_value(self.core.backend())
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_else(|| format!("{:?}", self.core.backend()))
    }

    pub fn resize(
        &mut self,
        logical_width: f32,
        logical_height: f32,
        device_pixel_ratio: f32,
    ) -> Result<(), JsValue> {
        self.core
            .resize(logical_width, logical_height, device_pixel_ratio)
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = attachCanvasWebgl2)]
    pub async fn attach_canvas_webgl2(
        &mut self,
        canvas: web_sys::HtmlCanvasElement,
        logical_width: f32,
        logical_height: f32,
        device_pixel_ratio: f32,
    ) -> Result<(), JsValue> {
        self.attach_canvas(
            BrowserBackend::WebGl2,
            canvas,
            logical_width,
            logical_height,
            device_pixel_ratio,
        )
        .await
    }

    #[wasm_bindgen(js_name = attachCanvasWebgpu)]
    pub async fn attach_canvas_webgpu(
        &mut self,
        canvas: web_sys::HtmlCanvasElement,
        logical_width: f32,
        logical_height: f32,
        device_pixel_ratio: f32,
    ) -> Result<(), JsValue> {
        self.attach_canvas(
            BrowserBackend::WebGpu,
            canvas,
            logical_width,
            logical_height,
            device_pixel_ratio,
        )
        .await
    }

    #[wasm_bindgen(js_name = setTag)]
    pub fn set_tag(&mut self, node: u64, tag: String) -> Result<(), JsValue> {
        self.core.set_tag(node, &tag).map_err(js_error)
    }

    #[wasm_bindgen(js_name = clearTag)]
    pub fn clear_tag(&mut self, node: u64, tag: String) -> Result<bool, JsValue> {
        self.core.clear_tag(node, &tag).map_err(js_error)
    }

    #[wasm_bindgen(js_name = findByTag)]
    pub fn find_by_tag(&mut self, tag: String) -> Vec<u64> {
        self.core.find_by_tag(&tag)
    }

    #[wasm_bindgen(js_name = instantiateUrl)]
    pub async fn instantiate_url(&mut self, url: String) -> Result<u64, JsValue> {
        self.core.instantiate_url(url).await.map_err(js_error)
    }

    #[wasm_bindgen(js_name = instantiateUrlUnder)]
    pub async fn instantiate_url_under(
        &mut self,
        parent: u64,
        url: String,
    ) -> Result<u64, JsValue> {
        self.core
            .instantiate_url_under(parent, url)
            .await
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = instantiateGlb)]
    pub async fn instantiate_glb(&mut self, bytes: Box<[u8]>) -> Result<u64, JsValue> {
        self.core
            .instantiate_glb(bytes.as_ref())
            .await
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = instantiateGlbUnder)]
    pub async fn instantiate_glb_under(
        &mut self,
        parent: u64,
        bytes: Box<[u8]>,
    ) -> Result<u64, JsValue> {
        self.core
            .instantiate_glb_under(parent, bytes.as_ref())
            .await
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = importRoots)]
    pub fn import_roots(&mut self, import: u64) -> Result<Vec<u64>, JsValue> {
        self.core.import_roots(import).map_err(js_error)
    }

    #[wasm_bindgen(js_name = nodeHandle)]
    pub fn node_handle(&mut self, import: u64, path: String) -> Result<u64, JsValue> {
        self.core.node_handle(import, &path).map_err(js_error)
    }

    #[wasm_bindgen(js_name = nodeHandleByName)]
    pub fn node_handle_by_name(&mut self, import: u64, name: String) -> Result<u64, JsValue> {
        self.core
            .node_handle_by_name(import, &name)
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = nodeHandleFromInspection)]
    pub fn node_handle_from_inspection(&self, handle: u64) -> Result<u64, JsValue> {
        self.core
            .node_handle_from_inspection(handle)
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = setNodeAnnotation)]
    pub fn set_node_annotation(
        &mut self,
        id: String,
        node: u64,
        local_offset: Box<[f32]>,
    ) -> Result<(), JsValue> {
        self.core
            .set_node_annotation(
                &id,
                node,
                vec3_array_from_slice("localOffset", &local_offset).map_err(js_error)?,
            )
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = setWorldAnnotation)]
    pub fn set_world_annotation(
        &mut self,
        id: String,
        position: Box<[f32]>,
    ) -> Result<(), JsValue> {
        self.core
            .set_world_annotation(
                &id,
                vec3_array_from_slice("position", &position).map_err(js_error)?,
            )
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = clearAnnotation)]
    pub fn clear_annotation(&mut self, id: String) -> bool {
        self.core.clear_annotation(&id)
    }

    #[wasm_bindgen(js_name = removeNode)]
    pub fn remove_node(&mut self, node: u64) -> Result<(), JsValue> {
        self.core.remove_node(node).map_err(js_error)
    }

    #[wasm_bindgen(js_name = removeImport)]
    pub fn remove_import(&mut self, import: u64) -> Result<(), JsValue> {
        self.core.remove_import(import).map_err(js_error)
    }

    pub fn prepare(&mut self) -> Result<(), JsValue> {
        self.core.prepare().map_err(js_error)
    }

    pub fn render(&mut self) -> Result<String, JsValue> {
        self.core
            .render()
            .map(render_outcome_json)
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = readPixels)]
    pub fn read_pixels(&self) -> Vec<u8> {
        self.browser_canvas
            .as_ref()
            .and_then(|canvas| browser_canvas_rgba8(canvas).ok().flatten())
            .map(|(_width, _height, rgba8)| rgba8)
            .unwrap_or_else(|| self.core.read_pixels())
    }

    #[wasm_bindgen(js_name = capture)]
    pub fn capture(&self) -> Result<JsValue, JsValue> {
        let capture = self.capture_rgba8_for_wasm().map_err(js_error)?;
        let descriptor_json = capture_descriptor_json(&capture).map_err(js_error)?;
        let object = js_sys::Object::new();
        let rgba8 = js_sys::Uint8Array::from(capture.rgba8.as_slice());
        let _ = js_sys::Reflect::set(
            &object,
            &JsValue::from_str("descriptorJson"),
            &JsValue::from_str(&descriptor_json),
        );
        let _ = js_sys::Reflect::set(&object, &JsValue::from_str("rgba8"), &rgba8);
        Ok(object.into())
    }

    #[wasm_bindgen(js_name = capturePng)]
    pub fn capture_png(&self) -> Result<JsValue, JsValue> {
        let capture = self.capture_rgba8_for_wasm().map_err(js_error)?;
        let descriptor_json = capture_descriptor_json(&capture).map_err(js_error)?;
        let png_bytes = capture.to_png_bytes().map_err(js_error)?;
        let object = js_sys::Object::new();
        let png = js_sys::Uint8Array::from(png_bytes.as_slice());
        let _ = js_sys::Reflect::set(
            &object,
            &JsValue::from_str("descriptorJson"),
            &JsValue::from_str(&descriptor_json),
        );
        let _ = js_sys::Reflect::set(&object, &JsValue::from_str("png"), &png);
        Ok(object.into())
    }

    #[wasm_bindgen(js_name = captureJson)]
    pub fn capture_json(&self) -> Result<String, JsValue> {
        self.core.capture_json().map_err(js_error)
    }

    pub fn pick(&mut self, x: f32, y: f32) -> Result<Option<u64>, JsValue> {
        self.core.pick(x, y).map_err(js_error)
    }

    pub fn hover(&mut self, x: f32, y: f32) -> Result<Option<u64>, JsValue> {
        self.core.hover(x, y).map_err(js_error)
    }

    pub fn select(&mut self, x: f32, y: f32) -> Result<Option<u64>, JsValue> {
        self.core.select(x, y).map_err(js_error)
    }

    #[wasm_bindgen(js_name = drainEventsJson)]
    pub fn drain_events_json(&self) -> Result<String, JsValue> {
        self.core.drain_events_json().map_err(js_error)
    }

    #[wasm_bindgen(js_name = frameNode)]
    pub fn frame_node(&mut self, node: u64) -> Result<(), JsValue> {
        self.core.frame_node(node).map_err(js_error)
    }

    #[wasm_bindgen(js_name = frameNodeProductView)]
    pub fn frame_node_product_view(&mut self, node: u64) -> Result<(), JsValue> {
        self.core.frame_node_product_view(node).map_err(js_error)
    }

    #[wasm_bindgen(js_name = frameNodeWithPreset)]
    pub fn frame_node_with_preset(&mut self, node: u64, preset: String) -> Result<(), JsValue> {
        self.core
            .frame_node_with_preset(node, &preset)
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = frameAll)]
    pub fn frame_all(&mut self) -> Result<(), JsValue> {
        self.core.frame_all().map_err(js_error)
    }

    #[wasm_bindgen(js_name = worldDistance)]
    pub fn world_distance(&self, a: u64, b: u64) -> Result<f32, JsValue> {
        self.core.world_distance(a, b).map_err(js_error)
    }

    #[wasm_bindgen(js_name = nodeWorldBoundsJson)]
    pub fn node_world_bounds_json(&self, node: u64) -> Result<String, JsValue> {
        self.core.node_world_bounds_json(node).map_err(js_error)
    }

    #[wasm_bindgen(js_name = inspectJson)]
    pub fn inspect_json(&self) -> Result<String, JsValue> {
        self.core.inspect_json().map_err(js_error)
    }

    #[wasm_bindgen(js_name = annotationProjectionsJson)]
    pub fn annotation_projections_json(&self) -> Result<String, JsValue> {
        self.core.annotation_projections_json().map_err(js_error)
    }

    #[wasm_bindgen(js_name = capabilitiesJson)]
    pub fn capabilities_json(&self) -> Result<String, JsValue> {
        self.core.capabilities_json().map_err(js_error)
    }

    #[wasm_bindgen(js_name = diagnosticsJson)]
    pub fn diagnostics_json(&self) -> String {
        self.core.diagnostics_json()
    }

    #[wasm_bindgen(js_name = statsJson)]
    pub fn stats_json(&self) -> String {
        self.core.stats_json()
    }
}

impl SceneHost {
    fn capture_rgba8_for_wasm(&self) -> Result<CaptureRgba8, SceneHostError> {
        match self
            .browser_canvas
            .as_ref()
            .map(browser_canvas_rgba8)
            .transpose()?
            .flatten()
        {
            Some((width, height, rgba8)) => self.core.capture_from_rgba8(width, height, rgba8),
            None => self.core.capture(),
        }
    }
}

impl SceneHost {
    async fn attach_canvas(
        &mut self,
        backend: BrowserBackend,
        canvas: web_sys::HtmlCanvasElement,
        logical_width: f32,
        logical_height: f32,
        device_pixel_ratio: f32,
    ) -> Result<(), JsValue> {
        let browser_canvas = canvas.clone();
        let (surface, viewport) = surface_from_canvas(
            backend,
            canvas,
            logical_width,
            logical_height,
            device_pixel_ratio,
        )?;
        self.core
            .resize(
                viewport.logical_width(),
                viewport.logical_height(),
                viewport.device_pixel_ratio(),
            )
            .map_err(js_error)?;
        self.core.attach_surface(surface).await.map_err(js_error)?;
        self.browser_canvas = Some(browser_canvas);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
enum BrowserBackend {
    WebGpu,
    WebGl2,
}

async fn build_from_canvas(
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

fn surface_from_canvas(
    backend: BrowserBackend,
    canvas: web_sys::HtmlCanvasElement,
    logical_width: f32,
    logical_height: f32,
    device_pixel_ratio: f32,
) -> Result<(PlatformSurface, SurfaceViewport), JsValue> {
    let viewport = SurfaceViewport::new(logical_width, logical_height, device_pixel_ratio)
        .ok_or_else(|| {
            js_error(SceneHostError::new(
                super::SceneHostErrorCode::InvalidViewport,
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

fn render_outcome_json(outcome: RenderOutcome) -> String {
    serde_json::json!({
        "width": outcome.width,
        "height": outcome.height,
        "draw_calls": outcome.draw_calls,
        "primitives": outcome.primitives,
        "skipped": outcome.skipped,
    })
    .to_string()
}

pub(super) fn js_error(error: impl Into<SceneHostError>) -> JsValue {
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

fn capture_descriptor_json(capture: &CaptureRgba8) -> Result<String, SceneHostError> {
    serde_json::to_string(&capture.descriptor).map_err(|error| {
        SceneHostError::new(
            super::SceneHostErrorCode::Capture,
            format!("capture descriptor serialization failed: {error}"),
        )
    })
}
