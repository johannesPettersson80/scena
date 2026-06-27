use wasm_bindgen::prelude::*;

use super::inputs::vec3_array_from_slice;
use super::wasm::{SceneHost, js_error};
use crate::Vec3;

#[wasm_bindgen]
impl SceneHost {
    #[wasm_bindgen(js_name = addDistanceMeasurement)]
    pub fn add_distance_measurement(
        &mut self,
        id: String,
        start: Box<[f32]>,
        end: Box<[f32]>,
        label: Option<String>,
        unit: String,
        precision: u8,
    ) -> Result<String, JsValue> {
        let start = vec3_array_from_slice("start", &start).map_err(js_error)?;
        let end = vec3_array_from_slice("end", &end).map_err(js_error)?;
        self.core
            .add_distance_measurement_json(
                &id,
                Vec3::new(start[0], start[1], start[2]),
                Vec3::new(end[0], end[1], end[2]),
                label.as_deref(),
                &unit,
                precision,
            )
            .map_err(js_error)
    }
}
