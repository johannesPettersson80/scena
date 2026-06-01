use serde::Deserialize;
use wasm_bindgen::prelude::*;

use super::{SceneHostCore, SceneHostError};
use crate::{
    Assets, Color, PlatformSurface, RenderOutcome, Renderer, SurfaceViewport, Transform, Vec3,
};

#[wasm_bindgen]
pub struct SceneHost {
    core: SceneHostCore,
}

#[derive(Debug, Deserialize)]
struct WasmTransformUpdate {
    node: u64,
    translation: [f32; 3],
    rotation: [f32; 4],
    scale: [f32; 3],
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

    #[wasm_bindgen(js_name = addEmpty)]
    pub fn add_empty(
        &mut self,
        parent: Option<u64>,
        translation: Box<[f32]>,
        rotation: Box<[f32]>,
        scale: Box<[f32]>,
        tag: Option<String>,
    ) -> Result<u64, JsValue> {
        self.core
            .add_empty(
                parent,
                transform_from_slices(&translation, &rotation, &scale)?,
                tag.as_deref(),
            )
            .map_err(js_error)
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

    #[wasm_bindgen(js_name = setTransform)]
    pub fn set_transform(
        &mut self,
        node: u64,
        translation: Box<[f32]>,
        rotation: Box<[f32]>,
        scale: Box<[f32]>,
    ) -> Result<(), JsValue> {
        self.core
            .set_transform(
                node,
                transform_from_slices(&translation, &rotation, &scale)?,
            )
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = setTransforms)]
    pub fn set_transforms(&mut self, batch_json: String) -> Result<(), JsValue> {
        let updates: Vec<WasmTransformUpdate> =
            serde_json::from_str(&batch_json).map_err(|error| {
                js_error(SceneHostError::new(
                    super::SceneHostErrorCode::InvalidInput,
                    format!("invalid setTransforms JSON: {error}"),
                ))
            })?;
        let updates = updates
            .into_iter()
            .map(|update| {
                (
                    update.node,
                    transform_from_components(update.translation, update.rotation, update.scale),
                )
            })
            .collect::<Vec<_>>();
        self.core.set_transforms(&updates).map_err(js_error)
    }

    #[wasm_bindgen(js_name = setNodeTint)]
    pub fn set_node_tint(
        &mut self,
        node: u64,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    ) -> Result<(), JsValue> {
        self.core
            .set_node_tint(node, Some(Color::from_linear_rgba(r, g, b, a)))
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = clearNodeTint)]
    pub fn clear_node_tint(&mut self, node: u64) -> Result<(), JsValue> {
        self.core.set_node_tint(node, None).map_err(js_error)
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
                vec3_array_from_slice("localOffset", &local_offset)?,
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
            .set_world_annotation(&id, vec3_array_from_slice("position", &position)?)
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
        self.core.read_pixels()
    }

    #[wasm_bindgen(js_name = capture)]
    pub fn capture(&self) -> Result<JsValue, JsValue> {
        let capture = self.core.capture().map_err(js_error)?;
        let descriptor_json = serde_json::to_string(&capture.descriptor).map_err(|error| {
            js_error(SceneHostError::new(
                super::SceneHostErrorCode::Capture,
                format!("capture descriptor serialization failed: {error}"),
            ))
        })?;
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

    #[wasm_bindgen(js_name = captureJson)]
    pub fn capture_json(&self) -> Result<String, JsValue> {
        self.core.capture_json().map_err(js_error)
    }

    pub fn pick(&mut self, x: f32, y: f32) -> Result<Option<u64>, JsValue> {
        self.core.pick(x, y).map_err(js_error)
    }

    #[wasm_bindgen(js_name = frameNode)]
    pub fn frame_node(&mut self, node: u64) -> Result<(), JsValue> {
        self.core.frame_node(node).map_err(js_error)
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
    async fn attach_canvas(
        &mut self,
        backend: BrowserBackend,
        canvas: web_sys::HtmlCanvasElement,
        logical_width: f32,
        logical_height: f32,
        device_pixel_ratio: f32,
    ) -> Result<(), JsValue> {
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
        self.core.attach_surface(surface).await.map_err(js_error)
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
    Ok(SceneHost { core })
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

fn transform_from_slices(
    translation: &[f32],
    rotation: &[f32],
    scale: &[f32],
) -> Result<Transform, JsValue> {
    if translation.len() != 3 {
        return Err(invalid_len("translation", 3, translation.len()));
    }
    if rotation.len() != 4 {
        return Err(invalid_len("rotation", 4, rotation.len()));
    }
    if scale.len() != 3 {
        return Err(invalid_len("scale", 3, scale.len()));
    }
    Ok(transform_from_components(
        [translation[0], translation[1], translation[2]],
        [rotation[0], rotation[1], rotation[2], rotation[3]],
        [scale[0], scale[1], scale[2]],
    ))
}

fn transform_from_components(
    translation: [f32; 3],
    rotation: [f32; 4],
    scale: [f32; 3],
) -> Transform {
    Transform {
        translation: Vec3::new(translation[0], translation[1], translation[2]),
        rotation: crate::Quat::from_xyzw(rotation[0], rotation[1], rotation[2], rotation[3]),
        scale: Vec3::new(scale[0], scale[1], scale[2]),
    }
}

fn invalid_len(field: &str, expected: usize, actual: usize) -> JsValue {
    js_error(SceneHostError::new(
        super::SceneHostErrorCode::InvalidInput,
        format!("{field} must contain {expected} values, got {actual}"),
    ))
}

fn vec3_array_from_slice(field: &str, values: &[f32]) -> Result<[f32; 3], JsValue> {
    if values.len() != 3 {
        return Err(invalid_len(field, 3, values.len()));
    }
    Ok([values[0], values[1], values[2]])
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

fn js_error(error: impl Into<SceneHostError>) -> JsValue {
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
