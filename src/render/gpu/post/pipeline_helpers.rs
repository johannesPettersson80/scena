use super::*;

pub(in crate::render::gpu) fn surface_blit_pipeline(
    resources: &PostResources,
) -> Option<&wgpu::RenderPipeline> {
    resources.surface_blit_pipeline.as_ref()
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(in crate::render::gpu) const fn output_blit_pipeline(
    resources: &PostResources,
) -> &wgpu::RenderPipeline {
    &resources.output_blit_pipeline
}

#[allow(dead_code)]
pub(in crate::render::gpu) fn readback_blit_pipeline(
    resources: &PostResources,
) -> Option<&wgpu::RenderPipeline> {
    resources.readback_blit_pipeline.as_ref()
}

#[allow(dead_code)]
pub(in crate::render::gpu) fn surface_fxaa_pipeline(
    resources: &PostResources,
) -> Option<&wgpu::RenderPipeline> {
    resources.surface_fxaa_pipeline.as_ref()
}

#[allow(dead_code)]
pub(in crate::render::gpu) fn surface_bloom_fxaa_pipeline(
    resources: &PostResources,
) -> Option<&wgpu::RenderPipeline> {
    resources.surface_bloom_fxaa_pipeline.as_ref()
}

pub(in crate::render::gpu) fn create_post_pipeline(
    device: &wgpu::Device,
    label: &'static str,
    shader_variant: ShaderVariantId,
    pipeline_layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = create_post_shader(device, label, shader_variant);
    create_post_pipeline_with_shader(device, label, &shader, pipeline_layout, format)
}

pub(in crate::render::gpu) fn create_post_shader(
    device: &wgpu::Device,
    label: &'static str,
    shader_variant: ShaderVariantId,
) -> wgpu::ShaderModule {
    create_shader_module(device, shader_variant, label)
}

pub(in crate::render::gpu) fn create_post_pipeline_with_shader(
    device: &wgpu::Device,
    label: &'static str,
    shader: &wgpu::ShaderModule,
    pipeline_layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(pipeline_layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
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

pub(super) fn write_uniform(
    encoder: &mut wgpu::CommandEncoder,
    queue: &wgpu::Queue,
    resources: &PostResources,
    slot: PostUniformSlot,
    values: [f32; 12],
) {
    let offset = slot.byte_offset();
    queue.write_buffer(
        &resources.uniform_staging,
        offset,
        bytemuck::cast_slice(&values),
    );
    encoder.copy_buffer_to_buffer(
        &resources.uniform_staging,
        offset,
        &resources.uniform,
        0,
        POST_UNIFORM_BYTE_LEN,
    );
}

pub(super) fn view(resources: &PostResources, slot: PostTextureSlot) -> &wgpu::TextureView {
    match slot {
        PostTextureSlot::Scene => &resources.scene_view,
        PostTextureSlot::Ping => &resources.ping_view,
        PostTextureSlot::Pong => &resources.pong_view,
    }
}

pub(super) fn bind_group(resources: &PostResources, slot: PostTextureSlot) -> &wgpu::BindGroup {
    match slot {
        PostTextureSlot::Scene => &resources.texture_bind_groups[0],
        PostTextureSlot::Ping => &resources.texture_bind_groups[1],
        PostTextureSlot::Pong => &resources.texture_bind_groups[2],
    }
}

pub(super) fn depth_bind_group(
    resources: &PostResources,
    slot: PostTextureSlot,
) -> Result<&wgpu::BindGroup, RenderError> {
    let groups = resources.depth_texture_bind_groups.as_ref().ok_or(
        RenderError::GpuResourcesNotPrepared {
            backend: resources.target.backend,
        },
    )?;
    Ok(match slot {
        PostTextureSlot::Scene => &groups[0],
        PostTextureSlot::Ping => &groups[1],
        PostTextureSlot::Pong => &groups[2],
    })
}
