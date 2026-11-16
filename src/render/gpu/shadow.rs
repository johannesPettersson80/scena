use super::alpha_mask::{self, AlphaMaskPipelineInputs};
use super::instancing::{INSTANCE_ATTRIBUTES, INSTANCE_BYTE_LEN, InstanceDrawBatch};
use super::materials::MaterialResources;
use super::output::DRAW_UNIFORM_ENTRY_STRIDE;
use super::shader_manifest::{ShaderVariantId, create_shader_module};
use super::shadow_cascades::ShadowCascadeUniform;
use super::vertices::{PrimitiveDrawBatch, VERTEX_ATTRIBUTES, VERTEX_BYTE_LEN};
use crate::render::camera::CameraProjection;

/// Comparison sampler for the directional shadow map. Linear filtering makes
/// every `textureSampleCompareLevel` a hardware 2x2 percentage-closer lookup,
/// so the shader's taps blend into a smooth footprint instead of stepping at
/// texel boundaries. Address mode `ClampToEdge` is sentinel-safe: the fragment
/// shader gates the sample on the receiver's NDC frustum so border reads never
/// produce false self-shadow streaks.
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
/// `light_counts.x > 0.0` before sampling, so
/// the placeholder is never read in practice.
pub(super) fn create_shadow_texture(
    device: &wgpu::Device,
    resolution: Option<u32>,
    cascade_count: usize,
) -> wgpu::Texture {
    let size = resolution.unwrap_or(1).max(1) * atlas_cells_per_edge(cascade_count);
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
    white_balance: vec4<f32>,
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

/// Cascades used by the native renderer. Browser backends keep one cascade:
/// WebGL2 runs the single-map shader and the WebGPU surface path has not been
/// taught to refit cascades per frame.
#[cfg(not(target_arch = "wasm32"))]
pub(super) const NATIVE_SHADOW_CASCADE_COUNT: usize = 4;

/// Byte offset of `CameraUniform::light_from_world`, the only field the caster
/// vertex shader reads.
const LIGHT_FROM_WORLD_UNIFORM_OFFSET: u64 = 48 * 4;

/// Constant + slope depth bias against shadow acne at grazing angles. The
/// receiver side adds a normal offset, so these stay small. Shared with the
/// alpha-tested caster so masked and opaque occluders agree where they meet.
const SHADOW_CASTER_DEPTH_BIAS: wgpu::DepthBiasState = wgpu::DepthBiasState {
    constant: 2,
    slope_scale: 1.5,
    clamp: 0.0,
};

/// Cascades share one square atlas: one cell for a single cascade, 2x2 beyond.
const fn atlas_cells_per_edge(cascade_count: usize) -> u32 {
    if cascade_count > 1 { 2 } else { 1 }
}

#[derive(Debug)]
pub(super) struct ShadowCasterResources {
    /// Owned to keep the GPU texture alive while `view` references it.
    #[allow(dead_code)]
    pub(super) texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    pub(super) pipeline: Option<wgpu::RenderPipeline>,
    /// Alpha-tested caster for glTF MASK batches, so a cut-out casts the shadow
    /// of its visible texels instead of a solid quad.
    masked_pipeline: Option<wgpu::RenderPipeline>,
    /// Camera-only bind group used by the shadow caster pass. Distinct from
    /// `output_bind_group` so that the shadow_map texture is not referenced
    /// inside the caster pass's render-pass usage scope.
    pub(super) camera_bind_group: Option<wgpu::BindGroup>,
    /// Empty bind group for slot @group(1) (the material slot) so the
    /// caster's pipeline layout aligns with the unlit pipeline's bind
    /// group indices.
    pub(super) dummy_material_group: Option<wgpu::BindGroup>,
    /// One camera uniform and bind group per cascade when the map is an
    /// atlas. Empty for a single cascade, which reads the frame's own camera
    /// uniform through `camera_bind_group`.
    cascade_uniforms: Vec<wgpu::Buffer>,
    cascade_bind_groups: Vec<wgpu::BindGroup>,
    /// Edge length of one cascade in texels.
    cascade_resolution: u32,
}

impl ShadowCasterResources {
    /// Fits this frame's cascades and uploads them for the caster pass.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub(super) fn update_cascades(
        &self,
        queue: &wgpu::Queue,
        camera: &CameraProjection,
        light_from_world: [f32; 16],
    ) -> ShadowCascadeUniform {
        let cascades = self.fit_cascades(camera, light_from_world);
        self.write_cascades(queue, &cascades);
        cascades
    }

    /// Fits this frame's cascades. Scenes without a shadow-casting light, and
    /// single-cascade maps, keep the scene-fitted matrix unchanged.
    fn fit_cascades(
        &self,
        camera: &CameraProjection,
        light_from_world: [f32; 16],
    ) -> ShadowCascadeUniform {
        if self.cascade_bind_groups.is_empty() {
            return ShadowCascadeUniform::single(light_from_world);
        }
        ShadowCascadeUniform::fit(
            camera,
            light_from_world,
            self.cascade_bind_groups.len(),
            self.cascade_resolution,
        )
    }

    /// Uploads each cascade matrix into the uniform its caster draws read.
    fn write_cascades(&self, queue: &wgpu::Queue, cascades: &ShadowCascadeUniform) {
        for (uniform, matrix) in self.cascade_uniforms.iter().zip(&cascades.light_from_world) {
            let mut bytes = [0_u8; 64];
            for (chunk, value) in bytes.chunks_exact_mut(4).zip(matrix) {
                chunk.copy_from_slice(&value.to_ne_bytes());
            }
            queue.write_buffer(uniform, LIGHT_FROM_WORLD_UNIFORM_OFFSET, &bytes);
        }
    }
}

pub(super) fn create_shadow_caster_resources(
    device: &wgpu::Device,
    resolution: Option<u32>,
    output_uniform: &wgpu::Buffer,
    draw_bind_group_layout: &wgpu::BindGroupLayout,
    cascade_count: usize,
    alpha_mask: Option<AlphaMaskPipelineInputs<'_>>,
) -> ShadowCasterResources {
    let texture = create_shadow_texture(device, resolution, cascade_count);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    if resolution.is_none() {
        return ShadowCasterResources {
            texture,
            view,
            pipeline: None,
            masked_pipeline: None,
            camera_bind_group: None,
            dummy_material_group: None,
            cascade_uniforms: Vec::new(),
            cascade_bind_groups: Vec::new(),
            cascade_resolution: 1,
        };
    }
    let shader = create_shader_module(
        device,
        ShaderVariantId::ShadowDirectional,
        "scena.m2.shadow_caster_shader",
    );
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
    let masked_pipeline = alpha_mask.map(|inputs| {
        alpha_mask::create_shadow_pipeline(
            device,
            inputs,
            &camera_bind_group_layout,
            draw_bind_group_layout,
            SHADOW_CASTER_DEPTH_BIAS,
        )
    });
    let (cascade_uniforms, cascade_bind_groups): (Vec<_>, Vec<_>) = (0..cascade_count)
        .filter(|_| cascade_count > 1)
        .map(|_| {
            let uniform = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("scena.m2.shadow_cascade_uniform"),
                size: output_uniform.size(),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("scena.m2.shadow_cascade_bind_group"),
                layout: &camera_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                }],
            });
            (uniform, bind_group)
        })
        .unzip();
    ShadowCasterResources {
        texture,
        view,
        pipeline: Some(pipeline),
        masked_pipeline,
        camera_bind_group: Some(camera_bind_group),
        dummy_material_group: Some(dummy_material_group),
        cascade_uniforms,
        cascade_bind_groups,
        cascade_resolution: resolution.unwrap_or(1).max(1),
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
            bias: SHADOW_CASTER_DEPTH_BIAS,
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
    let (Some(pipeline), Some(camera_bind_group), Some(dummy_material_group)) = (
        resources.pipeline.as_ref(),
        resources.camera_bind_group.as_ref(),
        resources.dummy_material_group.as_ref(),
    ) else {
        return;
    };
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
    pass.set_vertex_buffer(0, inputs.vertex_buffer.slice(..));
    let identity_instance_offset =
        u64::from(inputs.identity_instance).saturating_mul(INSTANCE_BYTE_LEN as u64);
    // Masked batches go through the alpha-tested pipeline when it exists;
    // without it they stay solid occluders rather than losing their shadow.
    let alpha_test = resources.masked_pipeline.is_some();
    let cascade_count = resources.cascade_bind_groups.len().max(1);
    let cell = resources.cascade_resolution;
    for cascade in 0..cascade_count {
        // The masked pipeline uses a different layout at @group(1), so the
        // plain pipeline and its empty material group are rebound per cascade.
        pass.set_pipeline(pipeline);
        pass.set_bind_group(1, dummy_material_group, &[]);
        // A single cascade reads the frame's camera uniform and fills the
        // whole map; atlas cascades bind their own matrix and draw into one
        // cell each.
        match resources.cascade_bind_groups.get(cascade) {
            Some(bind_group) => {
                let x = (cascade as u32 % 2) * cell;
                let y = (cascade as u32 / 2) * cell;
                pass.set_viewport(x as f32, y as f32, cell as f32, cell as f32, 0.0, 1.0);
                pass.set_scissor_rect(x, y, cell, cell);
                pass.set_bind_group(0, bind_group, &[]);
            }
            None => pass.set_bind_group(0, camera_bind_group, &[]),
        }
        pass.set_vertex_buffer(1, inputs.instance_buffer.slice(identity_instance_offset..));
        for batch in inputs.draw_batches {
            if alpha_test && batch.alpha_masked {
                continue;
            }
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
            if alpha_test && batch.alpha_masked {
                continue;
            }
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
        if let Some(masked_pipeline) = &resources.masked_pipeline {
            alpha_mask::encode_shadow_draws(
                &mut pass,
                masked_pipeline,
                &alpha_mask::MaskedDraws {
                    materials: inputs.material_resources,
                    instance_buffer: inputs.instance_buffer,
                    draw_bind_group: inputs.draw_bind_group,
                    draw_batches: inputs.draw_batches,
                    instance_batches: inputs.instance_batches,
                    identity_instance: inputs.identity_instance,
                },
                inputs.draw_submissions,
            );
        }
    }
}

pub(super) struct ShadowCasterPassInputs<'a> {
    pub(super) vertex_buffer: &'a wgpu::Buffer,
    pub(super) instance_buffer: &'a wgpu::Buffer,
    pub(super) draw_bind_group: &'a wgpu::BindGroup,
    pub(super) material_resources: &'a MaterialResources,
    pub(super) draw_batches: &'a [PrimitiveDrawBatch],
    pub(super) instance_batches: &'a [InstanceDrawBatch],
    pub(super) identity_instance: u32,
    pub(super) draw_submissions: &'a mut u64,
}

impl<'a> ShadowCasterPassInputs<'a> {
    /// Inputs that cast every batch of the prepared frame.
    pub(super) fn for_resources(
        resources: &'a super::GpuPreparedResources,
        draw_submissions: &'a mut u64,
    ) -> Self {
        Self {
            vertex_buffer: &resources.vertex_buffer,
            instance_buffer: &resources.instance_buffer,
            draw_bind_group: &resources.draw_bind_group,
            material_resources: &resources.material_resources,
            draw_batches: &resources.draw_batches,
            instance_batches: &resources.instance_batches,
            identity_instance: resources.identity_instance,
            draw_submissions,
        }
    }
}
