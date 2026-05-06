//! wgpu device/surface ownership, prepare lifecycle, passes, resource tables, and stats.

use std::cell::Cell;
use std::marker::PhantomData;
use std::sync::Weak;

mod gpu;

use crate::diagnostics::{
    Backend, BuildError, Capabilities, ChangeKind, NotPreparedReason, PrepareError, RenderError,
    RenderOutcome, RendererStats,
};
use crate::geometry::{Primitive, Vertex};
use crate::material::Color;
use crate::platform::{PlatformSurface, PlatformSurfaceAttachment, SurfaceEvent, SurfaceKind};
use crate::scene::{CameraKey, Scene};

use self::gpu::GpuDeviceState;

#[derive(Debug)]
pub struct Renderer {
    target: RasterTarget,
    prepared: Option<PreparedSceneState>,
    frame: Vec<u8>,
    stats: RendererStats,
    capabilities: Capabilities,
    gpu: Option<GpuDeviceState>,
    target_revision: u64,
    not_sync: PhantomData<Cell<()>>,
}

#[derive(Debug, Clone)]
struct PreparedSceneState {
    scene: Weak<()>,
    structure_revision: u64,
    target_revision: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RasterTarget {
    width: u32,
    height: u32,
    backend: Backend,
}

impl Renderer {
    pub fn headless(width: u32, height: u32) -> Result<Self, BuildError> {
        Self::from_raster_target(width, height, Backend::Headless, None, false)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn headless_gpu(width: u32, height: u32) -> Result<Self, BuildError> {
        validate_target_size(width, height)
            .map_err(|()| BuildError::InvalidTargetSize { width, height })?;
        let gpu = pollster::block_on(gpu::request_headless_gpu(Backend::HeadlessGpu))?;
        Self::from_raster_target(width, height, Backend::HeadlessGpu, Some(gpu), false)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn headless_gpu(width: u32, height: u32) -> Result<Self, BuildError> {
        validate_target_size(width, height)
            .map_err(|()| BuildError::InvalidTargetSize { width, height })?;
        Err(BuildError::UnsupportedBackend {
            backend: Backend::HeadlessGpu,
        })
    }

    pub fn from_surface(surface: PlatformSurface) -> Result<Self, BuildError> {
        let (kind, size, attachment) = surface.into_parts();
        let backend = match kind {
            SurfaceKind::NativeWindow => Backend::NativeSurface,
            SurfaceKind::BrowserWebGpuCanvas => Backend::WebGpu,
            SurfaceKind::BrowserWebGl2Canvas => Backend::WebGl2,
        };
        match attachment {
            PlatformSurfaceAttachment::Descriptor => {
                Self::from_raster_target(size.width, size.height, backend, None, false)
            }
            #[cfg(not(target_arch = "wasm32"))]
            PlatformSurfaceAttachment::NativeWindow(window) => {
                let gpu =
                    pollster::block_on(gpu::request_native_surface_gpu(backend, size, window))?;
                Self::from_raster_target(size.width, size.height, backend, Some(gpu), true)
            }
            #[cfg(target_arch = "wasm32")]
            PlatformSurfaceAttachment::BrowserWebGpuCanvas(_)
            | PlatformSurfaceAttachment::BrowserWebGl2Canvas(_) => {
                Err(BuildError::AsyncSurfaceRequired { backend })
            }
        }
    }

    pub async fn from_surface_async(surface: PlatformSurface) -> Result<Self, BuildError> {
        let (kind, size, attachment) = surface.into_parts();
        let backend = match kind {
            SurfaceKind::NativeWindow => Backend::NativeSurface,
            SurfaceKind::BrowserWebGpuCanvas => Backend::WebGpu,
            SurfaceKind::BrowserWebGl2Canvas => Backend::WebGl2,
        };
        let gpu = match attachment {
            PlatformSurfaceAttachment::Descriptor => {
                return Self::from_raster_target(size.width, size.height, backend, None, false);
            }
            #[cfg(not(target_arch = "wasm32"))]
            PlatformSurfaceAttachment::NativeWindow(window) => {
                gpu::request_native_surface_gpu(backend, size, window).await?
            }
            #[cfg(target_arch = "wasm32")]
            PlatformSurfaceAttachment::BrowserWebGpuCanvas(canvas)
            | PlatformSurfaceAttachment::BrowserWebGl2Canvas(canvas) => {
                gpu::request_browser_surface_gpu(backend, size, canvas).await?
            }
        };
        Self::from_raster_target(size.width, size.height, backend, Some(gpu), true)
    }

    fn from_raster_target(
        width: u32,
        height: u32,
        backend: Backend,
        gpu: Option<GpuDeviceState>,
        surface_attached: bool,
    ) -> Result<Self, BuildError> {
        validate_target_size(width, height)
            .map_err(|()| BuildError::InvalidTargetSize { width, height })?;
        let capabilities = if surface_attached {
            Capabilities::for_attached_gpu_backend(backend)
        } else if gpu.is_some() {
            Capabilities::for_gpu_backend(backend)
        } else {
            Capabilities::for_backend(backend)
        };
        let target = RasterTarget {
            width,
            height,
            backend,
        };
        Ok(Self {
            target,
            prepared: None,
            frame: vec![0; target.byte_len()],
            stats: RendererStats {
                target_width: width,
                target_height: height,
                ..RendererStats::default()
            },
            capabilities,
            gpu,
            target_revision: 0,
            not_sync: PhantomData,
        })
    }

    pub fn prepare(&mut self, scene: &mut Scene) -> Result<(), PrepareError> {
        validate_target_size(self.target.width, self.target.height).map_err(|()| {
            PrepareError::InvalidTargetSize {
                width: self.target.width,
                height: self.target.height,
            }
        })?;
        self.prepared = Some(PreparedSceneState {
            scene: scene.identity(),
            structure_revision: scene.structure_revision(),
            target_revision: self.target_revision,
        });
        if let Some(gpu) = &mut self.gpu {
            gpu.prepare(self.target);
        }
        Ok(())
    }

    pub fn render(
        &mut self,
        scene: &Scene,
        camera: CameraKey,
    ) -> Result<RenderOutcome, RenderError> {
        self.require_prepared(scene)?;
        if scene.camera(camera).is_none() {
            return Err(RenderError::CameraNotFound(camera));
        }

        let primitives = count_primitives(scene);
        if self.gpu.is_some() {
            self.draw_gpu(primitives)?;
        } else {
            self.clear(Color::BLACK);
            for renderable in scene.renderables() {
                for primitive in renderable.primitives() {
                    self.draw_primitive(primitive);
                }
            }
        }

        self.stats.frames_rendered = self.stats.frames_rendered.saturating_add(1);
        self.stats.draw_calls = primitives;
        self.stats.primitives = primitives;

        Ok(RenderOutcome {
            width: self.target.width,
            height: self.target.height,
            draw_calls: primitives,
            primitives,
        })
    }

    pub fn render_active(&mut self, scene: &Scene) -> Result<RenderOutcome, RenderError> {
        self.require_prepared(scene)?;
        let camera = scene.active_camera().ok_or(RenderError::NoActiveCamera)?;
        self.render(scene, camera)
    }

    pub fn handle_surface_event(&mut self, event: SurfaceEvent) -> Result<(), RenderError> {
        match event {
            SurfaceEvent::Resize { width, height } => {
                validate_target_size(width, height)
                    .map_err(|()| RenderError::InvalidSurfaceSize { width, height })?;
                self.target.width = width;
                self.target.height = height;
                self.frame.resize(self.target.byte_len(), 0);
                self.stats.target_width = width;
                self.stats.target_height = height;
                self.target_revision = self.target_revision.saturating_add(1);
            }
        }
        Ok(())
    }

    pub fn frame_rgba8(&self) -> &[u8] {
        &self.frame
    }

    pub fn stats(&self) -> RendererStats {
        self.stats
    }

    pub fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    pub fn has_gpu_device(&self) -> bool {
        self.gpu.is_some()
    }

    fn draw_gpu(&mut self, primitives: u64) -> Result<(), RenderError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let gpu = self
                .gpu
                .as_mut()
                .expect("draw_gpu is called only when a GPU device exists");
            self.frame = gpu.render_to_frame(self.target, primitives)?;
            self.stats.gpu_submissions = self.stats.gpu_submissions.saturating_add(1);
            Ok(())
        }

        #[cfg(target_arch = "wasm32")]
        {
            let _ = primitives;
            Err(RenderError::GpuResourcesNotPrepared {
                backend: self.target.backend,
            })
        }
    }

    fn require_prepared(&self, scene: &Scene) -> Result<(), RenderError> {
        let prepared = self.prepared.as_ref().ok_or(RenderError::NotPrepared {
            reason: NotPreparedReason::NeverPrepared,
        })?;

        if !prepared.scene.ptr_eq(&scene.identity()) {
            return Err(RenderError::NotPrepared {
                reason: NotPreparedReason::DifferentScene,
            });
        }

        let current_revision = scene.structure_revision();
        if prepared.structure_revision != current_revision {
            return Err(RenderError::NotPrepared {
                reason: NotPreparedReason::SceneChanged {
                    prepared_revision: prepared.structure_revision,
                    current_revision,
                    change: ChangeKind::SceneStructure,
                },
            });
        }

        if prepared.target_revision != self.target_revision {
            return Err(RenderError::NotPrepared {
                reason: NotPreparedReason::TargetChanged {
                    prepared_revision: prepared.target_revision,
                    current_revision: self.target_revision,
                    change: ChangeKind::RenderTarget,
                },
            });
        }

        Ok(())
    }

    fn clear(&mut self, color: Color) {
        let rgba = color.to_rgba8();
        for pixel in self.frame.chunks_exact_mut(4) {
            pixel.copy_from_slice(&rgba);
        }
    }

    fn draw_primitive(&mut self, primitive: &Primitive) {
        let [a, b, c] = primitive.vertices();
        let a = ScreenVertex::from_vertex(*a, self.target);
        let b = ScreenVertex::from_vertex(*b, self.target);
        let c = ScreenVertex::from_vertex(*c, self.target);

        let min_x = a.x.min(b.x).min(c.x).floor().max(0.0) as u32;
        let max_x =
            a.x.max(b.x)
                .max(c.x)
                .ceil()
                .min(self.target.width as f32 - 1.0) as u32;
        let min_y = a.y.min(b.y).min(c.y).floor().max(0.0) as u32;
        let max_y =
            a.y.max(b.y)
                .max(c.y)
                .ceil()
                .min(self.target.height as f32 - 1.0) as u32;

        let area = edge(a, b, c.x, c.y);
        if area.abs() <= f32::EPSILON {
            return;
        }

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;
                let w0 = edge(b, c, px, py) / area;
                let w1 = edge(c, a, px, py) / area;
                let w2 = edge(a, b, px, py) / area;
                if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                    let color = mix_color(a.color, b.color, c.color, w0, w1, w2);
                    self.write_pixel(x, y, color);
                }
            }
        }
    }

    fn write_pixel(&mut self, x: u32, y: u32, color: Color) {
        let index = ((y * self.target.width + x) * 4) as usize;
        self.frame[index..index + 4].copy_from_slice(&color.to_rgba8());
    }
}

impl RasterTarget {
    fn byte_len(self) -> usize {
        (self.width as usize) * (self.height as usize) * 4
    }
}

fn count_primitives(scene: &Scene) -> u64 {
    scene
        .renderables()
        .map(|renderable| renderable.primitives().len() as u64)
        .sum()
}

#[derive(Debug, Clone, Copy)]
struct ScreenVertex {
    x: f32,
    y: f32,
    color: Color,
}

impl ScreenVertex {
    fn from_vertex(vertex: Vertex, target: RasterTarget) -> Self {
        let width = target.width.saturating_sub(1) as f32;
        let height = target.height.saturating_sub(1) as f32;
        Self {
            x: (vertex.position.x * 0.5 + 0.5) * width,
            y: (1.0 - (vertex.position.y * 0.5 + 0.5)) * height,
            color: vertex.color,
        }
    }
}

fn edge(a: ScreenVertex, b: ScreenVertex, x: f32, y: f32) -> f32 {
    (x - a.x) * (b.y - a.y) - (y - a.y) * (b.x - a.x)
}

fn mix_color(a: Color, b: Color, c: Color, w0: f32, w1: f32, w2: f32) -> Color {
    Color::from_linear_rgba(
        a.r * w0 + b.r * w1 + c.r * w2,
        a.g * w0 + b.g * w1 + c.g * w2,
        a.b * w0 + b.b * w1 + c.b * w2,
        a.a * w0 + b.a * w1 + c.a * w2,
    )
}

fn validate_target_size(width: u32, height: u32) -> Result<(), ()> {
    if width == 0 || height == 0 {
        Err(())
    } else {
        Ok(())
    }
}
