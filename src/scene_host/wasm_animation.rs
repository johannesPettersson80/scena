use wasm_bindgen::prelude::*;

use super::animation::{SceneHostAnimationLoopMode, SceneHostAnimationPlayOptions};
use super::wasm::{SceneHost, js_error};
use super::{SceneHostError, SceneHostErrorCode};

#[wasm_bindgen]
impl SceneHost {
    #[wasm_bindgen(js_name = animationInventoryJson)]
    pub fn animation_inventory_json(&self, import: u64) -> Result<String, JsValue> {
        self.core.animation_inventory_json(import).map_err(js_error)
    }

    #[wasm_bindgen(js_name = playAnimation)]
    pub fn play_animation(
        &mut self,
        import: u64,
        clip_name: String,
        options: JsValue,
    ) -> Result<u64, JsValue> {
        self.core
            .play_animation(
                import,
                &clip_name,
                play_options_from_js(options).map_err(js_error)?,
            )
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = pauseAnimation)]
    pub fn pause_animation(&mut self, handle: u64) -> Result<(), JsValue> {
        self.core.pause_animation(handle).map_err(js_error)
    }

    #[wasm_bindgen(js_name = stopAnimation)]
    pub fn stop_animation(&mut self, handle: u64) -> Result<(), JsValue> {
        self.core.stop_animation(handle).map_err(js_error)
    }

    #[wasm_bindgen(js_name = seekAnimation)]
    pub fn seek_animation(&mut self, handle: u64, seconds: f64) -> Result<(), JsValue> {
        self.core.seek_animation(handle, seconds).map_err(js_error)
    }

    #[wasm_bindgen(js_name = setAnimationSpeed)]
    pub fn set_animation_speed(&mut self, handle: u64, speed: f64) -> Result<(), JsValue> {
        self.core
            .set_animation_speed(handle, speed)
            .map_err(js_error)
    }

    pub fn advance(&mut self, delta_seconds: f64) -> Result<(), JsValue> {
        self.core.advance(delta_seconds).map_err(js_error)
    }
}

fn play_options_from_js(options: JsValue) -> Result<SceneHostAnimationPlayOptions, SceneHostError> {
    if options.is_null() || options.is_undefined() {
        return Ok(SceneHostAnimationPlayOptions::default());
    }
    if !options.is_object() {
        return Err(invalid_options("playAnimation options must be an object"));
    }

    let loop_mode = js_property_string(&options, "loop_mode")?
        .or(js_property_string(&options, "loopMode")?)
        .map(|value| match value.as_str() {
            "once" => Ok(SceneHostAnimationLoopMode::Once),
            "repeat" => Ok(SceneHostAnimationLoopMode::Repeat),
            other => Err(invalid_options(format!(
                "unsupported playAnimation loop_mode {other}"
            ))),
        })
        .transpose()?
        .unwrap_or_default();
    let speed = js_property_number(&options, "speed")?.unwrap_or(1.0) as f32;

    Ok(SceneHostAnimationPlayOptions { loop_mode, speed })
}

fn js_property_string(object: &JsValue, name: &str) -> Result<Option<String>, SceneHostError> {
    let value = js_sys::Reflect::get(object, &JsValue::from_str(name))
        .map_err(|_| invalid_options(format!("could not read option {name}")))?;
    if value.is_null() || value.is_undefined() {
        return Ok(None);
    }
    value
        .as_string()
        .map(Some)
        .ok_or_else(|| invalid_options(format!("option {name} must be a string")))
}

fn js_property_number(object: &JsValue, name: &str) -> Result<Option<f64>, SceneHostError> {
    let value = js_sys::Reflect::get(object, &JsValue::from_str(name))
        .map_err(|_| invalid_options(format!("could not read option {name}")))?;
    if value.is_null() || value.is_undefined() {
        return Ok(None);
    }
    value
        .as_f64()
        .map(Some)
        .ok_or_else(|| invalid_options(format!("option {name} must be a number")))
}

fn invalid_options(message: impl Into<String>) -> SceneHostError {
    SceneHostError::new(SceneHostErrorCode::InvalidInput, message.into())
}
