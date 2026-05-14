use crate::diagnostics::BuildError;
#[cfg(all(target_arch = "wasm32", feature = "browser-probe"))]
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
    #[cfg(all(target_arch = "wasm32", feature = "browser-probe"))]
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

    #[cfg(all(target_arch = "wasm32", feature = "browser-probe"))]
    pub async fn browser_probe_readback_rgba8(&mut self) -> Result<Option<PixelReadback>, JsValue> {
        let Some(gpu) = &mut self.gpu else {
            return Ok(None);
        };
        let Some(rgba8) = gpu.browser_probe_readback_rgba8(self.target).await? else {
            return Ok(None);
        };
        Ok(Some(PixelReadback::from_rgba8(
            self.target.width,
            self.target.height,
            rgba8,
        )))
    }
}
