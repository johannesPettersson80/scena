//! wgpu device/surface ownership, prepare lifecycle, passes, resource tables, and stats.

use std::cell::Cell;
use std::marker::PhantomData;
use std::sync::Weak;

mod gpu;
mod output;
mod prepare;

use crate::assets::Assets;
use crate::diagnostics::{
    Backend, BuildError, Capabilities, ChangeKind, NotPreparedReason, PrepareError, RenderError,
    RenderOutcome, RendererStats,
};
use crate::geometry::{Primitive, Vertex};
use crate::material::Color;
use crate::platform::{PlatformSurface, PlatformSurfaceAttachment, SurfaceEvent, SurfaceKind};
use crate::scene::{CameraKey, Scene};

use self::gpu::GpuDeviceState;
use self::output::OutputTransform;
pub use self::output::Tonemapper;

#[derive(Debug)]
pub struct Renderer {
    target: RasterTarget,
    prepared: Option<PreparedSceneState>,
    frame: Vec<u8>,
    // CPU-only linear scene-referred straight-alpha accumulator. Stores the source of truth
    // before every pixel is ACES+sRGB encoded into `frame`.
    linear_frame: Option<Vec<Color>>,
    stats: RendererStats,
    capabilities: Capabilities,
    gpu: Option<GpuDeviceState>,
    output: OutputTransform,
    target_revision: u64,
    not_sync: PhantomData<Cell<()>>,
}

#[derive(Debug, Clone)]
struct PreparedSceneState {
    scene: Weak<()>,
    structure_revision: u64,
    target_revision: u64,
    primitives: Vec<Primitive>,
}

/// Row-major render target dimensions used for CPU frame and accumulator indexing.
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
        match attachment {
            PlatformSurfaceAttachment::Descriptor => Self::from_raster_target(
                size.width,
                size.height,
                Backend::SurfaceDescriptor,
                None,
                false,
            ),
            #[cfg(not(target_arch = "wasm32"))]
            PlatformSurfaceAttachment::NativeWindow(window) => {
                let backend = backend_for_attached_surface(kind);
                let gpu =
                    pollster::block_on(gpu::request_native_surface_gpu(backend, size, window))?;
                Self::from_raster_target(size.width, size.height, backend, Some(gpu), true)
            }
            #[cfg(target_arch = "wasm32")]
            PlatformSurfaceAttachment::BrowserWebGpuCanvas(_)
            | PlatformSurfaceAttachment::BrowserWebGl2Canvas(_) => {
                let backend = backend_for_attached_surface(kind);
                Err(BuildError::AsyncSurfaceRequired { backend })
            }
        }
    }

    pub async fn from_surface_async(surface: PlatformSurface) -> Result<Self, BuildError> {
        let (kind, size, attachment) = surface.into_parts();

        #[cfg(target_arch = "wasm32")]
        {
            match attachment {
                PlatformSurfaceAttachment::Descriptor => {
                    return Self::from_raster_target(
                        size.width,
                        size.height,
                        Backend::SurfaceDescriptor,
                        None,
                        false,
                    );
                }
                PlatformSurfaceAttachment::BrowserWebGpuCanvas(canvas)
                | PlatformSurfaceAttachment::BrowserWebGl2Canvas(canvas) => {
                    let _ = canvas;
                    let backend = backend_for_attached_surface(kind);
                    return Err(BuildError::UnsupportedBackend { backend });
                }
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let gpu = match attachment {
                PlatformSurfaceAttachment::Descriptor => {
                    return Self::from_raster_target(
                        size.width,
                        size.height,
                        Backend::SurfaceDescriptor,
                        None,
                        false,
                    );
                }
                PlatformSurfaceAttachment::NativeWindow(window) => {
                    let backend = backend_for_attached_surface(kind);
                    gpu::request_native_surface_gpu(backend, size, window).await?
                }
            };
            let backend = backend_for_attached_surface(kind);
            Self::from_raster_target(size.width, size.height, backend, Some(gpu), true)
        }
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
        let has_gpu = gpu.is_some();
        let capabilities = if surface_attached {
            Capabilities::for_attached_gpu_backend(backend)
        } else if has_gpu {
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
            linear_frame: (!has_gpu).then(|| vec![Color::BLACK; target.pixel_len()]),
            stats: RendererStats {
                target_width: width,
                target_height: height,
                ..RendererStats::default()
            },
            capabilities,
            gpu,
            output: OutputTransform::default(),
            target_revision: 0,
            not_sync: PhantomData,
        })
    }

    pub fn prepare(&mut self, scene: &mut Scene) -> Result<(), PrepareError> {
        self.prepare_inner::<()>(scene, None)
    }

    pub fn prepare_with_assets<F>(
        &mut self,
        scene: &mut Scene,
        assets: &Assets<F>,
    ) -> Result<(), PrepareError> {
        self.prepare_inner(scene, Some(assets))
    }

    fn prepare_inner<F>(
        &mut self,
        scene: &mut Scene,
        assets: Option<&Assets<F>>,
    ) -> Result<(), PrepareError> {
        validate_target_size(self.target.width, self.target.height).map_err(|()| {
            PrepareError::InvalidTargetSize {
                width: self.target.width,
                height: self.target.height,
            }
        })?;
        let primitives = prepare::collect_prepared_primitives(self.target, scene, assets)?;
        if let Some(gpu) = &mut self.gpu {
            gpu.prepare(self.target, &primitives);
        }
        self.prepared = Some(PreparedSceneState {
            scene: scene.identity(),
            structure_revision: scene.structure_revision(),
            target_revision: self.target_revision,
            primitives,
        });
        Ok(())
    }

    pub fn render(
        &mut self,
        scene: &Scene,
        camera: CameraKey,
    ) -> Result<RenderOutcome, RenderError> {
        self.prepared_state(scene)?;
        if scene.camera(camera).is_none() {
            return Err(RenderError::CameraNotFound(camera));
        }

        let primitives = self.prepared_state(scene)?.primitives.clone();
        let primitive_count = primitives.len() as u64;
        if self.gpu.is_some() {
            self.draw_gpu()?;
        } else {
            self.clear(Color::BLACK);
            for primitive in &primitives {
                self.draw_primitive(primitive);
            }
        }

        self.stats.frames_rendered = self.stats.frames_rendered.saturating_add(1);
        self.stats.draw_calls = primitive_count;
        self.stats.triangles = primitive_count;
        self.stats.primitives = primitive_count;

        Ok(RenderOutcome {
            width: self.target.width,
            height: self.target.height,
            draw_calls: primitive_count,
            primitives: primitive_count,
        })
    }

    pub fn render_active(&mut self, scene: &Scene) -> Result<RenderOutcome, RenderError> {
        self.prepared_state(scene)?;
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
                if let Some(linear_frame) = &mut self.linear_frame {
                    linear_frame.resize(self.target.pixel_len(), Color::BLACK);
                }
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

    pub fn exposure_ev(&self) -> f32 {
        self.output.exposure_ev()
    }

    pub fn set_exposure_ev(&mut self, exposure_ev: f32) {
        self.output.set_exposure_ev(exposure_ev);
    }

    pub fn tonemapper(&self) -> Tonemapper {
        self.output.tonemapper()
    }

    pub fn set_tonemapper(&mut self, tonemapper: Tonemapper) {
        self.output.set_tonemapper(tonemapper);
    }

    pub fn has_gpu_device(&self) -> bool {
        self.gpu.is_some()
    }

    fn draw_gpu(&mut self) -> Result<(), RenderError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let gpu = self
                .gpu
                .as_mut()
                .expect("draw_gpu is called only when a GPU device exists");
            gpu.render_to_frame(self.target, &mut self.frame)?;
            self.stats.gpu_submissions = self.stats.gpu_submissions.saturating_add(1);
            Ok(())
        }

        #[cfg(target_arch = "wasm32")]
        {
            Err(RenderError::GpuResourcesNotPrepared {
                backend: self.target.backend,
            })
        }
    }

    fn prepared_state(&self, scene: &Scene) -> Result<&PreparedSceneState, RenderError> {
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

        Ok(prepared)
    }

    fn clear(&mut self, color: Color) {
        let rgba = self.output.encode_rgba8(color);
        let linear_frame = self
            .linear_frame
            .as_mut()
            .expect("CPU renderer owns a linear accumulator");
        for (linear, pixel) in linear_frame.iter_mut().zip(self.frame.chunks_exact_mut(4)) {
            *linear = color;
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
        let pixel_index = self.target.pixel_index(x, y);
        let linear_frame = self
            .linear_frame
            .as_mut()
            .expect("CPU renderer owns a linear accumulator");
        let blended = blend_source_over(color, linear_frame[pixel_index]);
        linear_frame[pixel_index] = blended;

        let byte_index = pixel_index * 4;
        self.frame[byte_index..byte_index + 4].copy_from_slice(&self.output.encode_rgba8(blended));
    }
}

impl RasterTarget {
    fn pixel_len(self) -> usize {
        (self.width as usize) * (self.height as usize)
    }

    fn byte_len(self) -> usize {
        self.pixel_len() * 4
    }

    fn pixel_index(self, x: u32, y: u32) -> usize {
        (y as usize) * (self.width as usize) + (x as usize)
    }
}

fn backend_for_attached_surface(kind: SurfaceKind) -> Backend {
    match kind {
        SurfaceKind::NativeWindow => Backend::NativeSurface,
        SurfaceKind::BrowserWebGpuCanvas => Backend::WebGpu,
        SurfaceKind::BrowserWebGl2Canvas => Backend::WebGl2,
    }
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

fn blend_source_over(source: Color, destination: Color) -> Color {
    let source_alpha = clamp_alpha_or(source.a, 1.0);
    let destination_alpha = clamp_alpha_or(destination.a, 1.0);
    if source_alpha == 1.0 {
        return Color::from_linear_rgba(source.r, source.g, source.b, 1.0);
    }
    if source_alpha <= 0.0 {
        return destination;
    }

    let inverse_source_alpha = 1.0 - source_alpha;
    let output_alpha = source_alpha + destination_alpha * inverse_source_alpha;
    // RGB is intentionally unclamped so HDR linear input can reach the ACES output stage.
    let premultiplied_r =
        source.r * source_alpha + destination.r * destination_alpha * inverse_source_alpha;
    let premultiplied_g =
        source.g * source_alpha + destination.g * destination_alpha * inverse_source_alpha;
    let premultiplied_b =
        source.b * source_alpha + destination.b * destination_alpha * inverse_source_alpha;

    if output_alpha <= f32::EPSILON {
        Color::from_linear_rgba(0.0, 0.0, 0.0, 0.0)
    } else {
        Color::from_linear_rgba(
            premultiplied_r / output_alpha,
            premultiplied_g / output_alpha,
            premultiplied_b / output_alpha,
            output_alpha,
        )
    }
}

fn clamp_alpha_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        fallback
    }
}

fn validate_target_size(width: u32, height: u32) -> Result<(), ()> {
    if width == 0 || height == 0 {
        Err(())
    } else {
        Ok(())
    }
}
