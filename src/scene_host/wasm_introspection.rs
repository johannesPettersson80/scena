use wasm_bindgen::prelude::*;

use super::wasm::{SceneHost, js_error};

#[wasm_bindgen]
impl SceneHost {
    #[wasm_bindgen(js_name = renderIntrospectionJson)]
    pub fn render_introspection_json(&self, detail: bool) -> Result<String, JsValue> {
        let capture = self.capture_rgba8_for_wasm().map_err(js_error)?;
        self.core
            .render_introspection_json_from_capture(&capture, detail)
            .map_err(js_error)
    }
}
