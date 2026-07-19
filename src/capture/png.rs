use std::error::Error as StdError;
use std::fmt;
use std::io::Cursor;

use std::path::Path;

use super::CaptureError;

#[derive(Debug, Clone, PartialEq)]
pub enum CapturePngError {
    Capture(CaptureError),
    InvalidPixelBuffer {
        width: u32,
        height: u32,
        expected_len: usize,
        actual_len: usize,
    },
    Encode {
        reason: String,
    },
    Io {
        path: String,
        reason: String,
    },
}

pub(super) fn encode_png_rgba8(
    width: u32,
    height: u32,
    rgba8: &[u8],
) -> Result<Vec<u8>, CapturePngError> {
    let expected_len = width as usize * height as usize * 4;
    if rgba8.len() != expected_len {
        return Err(CapturePngError::InvalidPixelBuffer {
            width,
            height,
            expected_len,
            actual_len: rgba8.len(),
        });
    }

    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(Cursor::new(&mut bytes), width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|error| CapturePngError::Encode {
                reason: error.to_string(),
            })?;
        writer
            .write_image_data(rgba8)
            .map_err(|error| CapturePngError::Encode {
                reason: error.to_string(),
            })?;
    }
    Ok(bytes)
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn write_png(path: impl AsRef<Path>, bytes: Vec<u8>) -> Result<(), CapturePngError> {
    let path = path.as_ref();
    std::fs::write(path, bytes).map_err(|error| CapturePngError::Io {
        path: path.display().to_string(),
        reason: error.to_string(),
    })
}

#[cfg(target_arch = "wasm32")]
pub(super) fn write_png(path: impl AsRef<Path>, _bytes: Vec<u8>) -> Result<(), CapturePngError> {
    Err(CapturePngError::Io {
        path: path.as_ref().display().to_string(),
        reason: "filesystem PNG writing is unsupported on wasm32; use to_png_bytes() or the browser capture API"
            .to_owned(),
    })
}

impl From<CaptureError> for CapturePngError {
    fn from(error: CaptureError) -> Self {
        Self::Capture(error)
    }
}

impl fmt::Display for CapturePngError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Capture(error) => error.fmt(formatter),
            Self::InvalidPixelBuffer {
                width,
                height,
                expected_len,
                actual_len,
            } => write!(
                formatter,
                "capture frame buffer for {width}x{height} has {actual_len} bytes; expected {expected_len} RGBA8 bytes"
            ),
            Self::Encode { reason } => write!(formatter, "failed to encode PNG: {reason}"),
            Self::Io { path, reason } => write!(formatter, "failed to write PNG {path}: {reason}"),
        }
    }
}

impl StdError for CapturePngError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Capture(error) => Some(error),
            Self::InvalidPixelBuffer { .. } | Self::Encode { .. } | Self::Io { .. } => None,
        }
    }
}
