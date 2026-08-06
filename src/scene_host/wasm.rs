use wasm_bindgen::prelude::*;

use super::inputs::vec3_array_from_slice;
use super::wasm_capture::{capture_descriptor_json, capture_png_js, capture_rgba8_js};
use super::wasm_readback::browser_canvas_rgba8;
use super::{SceneHostCore, SceneHostError, SceneHostErrorCode};

mod support;
pub(super) use support::js_error;
use support::{
    BrowserBackend, build_from_canvas, render_outcome_js, render_outcome_json, surface_from_canvas,
};

#[wasm_bindgen]
pub struct SceneHost {
    pub(super) core: SceneHostCore,
    pub(super) browser_canvas: Option<web_sys::HtmlCanvasElement>,
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

    #[wasm_bindgen(js_name = addNodeCallout)]
    pub fn add_node_callout(
        &mut self,
        id: String,
        node: u64,
        local_offset: Box<[f32]>,
        label_offset: Box<[f32]>,
        text: String,
    ) -> Result<String, JsValue> {
        let report = self
            .core
            .add_node_callout(
                &id,
                node,
                vec3_array_from_slice("localOffset", &local_offset).map_err(js_error)?,
                vec3_array_from_slice("labelOffset", &label_offset).map_err(js_error)?,
                &text,
            )
            .map_err(js_error)?;
        serde_json::to_string(&report).map_err(|error| {
            js_error(SceneHostError::new(
                SceneHostErrorCode::InvalidInput,
                format!("callout report serialization failed: {error}"),
            ))
        })
    }

    #[wasm_bindgen(js_name = addWorldCallout)]
    pub fn add_world_callout(
        &mut self,
        id: String,
        position: Box<[f32]>,
        label_offset: Box<[f32]>,
        text: String,
    ) -> Result<String, JsValue> {
        let report = self
            .core
            .add_world_callout(
                &id,
                vec3_array_from_slice("position", &position).map_err(js_error)?,
                vec3_array_from_slice("labelOffset", &label_offset).map_err(js_error)?,
                &text,
            )
            .map_err(js_error)?;
        serde_json::to_string(&report).map_err(|error| {
            js_error(SceneHostError::new(
                SceneHostErrorCode::InvalidInput,
                format!("callout report serialization failed: {error}"),
            ))
        })
    }

    #[wasm_bindgen(js_name = clearCallout)]
    pub fn clear_callout(&mut self, id: String) -> bool {
        self.core.clear_callout(&id)
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

    #[wasm_bindgen(js_name = setSemanticAovCaptureEnabled)]
    pub fn set_semantic_aov_capture_enabled(&mut self, enabled: bool) {
        self.core.set_semantic_aov_capture_enabled(enabled);
    }

    #[wasm_bindgen(js_name = captureSemanticAovs)]
    pub async fn capture_semantic_aovs(&mut self) -> Result<JsValue, JsValue> {
        let capture = self
            .core
            .capture_semantic_aovs_gpu_async()
            .await
            .map_err(js_error)?;
        let metadata = serde_json::json!({
            "schema": capture.schema,
            "width": capture.width,
            "height": capture.height,
            "identity_scope": capture.identity_scope,
            "sample_pattern": capture.sample_pattern,
            "depth_convention": capture.depth_convention,
            "normal_space": capture.normal_space,
            "near": capture.near,
            "far": capture.far,
            "legend": capture.legend,
            "exclusions": capture.exclusions,
        });
        let metadata_json = serde_json::to_string(&metadata).map_err(|error| {
            js_error(SceneHostError::new(
                SceneHostErrorCode::Capture,
                error.to_string(),
            ))
        })?;
        let normals = capture
            .world_normals
            .iter()
            .flat_map(|normal| normal.iter().copied())
            .collect::<Vec<_>>();
        let object = js_sys::Object::new();
        let _ = js_sys::Reflect::set(
            &object,
            &JsValue::from_str("metadataJson"),
            &JsValue::from_str(&metadata_json),
        );
        let _ = js_sys::Reflect::set(
            &object,
            &JsValue::from_str("idIndices"),
            &js_sys::Uint32Array::from(capture.id_indices.as_slice()),
        );
        let beauty_id_indices = capture
            .beauty_id_indices
            .as_deref()
            .map(js_sys::Uint32Array::from)
            .map(JsValue::from)
            .unwrap_or(JsValue::NULL);
        let _ = js_sys::Reflect::set(
            &object,
            &JsValue::from_str("beautyIdIndices"),
            &beauty_id_indices,
        );
        let _ = js_sys::Reflect::set(
            &object,
            &JsValue::from_str("depthMeters"),
            &js_sys::Float32Array::from(capture.depth_meters.as_slice()),
        );
        let _ = js_sys::Reflect::set(
            &object,
            &JsValue::from_str("worldNormals"),
            &js_sys::Float32Array::from(normals.as_slice()),
        );
        let _ = js_sys::Reflect::set(
            &object,
            &JsValue::from_str("idRgba8"),
            &js_sys::Uint8Array::from(capture.id_rgba8().as_slice()),
        );
        let _ = js_sys::Reflect::set(
            &object,
            &JsValue::from_str("normalRgba8"),
            &js_sys::Uint8Array::from(capture.normal_rgba8().as_slice()),
        );
        Ok(object.into())
    }

    pub fn render(&mut self) -> Result<String, JsValue> {
        self.core
            .render()
            .map(render_outcome_json)
            .map_err(js_error)
    }

    /// Returns the render outcome as a native JavaScript object. The existing
    /// `render()` JSON-string contract remains available for compatibility.
    #[wasm_bindgen(js_name = renderTyped)]
    pub fn render_typed(&mut self) -> Result<JsValue, JsValue> {
        self.core.render().map(render_outcome_js).map_err(js_error)
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
        capture_rgba8_js(&capture).map_err(js_error)
    }

    #[wasm_bindgen(js_name = captureAsync)]
    pub async fn capture_async(&mut self) -> Result<JsValue, JsValue> {
        let capture = self
            .capture_rgba8_for_wasm_async()
            .await
            .map_err(js_error)?;
        capture_rgba8_js(&capture).map_err(js_error)
    }

    #[wasm_bindgen(js_name = capturePng)]
    pub fn capture_png(&self) -> Result<JsValue, JsValue> {
        let capture = self.capture_rgba8_for_wasm().map_err(js_error)?;
        capture_png_js(&capture).map_err(js_error)
    }

    #[wasm_bindgen(js_name = capturePngAsync)]
    pub async fn capture_png_async(&mut self) -> Result<JsValue, JsValue> {
        let capture = self
            .capture_rgba8_for_wasm_async()
            .await
            .map_err(js_error)?;
        capture_png_js(&capture).map_err(js_error)
    }

    #[wasm_bindgen(js_name = captureJson)]
    pub fn capture_json(&self) -> Result<String, JsValue> {
        self.core.capture_json().map_err(js_error)
    }

    #[wasm_bindgen(js_name = captureJsonAsync)]
    pub async fn capture_json_async(&mut self) -> Result<String, JsValue> {
        let capture = self
            .capture_rgba8_for_wasm_async()
            .await
            .map_err(js_error)?;
        capture_descriptor_json(&capture).map_err(js_error)
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

    #[wasm_bindgen(js_name = frameAllWithOverlays)]
    pub fn frame_all_with_overlays(&mut self) -> Result<(), JsValue> {
        self.core.frame_all_with_overlays().map_err(js_error)
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
