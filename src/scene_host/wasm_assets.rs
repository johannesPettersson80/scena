use wasm_bindgen::prelude::*;

use super::wasm::{SceneHost, js_error};

#[wasm_bindgen]
impl SceneHost {
    #[wasm_bindgen(js_name = instantiateUrlWithReportJson)]
    pub async fn instantiate_url_with_report_json(
        &mut self,
        url: String,
    ) -> Result<String, JsValue> {
        self.core
            .instantiate_url_with_report_json(url)
            .await
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = instantiateUrlUnderWithReportJson)]
    pub async fn instantiate_url_under_with_report_json(
        &mut self,
        parent: u64,
        url: String,
    ) -> Result<String, JsValue> {
        self.core
            .instantiate_url_under_with_report_json(parent, url)
            .await
            .map_err(js_error)
    }
}
