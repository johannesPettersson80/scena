#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

use super::super::RasterTarget;
use super::instancing::InstanceDrawBatch;
use super::material_bindings::MaterialTextureBindingMode;
use super::materials::MaterialResources;
use super::pipeline::{BYTES_PER_PIXEL, MeshPipelineSet, create_unlit_pipeline_set};
use super::pipeline::{ColorLoad, DrawFilter, UnlitPass, encode_unlit_pass};
use super::transmission::TransmissionResources;
use super::vertices::PrimitiveDrawBatch;

#[derive(Debug)]
pub(super) struct BrowserReadbackResources {
    pub(super) texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    pub(super) buffer: wgpu::Buffer,
    pub(super) pipelines: MeshPipelineSet,
    pub(super) padded_bytes_per_row: u32,
    #[allow(dead_code)]
    pub(super) unpadded_bytes_per_row: u32,
}

pub(super) fn create_browser_readback_resources(
    device: &wgpu::Device,
    target: RasterTarget,
    output_bind_group_layout: &wgpu::BindGroupLayout,
    material_bind_group_layout: &wgpu::BindGroupLayout,
    draw_bind_group_layout: &wgpu::BindGroupLayout,
    texture_binding_mode: MaterialTextureBindingMode,
    depth_compare: Option<wgpu::CompareFunction>,
) -> BrowserReadbackResources {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scena.browser.proof_readback_target"),
        size: wgpu::Extent3d {
            width: target.width,
            height: target.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let unpadded_bytes_per_row = target.width.saturating_mul(BYTES_PER_PIXEL);
    let padded_bytes_per_row = align_to(unpadded_bytes_per_row, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scena.browser.proof_readback_buffer"),
        size: u64::from(padded_bytes_per_row) * u64::from(target.height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let pipelines = create_unlit_pipeline_set(
        device,
        wgpu::TextureFormat::Rgba8Unorm,
        output_bind_group_layout,
        material_bind_group_layout,
        draw_bind_group_layout,
        texture_binding_mode,
        depth_compare,
    );
    BrowserReadbackResources {
        texture,
        view,
        buffer,
        pipelines,
        padded_bytes_per_row,
        unpadded_bytes_per_row,
    }
}

pub(super) struct BrowserReadbackPass<'a> {
    pub(super) target: RasterTarget,
    pub(super) readback: &'a BrowserReadbackResources,
    pub(super) depth_view: Option<&'a wgpu::TextureView>,
    pub(super) vertex_buffer: &'a wgpu::Buffer,
    pub(super) instance_buffer: &'a wgpu::Buffer,
    pub(super) output_bind_group: &'a wgpu::BindGroup,
    pub(super) opaque_output_bind_group: &'a wgpu::BindGroup,
    pub(super) draw_bind_group: &'a wgpu::BindGroup,
    pub(super) material_resources: &'a MaterialResources,
    pub(super) draw_batches: &'a [PrimitiveDrawBatch],
    pub(super) instance_batches: &'a [InstanceDrawBatch],
    pub(super) identity_instance: u32,
    pub(super) transmission: &'a TransmissionResources,
    pub(super) clear_color: wgpu::Color,
    pub(super) draw_submissions: &'a mut u64,
}

pub(super) fn encode_browser_readback_pass(
    encoder: &mut wgpu::CommandEncoder,
    pass: BrowserReadbackPass<'_>,
) {
    let draw_submissions = pass.draw_submissions;
    if has_transparent_batches(pass.draw_batches, pass.instance_batches) {
        encode_unlit_pass(
            encoder,
            UnlitPass {
                view: &pass.transmission.view,
                depth_view: None,
                vertex_buffer: pass.vertex_buffer,
                instance_buffer: pass.instance_buffer,
                output_bind_group: pass.opaque_output_bind_group,
                draw_bind_group: pass.draw_bind_group,
                material_resources: pass.material_resources,
                draw_batches: pass.draw_batches,
                instance_batches: pass.instance_batches,
                identity_instance: pass.identity_instance,
                pipelines: pass.transmission.pipelines.refs(),
                color_load: ColorLoad::Clear(pass.clear_color),
                draw_filter: DrawFilter::OpaqueOnly,
                label: "scena.browser.proof_transmission_scene_color_pass",
                draw_submissions: &mut *draw_submissions,
            },
        );
        encode_unlit_pass(
            encoder,
            UnlitPass {
                view: &pass.readback.view,
                depth_view: pass.depth_view,
                vertex_buffer: pass.vertex_buffer,
                instance_buffer: pass.instance_buffer,
                output_bind_group: pass.output_bind_group,
                draw_bind_group: pass.draw_bind_group,
                material_resources: pass.material_resources,
                draw_batches: pass.draw_batches,
                instance_batches: pass.instance_batches,
                identity_instance: pass.identity_instance,
                pipelines: pass.readback.pipelines.refs(),
                color_load: ColorLoad::Clear(pass.clear_color),
                draw_filter: DrawFilter::OpaqueOnly,
                label: "scena.browser.proof_readback_opaque_pass",
                draw_submissions: &mut *draw_submissions,
            },
        );
        encode_unlit_pass(
            encoder,
            UnlitPass {
                view: &pass.readback.view,
                depth_view: pass.depth_view,
                vertex_buffer: pass.vertex_buffer,
                instance_buffer: pass.instance_buffer,
                output_bind_group: pass.output_bind_group,
                draw_bind_group: pass.draw_bind_group,
                material_resources: pass.material_resources,
                draw_batches: pass.draw_batches,
                instance_batches: pass.instance_batches,
                identity_instance: pass.identity_instance,
                pipelines: pass.readback.pipelines.refs(),
                color_load: ColorLoad::Load,
                draw_filter: DrawFilter::TransparentOnly,
                label: "scena.browser.proof_readback_transparent_pass",
                draw_submissions: &mut *draw_submissions,
            },
        );
    } else {
        encode_unlit_pass(
            encoder,
            UnlitPass {
                view: &pass.readback.view,
                depth_view: pass.depth_view,
                vertex_buffer: pass.vertex_buffer,
                instance_buffer: pass.instance_buffer,
                output_bind_group: pass.output_bind_group,
                draw_bind_group: pass.draw_bind_group,
                material_resources: pass.material_resources,
                draw_batches: pass.draw_batches,
                instance_batches: pass.instance_batches,
                identity_instance: pass.identity_instance,
                pipelines: pass.readback.pipelines.refs(),
                color_load: ColorLoad::Clear(pass.clear_color),
                draw_filter: DrawFilter::All,
                label: "scena.browser.proof_readback_pass",
                draw_submissions: &mut *draw_submissions,
            },
        );
    }
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &pass.readback.texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &pass.readback.buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(pass.readback.padded_bytes_per_row),
                rows_per_image: None,
            },
        },
        wgpu::Extent3d {
            width: pass.target.width,
            height: pass.target.height,
            depth_or_array_layers: 1,
        },
    );
}

fn align_to(value: u32, alignment: u32) -> u32 {
    value.div_ceil(alignment) * alignment
}

fn has_transparent_batches(
    draw_batches: &[PrimitiveDrawBatch],
    instance_batches: &[InstanceDrawBatch],
) -> bool {
    draw_batches
        .iter()
        .any(|batch| !batch.depth_prepass_eligible)
        || instance_batches
            .iter()
            .any(|batch| !batch.depth_prepass_eligible)
}
