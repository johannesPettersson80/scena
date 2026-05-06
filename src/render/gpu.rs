#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc;

#[cfg(not(target_arch = "wasm32"))]
mod output;

#[cfg(not(target_arch = "wasm32"))]
use crate::diagnostics::RenderError;
use crate::diagnostics::{Backend, BuildError};
#[cfg(not(target_arch = "wasm32"))]
use crate::geometry::{Primitive, Vertex};
#[cfg(not(target_arch = "wasm32"))]
use crate::platform::BoxedNativeWindow;
use crate::platform::SurfaceSize;

#[cfg(not(target_arch = "wasm32"))]
use self::output::{
    GPU_TRIANGLE_SHADER, create_output_bind_group, create_output_bind_group_layout,
    create_output_uniform_buffer, encode_output_uniform,
};
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
    vertex_buffer: wgpu::Buffer,
    output_uniform: wgpu::Buffer,
    output_bind_group: wgpu::BindGroup,
    vertex_count: u32,
    offscreen_pipeline: wgpu::RenderPipeline,
    surface_pipeline: Option<wgpu::RenderPipeline>,
    padded_bytes_per_row: u32,
    unpadded_bytes_per_row: u32,
}

impl GpuDeviceState {
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn prepare(&mut self, target: RasterTarget, primitives: &[Primitive]) {
        self.configure_surface(target);
        let vertex_bytes = encode_vertices(primitives);
        let vertex_buffer_size = vertex_bytes.len().max(4) as u64;
        let vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scena.m0.scene_vertices"),
            size: vertex_buffer_size,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: true,
        });
        if !vertex_bytes.is_empty() {
            let mut mapped = vertex_buffer.slice(..).get_mapped_range_mut();
            mapped.copy_from_slice(&vertex_bytes);
        }
        vertex_buffer.unmap();

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
        let output_bind_group_layout = create_output_bind_group_layout(&self.device);
        let output_uniform = create_output_uniform_buffer(&self.device);
        let output_bind_group =
            create_output_bind_group(&self.device, &output_bind_group_layout, &output_uniform);
        let offscreen_pipeline =
            create_unlit_pipeline(&self.device, GPU_COLOR_FORMAT, &output_bind_group_layout);
        let surface_pipeline = self.surface.as_ref().map(|surface| {
            create_unlit_pipeline(
                &self.device,
                surface.config.format,
                &output_bind_group_layout,
            )
        });

        self.resources = Some(GpuPreparedResources {
            target,
            texture,
            view,
            readback,
            vertex_buffer,
            output_uniform,
            output_bind_group,
            vertex_count: (vertex_bytes.len() / VERTEX_BYTE_LEN) as u32,
            offscreen_pipeline,
            surface_pipeline,
            padded_bytes_per_row,
            unpadded_bytes_per_row,
        });
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn prepare(
        &mut self,
        target: RasterTarget,
        primitives: &[crate::geometry::Primitive],
    ) {
        let _ = primitives;
        self.configure_surface(target);
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn render_to_frame(
        &mut self,
        target: RasterTarget,
        exposure_ev: f32,
        frame: &mut Vec<u8>,
    ) -> Result<(), RenderError> {
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
        self.queue.write_buffer(
            &resources.output_uniform,
            0,
            &encode_output_uniform(exposure_ev),
        );
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
            &resources.vertex_buffer,
            &resources.output_bind_group,
            resources.vertex_count,
            &resources.offscreen_pipeline,
            "scena.headless_gpu.render_pass",
        );
        if let (Some(surface_view), Some(surface_pipeline)) =
            (surface_view.as_ref(), resources.surface_pipeline.as_ref())
        {
            encode_unlit_pass(
                &mut encoder,
                surface_view,
                &resources.vertex_buffer,
                &resources.output_bind_group,
                resources.vertex_count,
                surface_pipeline,
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
        if frame.len() != target.byte_len() {
            frame.resize(target.byte_len(), 0);
        }
        for row in 0..target.height as usize {
            let source_start = row * resources.padded_bytes_per_row as usize;
            let source_end = source_start + resources.unpadded_bytes_per_row as usize;
            let target_start = row * resources.unpadded_bytes_per_row as usize;
            let target_end = target_start + resources.unpadded_bytes_per_row as usize;
            frame[target_start..target_end].copy_from_slice(&mapped[source_start..source_end]);
        }
        drop(mapped);
        resources.readback.unmap();

        Ok(())
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
#[allow(dead_code)]
pub(super) async fn request_browser_surface_gpu(
    backend: crate::diagnostics::Backend,
    size: crate::platform::SurfaceSize,
    canvas: web_sys::HtmlCanvasElement,
) -> Result<GpuDeviceState, crate::diagnostics::BuildError> {
    request_surface_gpu(backend, size, wgpu::SurfaceTarget::Canvas(canvas)).await
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
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
    vertex_buffer: &wgpu::Buffer,
    output_bind_group: &wgpu::BindGroup,
    vertex_count: u32,
    pipeline: &wgpu::RenderPipeline,
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
    pass.set_bind_group(0, output_bind_group, &[]);
    pass.set_vertex_buffer(0, vertex_buffer.slice(..));
    if vertex_count > 0 {
        pass.draw(0..vertex_count, 0..1);
    }
}

#[cfg(not(target_arch = "wasm32"))]
const BYTES_PER_PIXEL: u32 = 4;
#[cfg(not(target_arch = "wasm32"))]
const GPU_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
#[cfg(not(target_arch = "wasm32"))]
const VERTEX_BYTE_LEN: usize = 7 * std::mem::size_of::<f32>();
#[cfg(not(target_arch = "wasm32"))]
const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 2] = [
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: 0,
        shader_location: 0,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 3 * std::mem::size_of::<f32>() as u64,
        shader_location: 1,
    },
];

#[cfg(not(target_arch = "wasm32"))]
fn align_to(value: u32, alignment: u32) -> u32 {
    value.div_ceil(alignment) * alignment
}

#[cfg(not(target_arch = "wasm32"))]
fn create_unlit_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    output_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("scena.m0.unlit_triangle"),
        source: wgpu::ShaderSource::Wgsl(GPU_TRIANGLE_SHADER.into()),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("scena.m0.pipeline_layout"),
        bind_group_layouts: &[Some(output_bind_group_layout)],
        immediate_size: 0,
    });
    let vertex_buffer = wgpu::VertexBufferLayout {
        array_stride: VERTEX_BYTE_LEN as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &VERTEX_ATTRIBUTES,
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("scena.m0.unlit_triangle_pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[vertex_buffer],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn encode_vertices(primitives: &[Primitive]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(primitives.len() * 3 * VERTEX_BYTE_LEN);
    for primitive in primitives {
        for vertex in primitive.vertices() {
            encode_vertex(&mut bytes, *vertex);
        }
    }
    bytes
}

#[cfg(not(target_arch = "wasm32"))]
fn encode_vertex(bytes: &mut Vec<u8>, vertex: Vertex) {
    for value in [
        vertex.position.x,
        vertex.position.y,
        vertex.position.z,
        vertex.color.r,
        vertex.color.g,
        vertex.color.b,
        vertex.color.a,
    ] {
        bytes.extend_from_slice(&value.to_ne_bytes());
    }
}
