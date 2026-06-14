//! Stable capture descriptors for binding pixels to scene state.

use std::error::Error as StdError;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::diagnostics::{Backend, Capabilities, LookupError};
use crate::geometry::Aabb;
use crate::platform::SurfaceViewport;
use crate::render::Renderer;
use crate::scene::{Scene, Transform};

mod metadata;
mod pixels;
mod png;
mod proof;

use metadata::{capture_auto_frame, capture_camera, capture_viewport, revisions_from_dirty};

pub use metadata::auto_frame_metadata;
pub use pixels::{
    CapturePixelBounds, CapturePixelSummary, fnv1a64_hex, sample_rgba8, summarize_pixel_readback,
    summarize_rgba8,
};
pub use png::CapturePngError;
pub use proof::{
    CAPTURE_BASELINE_SCHEMA_V1, CaptureBaselineDiff, CaptureBaselineError, CaptureBaselineReport,
    CaptureBaselineTolerance, CaptureContactSheet, CaptureContactSheetError,
    CaptureContactSheetTile, capture_contact_sheet_rgba8, compare_captures_with_tolerance,
};

pub const CAPTURE_SCHEMA_V1: &str = "scena.capture.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureDescriptor {
    pub schema: String,
    pub width: u32,
    pub height: u32,
    pub pixel_format: String,
    pub payload: CapturePayload,
    pub revisions: CaptureRevisions,
    pub camera: CaptureCamera,
    pub viewport: CaptureViewport,
    pub backend: Backend,
    pub capabilities: Capabilities,
    pub auto_frame: Option<CaptureAutoFrame>,
    pub pixels: CapturePixelSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturePayload {
    pub kind: CapturePayloadKind,
    pub byte_length: usize,
    pub fnv1a64: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapturePayloadKind {
    Rgba8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureRevisions {
    pub structure: u64,
    pub transform: u64,
    #[serde(default)]
    pub appearance: u64,
    pub interaction: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureCamera {
    pub active: bool,
    pub world_transform: Option<Transform>,
    pub projection: Option<CaptureProjection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CaptureProjection {
    Perspective {
        vertical_fov_radians: f32,
        aspect: f32,
        near: f32,
        far: f32,
    },
    Orthographic {
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near: f32,
        far: f32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CaptureViewport {
    pub width: u32,
    pub height: u32,
    pub logical_width: f32,
    pub logical_height: f32,
    pub device_pixel_ratio: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureAutoFrame {
    pub status: String,
    pub proof_class: String,
    pub viewport: CaptureAutoFrameViewport,
    pub projected_rect: CaptureScreenRect,
    pub center_error_px: CapturePoint2,
    pub fill_fraction: f32,
    pub inside_viewport: bool,
    pub centered: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CaptureAutoFrameViewport {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CapturePoint2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CaptureScreenRect {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
    pub width: f32,
    pub height: f32,
    pub center_x: f32,
    pub center_y: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaptureRgba8 {
    pub descriptor: CaptureDescriptor,
    pub rgba8: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaptureOptions {
    device_pixel_ratio: f32,
    logical_size: Option<(f32, f32)>,
    auto_frame_bounds: Option<Aabb>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CaptureError {
    InvalidDevicePixelRatio {
        value: f32,
    },
    InvalidPixelBuffer {
        width: u32,
        height: u32,
        expected_len: usize,
        actual_len: usize,
    },
    NoActiveCameraForAutoFrame,
    NoRenderedFrame,
    StaleRender {
        rendered: CaptureRevisions,
        current: CaptureRevisions,
    },
    AutoFrameProjection {
        reason: String,
    },
}

impl CaptureDescriptor {
    pub fn to_schema_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("capture descriptor contains only serializable fields")
    }
}

impl CaptureRgba8 {
    pub fn to_png_bytes(&self) -> Result<Vec<u8>, CapturePngError> {
        png::encode_png_rgba8(
            self.descriptor.width,
            self.descriptor.height,
            self.rgba8.as_slice(),
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn write_png(&self, path: impl AsRef<std::path::Path>) -> Result<(), CapturePngError> {
        png::write_png(path, self.to_png_bytes()?)
    }
}

impl CaptureProjection {
    pub const fn kind(self) -> &'static str {
        match self {
            Self::Perspective { .. } => "perspective",
            Self::Orthographic { .. } => "orthographic",
        }
    }
}

impl CaptureOptions {
    pub const fn new() -> Self {
        Self {
            device_pixel_ratio: 1.0,
            logical_size: None,
            auto_frame_bounds: None,
        }
    }

    pub const fn with_device_pixel_ratio(mut self, device_pixel_ratio: f32) -> Self {
        self.device_pixel_ratio = device_pixel_ratio;
        self
    }

    pub const fn with_logical_size(mut self, logical_width: f32, logical_height: f32) -> Self {
        self.logical_size = Some((logical_width, logical_height));
        self
    }

    pub fn with_surface_viewport(mut self, viewport: SurfaceViewport) -> Self {
        self.device_pixel_ratio = viewport.device_pixel_ratio();
        self.logical_size = Some((viewport.logical_width(), viewport.logical_height()));
        self
    }

    pub const fn with_auto_frame_bounds(mut self, bounds: Aabb) -> Self {
        self.auto_frame_bounds = Some(bounds);
        self
    }

    pub const fn without_auto_frame_bounds(mut self) -> Self {
        self.auto_frame_bounds = None;
        self
    }
}

impl Default for CaptureOptions {
    fn default() -> Self {
        Self::new()
    }
}

pub fn capture_rgba8(
    scene: &Scene,
    renderer: &Renderer,
    options: CaptureOptions,
) -> Result<CaptureRgba8, CaptureError> {
    let readback = renderer.read_pixels();
    let width = readback.width();
    let height = readback.height();
    let rgba8 = readback.into_rgba8();
    capture_rgba8_from_pixels(scene, renderer, options, width, height, rgba8)
}

pub fn capture_rgba8_from_pixels(
    scene: &Scene,
    renderer: &Renderer,
    options: CaptureOptions,
    width: u32,
    height: u32,
    rgba8: Vec<u8>,
) -> Result<CaptureRgba8, CaptureError> {
    let rendered = renderer
        .rendered_frame_state()
        .ok_or(CaptureError::NoRenderedFrame)?;
    let rendered_revisions = revisions_from_dirty(rendered.dirty_state());
    let current_revisions = revisions_from_dirty(scene.dirty_state());
    if rendered_revisions != current_revisions || scene.active_camera() != Some(rendered.camera()) {
        return Err(CaptureError::StaleRender {
            rendered: rendered_revisions,
            current: current_revisions,
        });
    }
    let camera = capture_camera(scene, rendered.camera());
    let capabilities = *renderer.capabilities();
    let backend = capabilities.backend;
    pixels::validate_rgba8_len(width, height, rgba8.len())?;
    let pixels = summarize_rgba8(width, height, rgba8.as_slice())?;
    let auto_frame = capture_auto_frame(
        scene,
        rendered.camera(),
        options.auto_frame_bounds,
        width,
        height,
    )?;
    let viewport = capture_viewport(width, height, options)?;

    let descriptor = CaptureDescriptor {
        schema: CAPTURE_SCHEMA_V1.to_owned(),
        width,
        height,
        pixel_format: "rgba8".to_owned(),
        payload: CapturePayload {
            kind: CapturePayloadKind::Rgba8,
            byte_length: rgba8.len(),
            fnv1a64: pixels.fnv1a64.clone(),
        },
        revisions: rendered_revisions,
        camera,
        viewport,
        backend,
        capabilities,
        auto_frame,
        pixels,
    };

    Ok(CaptureRgba8 { descriptor, rgba8 })
}

impl Renderer {
    pub fn capture_rgba8(
        &self,
        scene: &Scene,
        options: CaptureOptions,
    ) -> Result<CaptureRgba8, CaptureError> {
        capture_rgba8(scene, self, options)
    }

    pub fn capture_png_bytes(
        &self,
        scene: &Scene,
        options: CaptureOptions,
    ) -> Result<Vec<u8>, CapturePngError> {
        self.capture_rgba8(scene, options)
            .map_err(CapturePngError::from)?
            .to_png_bytes()
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn capture_png(
        &self,
        scene: &Scene,
        options: CaptureOptions,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(), CapturePngError> {
        self.capture_rgba8(scene, options)
            .map_err(CapturePngError::from)?
            .write_png(path)
    }
}

impl fmt::Display for CaptureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDevicePixelRatio { value } => {
                write!(formatter, "invalid capture device pixel ratio {value}")
            }
            Self::InvalidPixelBuffer {
                width,
                height,
                expected_len,
                actual_len,
            } => write!(
                formatter,
                "capture frame buffer for {width}x{height} has {actual_len} bytes; expected {expected_len} RGBA8 bytes"
            ),
            Self::NoActiveCameraForAutoFrame => {
                write!(
                    formatter,
                    "auto-frame capture requested without an active camera"
                )
            }
            Self::NoRenderedFrame => {
                write!(
                    formatter,
                    "capture requested before the renderer produced a frame"
                )
            }
            Self::StaleRender { rendered, current } => {
                write!(
                    formatter,
                    "capture scene state changed after render (rendered structure/transform/appearance/interaction = {}/{}/{}/{}, current = {}/{}/{}/{})",
                    rendered.structure,
                    rendered.transform,
                    rendered.appearance,
                    rendered.interaction,
                    current.structure,
                    current.transform,
                    current.appearance,
                    current.interaction
                )
            }
            Self::AutoFrameProjection { reason } => write!(formatter, "{reason}"),
        }
    }
}

impl StdError for CaptureError {}

impl From<LookupError> for CaptureError {
    fn from(error: LookupError) -> Self {
        Self::AutoFrameProjection {
            reason: error.to_string(),
        }
    }
}
