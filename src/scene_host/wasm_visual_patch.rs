use wasm_bindgen::prelude::*;

use super::wasm::{SceneHost, js_error};

#[wasm_bindgen]
impl SceneHost {
    #[wasm_bindgen(js_name = applyPatch)]
    pub fn apply_patch(&mut self, patch_json: String) -> Result<String, JsValue> {
        self.core.apply_patch_json(&patch_json).map_err(js_error)
    }
}
