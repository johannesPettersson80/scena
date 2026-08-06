use crate::render::semantic_aov::{
    GpuSemanticAttribution, RawSemanticAovExclusions, RawSemanticLegendEntry,
};

use super::GpuPreparedResources;
use super::instancing::{INSTANCE_ATTRIBUTES, INSTANCE_BYTE_LEN, InstanceDrawBatch};
use super::material_uniform::MATERIAL_UNIFORM_ENTRY_STRIDE;
use super::materials::MaterialResources;
use super::output::DRAW_UNIFORM_ENTRY_STRIDE;
use super::pipeline::SCENA_FRONT_FACE;
use super::stats::GpuResourceStats;
use super::vertices::{VERTEX_ATTRIBUTES, VERTEX_BYTE_LEN};
use crate::render::RasterTarget;

mod beauty;
mod capture;
#[cfg(target_arch = "wasm32")]
mod webgl2;

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(super) const WEBGL2_READBACK_SHADER: &str = concat!(
    include_str!("post/blit_srgb.wgsl"),
    "\n",
    include_str!("../color_contract.wgsl")
);

pub(super) const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const BYTES_PER_PIXEL: u32 = 4;

#[derive(Debug)]
struct SemanticTarget {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    readback: wgpu::Buffer,
}

#[derive(Debug)]
struct SemanticPipelines {
    single_sided: wgpu::RenderPipeline,
    double_sided: wgpu::RenderPipeline,
}

#[derive(Debug)]
pub(super) struct SemanticAovResources {
    target: RasterTarget,
    targets: [SemanticTarget; 3],
    #[allow(dead_code)]
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    reversed_z: bool,
    pipelines: SemanticPipelines,
    beauty: beauty::Target,
    padded_bytes_per_row: u32,
    unpadded_bytes_per_row: u32,
    legend: Vec<RawSemanticLegendEntry>,
    exclusions: RawSemanticAovExclusions,
    #[cfg(target_arch = "wasm32")]
    webgl2_readback: Option<webgl2::WebGl2ReadbackResources>,
}

pub(super) struct SemanticAovResourceDescriptor<'a> {
    pub(super) target: RasterTarget,
    pub(super) beauty_target: RasterTarget,
    pub(super) beauty_sample_count: u32,
    pub(super) output_layout: &'a wgpu::BindGroupLayout,
    pub(super) material_layout: &'a wgpu::BindGroupLayout,
    pub(super) draw_layout: &'a wgpu::BindGroupLayout,
    pub(super) triangle_shader: &'a wgpu::ShaderModule,
    pub(super) reversed_z: bool,
    pub(super) attribution: GpuSemanticAttribution,
    #[cfg(target_arch = "wasm32")]
    pub(super) webgl2_surface_format: Option<wgpu::TextureFormat>,
}

pub(super) fn create_resources(
    device: &wgpu::Device,
    descriptor: SemanticAovResourceDescriptor<'_>,
) -> SemanticAovResources {
    let SemanticAovResourceDescriptor {
        target,
        beauty_target,
        beauty_sample_count,
        output_layout,
        material_layout,
        draw_layout,
        triangle_shader,
        reversed_z,
        attribution,
        #[cfg(target_arch = "wasm32")]
        webgl2_surface_format,
    } = descriptor;
    let unpadded_bytes_per_row = target.width.saturating_mul(BYTES_PER_PIXEL);
    let padded_bytes_per_row = align_to(unpadded_bytes_per_row, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let targets = std::array::from_fn(|slot| {
        let label = match slot {
            0 => "scena.semantic_aov.id",
            1 => "scena.semantic_aov.depth",
            _ => "scena.semantic_aov.normal",
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: extent(target),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: u64::from(padded_bytes_per_row) * u64::from(target.height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        SemanticTarget {
            texture,
            view,
            readback,
        }
    });
    let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scena.semantic_aov.depth_attachment"),
        size: extent(target),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let beauty = beauty::create(device, beauty_target, beauty_sample_count);
    let pipelines = SemanticPipelines {
        single_sided: create_pipeline(
            device,
            output_layout,
            material_layout,
            draw_layout,
            triangle_shader,
            reversed_z,
            false,
        ),
        double_sided: create_pipeline(
            device,
            output_layout,
            material_layout,
            draw_layout,
            triangle_shader,
            reversed_z,
            true,
        ),
    };
    #[cfg(target_arch = "wasm32")]
    let webgl2_readback = webgl2_surface_format.map(|surface_format| {
        webgl2::create_resources(
            device,
            surface_format,
            [
                &targets[0].view,
                &targets[1].view,
                &targets[2].view,
                &beauty.view,
            ],
        )
    });
    SemanticAovResources {
        target,
        targets,
        depth_texture,
        depth_view,
        reversed_z,
        pipelines,
        beauty,
        padded_bytes_per_row,
        unpadded_bytes_per_row,
        legend: attribution.legend,
        exclusions: attribution.exclusions,
        #[cfg(target_arch = "wasm32")]
        webgl2_readback,
    }
}

pub(super) fn resource_stats(resources: &SemanticAovResources) -> GpuResourceStats {
    let target_bytes = GpuResourceStats::target_bytes(resources.target, 4, 1);
    let readback_bytes = u64::from(resources.padded_bytes_per_row)
        .saturating_mul(u64::from(resources.target.height));
    #[cfg(target_arch = "wasm32")]
    let webgl2_readback = resources.webgl2_readback.is_some();
    #[cfg(not(target_arch = "wasm32"))]
    let webgl2_readback = false;
    let beauty_target_bytes = GpuResourceStats::target_bytes(resources.beauty.target, 4, 1);
    let beauty_msaa_bytes =
        GpuResourceStats::target_bytes(resources.beauty.target, 4, resources.beauty.sample_count)
            .saturating_mul(u64::from(resources.beauty.sample_count > 1));
    let beauty_readback_bytes = u64::from(resources.beauty.padded_bytes_per_row)
        .saturating_mul(u64::from(resources.beauty.target.height));
    GpuResourceStats {
        buffers: 4,
        textures: 5 + u64::from(resources.beauty.sample_count > 1),
        render_targets: 5 + u64::from(resources.beauty.sample_count > 1),
        pipelines: 2 + u64::from(webgl2_readback),
        bind_groups: u64::from(webgl2_readback) * 4,
        shader_modules: u64::from(webgl2_readback),
        shader_module_creations: u64::from(webgl2_readback),
        approximate_gpu_memory_bytes: target_bytes
            .saturating_mul(4)
            .saturating_add(readback_bytes.saturating_mul(3))
            .saturating_add(beauty_target_bytes)
            .saturating_add(beauty_msaa_bytes)
            .saturating_add(beauty_readback_bytes),
        ..GpuResourceStats::default()
    }
}

pub(super) fn beauty_attachment_views(
    resources: &SemanticAovResources,
    target: RasterTarget,
) -> Option<(&wgpu::TextureView, Option<&wgpu::TextureView>)> {
    beauty::attachment_views(resources, target)
}

pub(super) fn mark_beauty_witness_written(resources: &mut SemanticAovResources) {
    resources.beauty.valid = true;
}

pub(super) fn invalidate_beauty_witness(resources: &mut SemanticAovResources) {
    resources.beauty.valid = false;
}

pub(super) fn update_attribution(
    resources: &mut SemanticAovResources,
    attribution: GpuSemanticAttribution,
) {
    resources.beauty.valid = false;
    resources.legend = attribution.legend;
    resources.exclusions = attribution.exclusions;
}

fn create_pipeline(
    device: &wgpu::Device,
    output_layout: &wgpu::BindGroupLayout,
    material_layout: &wgpu::BindGroupLayout,
    draw_layout: &wgpu::BindGroupLayout,
    shader: &wgpu::ShaderModule,
    reversed_z: bool,
    double_sided: bool,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("scena.semantic_aov.pipeline_layout"),
        bind_group_layouts: &[
            Some(output_layout),
            Some(material_layout),
            Some(draw_layout),
        ],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(if double_sided {
            "scena.semantic_aov.pipeline.double_sided"
        } else {
            "scena.semantic_aov.pipeline.single_sided"
        }),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[
                wgpu::VertexBufferLayout {
                    array_stride: VERTEX_BYTE_LEN as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &VERTEX_ATTRIBUTES,
                },
                wgpu::VertexBufferLayout {
                    array_stride: INSTANCE_BYTE_LEN as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &INSTANCE_ATTRIBUTES,
                },
            ],
        },
        primitive: wgpu::PrimitiveState {
            front_face: SCENA_FRONT_FACE,
            cull_mode: (!double_sided).then_some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(if reversed_z {
                wgpu::CompareFunction::GreaterEqual
            } else {
                wgpu::CompareFunction::LessEqual
            }),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_semantic"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[color_target(), color_target(), color_target()],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn color_target() -> Option<wgpu::ColorTargetState> {
    Some(wgpu::ColorTargetState {
        format: FORMAT,
        blend: None,
        write_mask: wgpu::ColorWrites::ALL,
    })
}

fn encode_pass(
    encoder: &mut wgpu::CommandEncoder,
    resources: &GpuPreparedResources,
    semantic: &SemanticAovResources,
) {
    let attachments = semantic.targets.each_ref().map(|target| {
        Some(wgpu::RenderPassColorAttachment {
            view: &target.view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        })
    });
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("scena.semantic_aov.pass"),
        color_attachments: &attachments,
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: &semantic.depth_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(if semantic.reversed_z { 0.0 } else { 1.0 }),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_bind_group(0, &resources.output_bind_group, &[]);
    pass.set_vertex_buffer(0, resources.vertex_buffer.slice(..));
    for double_sided in [false, true] {
        pass.set_pipeline(if double_sided {
            &semantic.pipelines.double_sided
        } else {
            &semantic.pipelines.single_sided
        });
        encode_batches(&mut pass, resources, double_sided);
    }
}

fn encode_batches<'a>(
    pass: &mut wgpu::RenderPass<'a>,
    resources: &'a GpuPreparedResources,
    double_sided: bool,
) {
    let identity_offset = u64::from(resources.identity_instance) * INSTANCE_BYTE_LEN as u64;
    pass.set_vertex_buffer(1, resources.instance_buffer.slice(identity_offset..));
    match &resources.material_resources {
        MaterialResources::PerMaterial(slots) => {
            let Some(fallback) = slots.first() else {
                return;
            };
            for batch in resources
                .draw_batches
                .iter()
                .filter(|batch| batch.semantic_eligible && batch.double_sided == double_sided)
            {
                let material = slots.get(batch.material_slot as usize).unwrap_or(fallback);
                pass.set_bind_group(1, &material.bind_group, &[0]);
                bind_draw(pass, resources, batch.draw_uniform_index);
                pass.draw(
                    batch.start_vertex..batch.start_vertex + batch.vertex_count,
                    0..1,
                );
            }
            for batch in resources
                .instance_batches
                .iter()
                .filter(|batch| batch.semantic_eligible && batch.double_sided == double_sided)
            {
                let material = slots.get(batch.material_slot as usize).unwrap_or(fallback);
                pass.set_bind_group(1, &material.bind_group, &[0]);
                bind_draw(pass, resources, batch.draw_uniform_index);
                draw_instances(pass, resources, batch);
            }
        }
        MaterialResources::Batched(materials) => {
            for batch in resources
                .draw_batches
                .iter()
                .filter(|batch| batch.semantic_eligible && batch.double_sided == double_sided)
            {
                let layer = batch
                    .material_slot
                    .min(materials.layer_count.saturating_sub(1));
                pass.set_bind_group(
                    1,
                    &materials.bind_group,
                    &[(u64::from(layer) * MATERIAL_UNIFORM_ENTRY_STRIDE) as u32],
                );
                bind_draw(pass, resources, batch.draw_uniform_index);
                pass.draw(
                    batch.start_vertex..batch.start_vertex + batch.vertex_count,
                    0..1,
                );
            }
            for batch in resources
                .instance_batches
                .iter()
                .filter(|batch| batch.semantic_eligible && batch.double_sided == double_sided)
            {
                let layer = batch
                    .material_slot
                    .min(materials.layer_count.saturating_sub(1));
                pass.set_bind_group(
                    1,
                    &materials.bind_group,
                    &[(u64::from(layer) * MATERIAL_UNIFORM_ENTRY_STRIDE) as u32],
                );
                bind_draw(pass, resources, batch.draw_uniform_index);
                draw_instances(pass, resources, batch);
            }
        }
    }
}

fn bind_draw<'a>(pass: &mut wgpu::RenderPass<'a>, resources: &'a GpuPreparedResources, index: u32) {
    pass.set_bind_group(
        2,
        &resources.draw_bind_group,
        &[(u64::from(index) * DRAW_UNIFORM_ENTRY_STRIDE) as u32],
    );
}

fn draw_instances<'a>(
    pass: &mut wgpu::RenderPass<'a>,
    resources: &'a GpuPreparedResources,
    batch: &InstanceDrawBatch,
) {
    let offset = u64::from(batch.start_instance) * INSTANCE_BYTE_LEN as u64;
    pass.set_vertex_buffer(1, resources.instance_buffer.slice(offset..));
    pass.draw(
        batch.start_vertex..batch.start_vertex + batch.vertex_count,
        0..batch.instance_count,
    );
}

fn encode_copies(encoder: &mut wgpu::CommandEncoder, semantic: &SemanticAovResources) {
    for target in &semantic.targets {
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &target.readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(semantic.padded_bytes_per_row),
                    rows_per_image: None,
                },
            },
            extent(semantic.target),
        );
    }
}

fn encode_beauty_copy(encoder: &mut wgpu::CommandEncoder, semantic: &SemanticAovResources) {
    beauty::encode_copy(encoder, semantic);
}

fn extent(target: RasterTarget) -> wgpu::Extent3d {
    wgpu::Extent3d {
        width: target.width,
        height: target.height,
        depth_or_array_layers: 1,
    }
}

fn align_to(value: u32, alignment: u32) -> u32 {
    value.div_ceil(alignment) * alignment
}
