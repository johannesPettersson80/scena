use std::fmt;
use std::io::Cursor;

#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

use crate::render::Renderer;

use super::{FirstRender, HeadlessGltfViewer, InteractiveGltfViewer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewerCaptureError {
    InvalidFrameBuffer {
        width: u32,
        height: u32,
        expected_len: usize,
        actual_len: usize,
    },
    EncodePng {
        reason: String,
    },
    Io {
        path: String,
        reason: String,
    },
}

impl FirstRender {
    /// Encodes the rendered frame as RGBA8 PNG bytes.
    pub fn capture_png_bytes(&self) -> Result<Vec<u8>, ViewerCaptureError> {
        capture_png_bytes_from_renderer(&self.renderer)
    }

    /// Writes the rendered frame as a PNG file.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn capture_png(&self, path: impl AsRef<Path>) -> Result<(), ViewerCaptureError> {
        capture_png_from_renderer(&self.renderer, path)
    }
}

impl HeadlessGltfViewer {
    /// Encodes the latest rendered frame as RGBA8 PNG bytes.
    pub fn capture_png_bytes(&self) -> Result<Vec<u8>, ViewerCaptureError> {
        capture_png_bytes_from_renderer(&self.renderer)
    }

    /// Writes the latest rendered frame as a PNG file.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn capture_png(&self, path: impl AsRef<Path>) -> Result<(), ViewerCaptureError> {
        capture_png_from_renderer(&self.renderer, path)
    }
}

impl InteractiveGltfViewer {
    /// Encodes the latest rendered frame as RGBA8 PNG bytes.
    pub fn capture_png_bytes(&self) -> Result<Vec<u8>, ViewerCaptureError> {
        capture_png_bytes_from_renderer(&self.renderer)
    }

    /// Writes the latest rendered frame as a PNG file.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn capture_png(&self, path: impl AsRef<Path>) -> Result<(), ViewerCaptureError> {
        capture_png_from_renderer(&self.renderer, path)
    }
}

fn capture_png_bytes_from_renderer(renderer: &Renderer) -> Result<Vec<u8>, ViewerCaptureError> {
    let stats = renderer.stats();
    let width = stats.target_width;
    let height = stats.target_height;
    let frame = renderer.frame_rgba8();
    let expected_len = width as usize * height as usize * 4;
    if frame.len() != expected_len {
        return Err(ViewerCaptureError::InvalidFrameBuffer {
            width,
            height,
            expected_len,
            actual_len: frame.len(),
        });
    }

    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(Cursor::new(&mut bytes), width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|error| ViewerCaptureError::EncodePng {
                reason: error.to_string(),
            })?;
        writer
            .write_image_data(frame)
            .map_err(|error| ViewerCaptureError::EncodePng {
                reason: error.to_string(),
            })?;
    }
    Ok(bytes)
}

#[cfg(not(target_arch = "wasm32"))]
fn capture_png_from_renderer(
    renderer: &Renderer,
    path: impl AsRef<Path>,
) -> Result<(), ViewerCaptureError> {
    let path = path.as_ref();
    let bytes = capture_png_bytes_from_renderer(renderer)?;
    std::fs::write(path, bytes).map_err(|error| ViewerCaptureError::Io {
        path: path.display().to_string(),
        reason: error.to_string(),
    })
}

impl fmt::Display for ViewerCaptureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFrameBuffer {
                width,
                height,
                expected_len,
                actual_len,
            } => write!(
                formatter,
                "renderer frame buffer for {width}x{height} has {actual_len} bytes; expected {expected_len} RGBA8 bytes"
            ),
            Self::EncodePng { reason } => write!(formatter, "failed to encode PNG: {reason}"),
            Self::Io { path, reason } => write!(formatter, "failed to write PNG {path}: {reason}"),
        }
    }
}

impl std::error::Error for ViewerCaptureError {}
