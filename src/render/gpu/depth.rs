use super::super::RasterTarget;
use super::instancing::{INSTANCE_ATTRIBUTES, INSTANCE_BYTE_LEN, InstanceDrawBatch};
use super::output::DRAW_UNIFORM_ENTRY_STRIDE;
use super::vertices::{PrimitiveDrawBatch, VERTEX_ATTRIBUTES, VERTEX_BYTE_LEN};

const DEPTH_PREPASS_SHADER: &str = r#"
struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(6) instance_world_0: vec4<f32>,
    @location(7) instance_world_1: vec4<f32>,
    @location(8) instance_world_2: vec4<f32>,
    @location(9) instance_world_3: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
};

struct LightingUniform {
    directional_light_direction_intensity: array<vec4<f32>, 4>,
    directional_light_color: array<vec4<f32>, 4>,
    directional_shadow_control: array<vec4<f32>, 4>,
    point_light_position_intensity: array<vec4<f32>, 4>,
    point_light_color_range: array<vec4<f32>, 4>,
    spot_light_position_intensity: array<vec4<f32>, 4>,
    spot_light_direction_cones: array<vec4<f32>, 4>,
    spot_light_cone_range: array<vec4<f32>, 4>,
    spot_light_color_range: array<vec4<f32>, 4>,
    light_counts: vec4<f32>,
    environment_diffuse_intensity: vec4<f32>,
    environment_specular_intensity: vec4<f32>,
};

struct CameraUniform {
    view_from_world: mat4x4<f32>,
    clip_from_view: mat4x4<f32>,
    clip_from_world: mat4x4<f32>,
    light_from_world: mat4x4<f32>,
    camera_position_exposure: vec4<f32>,
    viewport_near_far: vec4<f32>,
    color_management: vec4<f32>,
    lighting: LightingUniform,
    clipping_planes: array<vec4<f32>, 6>,
    clipping_control: vec4<f32>,
};

struct DrawUniform {
    world_from_model: mat4x4<f32>,
    normal_from_model: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(2) @binding(0)
var<uniform> draw: DrawUniform;

fn clipped_by_scene(world_position: vec3<f32>) -> bool {
    let plane_count = i32(clamp(camera.clipping_control.x, 0.0, 6.0));
    if plane_count <= 0 {
        return false;
    }
    var rejected = false;
    var inside_all = true;
    for (var index = 0; index < 6; index = index + 1) {
        if index < plane_count {
            let plane = camera.clipping_planes[index];
            let inside = dot(plane.xyz, world_position) + plane.w >= 0.0;
            rejected = rejected || !inside;
            inside_all = inside_all && inside;
        }
    }
    let inverted_section = camera.clipping_control.y > 0.5 && camera.clipping_control.z > 0.5;
    if inverted_section {
        return inside_all;
    }
    return rejected;
}

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    // Use the same matrix multiplication path as the color pass so depth
    // values are bit-identical. On low-precision WebGL2 drivers (Pi 5 V3D),
    // computing `clip_from_view * view_from_world * world_from_model * pos`
    // here while the color pass computes `clip_from_view * view_from_world *
    // (world_from_model * pos)` makes most color-pass fragments fail the
    // LessEqual depth test by a single ULP, producing a mostly-black render.
    let instance_world_from_model = mat4x4<f32>(
        in.instance_world_0,
        in.instance_world_1,
        in.instance_world_2,
        in.instance_world_3,
    );
    let world_position = draw.world_from_model * instance_world_from_model * vec4<f32>(in.position, 1.0);
    var out: VertexOut;
    out.position = camera.clip_from_world * world_position;
    out.world_position = world_position.xyz;
    return out;
}

fn pack_depth(depth: f32) -> vec4<f32> {
    let scaled = clamp(depth, 0.0, 1.0) * 65535.0;
    let high = floor(scaled / 256.0);
    let low = scaled - high * 256.0;
    return vec4<f32>(high / 255.0, low / 255.0, 0.0, 1.0);
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    if clipped_by_scene(in.world_position) {
        discard;
    }
    return pack_depth(in.position.z);
}
"#;

const DEPTH_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

#[derive(Debug)]
pub(super) struct DepthPrepassResources {
    texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    color_texture: Option<wgpu::Texture>,
    color_view: Option<wgpu::TextureView>,
    pipeline: wgpu::RenderPipeline,
    clear_depth: f32,
    reversed_z: bool,
    pub(super) color_compare: wgpu::CompareFunction,
}

pub(super) fn create_depth_prepass_resources(
    device: &wgpu::Device,
    target: RasterTarget,
    reversed_z: bool,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    draw_bind_group_layout: &wgpu::BindGroupLayout,
    depth_color_enabled: bool,
) -> DepthPrepassResources {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scena.m2.depth_prepass"),
        size: wgpu::Extent3d {
            width: target.width,
            height: target.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let (color_texture, color_view) = if depth_color_enabled {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("scena.m2.depth_prepass_color"),
            size: wgpu::Extent3d {
                width: target.width,
                height: target.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_COLOR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (Some(texture), Some(view))
    } else {
        (None, None)
    };
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("scena.m2.depth_prepass_shader"),
        source: wgpu::ShaderSource::Wgsl(DEPTH_PREPASS_SHADER.into()),
    });
    // Depth prepass binds camera at @group(0) and draw uniform at @group(2)
    // — material bind group is unused but the pipeline layout matches the
    // unlit pipeline so the same vertex buffer + draw indices apply.
    let dummy_material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("scena.m2.depth_prepass_material_dummy"),
        entries: &[],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("scena.m2.depth_prepass_pipeline_layout"),
        bind_group_layouts: &[
            Some(camera_bind_group_layout),
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
    let color_compare = if reversed_z {
        wgpu::CompareFunction::GreaterEqual
    } else {
        wgpu::CompareFunction::LessEqual
    };
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("scena.m2.depth_prepass_pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
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
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(color_compare),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: depth_color_enabled.then_some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: DEPTH_COLOR_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });

    DepthPrepassResources {
        texture,
        view,
        color_texture,
        color_view,
        pipeline,
        clear_depth: if reversed_z { 0.0 } else { 1.0 },
        reversed_z,
        color_compare,
    }
}

impl DepthPrepassResources {
    pub(super) const fn clear_depth(&self) -> f32 {
        self.clear_depth
    }

    pub(super) const fn reversed_z(&self) -> bool {
        self.reversed_z
    }

    pub(super) fn color_view(&self) -> Option<&wgpu::TextureView> {
        self.color_view.as_ref()
    }

    pub(super) const fn depth_color_enabled(&self) -> bool {
        self.color_view.is_some()
    }
}

pub(super) fn encode_depth_prepass(
    encoder: &mut wgpu::CommandEncoder,
    resources: &DepthPrepassResources,
    inputs: DepthPrepassInputs<'_>,
) {
    let depth_attachment = Some(wgpu::RenderPassDepthStencilAttachment {
        view: &resources.view,
        depth_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Clear(resources.clear_depth),
            store: wgpu::StoreOp::Store,
        }),
        stencil_ops: None,
    });
    let color_attachment =
        resources
            .color_view
            .as_ref()
            .map(|view| wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(depth_clear_color(resources.clear_depth)),
                    store: wgpu::StoreOp::Store,
                },
            });
    let color_attachments = if color_attachment.is_some() {
        std::slice::from_ref(&color_attachment)
    } else {
        &[]
    };
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("scena.m2.depth_prepass"),
        color_attachments,
        depth_stencil_attachment: depth_attachment,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(&resources.pipeline);
    pass.set_bind_group(0, inputs.camera_bind_group, &[]);
    pass.set_vertex_buffer(0, inputs.vertex_buffer.slice(..));
    let identity_instance_offset =
        u64::from(inputs.identity_instance).saturating_mul(INSTANCE_BYTE_LEN as u64);
    pass.set_vertex_buffer(1, inputs.instance_buffer.slice(identity_instance_offset..));
    for batch in inputs.draw_batches {
        if !batch.depth_prepass_eligible {
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
        if !batch.depth_prepass_eligible {
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
}

pub(super) struct DepthPrepassInputs<'a> {
    pub(super) vertex_buffer: &'a wgpu::Buffer,
    pub(super) instance_buffer: &'a wgpu::Buffer,
    pub(super) camera_bind_group: &'a wgpu::BindGroup,
    pub(super) draw_bind_group: &'a wgpu::BindGroup,
    pub(super) draw_batches: &'a [PrimitiveDrawBatch],
    pub(super) instance_batches: &'a [InstanceDrawBatch],
    pub(super) identity_instance: u32,
    pub(super) draw_submissions: &'a mut u64,
}

fn depth_clear_color(clear_depth: f32) -> wgpu::Color {
    let scaled = clear_depth.clamp(0.0, 1.0) * 65_535.0;
    let high = (scaled / 256.0).floor();
    let low = scaled - high * 256.0;
    wgpu::Color {
        r: f64::from(high / 255.0),
        g: f64::from(low / 255.0),
        b: 0.0,
        a: 1.0,
    }
}

impl Drop for DepthPrepassResources {
    fn drop(&mut self) {
        let _ = &self.texture;
        let _ = &self.color_texture;
    }
}
