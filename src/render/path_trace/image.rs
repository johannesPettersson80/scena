//! The result of a path-traced render.

use crate::capture::{CapturePngError, encode_rgba8_as_png, write_png_file};

use super::options::PathTraceStats;

/// A finished path-traced still.
///
/// The image exists in two forms: display-ready 8-bit sRGB, passed through
/// the renderer's exposure, white balance and tone mapping, and the linear
/// radiance it was made from.
#[derive(Debug, Clone, PartialEq)]
pub struct PathTracedImage {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) rgba8: Vec<u8>,
    pub(super) radiance: Vec<[f32; 4]>,
    pub(super) stats: PathTraceStats,
}

impl PathTracedImage {
    pub const fn width(&self) -> u32 {
        self.width
    }

    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Rows of 8-bit sRGB pixels from the top, with straight alpha. Alpha is
    /// below 255 only where a transparent background shows through.
    pub fn rgba8(&self) -> &[u8] {
        &self.rgba8
    }

    /// Linear scene radiance of the traced surfaces, one `[r, g, b, coverage]`
    /// per pixel from the top-left. Colour is weighted by coverage, the share
    /// of the pixel that surfaces fill; the background is not included.
    pub fn linear_radiance(&self) -> &[[f32; 4]] {
        &self.radiance
    }

    pub const fn stats(&self) -> PathTraceStats {
        self.stats
    }

    /// Consumes the image and returns its 8-bit pixels.
    pub fn into_rgba8(self) -> Vec<u8> {
        self.rgba8
    }

    /// Encodes the 8-bit image as a PNG file in memory.
    pub fn to_png_bytes(&self) -> Result<Vec<u8>, CapturePngError> {
        encode_rgba8_as_png(self.width, self.height, &self.rgba8)
    }

    /// Writes the 8-bit image as a PNG file.
    pub fn write_png(&self, path: impl AsRef<std::path::Path>) -> Result<(), CapturePngError> {
        write_png_file(path, self.to_png_bytes()?)
    }
}
