use super::wasm::SceneHost;
use super::wasm_readback::browser_canvas_rgba8;
use super::{SceneHostError, SceneHostErrorCode};
use crate::CaptureRgba8;
use wasm_bindgen::prelude::*;

impl SceneHost {
    pub(super) fn capture_rgba8_for_wasm(&self) -> Result<CaptureRgba8, SceneHostError> {
        match self
            .browser_canvas
            .as_ref()
            .map(browser_canvas_rgba8)
            .transpose()?
            .flatten()
        {
            Some((width, height, rgba8)) => self.core.capture_from_rgba8(width, height, rgba8),
            None => self.core.capture(),
        }
    }

    pub(super) async fn capture_rgba8_for_wasm_async(
        &mut self,
    ) -> Result<CaptureRgba8, SceneHostError> {
        let readback = self
            .core
            .renderer
            .browser_readback_rgba8()
            .await
            .map_err(|error| {
                let detail = error.as_string().unwrap_or_else(|| format!("{error:?}"));
                SceneHostError::new(
                    SceneHostErrorCode::Capture,
                    format!("browser GPU capture readback failed: {detail}"),
                )
            })?;
        match readback {
            Some(pixels) => {
                self.core
                    .capture_from_rgba8(pixels.width(), pixels.height(), pixels.into_rgba8())
            }
            None => self.capture_rgba8_for_wasm(),
        }
    }
}

pub(super) fn capture_descriptor_json(capture: &CaptureRgba8) -> Result<String, SceneHostError> {
    serde_json::to_string(&capture.descriptor).map_err(|error| {
        SceneHostError::new(
            SceneHostErrorCode::Capture,
            format!("capture descriptor serialization failed: {error}"),
        )
    })
}

pub(super) fn capture_rgba8_js(capture: &CaptureRgba8) -> Result<JsValue, SceneHostError> {
    let descriptor_json = capture_descriptor_json(capture)?;
    let object = js_sys::Object::new();
    let rgba8 = js_sys::Uint8Array::from(capture.rgba8.as_slice());
    let _ = js_sys::Reflect::set(
        &object,
        &JsValue::from_str("descriptorJson"),
        &JsValue::from_str(&descriptor_json),
    );
    let _ = js_sys::Reflect::set(&object, &JsValue::from_str("rgba8"), &rgba8);
    Ok(object.into())
}

pub(super) fn capture_png_js(capture: &CaptureRgba8) -> Result<JsValue, SceneHostError> {
    let descriptor_json = capture_descriptor_json(capture)?;
    let png_bytes = capture.to_png_bytes().map_err(SceneHostError::from)?;
    let object = js_sys::Object::new();
    let png = js_sys::Uint8Array::from(png_bytes.as_slice());
    let _ = js_sys::Reflect::set(
        &object,
        &JsValue::from_str("descriptorJson"),
        &JsValue::from_str(&descriptor_json),
    );
    let _ = js_sys::Reflect::set(&object, &JsValue::from_str("png"), &png);
    Ok(object.into())
}
