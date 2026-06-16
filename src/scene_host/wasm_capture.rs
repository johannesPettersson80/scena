use super::wasm::SceneHost;
use super::wasm_readback::browser_canvas_rgba8;
use super::{SceneHostError, SceneHostErrorCode};
use crate::CaptureRgba8;

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
}

pub(super) fn capture_descriptor_json(capture: &CaptureRgba8) -> Result<String, SceneHostError> {
    serde_json::to_string(&capture.descriptor).map_err(|error| {
        SceneHostError::new(
            SceneHostErrorCode::Capture,
            format!("capture descriptor serialization failed: {error}"),
        )
    })
}
