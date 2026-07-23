use crate::diagnostics::RenderError;
use crate::render::RasterTarget;

use super::capture::decode_capture;
use super::{SemanticAovResources, encode_pass, write_camera_uniform};
use crate::render::camera::CameraProjection;
use crate::render::gpu::shader_manifest::{ShaderVariantId, create_shader_module};
use crate::scene::{ClippingPlane, SectionBox};

#[derive(Debug)]
pub(super) struct WebGl2ReadbackResources {
    pipeline: wgpu::RenderPipeline,
    bind_groups: [wgpu::BindGroup; 3],
}

pub(super) fn create_resources(
    device: &wgpu::Device,
    surface_format: wgpu::TextureFormat,
    source_views: [&wgpu::TextureView; 3],
) -> WebGl2ReadbackResources {
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("scena.semantic_aov.webgl2_surface_bind_group_layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        }],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("scena.semantic_aov.webgl2_surface_pipeline_layout"),
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });
    let shader = create_shader_module(
        device,
        ShaderVariantId::SemanticAovWebgl2Readback,
        "scena.semantic_aov.webgl2_surface_shader",
    );
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("scena.semantic_aov.webgl2_surface_pipeline"),
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
                format: surface_format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });
    let bind_groups = source_views.map(|view| {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scena.semantic_aov.webgl2_surface_bind_group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(view),
            }],
        })
    });
    WebGl2ReadbackResources {
        pipeline,
        bind_groups,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn capture(
    state: &super::super::GpuDeviceState,
    resources: &super::super::GpuPreparedResources,
    semantic: &SemanticAovResources,
    target: RasterTarget,
    projection: &CameraProjection,
    clipping_planes: &[ClippingPlane],
    section_box: Option<SectionBox>,
) -> Result<crate::render::semantic_aov::RawSemanticAovCapture, RenderError> {
    let surface = state
        .surface
        .as_ref()
        .ok_or(RenderError::GpuResourcesNotPrepared {
            backend: target.backend,
        })?;
    let canvas = state
        .browser_canvas
        .as_ref()
        .ok_or(RenderError::GpuReadback {
            backend: target.backend,
        })?;
    let readback =
        semantic
            .webgl2_readback
            .as_ref()
            .ok_or(RenderError::GpuResourcesNotPrepared {
                backend: target.backend,
            })?;

    write_camera_uniform(
        &state.queue,
        resources,
        projection,
        target,
        clipping_planes,
        section_box,
    );
    let mut semantic_encoder =
        state
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("scena.semantic_aov.webgl2_encoder"),
            });
    encode_pass(&mut semantic_encoder, resources, semantic);
    state.queue.submit(Some(semantic_encoder.finish()));

    let mut frames = [Vec::new(), Vec::new(), Vec::new()];
    for (slot, frame) in frames.iter_mut().enumerate() {
        let surface_output = match surface.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(output)
            | wgpu::CurrentSurfaceTexture::Suboptimal(output) => output,
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Outdated
            | wgpu::CurrentSurfaceTexture::Lost
            | wgpu::CurrentSurfaceTexture::Validation => {
                return Err(RenderError::GpuReadback {
                    backend: target.backend,
                });
            }
        };
        let surface_view = surface_output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = state
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("scena.semantic_aov.webgl2_surface_encoder"),
            });
        encode_blit(
            &mut encoder,
            &readback.pipeline,
            &readback.bind_groups[slot],
            &surface_view,
        );
        state.queue.submit(Some(encoder.finish()));
        surface_output.present();
        *frame = super::super::browser_readback::read_webgl2_canvas_rgba8(canvas, target).map_err(
            |_| RenderError::GpuReadback {
                backend: target.backend,
            },
        )?;
    }
    Ok(decode_capture(semantic, projection.near_far(), frames))
}

fn encode_blit(
    encoder: &mut wgpu::CommandEncoder,
    pipeline: &wgpu::RenderPipeline,
    bind_group: &wgpu::BindGroup,
    surface_view: &wgpu::TextureView,
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("scena.semantic_aov.webgl2_surface_pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: surface_view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, bind_group, &[]);
    pass.draw(0..3, 0..1);
}
