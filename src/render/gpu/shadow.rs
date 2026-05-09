use super::output::DRAW_UNIFORM_ENTRY_STRIDE;
use super::vertices::{PrimitiveDrawBatch, VERTEX_ATTRIBUTES, VERTEX_BYTE_LEN};

/// Comparison sampler for the directional shadow map. Linear filtering with
/// `CompareFunction::LessEqual` runs the hardware percentage-closer filter,
/// turning each `textureSampleCompareLevel` call into a 2×2 PCF tap. Address
/// mode `ClampToEdge` is sentinel-safe — the fragment shader gates the sample
/// on the receiver's NDC frustum (review F6) so border reads never produce
/// false self-shadow streaks.
pub(super) fn create_shadow_sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("scena.m2.shadow_sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        compare: Some(wgpu::CompareFunction::LessEqual),
        ..Default::default()
    })
}

/// Allocates the directional shadow map. Always returns Some — when no
/// shadow-casting directional light is in the scene, a 1×1 placeholder is
/// returned so the fragment shader's depth-comparison sampler binding is
/// always valid. The shader checks
/// `directional_light_direction_intensity.w > 0.0` before sampling, so
/// the placeholder is never read in practice.
pub(super) fn create_shadow_texture(
    device: &wgpu::Device,
    resolution: Option<u32>,
) -> wgpu::Texture {
    let size = resolution.unwrap_or(1).max(1);
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scena.m2.directional_shadow_map"),
        size: wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

/// Phase 1B step 2: WGSL shadow caster shader. Vertex-only, depth-only.
/// Uses `camera.light_from_world * draw.world_from_model * position` to
/// project model-space vertices into light-clip space and writes depth.
pub(super) const SHADOW_CASTER_SHADER: &str = r#"
struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct CameraUniform {
    view_from_world: mat4x4<f32>,
    clip_from_view: mat4x4<f32>,
    clip_from_world: mat4x4<f32>,
    light_from_world: mat4x4<f32>,
    camera_position_exposure: vec4<f32>,
    viewport_near_far: vec4<f32>,
    color_management: vec4<f32>,
    light_block_padding_0: vec4<f32>,
    light_block_padding_1: vec4<f32>,
    light_block_padding_2: vec4<f32>,
    light_block_padding_3: vec4<f32>,
    light_block_padding_4: vec4<f32>,
    light_block_padding_5: vec4<f32>,
    light_block_padding_6: vec4<f32>,
    light_block_padding_7: vec4<f32>,
    light_block_padding_8: vec4<f32>,
    light_block_padding_9: vec4<f32>,
};

struct DrawUniform {
    world_from_model: mat4x4<f32>,
    normal_from_model: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(2) @binding(0)
var<uniform> draw: DrawUniform;

@vertex
fn vs_main(in: VertexIn) -> @builtin(position) vec4<f32> {
    return camera.light_from_world * draw.world_from_model * vec4<f32>(in.position, 1.0);
}
"#;

#[derive(Debug)]
pub(super) struct ShadowCasterResources {
    /// Owned to keep the GPU texture alive while `view` references it.
    #[allow(dead_code)]
    pub(super) texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    pub(super) pipeline: wgpu::RenderPipeline,
    pub(super) active: bool,
}

pub(super) fn create_shadow_caster_resources(
    device: &wgpu::Device,
    resolution: Option<u32>,
    output_bind_group_layout: &wgpu::BindGroupLayout,
    draw_bind_group_layout: &wgpu::BindGroupLayout,
) -> ShadowCasterResources {
    let texture = create_shadow_texture(device, resolution);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("scena.m2.shadow_caster_shader"),
        source: wgpu::ShaderSource::Wgsl(SHADOW_CASTER_SHADER.into()),
    });
    // Caster pipeline reuses the output + draw bind group layouts. The
    // material bind group at @group(1) is unused but must be in the layout
    // to keep the pipeline-layout indices aligned with the unlit pipeline so
    // the same vertex buffer + draw bind group can be bound without re-
    // binding camera/draw on the unlit pass.
    let dummy_material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("scena.m2.shadow_caster_material_dummy"),
        entries: &[],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("scena.m2.shadow_caster_pipeline_layout"),
        bind_group_layouts: &[
            Some(output_bind_group_layout),
            Some(&dummy_material_layout),
            Some(draw_bind_group_layout),
        ],
        immediate_size: 0,
    });
    let vertex_buffer = wgpu::VertexBufferLayout {
        array_stride: VERTEX_BYTE_LEN as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &VERTEX_ATTRIBUTES,
    };
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("scena.m2.shadow_caster_pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[vertex_buffer],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: wgpu::StencilState::default(),
            // Constant + slope depth bias to combat shadow acne on grazing
            // angles. Values match a standard ortho shadow map at
            // 1024–2048 resolution; tuning lives next to the shader.
            bias: wgpu::DepthBiasState {
                constant: 2,
                slope_scale: 1.5,
                clamp: 0.0,
            },
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: None,
        multiview_mask: None,
        cache: None,
    });
    ShadowCasterResources {
        texture,
        view,
        pipeline,
        active: resolution.is_some(),
    }
}

pub(super) fn encode_shadow_caster_pass(
    encoder: &mut wgpu::CommandEncoder,
    resources: &ShadowCasterResources,
    vertex_buffer: &wgpu::Buffer,
    output_bind_group: &wgpu::BindGroup,
    draw_bind_group: &wgpu::BindGroup,
    draw_batches: &[PrimitiveDrawBatch],
) {
    if !resources.active {
        return;
    }
    let depth_attachment = Some(wgpu::RenderPassDepthStencilAttachment {
        view: &resources.view,
        depth_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Clear(1.0),
            store: wgpu::StoreOp::Store,
        }),
        stencil_ops: None,
    });
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("scena.m2.shadow_caster_pass"),
        color_attachments: &[],
        depth_stencil_attachment: depth_attachment,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(&resources.pipeline);
    pass.set_bind_group(0, output_bind_group, &[]);
    pass.set_vertex_buffer(0, vertex_buffer.slice(..));
    for batch in draw_batches {
        let draw_offset =
            (batch.draw_uniform_index as u64).saturating_mul(DRAW_UNIFORM_ENTRY_STRIDE) as u32;
        pass.set_bind_group(2, draw_bind_group, &[draw_offset]);
        pass.draw(
            batch.start_vertex..batch.start_vertex.saturating_add(batch.vertex_count),
            0..1,
        );
    }
}
