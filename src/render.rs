//! wgpu device/surface ownership, prepare lifecycle, passes, resource tables, and stats.

use std::cell::Cell;
use std::marker::PhantomData;
use std::sync::Weak;

mod cpu;
mod gpu;
mod output;
mod prepare;

use crate::assets::{Assets, EnvironmentHandle};
use crate::diagnostics::{
    Backend, BuildError, Capabilities, ChangeKind, DevicePoll, NotPreparedReason, PrepareError,
    RenderError, RenderOutcome, RendererStats,
};
use crate::geometry::Primitive;
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
    environment: Option<EnvironmentHandle>,
    environment_revision: u64,
    target_revision: u64,
    not_sync: PhantomData<Cell<()>>,
}

#[derive(Debug, Clone)]
struct PreparedSceneState {
    scene: Weak<()>,
    structure_revision: u64,
    environment_revision: u64,
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
            environment: None,
            environment_revision: 0,
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
        self.poll_device();
        validate_target_size(self.target.width, self.target.height).map_err(|()| {
            PrepareError::InvalidTargetSize {
                width: self.target.width,
                height: self.target.height,
            }
        })?;
        let environment_count = match self.environment {
            Some(environment) => {
                let Some(assets) = assets else {
                    return Err(PrepareError::EnvironmentAssetsRequired { environment });
                };
                if assets.environment(environment).is_none() {
                    return Err(PrepareError::EnvironmentNotFound { environment });
                }
                1
            }
            None => 0,
        };
        let primitives = prepare::collect_prepared_primitives(self.target, scene, assets)?;
        if let Some(gpu) = &mut self.gpu {
            gpu.prepare(self.target, &primitives);
            let stats = gpu.prepared_resource_stats();
            let pending_destructions = gpu.pending_destructions();
            self.stats.buffers = stats.buffers;
            self.stats.textures = stats.textures;
            self.stats.render_targets = stats.render_targets;
            self.stats.pipelines = stats.pipelines;
            self.stats.bind_groups = stats.bind_groups;
            self.stats.shader_modules = stats.shader_modules;
            self.stats.pending_destructions = pending_destructions;
            self.stats.approximate_gpu_memory_bytes = (stats.approximate_gpu_memory_bytes > 0)
                .then_some(stats.approximate_gpu_memory_bytes);
        }
        self.stats.environments = environment_count;
        self.prepared = Some(PreparedSceneState {
            scene: scene.identity(),
            structure_revision: scene.structure_revision(),
            environment_revision: self.environment_revision,
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
            let linear_frame = self
                .linear_frame
                .as_mut()
                .expect("CPU renderer owns a linear accumulator");
            cpu::clear_cpu(
                self.target,
                self.output,
                linear_frame,
                &mut self.frame,
                Color::BLACK,
            );
            for primitive in &primitives {
                let linear_frame = self
                    .linear_frame
                    .as_mut()
                    .expect("CPU renderer owns a linear accumulator");
                cpu::draw_primitive_cpu(
                    self.target,
                    self.output,
                    linear_frame,
                    &mut self.frame,
                    primitive,
                );
            }
        }
        self.poll_device();

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

    pub fn poll_device(&mut self) -> DevicePoll {
        let before = self.stats.pending_destructions;
        let (destroyed_resources, gpu_polled) = self
            .gpu
            .as_mut()
            .map(|gpu| gpu.poll_device())
            .unwrap_or((before, false));
        let after = self
            .gpu
            .as_ref()
            .map(|gpu| gpu.pending_destructions())
            .unwrap_or(0);
        self.stats.pending_destructions = after;
        DevicePoll {
            pending_destructions_before: before,
            pending_destructions_after: after,
            destroyed_resources,
            gpu_polled,
        }
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

    pub fn environment(&self) -> Option<EnvironmentHandle> {
        self.environment
    }

    pub fn set_environment(&mut self, environment: EnvironmentHandle) {
        if self.environment != Some(environment) {
            self.environment = Some(environment);
            self.environment_revision = self.environment_revision.saturating_add(1);
        }
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
            let submitted =
                gpu.render_to_frame(self.target, self.output.exposure_ev(), &mut self.frame)?;
            if submitted {
                self.stats.gpu_submissions = self.stats.gpu_submissions.saturating_add(1);
            }
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

        if prepared.environment_revision != self.environment_revision {
            return Err(RenderError::NotPrepared {
                reason: NotPreparedReason::EnvironmentChanged {
                    prepared_revision: prepared.environment_revision,
                    current_revision: self.environment_revision,
                    change: ChangeKind::Environment,
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
}

impl Drop for Renderer {
    fn drop(&mut self) {
        if let Some(gpu) = &mut self.gpu {
            gpu.release_prepared_resources();
            let _ = gpu.poll_device();
        }
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

fn validate_target_size(width: u32, height: u32) -> Result<(), ()> {
    if width == 0 || height == 0 {
        Err(())
    } else {
        Ok(())
    }
}
