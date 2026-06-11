use wasm_bindgen::prelude::*;

use super::wasm::{SceneHost, js_error};

#[wasm_bindgen]
impl SceneHost {
    #[wasm_bindgen(js_name = setAntiAliasing)]
    pub fn set_anti_aliasing(&mut self, mode: String) -> Result<(), JsValue> {
        self.core.set_anti_aliasing(&mode).map_err(js_error)
    }

    #[wasm_bindgen(js_name = setBloom)]
    pub fn set_bloom(&mut self, config_json: Option<String>) -> Result<(), JsValue> {
        self.core
            .set_bloom_json(config_json.as_deref())
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = setAmbientOcclusion)]
    pub fn set_ambient_occlusion(&mut self, config_json: Option<String>) -> Result<(), JsValue> {
        self.core
            .set_ambient_occlusion_json(config_json.as_deref())
            .map_err(js_error)
    }
}
