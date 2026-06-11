use wasm_bindgen::prelude::*;

use super::wasm::{SceneHost, js_error};

#[wasm_bindgen]
impl SceneHost {
    #[wasm_bindgen(js_name = applyProductStudioVisuals)]
    pub fn apply_product_studio_visuals(&mut self, background: String) -> Result<(), JsValue> {
        self.core
            .apply_product_studio_visuals(&background)
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = addProductGridFloorUnderNode)]
    pub fn add_product_grid_floor_under_node(&mut self, node: u64) -> Result<Vec<u64>, JsValue> {
        self.core
            .add_product_grid_floor_under_node(node)
            .map_err(js_error)
    }
}
