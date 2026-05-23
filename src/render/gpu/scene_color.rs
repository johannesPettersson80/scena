use super::materials::MaterialResources;
use super::pipeline::{ColorLoad, DrawFilter, UnlitPass, encode_unlit_pass};
use super::vertices::PrimitiveDrawBatch;

pub(super) struct SceneColorPasses<'a> {
    pub(super) final_view: &'a wgpu::TextureView,
    pub(super) final_pipeline: &'a wgpu::RenderPipeline,
    pub(super) depth_view: Option<&'a wgpu::TextureView>,
    pub(super) vertex_buffer: &'a wgpu::Buffer,
    pub(super) output_bind_group: &'a wgpu::BindGroup,
    pub(super) opaque_output_bind_group: &'a wgpu::BindGroup,
    pub(super) draw_bind_group: &'a wgpu::BindGroup,
    pub(super) material_resources: &'a MaterialResources,
    pub(super) draw_batches: &'a [PrimitiveDrawBatch],
    pub(super) transmission_view: &'a wgpu::TextureView,
    pub(super) transmission_pipeline: &'a wgpu::RenderPipeline,
    pub(super) clear_color: wgpu::Color,
    pub(super) base_label: &'static str,
}

pub(super) fn encode_scene_color_passes(
    encoder: &mut wgpu::CommandEncoder,
    passes: SceneColorPasses<'_>,
) {
    if has_transparent_batches(passes.draw_batches) {
        encode_unlit_pass(
            encoder,
            UnlitPass {
                view: passes.transmission_view,
                depth_view: None,
                vertex_buffer: passes.vertex_buffer,
                output_bind_group: passes.opaque_output_bind_group,
                draw_bind_group: passes.draw_bind_group,
                material_resources: passes.material_resources,
                draw_batches: passes.draw_batches,
                pipeline: passes.transmission_pipeline,
                color_load: ColorLoad::Clear(passes.clear_color),
                draw_filter: DrawFilter::OpaqueOnly,
                label: "scena.transmission.scene_color_pass",
            },
        );
        encode_unlit_pass(
            encoder,
            UnlitPass {
                view: passes.final_view,
                depth_view: passes.depth_view,
                vertex_buffer: passes.vertex_buffer,
                output_bind_group: passes.output_bind_group,
                draw_bind_group: passes.draw_bind_group,
                material_resources: passes.material_resources,
                draw_batches: passes.draw_batches,
                pipeline: passes.final_pipeline,
                color_load: ColorLoad::Clear(passes.clear_color),
                draw_filter: DrawFilter::OpaqueOnly,
                label: "scena.final.opaque_pass",
            },
        );
        encode_unlit_pass(
            encoder,
            UnlitPass {
                view: passes.final_view,
                depth_view: passes.depth_view,
                vertex_buffer: passes.vertex_buffer,
                output_bind_group: passes.output_bind_group,
                draw_bind_group: passes.draw_bind_group,
                material_resources: passes.material_resources,
                draw_batches: passes.draw_batches,
                pipeline: passes.final_pipeline,
                color_load: ColorLoad::Load,
                draw_filter: DrawFilter::TransparentOnly,
                label: "scena.final.transparent_pass",
            },
        );
    } else {
        encode_unlit_pass(
            encoder,
            UnlitPass {
                view: passes.final_view,
                depth_view: passes.depth_view,
                vertex_buffer: passes.vertex_buffer,
                output_bind_group: passes.output_bind_group,
                draw_bind_group: passes.draw_bind_group,
                material_resources: passes.material_resources,
                draw_batches: passes.draw_batches,
                pipeline: passes.final_pipeline,
                color_load: ColorLoad::Clear(passes.clear_color),
                draw_filter: DrawFilter::All,
                label: passes.base_label,
            },
        );
    }
}

fn has_transparent_batches(draw_batches: &[PrimitiveDrawBatch]) -> bool {
    draw_batches
        .iter()
        .any(|batch| !batch.depth_prepass_eligible)
}
