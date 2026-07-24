use super::events::HostEventV1;
use super::{SceneHostCore, SceneHostError, SceneHostErrorCode};
#[cfg(target_arch = "wasm32")]
use crate::capture_rgba8_from_pixels;
use crate::{
    AssetFetcher, CaptureOptions, CapturePngError, CaptureRgba8, capture_rgba8,
    capture_unverified_rgba8_from_pixels,
};

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn read_pixels(&self) -> Vec<u8> {
        self.renderer.read_pixels().into_rgba8()
    }

    pub fn capture(&self) -> Result<CaptureRgba8, SceneHostError> {
        let capture = capture_rgba8(
            &self.scene,
            &self.renderer,
            CaptureOptions::default().with_surface_viewport(self.viewport),
        )?;
        self.emit_event(HostEventV1::capture_ready(&capture));
        Ok(capture)
    }

    pub fn capture_from_rgba8(
        &self,
        width: u32,
        height: u32,
        rgba8: Vec<u8>,
    ) -> Result<CaptureRgba8, SceneHostError> {
        let capture = capture_unverified_rgba8_from_pixels(
            &self.scene,
            &self.renderer,
            CaptureOptions::default().with_surface_viewport(self.viewport),
            width,
            height,
            rgba8,
        )?;
        self.emit_event(HostEventV1::capture_ready(&capture));
        Ok(capture)
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn capture_from_renderer_rgba8(
        &self,
        width: u32,
        height: u32,
        rgba8: Vec<u8>,
    ) -> Result<CaptureRgba8, SceneHostError> {
        let capture = capture_rgba8_from_pixels(
            &self.scene,
            &self.renderer,
            CaptureOptions::default().with_surface_viewport(self.viewport),
            width,
            height,
            rgba8,
        )?;
        self.emit_event(HostEventV1::capture_ready(&capture));
        Ok(capture)
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

    pub fn capture_png_bytes(&self) -> Result<Vec<u8>, SceneHostError> {
        self.capture()?.to_png_bytes().map_err(SceneHostError::from)
    }
}

impl From<CapturePngError> for SceneHostError {
    fn from(error: CapturePngError) -> Self {
        Self::new(SceneHostErrorCode::Capture, error.to_string())
    }
}
