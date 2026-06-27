use wasm_bindgen::prelude::*;

use super::section_box::aabb_from_arrays;
use super::wasm::{SceneHost, js_error};

#[wasm_bindgen]
impl SceneHost {
    #[wasm_bindgen(js_name = setSectionBox)]
    pub fn set_section_box(
        &mut self,
        min: Box<[f32]>,
        max: Box<[f32]>,
        margin: f32,
        inverted: bool,
        helper_wireframe: bool,
    ) -> Result<String, JsValue> {
        let min = section_box_vec3_array("min", &min).map_err(js_error)?;
        let max = section_box_vec3_array("max", &max).map_err(js_error)?;
        self.core
            .set_section_box_json(
                aabb_from_arrays(min, max),
                margin,
                inverted,
                helper_wireframe,
            )
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = clearSectionBox)]
    pub fn clear_section_box(&mut self) -> Result<String, JsValue> {
        self.core.clear_section_box_json().map_err(js_error)
    }

    #[wasm_bindgen(js_name = invertSectionBox)]
    pub fn invert_section_box(&mut self, inverted: bool) -> Result<String, JsValue> {
        self.core
            .invert_section_box_json(inverted)
            .map_err(js_error)
    }
}

fn section_box_vec3_array(
    field: &'static str,
    values: &[f32],
) -> Result<[f32; 3], super::SceneHostError> {
    super::inputs::vec3_array_from_slice(field, values)
}
