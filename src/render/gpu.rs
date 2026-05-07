#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc;

mod build;
#[cfg(not(target_arch = "wasm32"))]
mod culling;
#[cfg(not(target_arch = "wasm32"))]
mod depth;
mod lifecycle;
mod output;
mod pipeline;
#[cfg(not(target_arch = "wasm32"))]
mod shadow;
mod stats;
mod vertices;
#[cfg(target_arch = "wasm32")]
mod webgl2;

use crate::diagnostics::Backend;
use crate::diagnostics::RenderError;
#[cfg(not(target_arch = "wasm32"))]
use crate::geometry::Primitive;

use self::output::{
    create_output_bind_group, create_output_bind_group_layout, create_output_uniform_buffer,
    encode_output_uniform,
};
#[cfg(not(target_arch = "wasm32"))]
use self::pipeline::{BYTES_PER_PIXEL, GPU_COLOR_FORMAT};
use self::pipeline::{create_unlit_pipeline, encode_unlit_pass};
#[cfg(not(target_arch = "wasm32"))]
use self::shadow::create_shadow_texture;
pub(super) use self::stats::GpuResourceStats;
#[cfg(not(target_arch = "wasm32"))]
use self::stats::align_to;
use self::stats::estimate_prepared_resource_stats;
use self::vertices::{VERTEX_BYTE_LEN, encode_vertices};
use super::RasterTarget;
use super::prepare::{PreparedDepthStats, PreparedLightingStats};

#[allow(dead_code)]
#[derive(Debug)]
pub(super) struct GpuDeviceState {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: Option<GpuSurfaceState>,
    pending_destructions: u64,
    resources: Option<GpuPreparedResources>,
    #[cfg(target_arch = "wasm32")]
    browser_canvas: Option<web_sys::HtmlCanvasElement>,
}

#[cfg(target_arch = "wasm32")]
pub(super) use build::request_browser_surface_gpu;
#[cfg(not(target_arch = "wasm32"))]
pub(super) use build::{request_headless_gpu, request_native_surface_gpu};

#[derive(Debug)]
pub(super) struct GpuSurfaceState {
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
    // ARCH-SHADOW-MAP: M2 allocates shadow resources before the shadow render pass is
    // wired; the explicit fields keep the deferred binding visible to reviews and doctor.
    #[allow(dead_code)]
    shadow_texture: Option<wgpu::Texture>,
    #[allow(dead_code)]
    shadow_view: Option<wgpu::TextureView>,
    depth_prepass: Option<depth::DepthPrepassResources>,
    culling_pipeline: Option<wgpu::ComputePipeline>,
    culling_workgroups: u32,
    vertex_count: u32,
    offscreen_pipeline: wgpu::RenderPipeline,
    surface_pipeline: Option<wgpu::RenderPipeline>,
    padded_bytes_per_row: u32,
    unpadded_bytes_per_row: u32,
    stats: GpuResourceStats,
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug)]
struct GpuPreparedResources {
    target: RasterTarget,
    vertex_buffer: wgpu::Buffer,
    output_uniform: wgpu::Buffer,
    output_bind_group: wgpu::BindGroup,
    surface_pipeline: wgpu::RenderPipeline,
    vertex_count: u32,
    webgl2_vertices: Vec<f32>,
    stats: GpuResourceStats,
}

impl GpuDeviceState {
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn prepare(
        &mut self,
        target: RasterTarget,
        primitives: &[Primitive],
        lighting_stats: PreparedLightingStats,
        depth_stats: PreparedDepthStats,
    ) {
        self.configure_surface(target);
        self.release_prepared_resources();
        if primitives.is_empty() {
            return;
        }

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
        let shadow_texture = create_shadow_texture(
            &self.device,
            lighting_stats.directional_shadow_map_resolution,
        );
        let shadow_view = shadow_texture
            .as_ref()
            .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()));
        let depth_prepass = (depth_stats.passes > 0).then(|| {
            depth::create_depth_prepass_resources(&self.device, target, depth_stats.reversed_z)
        });
        let culling_pipeline = matches!(
            target.backend,
            Backend::HeadlessGpu | Backend::NativeSurface | Backend::WebGpu
        )
        .then(|| culling::create_culling_pipeline(&self.device));
        let offscreen_pipeline =
            create_unlit_pipeline(&self.device, GPU_COLOR_FORMAT, &output_bind_group_layout);
        let surface_pipeline = self.surface.as_ref().map(|surface| {
            create_unlit_pipeline(
                &self.device,
                surface.config.format,
                &output_bind_group_layout,
            )
        });
        let stats = estimate_prepared_resource_stats(
            target,
            vertex_bytes.len() / VERTEX_BYTE_LEN,
            surface_pipeline.is_some(),
            lighting_stats.shadow_maps,
            lighting_stats.directional_shadow_map_resolution,
            depth_stats.passes,
            culling_pipeline.is_some(),
        );

        self.resources = Some(GpuPreparedResources {
            target,
            texture,
            view,
            readback,
            vertex_buffer,
            output_uniform,
            output_bind_group,
            shadow_texture,
            shadow_view,
            depth_prepass,
            culling_pipeline,
            culling_workgroups: (primitives.len() as u32).max(1).div_ceil(64),
            vertex_count: (vertex_bytes.len() / VERTEX_BYTE_LEN) as u32,
            offscreen_pipeline,
            surface_pipeline,
            padded_bytes_per_row,
            unpadded_bytes_per_row,
            stats,
        });
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn prepare(
        &mut self,
        target: RasterTarget,
        primitives: &[crate::geometry::Primitive],
        lighting_stats: PreparedLightingStats,
        depth_stats: PreparedDepthStats,
    ) {
        let _ = lighting_stats;
        let _ = depth_stats;
        self.configure_surface(target);
        self.release_prepared_resources();
        let Some(surface) = self.surface.as_ref() else {
            return;
        };
        if primitives.is_empty() {
            return;
        }

        let vertex_bytes = encode_vertices(primitives);
        let webgl2_vertices = webgl2::encode_vertices(primitives);
        let vertex_buffer_size = vertex_bytes.len().max(4) as u64;
        let vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scena.browser.scene_vertices"),
            size: vertex_buffer_size,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: true,
        });
        if !vertex_bytes.is_empty() {
            let mut mapped = vertex_buffer.slice(..).get_mapped_range_mut();
            mapped.copy_from_slice(&vertex_bytes);
        }
        vertex_buffer.unmap();

        let output_bind_group_layout = create_output_bind_group_layout(&self.device);
        let output_uniform = create_output_uniform_buffer(&self.device);
        let output_bind_group =
            create_output_bind_group(&self.device, &output_bind_group_layout, &output_uniform);
        let surface_pipeline = create_unlit_pipeline(
            &self.device,
            surface.config.format,
            &output_bind_group_layout,
        );
        let vertex_count = (vertex_bytes.len() / VERTEX_BYTE_LEN) as u32;
        let stats = estimate_prepared_resource_stats(
            target,
            vertex_count as usize,
            true,
            0,
            None,
            0,
            false,
        );

        self.resources = Some(GpuPreparedResources {
            target,
            vertex_buffer,
            output_uniform,
            output_bind_group,
            surface_pipeline,
            vertex_count,
            webgl2_vertices,
            stats,
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn render_to_frame(
        &mut self,
        target: RasterTarget,
        exposure_ev: f32,
        frame: &mut Vec<u8>,
    ) -> Result<(bool, u64), RenderError> {
        let Some(resources) = self.resources.as_ref() else {
            frame.resize(target.byte_len(), 0);
            frame.fill(0);
            return Ok((false, 0));
        };
        if resources.target != target {
            return Err(RenderError::GpuResourcesNotPrepared {
                backend: target.backend,
            });
        }
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
        let culling_dispatches = if let Some(culling_pipeline) = &resources.culling_pipeline {
            culling::encode_culling_dispatch(
                &mut encoder,
                culling_pipeline,
                resources.culling_workgroups,
            );
            1
        } else {
            0
        };
        if let Some(depth_prepass) = &resources.depth_prepass {
            depth::encode_depth_prepass(
                &mut encoder,
                depth_prepass,
                &resources.vertex_buffer,
                resources.vertex_count,
            );
        }
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

        Ok((true, culling_dispatches))
    }

    pub(super) fn prepared_resource_stats(&self) -> GpuResourceStats {
        self.resources
            .as_ref()
            .map(|resources| resources.stats)
            .unwrap_or_default()
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn render_to_surface(
        &mut self,
        target: RasterTarget,
        exposure_ev: f32,
    ) -> Result<bool, RenderError> {
        let Some(resources) = self.resources.as_ref() else {
            return Err(RenderError::GpuResourcesNotPrepared {
                backend: target.backend,
            });
        };
        if resources.target != target {
            return Err(RenderError::GpuResourcesNotPrepared {
                backend: target.backend,
            });
        }
        let Some(surface) = self.surface.as_ref() else {
            return Err(RenderError::GpuResourcesNotPrepared {
                backend: target.backend,
            });
        };
        if target.backend == Backend::WebGl2 {
            let Some(canvas) = self.browser_canvas.as_ref() else {
                return Err(RenderError::GpuResourcesNotPrepared {
                    backend: target.backend,
                });
            };
            webgl2::render_canvas(canvas, &resources.webgl2_vertices).map_err(|_| {
                RenderError::GpuResourcesNotPrepared {
                    backend: target.backend,
                }
            })?;
            return Ok(true);
        }
        self.queue.write_buffer(
            &resources.output_uniform,
            0,
            &encode_output_uniform(exposure_ev),
        );
        let surface_output = match surface.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(output)
            | wgpu::CurrentSurfaceTexture::Suboptimal(output) => output,
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Outdated
            | wgpu::CurrentSurfaceTexture::Lost
            | wgpu::CurrentSurfaceTexture::Validation => return Ok(false),
        };
        let surface_view = surface_output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("scena.browser.encoder"),
            });
        encode_unlit_pass(
            &mut encoder,
            &surface_view,
            &resources.vertex_buffer,
            &resources.output_bind_group,
            resources.vertex_count,
            &resources.surface_pipeline,
            "scena.browser.surface_pass",
        );
        self.queue.submit(Some(encoder.finish()));
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        surface_output.present();
        Ok(true)
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
