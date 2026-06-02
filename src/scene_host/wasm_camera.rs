use wasm_bindgen::prelude::*;

use super::camera::orbit_action_name;
use super::wasm::{SceneHost, js_error};
use super::wasm_inputs::vec3_array_from_slice;
use super::{SceneHostError, SceneHostErrorCode};
use crate::{PointerButton, SceneHostCameraState, Vec3};

#[wasm_bindgen]
impl SceneHost {
    #[wasm_bindgen(js_name = getCameraJson)]
    pub fn get_camera_json(&self) -> Result<String, JsValue> {
        self.core.camera_json().map_err(js_error)
    }

    #[wasm_bindgen(js_name = setCamera)]
    pub fn set_camera(
        &mut self,
        target: Box<[f32]>,
        yaw_radians: f32,
        pitch_radians: f32,
        distance: f32,
    ) -> Result<(), JsValue> {
        let target = vec3_array_from_slice("target", &target).map_err(js_error)?;
        self.core
            .set_camera(SceneHostCameraState {
                target: Vec3::new(target[0], target[1], target[2]),
                distance,
                yaw_radians,
                pitch_radians,
            })
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = setCameraJson)]
    pub fn set_camera_json(&mut self, camera_json: String) -> Result<(), JsValue> {
        self.core.set_camera_json(&camera_json).map_err(js_error)
    }

    #[wasm_bindgen(js_name = cameraPointerDown)]
    pub fn camera_pointer_down(
        &mut self,
        x: f32,
        y: f32,
        button: String,
    ) -> Result<String, JsValue> {
        let button = pointer_button_from_name(&button)?;
        self.core
            .camera_pointer_down(x, y, button)
            .map(|action| orbit_action_name(action).to_owned())
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = cameraPointerMove)]
    pub fn camera_pointer_move(
        &mut self,
        x: f32,
        y: f32,
        delta_x: f32,
        delta_y: f32,
    ) -> Result<String, JsValue> {
        self.core
            .camera_pointer_move(x, y, delta_x, delta_y)
            .map(|action| orbit_action_name(action).to_owned())
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = cameraPointerUp)]
    pub fn camera_pointer_up(&mut self, x: f32, y: f32) -> Result<String, JsValue> {
        self.core
            .camera_pointer_up(x, y)
            .map(|action| orbit_action_name(action).to_owned())
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = cameraWheel)]
    pub fn camera_wheel(&mut self, x: f32, y: f32, delta_y: f32) -> Result<String, JsValue> {
        self.core
            .camera_wheel(x, y, delta_y)
            .map(|action| orbit_action_name(action).to_owned())
            .map_err(js_error)
    }
}

fn pointer_button_from_name(value: &str) -> Result<PointerButton, JsValue> {
    match value {
        "primary" | "left" | "0" => Ok(PointerButton::Primary),
        "secondary" | "right" | "2" => Ok(PointerButton::Secondary),
        "auxiliary" | "middle" | "1" => Ok(PointerButton::Auxiliary),
        _ => Err(js_error(SceneHostError::new(
            SceneHostErrorCode::InvalidInput,
            format!("unknown pointer button '{value}'"),
        ))),
    }
}
