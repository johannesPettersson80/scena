use wasm_bindgen::prelude::*;

use super::wasm::{SceneHost, js_error};

#[wasm_bindgen]
impl SceneHost {
    #[wasm_bindgen(js_name = timelinePatchJson)]
    pub fn timeline_patch_json(
        &self,
        timeline_json: String,
        seconds: f64,
    ) -> Result<String, JsValue> {
        self.core
            .timeline_patch_json(&timeline_json, seconds)
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = seekTimelineJson)]
    pub fn seek_timeline_json(
        &mut self,
        timeline_json: String,
        seconds: f64,
    ) -> Result<String, JsValue> {
        self.core
            .seek_timeline_json(&timeline_json, seconds)
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = advanceTimelineJson)]
    pub fn advance_timeline_json(
        &mut self,
        timeline_json: String,
        current_seconds: f64,
        delta_seconds: f64,
    ) -> Result<String, JsValue> {
        self.core
            .advance_timeline_json(&timeline_json, current_seconds, delta_seconds)
            .map_err(js_error)
    }
}
