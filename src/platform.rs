//! Optional native/browser platform adapters. Renderer logic must stay outside this module.

use std::fmt;

#[cfg(not(target_arch = "wasm32"))]
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceKind {
    NativeWindow,
    BrowserWebGpuCanvas,
    BrowserWebGl2Canvas,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SurfaceEvent {
    Resize { width: u32, height: u32 },
    ViewportChanged(SurfaceViewport),
    ScaleFactorChanged { scale_factor: f64 },
    Occluded { occluded: bool },
    Hidden,
    Shown,
    Lost,
    ContextLost { recoverable: bool },
    ContextRestored,
    DeviceLost { recoverable: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceViewport {
    logical_width: f32,
    logical_height: f32,
    device_pixel_ratio: f32,
}

pub enum PlatformSurface {
    NativeWindow(SurfaceSize),
    BrowserWebGpuCanvas(SurfaceSize),
    BrowserWebGl2Canvas(SurfaceSize),
    #[cfg(not(target_arch = "wasm32"))]
    AttachedNativeWindow {
        size: SurfaceSize,
        window: BoxedNativeWindow,
    },
    #[cfg(target_arch = "wasm32")]
    AttachedBrowserWebGpuCanvas {
        size: SurfaceSize,
        canvas: web_sys::HtmlCanvasElement,
    },
    #[cfg(target_arch = "wasm32")]
    AttachedBrowserWebGl2Canvas {
        size: SurfaceSize,
        canvas: web_sys::HtmlCanvasElement,
    },
}

#[cfg(not(target_arch = "wasm32"))]
pub trait NativeWindowHandle: HasDisplayHandle + HasWindowHandle + Send + Sync {}

#[cfg(not(target_arch = "wasm32"))]
impl<T> NativeWindowHandle for T where T: HasDisplayHandle + HasWindowHandle + Send + Sync {}

#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(not(target_arch = "wasm32"), doc(hidden))]
pub struct BoxedNativeWindow {
    inner: Box<dyn NativeWindowHandle + 'static>,
}

pub(crate) enum PlatformSurfaceAttachment {
    Descriptor,
    #[cfg(not(target_arch = "wasm32"))]
    NativeWindow(BoxedNativeWindow),
    #[cfg(target_arch = "wasm32")]
    BrowserWebGpuCanvas(web_sys::HtmlCanvasElement),
    #[cfg(target_arch = "wasm32")]
    BrowserWebGl2Canvas(web_sys::HtmlCanvasElement),
}

impl PlatformSurface {
    pub const fn native_window(width: u32, height: u32) -> Self {
        Self::NativeWindow(SurfaceSize { width, height })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn native_window_handle(
        window: impl NativeWindowHandle + 'static,
        width: u32,
        height: u32,
    ) -> Self {
        Self::AttachedNativeWindow {
            size: SurfaceSize { width, height },
            window: BoxedNativeWindow {
                inner: Box::new(window),
            },
        }
    }

    pub const fn browser_canvas(width: u32, height: u32) -> Self {
        Self::browser_webgpu_canvas(width, height)
    }

    pub const fn browser_webgpu_canvas(width: u32, height: u32) -> Self {
        Self::BrowserWebGpuCanvas(SurfaceSize { width, height })
    }

    pub const fn browser_webgl2_canvas(width: u32, height: u32) -> Self {
        Self::BrowserWebGl2Canvas(SurfaceSize { width, height })
    }

    #[cfg(target_arch = "wasm32")]
    pub fn browser_webgpu_canvas_element(
        canvas: web_sys::HtmlCanvasElement,
        width: u32,
        height: u32,
    ) -> Self {
        Self::AttachedBrowserWebGpuCanvas {
            size: SurfaceSize { width, height },
            canvas,
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn browser_webgl2_canvas_element(
        canvas: web_sys::HtmlCanvasElement,
        width: u32,
        height: u32,
    ) -> Self {
        Self::AttachedBrowserWebGl2Canvas {
            size: SurfaceSize { width, height },
            canvas,
        }
    }

    pub fn kind(&self) -> SurfaceKind {
        match self {
            Self::NativeWindow(_) => SurfaceKind::NativeWindow,
            Self::BrowserWebGpuCanvas(_) => SurfaceKind::BrowserWebGpuCanvas,
            Self::BrowserWebGl2Canvas(_) => SurfaceKind::BrowserWebGl2Canvas,
            #[cfg(not(target_arch = "wasm32"))]
            Self::AttachedNativeWindow { .. } => SurfaceKind::NativeWindow,
            #[cfg(target_arch = "wasm32")]
            Self::AttachedBrowserWebGpuCanvas { .. } => SurfaceKind::BrowserWebGpuCanvas,
            #[cfg(target_arch = "wasm32")]
            Self::AttachedBrowserWebGl2Canvas { .. } => SurfaceKind::BrowserWebGl2Canvas,
        }
    }

    pub fn size(&self) -> SurfaceSize {
        match self {
            Self::NativeWindow(size)
            | Self::BrowserWebGpuCanvas(size)
            | Self::BrowserWebGl2Canvas(size) => *size,
            #[cfg(not(target_arch = "wasm32"))]
            Self::AttachedNativeWindow { size, .. } => *size,
            #[cfg(target_arch = "wasm32")]
            Self::AttachedBrowserWebGpuCanvas { size, .. }
            | Self::AttachedBrowserWebGl2Canvas { size, .. } => *size,
        }
    }

    pub fn is_attached(&self) -> bool {
        !matches!(
            self,
            Self::NativeWindow(_) | Self::BrowserWebGpuCanvas(_) | Self::BrowserWebGl2Canvas(_)
        )
    }

    pub(crate) fn into_parts(self) -> (SurfaceKind, SurfaceSize, PlatformSurfaceAttachment) {
        let kind = self.kind();
        let size = self.size();
        let attachment = match self {
            Self::NativeWindow(_) | Self::BrowserWebGpuCanvas(_) | Self::BrowserWebGl2Canvas(_) => {
                PlatformSurfaceAttachment::Descriptor
            }
            #[cfg(not(target_arch = "wasm32"))]
            Self::AttachedNativeWindow { window, .. } => {
                PlatformSurfaceAttachment::NativeWindow(window)
            }
            #[cfg(target_arch = "wasm32")]
            Self::AttachedBrowserWebGpuCanvas { canvas, .. } => {
                PlatformSurfaceAttachment::BrowserWebGpuCanvas(canvas)
            }
            #[cfg(target_arch = "wasm32")]
            Self::AttachedBrowserWebGl2Canvas { canvas, .. } => {
                PlatformSurfaceAttachment::BrowserWebGl2Canvas(canvas)
            }
        };
        (kind, size, attachment)
    }
}

impl SurfaceViewport {
    pub fn new(logical_width: f32, logical_height: f32, device_pixel_ratio: f32) -> Option<Self> {
        (logical_width.is_finite()
            && logical_height.is_finite()
            && device_pixel_ratio.is_finite()
            && logical_width > 0.0
            && logical_height > 0.0
            && device_pixel_ratio > 0.0)
            .then_some(Self {
                logical_width,
                logical_height,
                device_pixel_ratio,
            })
    }

    pub const fn logical_width(self) -> f32 {
        self.logical_width
    }

    pub const fn logical_height(self) -> f32 {
        self.logical_height
    }

    pub const fn device_pixel_ratio(self) -> f32 {
        self.device_pixel_ratio
    }

    pub fn physical_size(self) -> SurfaceSize {
        SurfaceSize {
            width: logical_to_physical(self.logical_width, self.device_pixel_ratio),
            height: logical_to_physical(self.logical_height, self.device_pixel_ratio),
        }
    }
}

fn logical_to_physical(logical: f32, device_pixel_ratio: f32) -> u32 {
    (logical * device_pixel_ratio).round().max(1.0) as u32
}

impl fmt::Debug for PlatformSurface {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PlatformSurface")
            .field("kind", &self.kind())
            .field("size", &self.size())
            .field("attached", &self.is_attached())
            .finish()
    }
}

impl PartialEq for PlatformSurface {
    fn eq(&self, other: &Self) -> bool {
        self.kind() == other.kind()
            && self.size() == other.size()
            && self.is_attached() == other.is_attached()
    }
}

impl Eq for PlatformSurface {}

#[cfg(not(target_arch = "wasm32"))]
impl fmt::Debug for BoxedNativeWindow {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BoxedNativeWindow")
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl HasDisplayHandle for BoxedNativeWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        self.inner.display_handle()
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl HasWindowHandle for BoxedNativeWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        self.inner.window_handle()
    }
}
