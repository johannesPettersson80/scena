use super::{SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::{AssetFetcher, CaptureOptions, CaptureRgba8, capture_rgba8, capture_rgba8_from_pixels};

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn read_pixels(&self) -> Vec<u8> {
        self.renderer.read_pixels().into_rgba8()
    }

    pub fn capture(&self) -> Result<CaptureRgba8, SceneHostError> {
        Ok(capture_rgba8(
            &self.scene,
            &self.renderer,
            CaptureOptions::default().with_surface_viewport(self.viewport),
        )?)
    }

    pub fn capture_from_rgba8(
        &self,
        width: u32,
        height: u32,
        rgba8: Vec<u8>,
    ) -> Result<CaptureRgba8, SceneHostError> {
        Ok(capture_rgba8_from_pixels(
            &self.scene,
            &self.renderer,
            CaptureOptions::default().with_surface_viewport(self.viewport),
            width,
            height,
            rgba8,
        )?)
    }

    pub fn capture_json(&self) -> Result<String, SceneHostError> {
        let capture = self.capture()?;
        serde_json::to_string(&capture.descriptor).map_err(|error| {
            SceneHostError::new(
                SceneHostErrorCode::Capture,
                format!("capture descriptor serialization failed: {error}"),
            )
        })
    }
}
