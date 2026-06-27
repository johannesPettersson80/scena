use super::instancing::InstanceDrawBatch;
use super::materials::MaterialResources;
use super::pipeline::{ColorLoad, DrawFilter, UnlitPass, UnlitPipelines, encode_unlit_pass};
use super::vertices::PrimitiveDrawBatch;

pub(super) struct SceneColorPasses<'a> {
    pub(super) final_view: &'a wgpu::TextureView,
    pub(super) final_resolve_target: Option<&'a wgpu::TextureView>,
    pub(super) final_pipelines: UnlitPipelines<'a>,
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
    pub(super) transmission_view: &'a wgpu::TextureView,
    pub(super) transmission_pipelines: UnlitPipelines<'a>,
    pub(super) force_scene_color_pass: bool,
    pub(super) clear_color: wgpu::Color,
    pub(super) base_label: &'static str,
    pub(super) draw_submissions: &'a mut u64,
}

pub(super) fn encode_scene_color_passes(
    encoder: &mut wgpu::CommandEncoder,
    passes: SceneColorPasses<'_>,
) {
    let draw_submissions = passes.draw_submissions;
    if passes.force_scene_color_pass
        || has_transparent_batches(passes.draw_batches, passes.instance_batches)
    {
        encode_unlit_pass(
            encoder,
            UnlitPass {
                view: passes.transmission_view,
                resolve_target: None,
                depth_view: None,
                vertex_buffer: passes.vertex_buffer,
                instance_buffer: passes.instance_buffer,
                output_bind_group: passes.opaque_output_bind_group,
                draw_bind_group: passes.draw_bind_group,
                material_resources: passes.material_resources,
                draw_batches: passes.draw_batches,
                instance_batches: passes.instance_batches,
                identity_instance: passes.identity_instance,
                pipelines: passes.transmission_pipelines,
                color_load: ColorLoad::Clear(passes.clear_color),
                draw_filter: DrawFilter::OpaqueOnly,
                label: "scena.transmission.scene_color_pass",
                draw_submissions: &mut *draw_submissions,
            },
        );
        encode_unlit_pass(
            encoder,
            UnlitPass {
                view: passes.final_view,
                resolve_target: passes.final_resolve_target,
                depth_view: passes.depth_view,
                vertex_buffer: passes.vertex_buffer,
                instance_buffer: passes.instance_buffer,
                output_bind_group: passes.output_bind_group,
                draw_bind_group: passes.draw_bind_group,
                material_resources: passes.material_resources,
                draw_batches: passes.draw_batches,
                instance_batches: passes.instance_batches,
                identity_instance: passes.identity_instance,
                pipelines: passes.final_pipelines,
                color_load: ColorLoad::Clear(passes.clear_color),
                draw_filter: DrawFilter::OpaqueOnly,
                label: "scena.final.opaque_pass",
                draw_submissions: &mut *draw_submissions,
            },
        );
        encode_unlit_pass(
            encoder,
            UnlitPass {
                view: passes.final_view,
                resolve_target: passes.final_resolve_target,
                depth_view: passes.depth_view,
                vertex_buffer: passes.vertex_buffer,
                instance_buffer: passes.instance_buffer,
                output_bind_group: passes.output_bind_group,
                draw_bind_group: passes.draw_bind_group,
                material_resources: passes.material_resources,
                draw_batches: passes.draw_batches,
                instance_batches: passes.instance_batches,
                identity_instance: passes.identity_instance,
                pipelines: passes.final_pipelines,
                color_load: ColorLoad::Load,
                draw_filter: DrawFilter::TransparentOnly,
                label: "scena.final.transparent_pass",
                draw_submissions: &mut *draw_submissions,
            },
        );
    } else {
        encode_unlit_pass(
            encoder,
            UnlitPass {
                view: passes.final_view,
                resolve_target: passes.final_resolve_target,
                depth_view: passes.depth_view,
                vertex_buffer: passes.vertex_buffer,
                instance_buffer: passes.instance_buffer,
                output_bind_group: passes.output_bind_group,
                draw_bind_group: passes.draw_bind_group,
                material_resources: passes.material_resources,
                draw_batches: passes.draw_batches,
                instance_batches: passes.instance_batches,
                identity_instance: passes.identity_instance,
                pipelines: passes.final_pipelines,
                color_load: ColorLoad::Clear(passes.clear_color),
                draw_filter: DrawFilter::All,
                label: passes.base_label,
                draw_submissions: &mut *draw_submissions,
            },
        );
    }
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
