use super::instancing::{INSTANCE_ATTRIBUTES, INSTANCE_BYTE_LEN, InstanceDrawBatch};
use super::output::DRAW_UNIFORM_ENTRY_STRIDE;
use super::vertices::{PrimitiveDrawBatch, VERTEX_ATTRIBUTES, VERTEX_BYTE_LEN};

/// Comparison sampler for the directional shadow map. The fragment shader
/// averages an explicit 3×3 texel grid, so nearest filtering keeps each of its
/// nine `textureSampleCompareLevel` calls to one comparison instead of silently
/// widening every grid point into another 2×2 footprint. Address mode
/// `ClampToEdge` is sentinel-safe — the fragment shader gates the sample on the
/// receiver's NDC frustum (review F6) so border reads never produce false
/// self-shadow streaks.
pub(super) fn create_shadow_sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("scena.m2.shadow_sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        compare: Some(wgpu::CompareFunction::LessEqual),
        ..Default::default()
    })
}

/// Allocates the directional shadow map. Always returns Some — when no
/// shadow-casting directional light is in the scene, a 1×1 placeholder is
/// returned so the fragment shader's depth-comparison sampler binding is
/// always valid. The shader checks
/// `light_counts.x > 0.0` before sampling, so
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
    @location(6) instance_world_0: vec4<f32>,
    @location(7) instance_world_1: vec4<f32>,
    @location(8) instance_world_2: vec4<f32>,
    @location(9) instance_world_3: vec4<f32>,
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
    let instance_world_from_model = mat4x4<f32>(
        in.instance_world_0,
        in.instance_world_1,
        in.instance_world_2,
        in.instance_world_3,
    );
    return camera.light_from_world * draw.world_from_model * instance_world_from_model * vec4<f32>(in.position, 1.0);
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
    /// Camera-only bind group used by the shadow caster pass. Distinct from
    /// `output_bind_group` so that the shadow_map texture is not referenced
    /// inside the caster pass's render-pass usage scope.
    pub(super) camera_bind_group: wgpu::BindGroup,
    /// Empty bind group for slot @group(1) (the material slot) so the
    /// caster's pipeline layout aligns with the unlit pipeline's bind
    /// group indices.
    pub(super) dummy_material_group: wgpu::BindGroup,
}

pub(super) fn create_shadow_caster_resources(
    device: &wgpu::Device,
    resolution: Option<u32>,
    output_uniform: &wgpu::Buffer,
    draw_bind_group_layout: &wgpu::BindGroupLayout,
) -> ShadowCasterResources {
    let texture = create_shadow_texture(device, resolution);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("scena.m2.shadow_caster_shader"),
        source: wgpu::ShaderSource::Wgsl(SHADOW_CASTER_SHADER.into()),
    });
    // Shadow caster needs only the camera uniform. We must NOT bind the
    // unlit pass's `output_bind_group` here because that group also
    // references the shadow_map (as a resource); binding it inside the
    // shadow caster render pass — which uses the same shadow_map as a
    // depth-stencil write target — triggers wgpu's
    // `TextureUses(DEPTH_STENCIL_WRITE) is an exclusive usage` validation
    // error. Allocate a dedicated camera-only bind group instead. Test:
    // `shadow_casting_light_with_multiple_meshes_renders_without_validation_error`.
    let camera_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scena.m2.shadow_caster_camera_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
    let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("scena.m2.shadow_caster_camera_bind_group"),
        layout: &camera_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: output_uniform.as_entire_binding(),
        }],
    });
    // Empty material bind group keeps @group(1) slot pinned to match the
    // shadow caster's pipeline layout indices with the unlit pipeline
    // (so the shared vertex buffer + draw bind group bind cleanly).
    let dummy_material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("scena.m2.shadow_caster_material_dummy"),
        entries: &[],
    });
    let dummy_material_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("scena.m2.shadow_caster_material_dummy_group"),
        layout: &dummy_material_layout,
        entries: &[],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("scena.m2.shadow_caster_pipeline_layout"),
        bind_group_layouts: &[
            Some(&camera_bind_group_layout),
            Some(&dummy_material_layout),
            Some(draw_bind_group_layout),
        ],
        immediate_size: 0,
    });
    let pipeline = create_shadow_pipeline(device, &pipeline_layout, &shader);
    ShadowCasterResources {
        texture,
        view,
        pipeline,
        active: resolution.is_some(),
        camera_bind_group,
        dummy_material_group,
    }
}

fn create_shadow_pipeline(
    device: &wgpu::Device,
    pipeline_layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
) -> wgpu::RenderPipeline {
    let vertex_buffer = wgpu::VertexBufferLayout {
        array_stride: VERTEX_BYTE_LEN as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &VERTEX_ATTRIBUTES,
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("scena.m2.shadow_caster_pipeline"),
        layout: Some(pipeline_layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[
                vertex_buffer,
                wgpu::VertexBufferLayout {
                    array_stride: INSTANCE_BYTE_LEN as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &INSTANCE_ATTRIBUTES,
                },
            ],
        },
        // Shadow occluders match the CPU shadow path, which treats authored
        // triangles as occluders from either side. Visible mesh sidedness is
        // still enforced by the color/depth pipelines.
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: wgpu::StencilState::default(),
            // Constant + slope depth bias to combat shadow acne on grazing
            // angles. Values match a standard ortho shadow map at
            // 1024-2048 resolution; tuning lives next to the shader.
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
    })
}

pub(super) fn encode_shadow_caster_pass(
    encoder: &mut wgpu::CommandEncoder,
    resources: &ShadowCasterResources,
    inputs: ShadowCasterPassInputs<'_>,
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
    // Bind the caster-private camera bind group at @group(0) so the
    // render-pass usage scope never references the shadow_map texture.
    // The output bind group (which references the shadow_map) only enters
    // the encoder during the later unlit pass, after the caster pass
    // closes and the texture transitions from DEPTH_STENCIL_WRITE to
    // RESOURCE.
    pass.set_bind_group(0, &resources.camera_bind_group, &[]);
    pass.set_bind_group(1, &resources.dummy_material_group, &[]);
    pass.set_vertex_buffer(0, inputs.vertex_buffer.slice(..));
    let identity_instance_offset =
        u64::from(inputs.identity_instance).saturating_mul(INSTANCE_BYTE_LEN as u64);
    pass.set_vertex_buffer(1, inputs.instance_buffer.slice(identity_instance_offset..));
    pass.set_pipeline(&resources.pipeline);
    for batch in inputs.draw_batches {
        let draw_offset =
            (batch.draw_uniform_index as u64).saturating_mul(DRAW_UNIFORM_ENTRY_STRIDE) as u32;
        pass.set_bind_group(2, inputs.draw_bind_group, &[draw_offset]);
        pass.draw(
            batch.start_vertex..batch.start_vertex.saturating_add(batch.vertex_count),
            0..1,
        );
        *inputs.draw_submissions = inputs.draw_submissions.saturating_add(1);
    }
    for batch in inputs.instance_batches {
        let draw_offset =
            (batch.draw_uniform_index as u64).saturating_mul(DRAW_UNIFORM_ENTRY_STRIDE) as u32;
        pass.set_bind_group(2, inputs.draw_bind_group, &[draw_offset]);
        let instance_offset =
            u64::from(batch.start_instance).saturating_mul(INSTANCE_BYTE_LEN as u64);
        pass.set_vertex_buffer(1, inputs.instance_buffer.slice(instance_offset..));
        pass.draw(
            batch.start_vertex..batch.start_vertex.saturating_add(batch.vertex_count),
            0..batch.instance_count,
        );
        *inputs.draw_submissions = inputs.draw_submissions.saturating_add(1);
    }
}

pub(super) struct ShadowCasterPassInputs<'a> {
    pub(super) vertex_buffer: &'a wgpu::Buffer,
    pub(super) instance_buffer: &'a wgpu::Buffer,
    pub(super) draw_bind_group: &'a wgpu::BindGroup,
    pub(super) draw_batches: &'a [PrimitiveDrawBatch],
    pub(super) instance_batches: &'a [InstanceDrawBatch],
    pub(super) identity_instance: u32,
    pub(super) draw_submissions: &'a mut u64,
}
