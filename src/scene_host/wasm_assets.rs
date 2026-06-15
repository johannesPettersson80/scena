use wasm_bindgen::prelude::*;

use super::wasm::{SceneHost, js_error};

#[wasm_bindgen]
impl SceneHost {
    #[wasm_bindgen(js_name = assetDoctorJson)]
    pub async fn asset_doctor_json(&mut self, url: String) -> Result<String, JsValue> {
        self.core.asset_doctor_json(url).await.map_err(js_error)
    }

    #[wasm_bindgen(js_name = instantiateUrlInstanced)]
    pub async fn instantiate_url_instanced(
        &mut self,
        url: String,
        count: usize,
    ) -> Result<js_sys::BigUint64Array, JsValue> {
        let handles = self
            .core
            .instantiate_url_instanced(url, count)
            .await
            .map_err(js_error)?;
        Ok(js_sys::BigUint64Array::from(handles.as_slice()))
    }

    #[wasm_bindgen(js_name = instantiateUrlInstancedUnder)]
    pub async fn instantiate_url_instanced_under(
        &mut self,
        parent: u64,
        url: String,
        count: usize,
    ) -> Result<js_sys::BigUint64Array, JsValue> {
        let handles = self
            .core
            .instantiate_url_instanced_under(parent, url, count)
            .await
            .map_err(js_error)?;
        Ok(js_sys::BigUint64Array::from(handles.as_slice()))
    }

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
