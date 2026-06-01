use std::error::Error as StdError;
use std::fmt;
use std::io::Cursor;

#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

use crate::render::Renderer;
use crate::{CaptureError, CaptureOptions, CaptureRgba8, capture_rgba8};

use super::{FirstRender, HeadlessGltfViewer, HeadlessGltfViewerBuilder, InteractiveGltfViewer};

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

#[derive(Debug, Clone, PartialEq)]
pub enum ViewerPngError {
    Render(crate::Error),
    Capture(ViewerCaptureError),
}

impl HeadlessGltfViewerBuilder {
    /// Loads, frames, renders, and encodes a glTF/GLB scene as PNG bytes using
    /// the CPU headless renderer. This path does not request a GPU adapter or
    /// attach a platform surface.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use scena::headless_gltf_viewer;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let png = headless_gltf_viewer("machine.glb")
    ///     .render_png_bytes()
    ///     .await?;
    /// assert!(!png.is_empty());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn render_png_bytes(self) -> Result<Vec<u8>, ViewerPngError> {
        let first = self.render().await?;
        Ok(first.capture_png_bytes()?)
    }

    /// Loads, frames, renders, and writes a glTF/GLB scene as a PNG file using
    /// the CPU headless renderer. Native targets only.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use scena::headless_gltf_viewer;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// headless_gltf_viewer("machine.glb")
    ///     .render_png("frame.png")
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn render_png(self, path: impl AsRef<Path>) -> Result<(), ViewerPngError> {
        let path = path.as_ref().to_path_buf();
        let bytes = self.render_png_bytes().await?;
        std::fs::write(&path, bytes)
            .map_err(|error| ViewerCaptureError::Io {
                path: path.display().to_string(),
                reason: error.to_string(),
            })
            .map_err(ViewerPngError::from)
    }
}

impl FirstRender {
    /// Captures the rendered frame as RGBA8 plus a versioned descriptor that
    /// binds the pixels to the renderer's last rendered scene revisions, camera
    /// state, backend capabilities, and pixel statistics.
    pub fn capture(&self) -> Result<CaptureRgba8, CaptureError> {
        capture_rgba8(
            &self.scene,
            &self.renderer,
            capture_options_for_import(&self.import, &self.scene),
        )
    }

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
    /// Captures the latest rendered frame as RGBA8 plus a versioned descriptor
    /// that binds the pixels to the renderer's last rendered scene revisions,
    /// camera state, backend capabilities, and pixel statistics.
    pub fn capture(&self) -> Result<CaptureRgba8, CaptureError> {
        capture_rgba8(
            &self.scene,
            &self.renderer,
            capture_options_for_import(&self.import, &self.scene),
        )
    }

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
    /// Captures the latest rendered frame as RGBA8 plus a versioned descriptor
    /// that binds the pixels to the renderer's last rendered scene revisions,
    /// camera state, backend capabilities, and pixel statistics.
    pub fn capture(&self) -> Result<CaptureRgba8, CaptureError> {
        capture_rgba8(
            &self.scene,
            &self.renderer,
            capture_options_for_import(&self.import, &self.scene),
        )
    }

    /// Encodes the latest rendered frame as RGBA8 PNG bytes.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use scena::InteractiveGltfViewer;
    /// # fn example(viewer: &InteractiveGltfViewer) -> Result<(), scena::ViewerCaptureError> {
    /// let bytes = viewer.capture_png_bytes()?;
    /// assert!(!bytes.is_empty());
    /// # Ok(())
    /// # }
    /// ```
    pub fn capture_png_bytes(&self) -> Result<Vec<u8>, ViewerCaptureError> {
        capture_png_bytes_from_renderer(&self.renderer)
    }

    /// Writes the latest rendered frame as a PNG file. Native targets only.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use scena::InteractiveGltfViewer;
    /// # fn example(viewer: &InteractiveGltfViewer) -> Result<(), scena::ViewerCaptureError> {
    /// viewer.capture_png("frame.png")?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(not(target_arch = "wasm32"))]
    pub fn capture_png(&self, path: impl AsRef<Path>) -> Result<(), ViewerCaptureError> {
        capture_png_from_renderer(&self.renderer, path)
    }
}

fn capture_options_for_import(import: &crate::SceneImport, scene: &crate::Scene) -> CaptureOptions {
    import
        .bounds_world(scene)
        .map_or_else(CaptureOptions::default, |bounds| {
            CaptureOptions::default().with_auto_frame_bounds(bounds)
        })
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

impl fmt::Display for ViewerPngError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Render(error) => write!(formatter, "failed to render glTF PNG: {error}"),
            Self::Capture(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ViewerPngError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Render(error) => Some(error),
            Self::Capture(error) => Some(error),
        }
    }
}

impl From<crate::Error> for ViewerPngError {
    fn from(error: crate::Error) -> Self {
        Self::Render(error)
    }
}

impl From<ViewerCaptureError> for ViewerPngError {
    fn from(error: ViewerCaptureError) -> Self {
        Self::Capture(error)
    }
}
