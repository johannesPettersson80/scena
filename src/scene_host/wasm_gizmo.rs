use wasm_bindgen::prelude::*;

use super::wasm::{SceneHost, js_error};

#[wasm_bindgen]
impl SceneHost {
    #[wasm_bindgen(js_name = applyGizmoDragJson)]
    pub fn apply_gizmo_drag_json(
        &mut self,
        target: u64,
        request_json: String,
    ) -> Result<String, JsValue> {
        self.core
            .apply_gizmo_drag_json(target, &request_json)
            .map_err(js_error)
    }
}
