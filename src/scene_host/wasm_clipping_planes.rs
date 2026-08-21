use wasm_bindgen::prelude::*;

use super::wasm::{SceneHost, js_error};

#[wasm_bindgen]
impl SceneHost {
    #[wasm_bindgen(js_name = setClippingPlanesJson)]
    pub fn set_clipping_planes_json(&mut self, json: &str) -> Result<String, JsValue> {
        self.core.set_clipping_planes_json(json).map_err(js_error)
    }
}
