use crate::diagnostics::BuildError;
#[cfg(any(feature = "scene-host", test))]
use crate::diagnostics::RenderError;
#[cfg(all(
    target_arch = "wasm32",
    any(feature = "browser-probe", feature = "scene-host")
))]
use wasm_bindgen::JsValue;

use super::Renderer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OffscreenTarget {
    width: u32,
    height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PixelReadback {
    width: u32,
    height: u32,
    rgba8: Vec<u8>,
}

#[cfg(any(feature = "scene-host", test))]
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SceneLinearCapture {
    width: u32,
    height: u32,
    rgba32f: Vec<[f32; 4]>,
}

#[cfg(any(feature = "scene-host", test))]
impl SceneLinearCapture {
    pub(crate) const fn width(&self) -> u32 {
        self.width
    }

    pub(crate) const fn height(&self) -> u32 {
        self.height
    }

    #[cfg(test)]
    pub(crate) fn rgba32f(&self) -> &[[f32; 4]] {
        &self.rgba32f
    }

    #[cfg(feature = "scene-host")]
    pub(crate) fn into_rgba32f(self) -> Vec<[f32; 4]> {
        self.rgba32f
    }
}

impl OffscreenTarget {
    pub const fn new(width: u32, height: u32) -> Result<Self, BuildError> {
        if width == 0 || height == 0 {
            Err(BuildError::InvalidTargetSize { width, height })
        } else {
            Ok(Self { width, height })
        }
    }

    pub const fn width(self) -> u32 {
        self.width
    }

    pub const fn height(self) -> u32 {
        self.height
    }
}

impl PixelReadback {
    #[cfg(any(
        not(target_arch = "wasm32"),
        feature = "browser-probe",
        feature = "scene-host"
    ))]
    pub(in crate::render) fn from_rgba8(width: u32, height: u32, rgba8: Vec<u8>) -> Self {
        Self {
            width,
            height,
            rgba8,
        }
    }

    pub const fn width(&self) -> u32 {
        self.width
    }

    pub const fn height(&self) -> u32 {
        self.height
    }

    pub fn rgba8(&self) -> &[u8] {
        &self.rgba8
    }

    pub fn into_rgba8(self) -> Vec<u8> {
        self.rgba8
    }
}

impl Renderer {
    pub fn offscreen(target: OffscreenTarget) -> Result<Self, BuildError> {
        Self::headless(target.width, target.height)
    }

    pub fn read_pixels(&self) -> PixelReadback {
        PixelReadback {
            width: self.target.width,
            height: self.target.height,
            rgba8: self.frame.clone(),
        }
    }

    pub fn screenshot_rgba8(&self) -> PixelReadback {
        self.read_pixels()
    }

    #[cfg(any(feature = "scene-host", test))]
    pub(crate) fn scene_linear_capture(&mut self) -> Result<SceneLinearCapture, RenderError> {
        if let Some(gpu) = &mut self.gpu {
            let (target, rgba32f) = gpu.read_scene_linear_rgba32f(self.target.backend)?;
            return Ok(SceneLinearCapture {
                width: target.width,
                height: target.height,
                rgba32f,
            });
        }
        let rgba32f = self
            .linear_frame
            .as_ref()
            .ok_or(RenderError::GpuResourcesNotPrepared {
                backend: self.target.backend,
            })?
            .iter()
            .map(|color| [color.r, color.g, color.b, color.a])
            .collect();
        Ok(SceneLinearCapture {
            width: self.target.width,
            height: self.target.height,
            rgba32f,
        })
    }

    #[cfg(all(
        target_arch = "wasm32",
        any(feature = "browser-probe", feature = "scene-host")
    ))]
    pub(crate) async fn browser_readback_rgba8(
        &mut self,
    ) -> Result<Option<PixelReadback>, JsValue> {
        let Some(gpu) = &mut self.gpu else {
            return Ok(None);
        };
        let Some(rgba8) = gpu.browser_readback_rgba8(self.target).await? else {
            return Ok(None);
        };
        self.frame.clear();
        self.frame.extend_from_slice(&rgba8);
        self.last_readback_frame = self
            .last_rendered_frame
            .map(super::state::RenderedFrameState::with_readback_completed_now);
        Ok(Some(PixelReadback::from_rgba8(
            self.target.width,
            self.target.height,
            rgba8,
        )))
    }
}
