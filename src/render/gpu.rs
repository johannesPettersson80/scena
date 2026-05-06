#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc;

#[cfg(not(target_arch = "wasm32"))]
use crate::diagnostics::RenderError;
use crate::diagnostics::{Backend, BuildError};
#[cfg(not(target_arch = "wasm32"))]
use crate::platform::BoxedNativeWindow;
use crate::platform::SurfaceSize;

use super::RasterTarget;

#[allow(dead_code)]
#[derive(Debug)]
pub(super) struct GpuDeviceState {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: Option<GpuSurfaceState>,
    #[cfg(not(target_arch = "wasm32"))]
    resources: Option<GpuPreparedResources>,
}

#[derive(Debug)]
struct GpuSurfaceState {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
struct GpuPreparedResources {
    target: RasterTarget,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    readback: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
    padded_bytes_per_row: u32,
    unpadded_bytes_per_row: u32,
}

impl GpuDeviceState {
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn prepare(&mut self, target: RasterTarget) {
        self.configure_surface(target);
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("scena.headless_gpu.target"),
            size: wgpu::Extent3d {
                width: target.width,
                height: target.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: GPU_COLOR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let unpadded_bytes_per_row = target.width.saturating_mul(BYTES_PER_PIXEL);
        let padded_bytes_per_row =
            align_to(unpadded_bytes_per_row, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scena.headless_gpu.readback"),
            size: u64::from(padded_bytes_per_row) * u64::from(target.height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("scena.m0.unlit_triangle"),
                source: wgpu::ShaderSource::Wgsl(GPU_TRIANGLE_SHADER.into()),
            });
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("scena.m0.pipeline_layout"),
                bind_group_layouts: &[],
                immediate_size: 0,
            });
        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("scena.m0.unlit_triangle_pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: GPU_COLOR_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });

        self.resources = Some(GpuPreparedResources {
            target,
            texture,
            view,
            readback,
            pipeline,
            padded_bytes_per_row,
            unpadded_bytes_per_row,
        });
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn prepare(&mut self, target: RasterTarget) {
        self.configure_surface(target);
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn render_to_frame(
        &mut self,
        target: RasterTarget,
        primitives: u64,
    ) -> Result<Vec<u8>, RenderError> {
        if self
            .resources
            .as_ref()
            .is_none_or(|resources| resources.target != target)
        {
            return Err(RenderError::GpuResourcesNotPrepared {
                backend: target.backend,
            });
        }
        let resources = self
            .resources
            .as_ref()
            .expect("resources are checked before rendering");
        let surface_output =
            self.surface
                .as_ref()
                .and_then(|surface| match surface.surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(output)
                    | wgpu::CurrentSurfaceTexture::Suboptimal(output) => Some(output),
                    wgpu::CurrentSurfaceTexture::Timeout
                    | wgpu::CurrentSurfaceTexture::Occluded
                    | wgpu::CurrentSurfaceTexture::Outdated
                    | wgpu::CurrentSurfaceTexture::Lost
                    | wgpu::CurrentSurfaceTexture::Validation => None,
                });
        let surface_view = surface_output.as_ref().map(|output| {
            output
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default())
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("scena.headless_gpu.encoder"),
            });
        encode_unlit_pass(
            &mut encoder,
            &resources.view,
            &resources.pipeline,
            primitives,
            "scena.headless_gpu.render_pass",
        );
        if let Some(surface_view) = surface_view.as_ref() {
            encode_unlit_pass(
                &mut encoder,
                surface_view,
                &resources.pipeline,
                primitives,
                "scena.surface.render_pass",
            );
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &resources.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &resources.readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(resources.padded_bytes_per_row),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: target.width,
                height: target.height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));
        if let Some(surface_output) = surface_output {
            surface_output.present();
        }

        let readback = resources.readback.slice(..);
        let (sender, receiver) = mpsc::channel();
        readback.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|_| RenderError::GpuReadback {
                backend: target.backend,
            })?;
        receiver
            .recv()
            .map_err(|_| RenderError::GpuReadback {
                backend: target.backend,
            })?
            .map_err(|_| RenderError::GpuReadback {
                backend: target.backend,
            })?;

        let mapped = readback.get_mapped_range();
        let mut frame = vec![0; target.byte_len()];
        for row in 0..target.height as usize {
            let source_start = row * resources.padded_bytes_per_row as usize;
            let source_end = source_start + resources.unpadded_bytes_per_row as usize;
            let target_start = row * resources.unpadded_bytes_per_row as usize;
            let target_end = target_start + resources.unpadded_bytes_per_row as usize;
            frame[target_start..target_end].copy_from_slice(&mapped[source_start..source_end]);
        }
        drop(mapped);
        resources.readback.unmap();

        Ok(frame)
    }

    fn configure_surface(&mut self, target: RasterTarget) {
        if let Some(surface) = &mut self.surface {
            if surface.config.width != target.width || surface.config.height != target.height {
                surface.config.width = target.width;
                surface.config.height = target.height;
            }
            surface.surface.configure(&self.device, &surface.config);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) async fn request_headless_gpu(backend: Backend) -> Result<GpuDeviceState, BuildError> {
    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .map_err(|_| BuildError::NoAdapter { backend })?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .map_err(|_| BuildError::RequestDevice { backend })?;

    Ok(GpuDeviceState {
        instance,
        adapter,
        device,
        queue,
        surface: None,
        #[cfg(not(target_arch = "wasm32"))]
        resources: None,
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) async fn request_native_surface_gpu(
    backend: Backend,
    size: SurfaceSize,
    window: BoxedNativeWindow,
) -> Result<GpuDeviceState, BuildError> {
    request_surface_gpu(backend, size, wgpu::SurfaceTarget::from(window)).await
}

#[cfg(target_arch = "wasm32")]
pub(super) async fn request_browser_surface_gpu(
    backend: crate::diagnostics::Backend,
    size: crate::platform::SurfaceSize,
    canvas: web_sys::HtmlCanvasElement,
) -> Result<GpuDeviceState, crate::diagnostics::BuildError> {
    request_surface_gpu(backend, size, wgpu::SurfaceTarget::Canvas(canvas)).await
}

async fn request_surface_gpu(
    backend: Backend,
    size: SurfaceSize,
    target: wgpu::SurfaceTarget<'static>,
) -> Result<GpuDeviceState, BuildError> {
    let instance = wgpu::Instance::default();
    let surface = instance
        .create_surface(target)
        .map_err(|_| BuildError::CreateSurface { backend })?;
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..wgpu::RequestAdapterOptions::default()
        })
        .await
        .map_err(|_| BuildError::NoAdapter { backend })?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .map_err(|_| BuildError::RequestDevice { backend })?;
    let config = surface
        .get_default_config(&adapter, size.width, size.height)
        .ok_or(BuildError::SurfaceUnsupported { backend })?;
    surface.configure(&device, &config);

    Ok(GpuDeviceState {
        instance,
        adapter,
        device,
        queue,
        surface: Some(GpuSurfaceState { surface, config }),
        #[cfg(not(target_arch = "wasm32"))]
        resources: None,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn encode_unlit_pass(
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    primitives: u64,
    label: &'static str,
) {
    let color_attachment = Some(wgpu::RenderPassColorAttachment {
        view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            store: wgpu::StoreOp::Store,
        },
    });
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[color_attachment],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(pipeline);
    if primitives > 0 {
        let instances = primitives.min(u64::from(u32::MAX)) as u32;
        pass.draw(0..3, 0..instances);
    }
}

#[cfg(not(target_arch = "wasm32"))]
const BYTES_PER_PIXEL: u32 = 4;
#[cfg(not(target_arch = "wasm32"))]
const GPU_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
#[cfg(not(target_arch = "wasm32"))]
const GPU_TRIANGLE_SHADER: &str = r#"
struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOut {
    let index = vertex_index % 3u;
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-0.6, -0.5),
        vec2<f32>(0.6, -0.5),
        vec2<f32>(0.0, 0.6),
    );
    let colors = array<vec4<f32>, 3>(
        vec4<f32>(1.0, 0.2, 0.1, 1.0),
        vec4<f32>(0.1, 0.8, 0.2, 1.0),
        vec4<f32>(0.1, 0.3, 1.0, 1.0),
    );

    var out: VertexOut;
    out.position = vec4<f32>(positions[index], 0.0, 1.0);
    out.color = colors[index];
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

#[cfg(not(target_arch = "wasm32"))]
fn align_to(value: u32, alignment: u32) -> u32 {
    value.div_ceil(alignment) * alignment
}
